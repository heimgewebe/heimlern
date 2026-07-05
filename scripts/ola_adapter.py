#!/usr/bin/env python3
"""Convert redacted Grabowski friction summaries into routing outcome records."""
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

OUTCOME_VERSION = "operator.routing_outcome.v1"
VALID_COMPLETION_STATES = {"completed", "blocked", "deferred", "failed", "unknown"}
VALID_CI_STATES = {"pass", "fail", "pending", "not_applicable", "unknown"}
VALID_PR_STATES = {"merged", "open", "closed", "not_applicable", "unknown"}


def iso_now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def clamp_reward(value: float) -> float:
    return max(-1.0, min(1.0, round(value, 3)))


def normalized_state(value: Any, allowed: set[str], fallback: str = "unknown") -> str:
    if isinstance(value, str) and value in allowed:
        return value
    return fallback


def bool_from(value: Any) -> bool:
    return bool(value) if isinstance(value, bool) else False


def normalized_friction(raw_items: Any) -> list[dict[str, Any]]:
    if not isinstance(raw_items, list):
        return []
    items: list[dict[str, Any]] = []
    for raw in raw_items:
        if not isinstance(raw, dict):
            continue
        item = {
            "kind": str(raw.get("kind") or "unknown"),
            "surface": str(raw.get("surface") or "unknown"),
            "resolved": bool_from(raw.get("resolved")),
        }
        operation = raw.get("operation")
        if isinstance(operation, str) and operation:
            item["operation"] = operation[:160]
        fallback = raw.get("fallback")
        if isinstance(fallback, str) and fallback:
            item["fallback"] = fallback[:500]
        items.append(item)
    return items


def classify_outcome(completion_state: str, unresolved_friction: int) -> str:
    if completion_state == "completed":
        return "success"
    if completion_state == "failed":
        return "failure"
    if completion_state in {"blocked", "deferred"}:
        return "partial"
    if unresolved_friction > 0:
        return "partial"
    return "unknown"


def compute_reward(
    completion_state: str,
    friction_count: int,
    unresolved_friction: int,
    blocked_by_platform_filter: bool,
    manual_operator_needed: bool,
    ci_state: str,
    pr_state: str,
    rework_count: int,
) -> float:
    reward = {
        "completed": 0.7,
        "blocked": -0.35,
        "deferred": -0.1,
        "failed": -0.75,
        "unknown": 0.0,
    }.get(completion_state, 0.0)
    reward -= min(friction_count, 5) * 0.05
    reward -= min(unresolved_friction, 5) * 0.1
    if blocked_by_platform_filter:
        reward -= 0.1
    if manual_operator_needed:
        reward -= 0.15
    if ci_state == "pass":
        reward += 0.1
    elif ci_state == "fail":
        reward -= 0.2
    if pr_state == "merged":
        reward += 0.1
    elif pr_state == "closed":
        reward -= 0.1
    reward -= min(max(rework_count, 0), 5) * 0.05
    return clamp_reward(reward)


def adapt(input_record: dict[str, Any]) -> dict[str, Any]:
    friction = normalized_friction(input_record.get("friction"))
    friction_count = len(friction)
    unresolved_friction = sum(1 for item in friction if not item.get("resolved"))
    completion_state = normalized_state(
        input_record.get("completion_state"), VALID_COMPLETION_STATES
    )
    ci_state = normalized_state(input_record.get("ci_state"), VALID_CI_STATES, "unknown")
    pr_state = normalized_state(input_record.get("pr_state"), VALID_PR_STATES, "unknown")
    blocked_by_platform_filter = any(
        item.get("kind") == "platform_filter" for item in friction
    )
    manual_operator_needed = bool_from(input_record.get("manual_operator_needed")) or any(
        item.get("kind") == "user_input" for item in friction
    )
    rework_count = int(input_record.get("rework_count") or 0)
    metrics = {
        "completion_state": completion_state,
        "friction_count": friction_count,
        "blocked_by_platform_filter": blocked_by_platform_filter,
        "manual_operator_needed": manual_operator_needed,
        "ci_state": ci_state,
        "pr_state": pr_state,
        "rework_count": rework_count,
    }
    elapsed = input_record.get("elapsed_seconds")
    if isinstance(elapsed, int) and elapsed >= 0:
        metrics["elapsed_seconds"] = elapsed
    outcome = classify_outcome(completion_state, unresolved_friction)
    return {
        "version": OUTCOME_VERSION,
        "decision_id": str(input_record.get("decision_id") or "unknown-decision"),
        "ts": str(input_record.get("ts") or iso_now()),
        "task_class": str(input_record.get("task_class") or "unknown_task"),
        "route_used": str(input_record.get("route_used") or "unknown_route"),
        "outcome": outcome,
        "resolved": completion_state == "completed" and unresolved_friction == 0,
        "reward": compute_reward(
            completion_state,
            friction_count,
            unresolved_friction,
            blocked_by_platform_filter,
            manual_operator_needed,
            ci_state,
            pr_state,
            rework_count,
        ),
        "metrics": metrics,
        "friction": friction,
        "does_not_establish": [
            "causal_route_superiority",
            "routing_policy_readiness",
            "auto_apply_permission",
        ],
    }


def run_self_test() -> None:
    sample = {
        "decision_id": "gr-example-001",
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


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", nargs="?", help="Path to a redacted JSON input record")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        run_self_test()
        print("ola-adapter self-test ok")
        return 0
    if not args.input:
        parser.error("input path required unless --self-test is used")
    input_record = json.loads(Path(args.input).read_text(encoding="utf-8"))
    print(json.dumps(adapt(input_record), indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
