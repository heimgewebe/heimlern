# Contracts für heimlern (Snapshots, Feedback & Weight Adjustments)

Diese Verträge definieren das externe Austauschformat:
- **PolicySnapshot**: Zustandsstand einer Policy (Arme, Zähler, Werte …)
- **PolicyFeedback**: Rückmeldung zu einer Entscheidung (Reward, Notizen)
- **DecisionOutcome**: Retrospektive Bewertung einer Entscheidung (Erfolg/Misserfolg, Kontext)
- **WeightAdjustment**: Vorschlag für Gewichtsanpassungen (Deltas, Evidenz, Konfidenz)
- **Außensensor-Event**: Normalisierte JSON-Struktur für eingehende Sensor-Events

Ziele:
- Reproduzierbarkeit (Versionierung)
- Strikte Validierung (keine schleichende Schema-Drift)
- Tool-agnostisch (Rust, Python, Shell …)

## Payload vs. Event Envelope

**Wichtig:** Diese Schemas definieren **Payload-Strukturen**, nicht Event-Envelopes.

Wenn diese Daten über ein Event-System (z.B. chronik, plexer) transportiert werden, 
werden sie in ein standardisiertes Envelope eingebettet mit Feldern wie:
- `type`: Event-Typ
- `source`: Quelle des Events
- `payload`: Die hier definierten Strukturen
- `ts`: Event-Zeitstempel
- `id`: Event-ID

Die Envelope-Spezifikation ist Teil der übergeordneten Event-Architektur und 
wird separat definiert (idealerweise im metarepo).

## Quickstart
```sh
just snapshot:example
just feedback:example
just schema:validate
```

## Schema-Übersicht

| Schema | Zweck | Produzenten | Konsumenten |
|--------|-------|-------------|-------------|
| `policy.snapshot.schema.json` | Policy-Zustand persistieren | heimlern | hausKI, chronik |
| `policy.decision.schema.json` | Entscheidungsdokumentation | heimlern | hausKI, chronik |
| `policy.feedback.schema.json` | Feedback zu Entscheidungen | hausKI | heimlern |
| `decision.outcome.schema.json` | Retrospektive Outcome-Bewertung | hausKI, chronik | heimlern |
| `policy.weight_adjustment.schema.json` | Gewichtsanpassungsvorschläge | heimlern | hausKI |
| `aussen.event.schema.json` | Externe Sensor-Events | Sensoren, APIs | heimlern, hausKI |
