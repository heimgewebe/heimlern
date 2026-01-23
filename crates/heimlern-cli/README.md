# heimlern-cli

Der `heimlern-cli` ist der **operationalisierte Consumer** für Heimlern: ein kleines, robustes Werkzeug, das Events **aus der Chronik** (oder im Simulationsmodus aus Dateien) abholt, verarbeitet und den **Fortschritt persistent** hält.

## Zweck

Er dient als Brücke zwischen „Events existieren“ und „Heimlern hat sie tatsächlich konsumiert“.

### Rolle im Organismus

* **Chronik** ist Backbone und Quelle für zeitlich geordnete Events.
* **Heimlern** ist Lern-Organ (Bandits/Feedback/Policies) – braucht Eingänge, aber soll nicht zwingend als dauerlaufender Service starten müssen.
* **heimlern-cli** ist der **ausführbare Ingest-Orchestrator**: ideal für Cron/systemd, lokale Entwicklung und Debugging.

## Designprinzipien

1. **Statefulness ohne Datenbank**
   Der Fortschritt wird in einer lokalen State-Datei gehalten (`cursor`, `mode`, `last_ok`, `last_error`). Das ist bewusst „klein“ und deploymentfreundlich.

2. **Resumierbarkeit als Default**
   Wenn kein Cursor übergeben wird, wird aus der State-Datei fortgesetzt.

3. **Protokollhärtung gegen Drift**
   Es gibt explizite Checks für fehlerhafte Serverantworten:
   * `has_more=true` aber `next_cursor` fehlt → Protokollfehler
   * Cursor „stallt“ (`next_cursor == current` bei `has_more=true`) → Protokollfehler

4. **Simulation zuerst**
   `ingest file` ermöglicht reproduzierbare Tests/Debugging ohne Chronik-HTTP.
   * **Cursor-Semantik:** Im File-Mode ist der Cursor ein **Line-Offset** (0-basiert). In Chronik-Mode ein **Byte-Offset** (opaque u64).

5. **Stats als operatives Nebenprodukt**
   Eine Stats-Datei zählt Events nach Typ/Quelle.

## Nutzung

### Ingest aus Chronik (Produktion)

```bash
# URL-Format: Basis-URL (z.B. http://localhost:3000). /v1/events wird automatisch angehängt.
export CHRONIK_BASE_URL="http://localhost:3000"
export CHRONIK_TOKEN="secret-token"

# Startet Ingest. Domain muss alphanumerisch (+ . -) sein.
heimlern ingest chronik --domain aussen
```

### Ingest aus Datei (Simulation/Test)

```bash
# Liest Events aus lokaler JSONL-Datei
heimlern ingest file --path events.jsonl
```

## Abgrenzung

* Die State-/Stats-Dateien sind **nicht-kanonisch** (lokal, operational).
* `last_ok`: Ein fehlendes oder `null` Feld im State bedeutet, dass der Ingest noch nie erfolgreich war.
