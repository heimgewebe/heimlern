#!/usr/bin/env python3
"""Benchmark Heimlern release opt-level profiles.

The script builds the existing `heimlern-bandits` feedback benchmark with isolated
Cargo target directories and `CARGO_PROFILE_RELEASE_OPT_LEVEL` set to each
requested profile. It then records binary size and benchmark timings as JSON.
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "docs" / "benchmarks" / "release-profile-comparison.latest.json"
BENCH_EXAMPLE = "bench_feedback"
BENCH_PACKAGE = "heimlern-bandits"


def run(cmd: list[str], *, env: dict[str, str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        cmd,
        cwd=ROOT,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=True,
    )


def median(values: list[float]) -> float:
    return float(statistics.median(values))


def build_and_run_profile(profile: str, args: argparse.Namespace) -> dict[str, Any]:
    target_dir = ROOT / "target" / "release-profile-bench" / profile
    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(target_dir)
    env["CARGO_PROFILE_RELEASE_OPT_LEVEL"] = profile

    started = time.perf_counter()
    build = run(
        [
            "cargo",
            "build",
            "--release",
            "-p",
            BENCH_PACKAGE,
            "--example",
            BENCH_EXAMPLE,
        ],
        env=env,
    )
    build_seconds = time.perf_counter() - started

    binary = target_dir / "release" / "examples" / BENCH_EXAMPLE
    if sys.platform.startswith("win"):
        binary = binary.with_suffix(".exe")
    if not binary.is_file():
        raise SystemExit(f"built benchmark binary not found: {binary}")

    bench = run(
        [
            str(binary),
            "--json",
            "--iterations",
            str(args.iterations),
            "--replay-iterations",
            str(args.replay_iterations),
            "--fill-cap",
            str(args.fill_cap),
            "--warmup",
            str(args.warmup),
            "--samples",
            str(args.samples),
        ],
        env=env,
    )
    report = json.loads(bench.stdout)
    samples = report["samples"]
    return {
        "profile": profile,
        "cargo_profile_release_opt_level": profile,
        "target_dir": str(target_dir.relative_to(ROOT)),
        "binary": str(binary.relative_to(ROOT)),
        "binary_size_bytes": binary.stat().st_size,
        "build_seconds": build_seconds,
        "benchmark": report,
        "summary": {
            "existing_slot_ns_per_call_median": median(
                [float(s["existing_slot_ns_per_call"]) for s in samples]
            ),
            "filling_slots_ns_per_call_median": median(
                [float(s["filling_slots_ns_per_call"]) for s in samples]
            ),
            "snapshot_replay_ns_per_call_median": median(
                [float(s["snapshot_replay_ns_per_call"]) for s in samples]
            ),
        },
        "build_stderr_tail": "\n".join(build.stderr.splitlines()[-20:]),
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--profiles",
        default="z,s,3",
        help="Comma-separated opt-level profiles to compare, default: z,s,3",
    )
    parser.add_argument("--iterations", type=int, default=200_000)
    parser.add_argument("--replay-iterations", type=int, default=10_000)
    parser.add_argument("--fill-cap", type=int, default=1_000)
    parser.add_argument("--warmup", type=int, default=1_000)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--out", type=Path, default=DEFAULT_OUTPUT)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    profiles = [item.strip() for item in args.profiles.split(",") if item.strip()]
    if not profiles:
        raise SystemExit("at least one profile is required")
    args.out = args.out if args.out.is_absolute() else ROOT / args.out
    args.out.parent.mkdir(parents=True, exist_ok=True)

    generated_at = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    git_head = run(["git", "rev-parse", "HEAD"], env=os.environ.copy()).stdout.strip()
    git_dirty = bool(run(["git", "status", "--short"], env=os.environ.copy()).stdout.strip())

    results = [build_and_run_profile(profile, args) for profile in profiles]
    smallest = min(results, key=lambda item: int(item["binary_size_bytes"]))
    fastest_existing = min(
        results,
        key=lambda item: float(item["summary"]["existing_slot_ns_per_call_median"]),
    )

    output = {
        "schema_version": 1,
        "kind": "heimlern.release_profile_benchmark.v1",
        "generated_at": generated_at,
        "git_head": git_head,
        "git_dirty": git_dirty,
        "command": {
            "profiles": profiles,
            "iterations": args.iterations,
            "replay_iterations": args.replay_iterations,
            "fill_cap": args.fill_cap,
            "warmup": args.warmup,
            "samples": args.samples,
        },
        "host": {
            "platform": platform.platform(),
            "processor": platform.processor(),
            "python": platform.python_version(),
        },
        "results": results,
        "decision_inputs": {
            "smallest_binary_profile": smallest["profile"],
            "fastest_existing_slot_profile": fastest_existing["profile"],
        },
        "does_not_establish": [
            "production_runtime_performance",
            "all_workload_performance",
            "cross_machine_reproducibility",
            "policy_quality",
            "learning_quality",
        ],
    }
    args.out.write_text(json.dumps(output, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"out": str(args.out), "profiles": profiles}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
