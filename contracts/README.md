# Contracts für heimlern (Snapshots & Feedback)

Diese Verträge definieren das externe Austauschformat:
- **PolicySnapshot**: Zustandsstand einer Policy (Arme, Zähler, Werte …)
- **PolicyFeedback**: Rückmeldung zu einer Entscheidung (Reward, Notizen)

Ziele:
- Reproduzierbarkeit (Versionierung)
- Strikte Validierung (keine schleichende Schema-Drift)
- Tool-agnostisch (Rust, Python, Shell …)

## Quickstart
```sh
just snapshot:example
just feedback:example
just schema:validate
```
