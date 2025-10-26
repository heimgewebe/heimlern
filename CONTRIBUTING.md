# Beitrag leisten

Danke für deinen Beitrag zu **heimlern**.

## Setup
```bash
git clone https://github.com/heimgewebe/heimlern.git
cd heimlern
cargo build --workspace
cargo test  --workspace
```

## Coding-Guidelines (Kurz)
- Keine `unwrap()`/`expect()` in Bibliothekscode – stattdessen `Result` zurückgeben.
- Unit-Tests in jeder Crate für Kernstrukturen.
- PRs gegen `main` mit „grüner“ CI.

## Commit-Stil
- Präfixe: `feat:`, `fix:`, `docs:`, `ci:`, `refactor:`, `test:`, `chore:`

## Lizenz
Durch das Einreichen eines PR stimmst du der Projektlizenz zu.
