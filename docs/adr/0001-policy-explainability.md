# ADR-0001: Policy/Decision als Rust-Lib, Explainability (`why`)
Status: Accepted
Date: 2025-10-12

## Kontext
Entscheidungen sollen reproduzierbar und begründet sein.

## Entscheidung
- Rust-Lib (heimlern-core + -bandits).
- Jede Decision hat `action`, `score`, `why`.

## Konsequenzen
- Klare Schnittstelle zu hausKI; leicht testbar.
- `why` wird im Leitstand angezeigt.

## Alternativen
- Ad-hoc Heuristiken ohne Begründung: verworfen.
