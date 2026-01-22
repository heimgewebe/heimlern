# heimlern-cli

Der `heimlern-cli` ist der **operationalisierte Consumer** für Heimlern: ein kleines, robustes Werkzeug, das Events **aus der Chronik** (oder im Simulationsmodus aus Dateien) abholt, verarbeitet und den **Fortschritt persistent** hält.

## Zweck

Er dient als Brücke zwischen „Events existieren“ und „Heimlern hat sie tatsächlich konsumiert“.

### Rolle im Organismus

* **Chronik** ist Backbone und Quelle für zeitlich geordnete Events.
* **Heimlern** ist Lern-Organ (Bandits/Feedback/Policies) – braucht Eingänge, aber soll nicht zwingend als dauerlaufender Service starten müssen.
* **heimlern-cli** ist der **ausführbare Ingest-Orchestrator**: ideal für Cron/systemd, lokale Entwicklung und Debugging.

Damit bleibt das Lernen **service-unabhängig**: man kann Ingest laufen lassen, ohne eine komplette Heimlern-Deployment-Topologie zu benötigen.

## Designprinzipien

1. **Statefulness ohne Datenbank**
   Der Fortschritt wird in einer lokalen State-Datei gehalten (`cursor`, `mode`, `last_ok`, `last_error`). Das ist bewusst „klein“ und deploymentfreundlich.

2. **Resumierbarkeit als Default**
   Wenn kein Cursor übergeben wird, wird aus der State-Datei fortgesetzt. Damit sind Runs idempotent-ish und operativ stabil.

3. **Protokollhärtung gegen Drift**
   Es gibt explizite Checks für fehlerhafte Serverantworten:
   * `has_more=true` aber `next_cursor` fehlt → Protokollfehler
   * Cursor „stallt“ (`next_cursor == current` bei `has_more=true`) → Protokollfehler
   Solche Fälle werden im State als Fehler vermerkt und brechen mit Exit-Code ≠ 0 ab.

4. **Simulation zuerst**
   `ingest file` ermöglicht reproduzierbare Tests/Debugging ohne Chronik-HTTP. Das reduziert Integrationsstress und beschleunigt Entwicklung.

5. **Stats als operatives Nebenprodukt**
   Eine Stats-Datei zählt Events nach Typ/Quelle und liefert ein minimales Observability-Signal („läuft es überhaupt“, „was kommt rein“), ohne gleich eine Monitoring-Infrastruktur vorauszusetzen.

## Nutzung

### Ingest aus Chronik (Produktion)

```bash
export CHRONIK_BASE_URL="http://localhost:3000"
export CHRONIK_TOKEN="secret-token"

# Startet Ingest, resumed automatisch aus State-Datei
heimlern ingest chronik --domain aussen
```

### Ingest aus Datei (Simulation/Test)

```bash
# Liest Events aus lokaler JSONL-Datei
heimlern ingest file --path events.jsonl
```

## Abgrenzung

* `heimlern-cli` enthält **Orchestrationslogik**, nicht die Lernlogik.
* Die Lernlogik bleibt in `heimlern-core` / `heimlern-bandits` / `heimlern-feedback`.
* Die State-/Stats-Dateien sind **nicht-kanonisch** (lokal, operational).
