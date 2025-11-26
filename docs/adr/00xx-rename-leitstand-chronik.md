# ADR: Rename leitstand → chronik, introduce leitstand UI
Status: Accepted
Date: 2025-11-26

## Context

The `leitstand` repository currently serves as an event ingest and persistence layer, essentially an event store. A planned UI/Dashboard, which is semantically the real "leitstand" (control room), is missing. This leads to a semantic mismatch between the repository's name and its function.

## Decision

To resolve this, we:

1.  **Renamed the backend repository:** `chronik` is now the canonical name for the event store backend.
2.  **Created a new UI repository:** The `leitstand` UI/Dashboard exists as the central control room for the Heimgewebe ecosystem.

## Consequences

*   **Clarity:** The names of the repositories will now accurately reflect their functions.
*   **Consistency:** The Schichtenmodell (layer model) will be clearer, with `chronik` as the backend and `leitstand` as the UI.
*   **Technical Debt:** All references to the old `leitstand` backend in code, CI/CD pipelines, and documentation must be updated to `chronik`.
