# Operator ecosystem alignment

Heimlern is the retrospective learning and policy-adaptation proposal engine of the Heimgewebe operator ecosystem.

- Chronik owns the append-only routing-outcome transport envelope and historical export.
- Bureau owns tasks, claims and completion truth.
- Grabowski owns local execution and receipts.
- Leitstand may display learning reports and proposals.
- Heimlern owns the routing-outcome payload and may propose adjustments; it must not silently apply them.

The useful loop is evidence -> outcome analysis -> auditable proposal -> separate decision gate. Anything shorter is just a model wearing a lab coat.


## Review-only Chronik outcome path

The dedicated consumer validates pinned Chronik and Metarepo mirrors, the local
Heimlern payload contract, canonical event/payload identities, evidence digests,
redaction, freshness and duplicates before OLA analysis. It emits an auditable
report or schema-valid proposal candidate with `writes: []`. The generic
`AussenEvent` ingest path is deliberately not reused for routing outcomes. See
`docs/chronik-routing-outcome-consumer.md`.
