# ADR-0004: Release-Profil `opt-level = "s"`

Status: accepted
Date: 2026-07-08

## Context

The workspace previously used `opt-level = "z"` for release builds. That setting
optimizes for size, but the decision had not been backed by a reproducible local
measurement for Heimlern's current feedback-oriented workload.

`HEIMLERN-OPTIMIZATION-V1-T004` requires a reproducible comparison of `z`, `s`
and `3`, including size and runtime, plus a short release-profile decision.

## Benchmark

The benchmark is intentionally small and local:

```text
python3 scripts/benchmark_release_profiles.py \
  --profiles z,s,3 \
  --iterations 200000 \
  --replay-iterations 5000 \
  --fill-cap 1000 \
  --warmup 1000 \
  --samples 3 \
  --out docs/benchmarks/release-profile-comparison-2026-07-08.json
```

It builds the `heimlern-bandits` `bench_feedback` example once per profile with
`CARGO_PROFILE_RELEASE_OPT_LEVEL` and isolated target directories. It records:

- release example binary size;
- median existing-slot feedback time;
- median new-slot fill time;
- median snapshot replay time.

Raw evidence:

- `docs/benchmarks/release-profile-comparison-2026-07-08.json`
- `scripts/benchmark_release_profiles.py`
- `crates/heimlern-bandits/examples/bench_feedback.rs`

## Results

| opt-level | binary size bytes | existing-slot ns/call median | fill-slots ns/call median | snapshot-replay ns/call median |
| --- | ---: | ---: | ---: | ---: |
| `z` | 538056 | 30.489175 | 1339.547000 | 284688.445000 |
| `s` | 535680 | 21.674825 | 1401.828000 | 237552.617000 |
| `3` | 579272 | 19.734960 | 1362.578000 | 218003.675000 |

## Decision

Use `opt-level = "s"` for the workspace release profile.

Rationale:

- `s` produced the smallest measured binary in this run.
- `s` was materially faster than `z` on the existing-slot and snapshot-replay paths.
- `3` was fastest overall, but increased binary size enough that it does not fit the current size-conscious release premise.
- The previous `z` setting is not retained because it was neither the smallest nor the fastest measured profile in this benchmark.

## Consequences

- The workspace release profile changes from `z` to `s`.
- The comparison remains reproducible through the checked-in benchmark script and raw JSON report.
- Future production-like workloads may justify a different profile, but that requires a new measured decision.

## Limits / Non-claims

This decision does not establish:

- production runtime performance;
- all-workload performance;
- cross-machine reproducibility;
- learning quality;
- policy quality;
- that `s` is optimal for future workloads.
