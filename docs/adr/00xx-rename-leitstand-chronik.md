# ADR: Rename leitstand → chronik, introduce leitstand UI

## Context

The `leitstand` repository currently serves as an event ingest and persistence layer, essentially an event store. A planned UI/Dashboard, which is semantically the real "leitstand" (control room), is missing. This leads to a semantic mismatch between the repository's name and its function.

## Decision

To resolve this, we will:

1.  **Rename the backend repository:** `leitstand` will be renamed to `chronik` to accurately reflect its role as an event store.
2.  **Create a new UI repository:** A new repository named `leitstand` will be created for the UI/Dashboard, which will act as the central control room for the Heimgewebe ecosystem.

## Consequences

*   **Clarity:** The names of the repositories will now accurately reflect their functions.
*   **Consistency:** The Schichtenmodell (layer model) will be clearer, with `chronik` as the backend and `leitstand` as the UI.
*   **Technical Debt:** All references to the old `leitstand` backend in code, CI/CD pipelines, and documentation must be updated to `chronik`.
