### 📄 docs/adr/0001-policy-explainability.md

**Größe:** 441 B | **md5:** `3db85bdaaa67b3e98d3352ba948b16db`

```markdown
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
```

### 📄 docs/adr/0002-policy-snapshot-persistenz.md

**Größe:** 373 B | **md5:** `2aa6b08424d3ec705dc98b1cd24acc12`

```markdown
# ADR-0002: Policy-Snapshot-Persistenz (SQLite via hausKI)
Status: Accepted
Date: 2025-10-12

## Kontext
Lernen benötigt zustandsvolle Parameter.

## Entscheidung
- Snapshots (JSON) persistieren; Laden/Speichern via hausKI.

## Konsequenzen
- Wiederaufnahme nach Neustart möglich.
- Migrationen über Versionsfeld.

## Alternativen
- Nur In-Memory: Verlust bei Neustart.
```

### 📄 docs/adr/README.md

**Größe:** 253 B | **md5:** `e407c51b96ab4a129adcc0c9d1f71441`

```markdown
# Architekturentscheidungsaufzeichnungen (ADR)

## Index

- [ADR-0001: Policy/Decision als Rust-Lib, Explainability (`why`)](0001-policy-explainability.md)
- [ADR-0002: Policy-Snapshot-Persistenz (SQLite via hausKI)](0002-policy-snapshot-persistenz.md)
```

