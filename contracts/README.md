# Heimlern contract archive

This directory is preserved as implementation history. It is not an active contract authority. The exact classification and SHA-256 binding of every local JSON schema is recorded in `../docs/archive-readiness.v1.json`.

## Normative external mirrors

Only two repository copies are treated as exact, authority-bound mirrors:

- `mirrors/metarepo/policy.weight_adjustment.v1.schema.json`, owned by `heimgewebe/metarepo`;
- `mirrors/chronik/operator-routing-outcome-export-v1.schema.json`, owned by `heimgewebe/chronik`.

Each mirror has a neighboring pin containing the exact source repository, commit, path and digest. Heimlern does not own those contracts. CI validates the local pinned copies and performs no live schema download.

## Historical local schemas

Top-level schemas in this directory are frozen historical references. Some have a current contract with the same filename in Metarepo but differ byte-for-byte; the archive manifest labels them `historical_divergent_copy`. `aussen.event.schema.json` is currently byte-identical to its Metarepo authority, but ownership still remains with Metarepo. Local-only learning and routing schemas are historical experiment formats and grant no runtime or routing authority.

`operator.routing_outcome.v1.schema.json` remains pinned as the historical payload expected by the exact Chronik envelope mirror. That pin preserves replayability only; it does not keep Heimlern active.

## Proposal-only boundary

The preserved examples and analyzers may validate historical data and produce review-only proposal candidates. They must retain `writes_production: false`, `writes: []`, no automatic policy or routing changes, no queue authority and no live Grabowski producer.
