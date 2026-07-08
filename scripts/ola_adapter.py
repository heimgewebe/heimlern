#!/usr/bin/env python3
"""Python wrapper for Rust-owned OLA routing outcome normalization.

The transformation contract lives in `heimlern_core::ola` and the
`heimlern-ola` CLI. This script remains as a compatibility wrapper for probes
and fixtures.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

OUTCOME_VERSION = "operator.routing_outcome.v1"
DEFAULT_POLICY_ID = "grabowski-routing-v0"
_ROUTE_DELTA_KEY_INVALID = "route_delta_key_invalid"
_ROUTE_DELTA_KEY_COLLISION = "route_delta_key_collision"
_ROUTE_DELTA_KEY_ERROR_KINDS = frozenset({_ROUTE_DELTA_KEY_INVALID, _ROUTE_DELTA_KEY_COLLISION})


class RouteDeltaKeyError(ValueError):
    """Raised when Rust rejects a routing action to delta-key mapping."""

    def __init__(self, kind: str, message: str) -> None:
        if kind not in _ROUTE_DELTA_KEY_ERROR_KINDS:
            raise ValueError(f"unknown route delta key error kind: {kind!r}")
        super().__init__(message)
        self.kind = kind


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def iso_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _json_from_text(text: str) -> dict[str, Any]:
    start = text.find("{")
    if start == -1:
        raise ValueError(f"no JSON object in command output: {text!r}")
    return json.loads(text[start:])


def _run_ola(args: list[str]) -> dict[str, Any]:
    cmd = ["cargo", "run", "-q", "-p", "heimlern-cli", "--bin", "heimlern-ola", "--", *args]
    result = subprocess.run(
        cmd,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or f"heimlern-ola exited {result.returncode}")
    return _json_from_text(result.stdout)


def _with_temp_json(payload: dict[str, Any], args: list[str]) -> dict[str, Any]:
    with tempfile.NamedTemporaryFile("w", suffix=".json", encoding="utf-8", delete=True) as tmp:
        json.dump(payload, tmp, ensure_ascii=False)
        tmp.flush()
        return _run_ola([*args, "--input", tmp.name])


def adapt(input_record: dict[str, Any]) -> dict[str, Any]:
    return _with_temp_json(input_record, ["adapt"])


def to_decision_outcome(routing_outcome: dict[str, Any], policy_id: str = DEFAULT_POLICY_ID) -> dict[str, Any]:
    return _with_temp_json(routing_outcome, ["decision-outcome", "--policy-id", policy_id])


def route_delta_key(action: str) -> tuple[str, str]:
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "heimlern-cli",
        "--bin",
        "heimlern-ola",
        "--",
        "route-delta-key",
        action,
    ]
    result = subprocess.run(
        cmd,
        cwd=repo_root(),
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        try:
            payload = _json_from_text(result.stderr)
            kind = str(payload.get("kind") or _ROUTE_DELTA_KEY_INVALID)
            message = str(payload.get("message") or result.stderr.strip())
        except Exception:
            kind = _ROUTE_DELTA_KEY_INVALID
            message = result.stderr.strip() or result.stdout.strip() or f"heimlern-ola exited {result.returncode}"
        raise RouteDeltaKeyError(kind, message)
    payload = _json_from_text(result.stdout)
    return str(payload["delta_key"]), str(payload["route"])


def run_self_test() -> None:
    subprocess.run(
        ["cargo", "test", "-q", "-p", "heimlern-core", "ola::"],
        cwd=repo_root(),
        check=True,
    )
    sample = {
        "decision_id": "gr-example-001",
        "ts": "2026-07-08T18:00:00Z",
        "task_class": "contract_slice",
        "route_used": "typed_tool",
        "completion_state": "blocked",
        "ci_state": "unknown",
        "pr_state": "open",
        "friction": [
            {
                "kind": "platform_filter",
                "surface": "chat_tool",
                "operation": "bounded_write",
                "resolved": False,
                "fallback": "narrowed scope and stopped before mutation",
            }
        ],
    }
    outcome = adapt(sample)
    assert outcome["outcome"] == "partial"
    assert outcome["metrics"]["friction_count"] == 1
    assert outcome["metrics"]["blocked_by_platform_filter"] is True
    assert outcome["reward"] < 0.0
    assert route_delta_key("route.direct:patch") == ("route.direct_patch.weight", "direct:patch")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", nargs="?", help="Path to a redacted JSON input record")
    parser.add_argument("--emit", choices=["routing-outcome", "decision-outcome"], default="routing-outcome")
    parser.add_argument("--policy-id", default=DEFAULT_POLICY_ID)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        run_self_test()
        print("ola-adapter wrapper self-test ok")
        return 0
    if not args.input:
        parser.error("input path required unless --self-test is used")
    if args.emit == "decision-outcome":
        payload = _run_ola(["adapt", "--emit", "decision-outcome", "--policy-id", args.policy_id, "--input", args.input])
    else:
        payload = _run_ola(["adapt", "--emit", "routing-outcome", "--input", args.input])
    print(json.dumps(payload, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
