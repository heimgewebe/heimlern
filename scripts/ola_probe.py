#!/usr/bin/env python3
"""Probe OPLEARN routing outcomes before any routing proposal is allowed."""
from __future__ import annotations

import argparse
import json
from collections import defaultdict
from pathlib import Path
from typing import Any

from ola_adapter import DEFAULT_POLICY_ID, adapt, iso_now, to_decision_outcome

DEFAULT_MIN_DECISIONS = 10
DEFAULT_MIN_ACTION_COUNT = 3
DEFAULT_FAILURE_THRESHOLD = 0.6
_ALLOWED_DELTA_KEY_CHARS = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._-")
_DOES_NOT_ESTABLISH = (
    "routing_policy_readiness",
    "automatic_rule_change_permission",
    "sample_representativeness",
)


class RouteDeltaKeyError(ValueError):
    """Raised when a routing action cannot be mapped to a safe delta key."""

    def __init__(self, kind: str, message: str) -> None:
        super().__init__(message)
        self.kind = kind


def ratio(num: int, den: int) -> float:
    if den == 0:
        return 0.0
    return round(num / den, 4)


def average(values: list[float]) -> float:
    if not values:
        return 0.0
    return round(sum(values) / len(values), 4)


def load_inputs(paths: list[Path]) -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in paths:
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data, list):
            records.extend(item for item in data if isinstance(item, dict))
        elif isinstance(data, dict):
            records.append(data)
    return records


def summarize(decision_outcomes: list[dict[str, Any]]) -> dict[str, Any]:
    by_action: dict[str, dict[str, Any]] = defaultdict(lambda: {"total": 0, "successes": 0, "failures": 0, "rewards": []})
    by_task_class: dict[str, dict[str, Any]] = defaultdict(lambda: {"total": 0, "successes": 0, "failures": 0, "rewards": []})
    for item in decision_outcomes:
        action = str(item.get("action") or "unknown")
        task_class = str((item.get("context") or {}).get("task_class") or "unknown")
        success = bool(item.get("success"))
        reward = item.get("reward")
        for bucket, key in ((by_action, action), (by_task_class, task_class)):
            bucket[key]["total"] += 1
            if success:
                bucket[key]["successes"] += 1
            else:
                bucket[key]["failures"] += 1
            if isinstance(reward, (int, float)):
                bucket[key]["rewards"].append(float(reward))

    def finalize(bucket: dict[str, dict[str, Any]]) -> dict[str, Any]:
        output = {}
        for key, stats in sorted(bucket.items()):
            output[key] = {
                "total": stats["total"],
                "success_rate": ratio(stats["successes"], stats["total"]),
                "failure_rate": ratio(stats["failures"], stats["total"]),
                "average_reward": average(stats["rewards"]),
            }
        return output

    total = len(decision_outcomes)
    successes = sum(1 for item in decision_outcomes if item.get("success"))
    rewards = [float(item["reward"]) for item in decision_outcomes if isinstance(item.get("reward"), (int, float))]
    return {
        "total": total,
        "success_rate": ratio(successes, total),
        "failure_rate": ratio(total - successes, total),
        "average_reward": average(rewards),
        "by_action": finalize(by_action),
        "by_task_class": finalize(by_task_class),
    }


def safe_route(value: str) -> str:
    out = []
    for ch in value:
        out.append(ch if ch in _ALLOWED_DELTA_KEY_CHARS else "_")
    result = "".join(out).strip("._-")
    return result or "unknown_route"


def route_delta_key(action: str) -> tuple[str, str]:
    if not action.startswith("route."):
        raise RouteDeltaKeyError("route_delta_key_invalid", f"route action must start with 'route.': {action!r}")
    route = action.removeprefix("route.")
    return f"route.{safe_route(route)}.weight", route


def maybe_propose(summary: dict[str, Any], min_decisions: int, min_action_count: int, failure_threshold: float) -> dict[str, Any] | None:
    if summary["total"] < min_decisions:
        return None
    deltas: dict[str, Any] = {}
    original_routes_by_key: dict[str, str] = {}
    patterns: list[str] = []
    for action, stats in summary["by_action"].items():
        if stats["total"] >= min_action_count and stats["failure_rate"] >= failure_threshold:
            delta_key, route = route_delta_key(action)
            previous_route = original_routes_by_key.get(delta_key)
            if previous_route is not None and previous_route != route:
                raise RouteDeltaKeyError(
                    "route_delta_key_collision",
                    f"sanitized route delta key collision: {previous_route!r} and {route!r} both map to {delta_key!r}",
                )
            original_routes_by_key[delta_key] = route
            deltas[delta_key] = {"kind": "relative", "value": -5.0, "unit": "percent"}
            patterns.append(
                f"{action} -> {delta_key} failure_rate={stats['failure_rate']} total={stats['total']} average_reward={stats['average_reward']}"
            )
    if not deltas:
        return None
    sample_confidence = min(summary["total"] / 50.0, 1.0) * 0.4
    pattern_confidence = 0.6 if len(patterns) >= 2 else 0.45
    confidence = round(min(0.95, sample_confidence + pattern_confidence), 3)
    return {
        "version": "v1",
        "basis_policy": DEFAULT_POLICY_ID,
        "ts": iso_now(),
        "deltas": deltas,
        "confidence": confidence,
        "evidence": {
            "decisions_analyzed": summary["total"],
            "failure_rate_before": summary["failure_rate"],
            "failure_rate_after_sim": summary["failure_rate"],
            "simulation_method": "heuristic_failure_rate_probe_no_replay",
            "patterns": patterns,
        },
        "reasoning": "Candidate only: reduce routes whose observed failure rate exceeds the review threshold; no automatic rule change is authorized.",
        "status": "proposed",
    }


