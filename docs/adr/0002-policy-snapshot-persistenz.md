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
