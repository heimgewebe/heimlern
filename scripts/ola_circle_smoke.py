#!/usr/bin/env python3
"""Review-only OLA learning circle smoke.

This script exercises the bounded offline path:

routing fixtures -> Rust-owned OLA normalization -> offline proposal probe ->
policy.weight_adjustment.v1 validation.

It intentionally does not apply proposals or mutate routing policy.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

from ola_probe import load_inputs, probe

_DOES_NOT_ESTABLISH = [
    "routing_policy_readiness",
    "automatic_rule_change_permission",
    "runtime_policy_mutation",
    "causal_route_superiority",
]


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _validate_proposal(repo_root: Path, schema_path: Path, proposal: dict[str, Any]) -> None:
    with tempfile.NamedTemporaryFile("w", suffix=".json", encoding="utf-8", delete=True) as temp:
        json.dump(proposal, temp, ensure_ascii=False)
        temp.flush()
        subprocess.run(
            [sys.executable, "scripts/validate_json.py", str(schema_path), temp.name],
            cwd=repo_root,
            check=True,
        )


def _single_case(path: Path, expected_status: str) -> dict[str, Any]:
    report = probe(load_inputs([path]), min_decisions=10)
    if report["status"] != expected_status:
        raise AssertionError(f"{path.name}: expected {expected_status}, got {report['status']}")
    return {
        "fixture": str(path),
        "status": report["status"],
        "proposal": report["proposal"],
        "summary": report["summary"],
    }


def _proposal_case(path: Path, schema_path: Path, repo_root: Path) -> dict[str, Any]:
    failed = load_inputs([path])[0]
    amplified = [failed | {"decision_id": f"gr-circle-failed-{index:03d}", "route_used": "direct:patch"} for index in range(10)]
    report = probe(amplified, min_decisions=10)
    if report["status"] != "proposal_candidate" or not isinstance(report.get("proposal"), dict):
        raise AssertionError(f"{path.name}: expected proposal_candidate")
    proposal = report["proposal"]
    _validate_proposal(repo_root, schema_path, proposal)
    return {
        "fixture": str(path),
        "status": report["status"],
        "proposal_keys": sorted(proposal["deltas"]),
        "validated_schema": str(schema_path),
        "summary": report["summary"],
    }


def _fail_closed_case(path: Path) -> dict[str, Any]:
    report = probe(load_inputs([path]), min_decisions=10)
    if report["status"] != "proposal_blocked":
        raise AssertionError(f"{path.name}: expected proposal_blocked, got {report['status']}")
    reason = report.get("blocked_reason") or {}
    if reason.get("kind") != "route_delta_key_collision":
        raise AssertionError(f"{path.name}: expected route_delta_key_collision, got {reason}")
    return {
        "fixture": str(path),
        "status": report["status"],
        "blocked_reason": reason,
        "proposal": report["proposal"],
        "summary": report["summary"],
    }


def run(repo_root: Path, schema_path: Path) -> dict[str, Any]:
    fixture_root = repo_root / "tests" / "fixtures" / "ola"
    cases = {
        "success": _single_case(fixture_root / "success.ok.json", "insufficient_evidence"),
        "blocked": _single_case(fixture_root / "blocked.ok.json", "insufficient_evidence"),
        "failed_single": _single_case(fixture_root / "failed.ok.json", "insufficient_evidence"),
        "failed_proposal": _proposal_case(fixture_root / "failed.ok.json", schema_path, repo_root),
        "fail_closed_collision": _fail_closed_case(fixture_root / "fail_closed_collision.corpus.json"),
    }
    return {
        "schema_version": 1,
        "kind": "ola_review_only_circle_smoke",
        "status": "pass",
        "cases": cases,
        "review_only": True,
        "writes": [],
        "does_not_establish": _DOES_NOT_ESTABLISH,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo-root", type=Path, default=Path("."))
    parser.add_argument("--schema", required=True, type=Path)
    args = parser.parse_args()
    repo_root = args.repo_root.resolve()
    schema_path = args.schema if args.schema.is_absolute() else repo_root / args.schema
    report = run(repo_root, schema_path.resolve())
    print(json.dumps(report, indent=2, ensure_ascii=False, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
