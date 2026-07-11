# Chronik routing-outcome consumer

Status: review-only contract consumer

Bureau task: `CHRONIK-HEIMLERN-OUTCOME-BRIDGE-V1-T003`

## Role boundary

Heimlern consumes a Chronik-owned append-only envelope around the Heimlern-owned
`operator.routing_outcome.v1` payload. The consumer validates and analyzes; it
does not fetch Chronik, append events, change route weights or apply proposals.

Authority stays separated:

| Surface | Owner | Consumer treatment |
| --- | --- | --- |
| Transport envelope | `heimgewebe/chronik` | exact digest-pinned mirror |
| Routing-outcome payload | `heimgewebe/heimlern` | canonical local contract |
| Weight-adjustment proposal | `heimgewebe/metarepo` | exact digest-pinned mirror |
| Execution receipts and future producer | `heimgewebe/grabowski` | not implemented by this slice |

## Validation order

`scripts/chronik_outcome_consumer.py` fails closed before analysis unless all of
the following hold:

1. Both external mirrors match their source revision/path SHA-256 pins and remain
   marked `mirror_only`.
2. The Chronik envelope and embedded Heimlern payload validate against their
   respective schemas.
3. `payload_contract`, canonical payload SHA-256 and deterministic event ID
   match the pinned identities.
4. Outer and payload evidence references carry the same non-duplicated digests.
5. Envelope export time equals `ts`, payload time equals `observed_at`, and no
   export lies in the future relative to the supplied review time.
6. Raw-output fields, secret-shaped strings and private absolute paths are
   rejected before schema errors can echo input values.
7. Input bytes, event size and event count remain bounded.
8. Duplicate event and decision IDs are ignored deterministically; conflicting decision payloads fail closed; stale events are excluded
   under the explicit consumer-side freshness policy.
9. Proposal timestamps are bound to the explicit review time rather than the
   machine clock, and Rust adapter calls are time-bounded.

Only fresh, unique payloads reach the existing OLA analysis path. A resulting
proposal must validate against the pinned `policy.weight_adjustment.v1` mirror.
Otherwise the output is a typed `insufficient_evidence`, `proposal_blocked` or
`invalid_input` result.

## Usage

The review time is explicit in reproducible runs. Without it, current UTC is
used.

```bash
python scripts/chronik_outcome_consumer.py \
  --review-time 2026-07-10T23:00:00Z \
  --max-age-seconds 7200 \
  tests/fixtures/chronik-outcome/operator-routing-outcome-export.v1.json
```

The JSON report includes accepted event IDs and envelope digests, event/decision duplicates,
stale exclusions, contract revisions/digests, analysis output, proposal
validation, `writes: []` and explicit non-claims.

## Mirror update rule

A mirror update is a reviewed contract migration, not an automatic refresh:

1. read the canonical source at an immutable revision;
2. replace the mirror with exact bytes;
3. update revision, path and SHA-256 in the adjacent pin;
4. inspect semantic differences and update compatibility tests;
5. run the full repository CI before merge.

## Non-claims

This consumer does not establish a live Grabowski producer, Chronik runtime
readiness, production sample sufficiency, causal route superiority, routing
policy readiness or permission to apply a proposal.
