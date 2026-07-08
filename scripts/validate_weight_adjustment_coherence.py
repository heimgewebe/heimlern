#!/usr/bin/env python3
"""Validate Rust/Python weight-adjustment coherence against one v1 contract.

The metarepo owns ``policy.weight_adjustment.v1.schema.json``. Heimlern has two
producer-adjacent surfaces:

* the Rust feedback fixture/types, which must continue to deserialize v1
  proposals; and
* the Python OPLEARN probe, which may emit candidate routing adjustments.

This script is a drift guard. It validates both surfaces against the same schema
file supplied by CI and asserts that the Python probe remains a probe/boundary
surface rather than becoming a second contract owner.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

_ALLOWED_DELTA_KEY_CHARS = set("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-")
_EXPECTED_PROBE_DELTA_KEYS = {"route.direct_patch.weight"}


class CoherenceError(AssertionError):
    """Raised when the Rust/Python proposal surfaces drift apart."""


def _resolve(repo_root: Path, value: Path) -> Path:
    return value if value.is_absolute() else repo_root / value


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _run(argv: list[str], *, cwd: Path, capture: bool = False) -> str:
    result = subprocess.run(
        argv,
        cwd=cwd,
        check=False,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.PIPE if capture else None,
    )
    if result.returncode != 0:
        detail = ""
        if capture:
            detail = f"\nstdout:\n{result.stdout}\nstderr:\n{result.stderr}"
        raise CoherenceError(f"command failed ({result.returncode}): {' '.join(argv)}{detail}")
    return result.stdout if capture and result.stdout is not None else ""


def _validate_against_schema(repo_root: Path, schema_path: Path, document_path: Path) -> None:
    _run(
        [
            sys.executable,
            "scripts/validate_json.py",
            str(schema_path),
            str(document_path),
        ],
        cwd=repo_root,
    )


def _assert_common_v1_shape(payload: dict[str, Any], *, label: str) -> None:
    if payload.get("version") != "v1":
        raise CoherenceError(f"{label}: expected version 'v1'")
    if payload.get("status") != "proposed":
        raise CoherenceError(f"{label}: expected status 'proposed'")
    if not isinstance(payload.get("basis_policy"), str) or not payload["basis_policy"]:
        raise CoherenceError(f"{label}: basis_policy must be a non-empty string")
    confidence = payload.get("confidence")
    if not isinstance(confidence, (int, float)) or not 0.0 <= float(confidence) <= 1.0:
        raise CoherenceError(f"{label}: confidence must be numeric in [0, 1]")
    deltas = payload.get("deltas")
    if not isinstance(deltas, dict) or not deltas:
        raise CoherenceError(f"{label}: deltas must be a non-empty object")
    evidence = payload.get("evidence")
    if not isinstance(evidence, dict):
        raise CoherenceError(f"{label}: evidence must be an object")
    if not isinstance(evidence.get("decisions_analyzed"), int) or evidence["decisions_analyzed"] < 1:
        raise CoherenceError(f"{label}: evidence.decisions_analyzed must be a positive integer")
    if not isinstance(payload.get("reasoning"), str) or not payload["reasoning"].strip():
        raise CoherenceError(f"{label}: reasoning must be a non-empty string")


def _assert_python_probe_boundary(proposal: dict[str, Any]) -> None:
    deltas = proposal.get("deltas")
    if not isinstance(deltas, dict):
        raise CoherenceError("Python probe: deltas must be an object")
    if set(deltas) != _EXPECTED_PROBE_DELTA_KEYS:
        raise CoherenceError(
            "Python probe: expected only routing delta keys "
            f"{sorted(_EXPECTED_PROBE_DELTA_KEYS)}, got {sorted(deltas)}"
        )
    for key, delta in deltas.items():
        if set(key) - _ALLOWED_DELTA_KEY_CHARS:
            raise CoherenceError(f"Python probe: unsafe delta key {key!r}")
        if not isinstance(delta, dict):
            raise CoherenceError(f"Python probe: delta {key!r} must be an object")
        if delta.get("kind") != "relative" or delta.get("unit") != "percent":
            raise CoherenceError(f"Python probe: delta {key!r} must stay a relative percent proposal")
    evidence = proposal.get("evidence") or {}
    if evidence.get("simulation_method") != "heuristic_failure_rate_probe_no_replay":
        raise CoherenceError("Python probe: unexpected simulation method")
    if evidence.get("decisions_analyzed") != 10:
        raise CoherenceError("Python probe: amplified fixture should analyze exactly 10 decisions")


def _write_amplified_failed_input(failed_input: Path, output_path: Path) -> None:
    failed = _load_json(failed_input)
    if not isinstance(failed, dict):
        raise CoherenceError("failed-input fixture must be a JSON object")
    amplified = []
    for index in range(10):
        item = dict(failed)
        item["decision_id"] = f"gr-failed-{index:03d}"
        item["route_used"] = "direct:patch"
        amplified.append(item)
    output_path.write_text(json.dumps(amplified), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--schema", required=True, type=Path, help="policy.weight_adjustment.v1 schema path")
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument(
        "--rust-fixture",
        type=Path,
        default=Path("tests/fixtures/feedback/adjustment.ok.json"),
        help="Checked-in fixture used by the Rust feedback tests",
    )
    parser.add_argument(
        "--failed-input",
        type=Path,
        default=Path("tests/fixtures/ola/failed.ok.json"),
        help="OLA fixture amplified to force a deterministic Python proposal",
    )
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    schema_path = _resolve(repo_root, args.schema).resolve()
    rust_fixture = _resolve(repo_root, args.rust_fixture).resolve()
    failed_input = _resolve(repo_root, args.failed_input).resolve()

    _validate_against_schema(repo_root, schema_path, rust_fixture)
    rust_payload = _load_json(rust_fixture)
    if not isinstance(rust_payload, dict):
        raise CoherenceError("Rust fixture must be a JSON object")
    _assert_common_v1_shape(rust_payload, label="Rust fixture")

    with tempfile.TemporaryDirectory(prefix="heimlern-weight-adjustment-") as temp_dir_raw:
        temp_dir = Path(temp_dir_raw)
        amplified = temp_dir / "ola-amplified.json"
        proposal_path = temp_dir / "ola-proposal.json"
        _write_amplified_failed_input(failed_input, amplified)
        proposal_stdout = _run(
            [
                sys.executable,
                "scripts/ola_probe.py",
                "--min-decisions",
                "10",
                "--emit",
                "proposal",
                str(amplified),
            ],
            cwd=repo_root,
            capture=True,
        )
        proposal_path.write_text(proposal_stdout, encoding="utf-8")
        _validate_against_schema(repo_root, schema_path, proposal_path)
        proposal = _load_json(proposal_path)
        if not isinstance(proposal, dict):
            raise CoherenceError("Python probe proposal must be a JSON object")
        _assert_common_v1_shape(proposal, label="Python probe proposal")
        _assert_python_probe_boundary(proposal)

    print(f"✓ weight adjustment coherence validated against {schema_path}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except CoherenceError as exc:
        print(f"❌ Weight adjustment coherence failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
