### 📄 contracts/README.md

**Größe:** 576 B | **md5:** `bc7f4000f172ba170998c78dcd6a8442`

```markdown
# Contracts für heimlern (Snapshots & Feedback)

Diese Verträge definieren das externe Austauschformat:
- **PolicySnapshot**: Zustandsstand einer Policy (Arme, Zähler, Werte …)
- **PolicyFeedback**: Rückmeldung zu einer Entscheidung (Reward, Notizen)
- **Außensensor-Event**: Normalisierte JSON-Struktur für eingehende Sensor-Events

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
```

### 📄 contracts/aussen_event.schema.json

**Größe:** 2 KB | **md5:** `a921ba479db5173dd1af312d39b1c320`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://heimlern.schemas/aussen-event.schema.json",
  "title": "Außensensor-Event",
  "description": "Normiertes Ereignis aus externen Sensoren oder Integrationen.",
  "type": "object",
  "required": ["type", "source"],
  "additionalProperties": false,
  "properties": {
    "id": {
      "type": "string",
      "description": "Eindeutige Kennung des Events (frei wählbar)."
    },
    "type": {
      "type": "string",
      "description": "Kategorie des Events, z. B. 'link' oder 'note'."
    },
    "source": {
      "type": "string",
      "description": "Herkunft oder Integrationsquelle, z. B. 'sensor-hof'."
    },
    "title": {
      "type": "string",
      "description": "Optionaler Titel zur Anzeige."
    },
    "summary": {
      "type": "string",
      "description": "Kurzbeschreibung oder Erklärung zum Event."
    },
    "url": {
      "type": "string",
      "format": "uri",
      "description": "Referenz-URL für weiterführende Informationen."
    },
    "tags": {
      "type": "array",
      "description": "Kategorisierende Tags.",
      "items": {
        "type": "string"
      }
    },
    "ts": {
      "type": "string",
      "format": "date-time",
      "description": "Zeitstempel im ISO-8601-Format."
    },
    "features": {
      "type": "object",
      "description": "Beliebige strukturierte Zusatzinformationen.",
      "additionalProperties": true
    },
    "meta": {
      "type": "object",
      "description": "Weitere Metadaten ohne feste Struktur.",
      "additionalProperties": true
    }
  },
  "examples": [
    {
      "type": "link",
      "source": "sensor-hof",
      "title": "Regenwarnung",
      "url": "https://example.com/wetter",
      "tags": ["regen", "sensor"],
      "features": {
        "level": 0.82
      }
    }
  ]
}
```

### 📄 contracts/policy_feedback.schema.json

**Größe:** 579 B | **md5:** `b1178697a59d0a7b73d9f7d3c4d4fe0a`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://heimgewebe/contracts/policy_feedback.schema.json",
  "title": "PolicyFeedback",
  "type": "object",
  "additionalProperties": false,
  "required": ["version","policy_id","ts","decision_id","reward"],
  "properties": {
    "version": { "type": "string" },
    "policy_id": { "type": "string", "minLength": 1 },
    "ts": { "type": "string", "format": "date-time" },
    "decision_id": { "type": "string", "minLength": 1 },
    "reward": { "type": "number" },
    "notes": { "type": "string" }
  }
}
```

### 📄 contracts/policy_snapshot.schema.json

**Größe:** 859 B | **md5:** `40baa78d6951bcdbe5f2fa9fdc495a07`

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://heimgewebe/contracts/policy_snapshot.schema.json",
  "title": "PolicySnapshot",
  "type": "object",
  "additionalProperties": false,
  "required": ["version","policy_id","ts","arms","counts","values","epsilon"],
  "properties": {
    "version": { "type": "string" },
    "policy_id": { "type": "string", "minLength": 1 },
    "ts": { "type": "string", "format": "date-time" },
    "arms": {
      "type": "array",
      "minItems": 1,
      "items": { "type": "string", "minLength": 1 }
    },
    "counts": {
      "type": "array",
      "items": { "type": "integer", "minimum": 0 }
    },
    "values": {
      "type": "array",
      "items": { "type": "number" }
    },
    "epsilon": { "type": "number", "minimum": 0, "maximum": 1 },
    "seed": { "type": "integer" }
  }
}
```

