# Operator ecosystem alignment

> Historical note: Heimlern no longer has an active operator-ecosystem role. The relationships below describe the implemented proposal-only boundary retained for audit. See `docs/historical-status.md`.

Heimlern was implemented as a retrospective learning and policy-adaptation proposal engine of the Heimgewebe operator ecosystem.

- Chronik owns the append-only routing-outcome transport envelope and historical export.
- Bureau owns tasks, claims and completion truth.
- Grabowski owns local execution and receipts.
- Leitstand could display learning reports and proposals.
- Heimlern historically owned the routing-outcome payload and could propose adjustments; it never had authority to apply them.

The useful loop is evidence -> outcome analysis -> auditable proposal -> separate decision gate. Anything shorter is just a model wearing a lab coat.


## Review-only Chronik outcome path

The dedicated consumer validates pinned Chronik and Metarepo mirrors, the local
Heimlern payload contract, canonical event/payload identities, evidence digests,
redaction, freshness and duplicates before OLA analysis. It emits an auditable
report or schema-valid proposal candidate with `writes: []`. The generic
`AussenEvent` ingest path is deliberately not reused for routing outcomes. See
`docs/chronik-routing-outcome-consumer.md`.
