# Contracts für heimlern (Snapshots & Feedback)

**Note:** This repository **consumes** contracts but does not own them. The canonical 
source of truth for internal Heimgewebe contracts is the **metarepo**.

Diese Verträge definieren das externe Austauschformat:
- **PolicySnapshot**: Zustandsstand einer Policy (Arme, Zähler, Werte …)
- **PolicyFeedback**: Rückmeldung zu einer Entscheidung (Reward, Notizen)
- **Außensensor-Event**: Normalisierte JSON-Struktur für eingehende Sensor-Events

Ziele:
- Reproduzierbarkeit (Versionierung)
- Strikte Validierung (keine schleichende Schema-Drift)
- Tool-agnostisch (Rust, Python, Shell …)

## Contract Ownership

Heimlern **konsumiert** folgende Contracts aus dem metarepo:
- `decision.outcome.v1` - Retrospektive Bewertung von Entscheidungen (Producer: hausKI, chronik)
- `policy.weight_adjustment.v1` - Gewichtsanpassungsvorschläge (Producer: heimlern → Consumer: hausKI)

Die hier aufgeführten Schemas dienen als lokale Referenz für Entwicklung und Tests.
Die kanonischen Definitionen liegen im **heimgewebe/metarepo/contracts/**.

## Payload vs Event Envelope

**Wichtig:** Die referenzierten Schemas definieren **Payload-Strukturen**, nicht Event-Envelopes.

Wenn diese Daten über ein Event-System (z.B. chronik, plexer) transportiert werden, 
werden sie in ein standardisiertes Envelope eingebettet mit Feldern wie:
- `type`: Event-Typ
- `source`: Quelle des Events
- `payload`: Die hier definierten Strukturen
- `ts`: Event-Zeitstempel
- `id`: Event-ID

Die Envelope-Spezifikation ist Teil der übergeordneten Event-Architektur und 
wird im metarepo definiert.

## Quickstart
```sh
just snapshot:example
just feedback:example
just schema:validate
```

## Schema-Übersicht (Lokale Referenzen)

**Schemas in diesem Repo:**

| Schema | Zweck | Produzenten | Konsumenten |
|--------|-------|-------------|-------------|
| `policy.snapshot.schema.json` | Policy-Zustand persistieren | heimlern | hausKI, chronik |
| `policy.decision.schema.json` | Entscheidungsdokumentation | heimlern | hausKI, chronik |
| `policy.feedback.schema.json` | Feedback zu Entscheidungen | hausKI | heimlern |
| `aussen.event.schema.json` | Externe Sensor-Events | Sensoren, APIs | heimlern, hausKI |

**Schemas konsumiert aus metarepo:**

| Schema | Zweck | Produzenten | Konsumenten |
|--------|-------|-------------|-------------|
| `decision.outcome.v1` | Retrospektive Outcome-Bewertung | hausKI, chronik | heimlern |
| `policy.weight_adjustment.v1` | Gewichtsanpassungsvorschläge | heimlern | hausKI |