def probe(inputs: list[dict[str, Any]], min_decisions: int = DEFAULT_MIN_DECISIONS) -> dict[str, Any]:
    routing_outcomes = [adapt(item) for item in inputs]
    decision_outcomes = [to_decision_outcome(item) for item in routing_outcomes]
    summary = summarize(decision_outcomes)
    try:
        proposal = maybe_propose(summary, min_decisions, DEFAULT_MIN_ACTION_COUNT, DEFAULT_FAILURE_THRESHOLD)
    except RouteDeltaKeyError as exc:
        return {
            "schema_version": 1,
            "kind": "ola_analyzer_probe",
            "status": "proposal_blocked",
            "summary": summary,
            "proposal": None,
            "blocked_reason": {
                "kind": exc.kind,
                "message": str(exc),
            },
            "does_not_establish": list(_DOES_NOT_ESTABLISH),
        }
    return {
        "schema_version": 1,
        "kind": "ola_analyzer_probe",
        "status": "proposal_candidate" if proposal else "insufficient_evidence",
        "summary": summary,
        "proposal": proposal,
        "does_not_establish": list(_DOES_NOT_ESTABLISH),
    }


def run_self_test() -> None:
    assert safe_route("") == "unknown_route"
    assert safe_route("direct:patch") == "direct_patch"
    assert safe_route("direct/patch/v2") == "direct_patch_v2"
    assert safe_route("direct_patch") == "direct_patch"
    assert safe_route("foo__bar") == "foo__bar"
    assert safe_route("route.with.dots-and-dashes") == "route.with.dots-and-dashes"
    assert safe_route("röute") == "r_ute"
    assert safe_route("中") == "unknown_route"
    assert route_delta_key("route.direct:patch") == ("route.direct_patch.weight", "direct:patch")
    assert route_delta_key("route.foo__bar") == ("route.foo__bar.weight", "foo__bar")

    try:
        route_delta_key("direct_patch")
    except RouteDeltaKeyError as exc:
        assert exc.kind == "route_delta_key_invalid"
        assert "route action must start with 'route.'" in str(exc)
    else:
        raise AssertionError("route actions without route. prefix must fail closed")

    fixture_dir = Path("tests/fixtures/ola")
    report = probe(load_inputs(sorted(fixture_dir.glob("*.ok.json"))))
    assert report["status"] == "insufficient_evidence"
    failed = json.loads((fixture_dir / "failed.ok.json").read_text(encoding="utf-8"))
    amplified = [failed | {"decision_id": f"gr-failed-{i:03d}", "route_used": "direct" + ":" + "patch"} for i in range(10)]
    proposed = probe(amplified, min_decisions=10)
    assert proposed["status"] == "proposal_candidate"
    assert proposed["proposal"] is not None
    assert set(proposed["proposal"]["deltas"]) == {"route.direct_patch.weight"}
    assert all(":" not in key and "/" not in key for key in proposed["proposal"]["deltas"])
    assert proposed["proposal"]["version"] == "v1"
    assert proposed["proposal"]["confidence"] >= 0.5
    assert proposed["proposal"]["evidence"]["decisions_analyzed"] == 10

    colliding = []
    for index, route in enumerate(["direct" + ":" + "patch", "direct/patch"] * 5):
        colliding.append(failed | {"decision_id": f"gr-collide-{index:03d}", "route_used": route})
    blocked = probe(colliding, min_decisions=10)
    assert blocked["status"] == "proposal_blocked"
    assert blocked["proposal"] is None
    assert blocked["blocked_reason"]["kind"] == "route_delta_key_collision"
    assert "sanitized route delta key collision" in blocked["blocked_reason"]["message"]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="*", type=Path, help="Redacted OPLEARN input JSON files")
    parser.add_argument("--min-decisions", type=int, default=DEFAULT_MIN_DECISIONS)
    parser.add_argument("--emit", choices=["report", "proposal"], default="report")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        run_self_test()
        print("ola-probe self-test ok")
        return 0
    paths = args.inputs or sorted(Path("tests/fixtures/ola").glob("*.ok.json"))
    report = probe(load_inputs(paths), args.min_decisions)
    if args.emit == "proposal":
        if report["proposal"] is None:
            parser.error("no proposal candidate was produced")
        payload = report["proposal"]
    else:
        payload = report
    print(json.dumps(payload, indent=2, ensure_ascii=False))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
