### 📄 docs/policies-os-context.md

**Größe:** 1 KB | **md5:** `f3ff72422ef8adfeed51eeff785b03f9`

```markdown
# heimlern: Policies für OS-Kontext

## Ziele
- Consent erzwingen
- Sensitive Kontexte blocken
- Rate-Limits & PII-Gate
- Automations (Deep-Work, Selbstheilung)

## Kern-Policies (YAML-Skizze)
```yaml
consent:
  text_capture: false   # muss aktiv vom Nutzer gesetzt werden

pii_gate:
  min_confidence: 0.85
  on_violation: drop_and_shred

rate_limits:
  embed_per_app_per_min: 12
  on_exceed: drop

allow_block:
  allow_apps: [code, obsidian]
  allow_domains: ["localhost", "dev.local"]
  block_apps: ["org.keepassxc.KeePassXC", "com.bank.app"]
  block_domains: ["login.microsoftonline.com", "accounts.google.com"]

modes:
  deep_work:
    enter_if:
      - os.context.state.focus == true
      - hauski_audio.vibe in ["fokussiert", "neutral"]
      - app in ["code", "obsidian"]
    actions:
      - hausKI.hold_notifications
    exit_if:
      - focus == false OR inactivity > 10m
    exit_actions:
      - hausKI.release_notifications
```

## Selbstheilung

- Metriken aus hausKI/wgx beobachten; bei Silence/Latenz → Entscheidung: `wgx doctor`/Restart (lokal), auditierbar.
```

### 📄 docs/policy-lifecycle.md

**Größe:** 3 KB | **md5:** `89e3ddb50dfe689dd1a79174012537f7`

```markdown
# Policy Lifecycle (Snapshot → Validate → Apply)

Dieser Leitfaden beschreibt den Lebenszyklus einer heimlern-Policy von der
Zustandssicherung über die Schema-Validierung bis zur erneuten Aktivierung.
Er ergänzt den Überblick im [README](../README.md) und fokussiert auf
praktische Schritte, die mit den vorhandenen Werkzeugen wiederholt werden
können.

## 1. Snapshot erstellen

1. Policy ausführen und Entscheidung treffen, z. B. über das Beispiel
   `integrate_hauski`:
   ```bash
   cargo run --example integrate_hauski
   ```
2. Der Agent exportiert seinen Zustand als JSON-Snapshot über das
   [`Policy`-Trait](../crates/heimlern-core/src/lib.rs) mit den Methoden
   [`snapshot`](../crates/heimlern-core/src/lib.rs) und
   [`load`](../crates/heimlern-core/src/lib.rs). Dadurch bleiben Zähler,
   Zufalls-Seed und weitere Parameter zwischen Sessions erhalten.
3. Für reproduzierbare Testdaten können die Beispielskripte verwendet werden:
   ```bash
   just snapshot:example
   # legt /tmp/heimlern_snapshot.json an
   ```

Snapshots sind reine JSON-Dokumente, die keine Binärdaten enthalten und in
Versionskontrolle oder Objektspeichern abgelegt werden können.

## 2. Snapshot validieren

1. Werkzeuge vorbereiten (Python-Venv und `jsonschema`):
   ```bash
   just venv
   ```
2. Automatisierte Prüfung mit dem vorhandenen Validator:
   ```bash
   python scripts/validate_json.py contracts/policy_snapshot.schema.json /tmp/heimlern_snapshot.json
   ```
   Alternativ validiert `just schema:validate` sowohl Snapshot als auch
   Feedback-Beispiel in einem Lauf.
3. Der Validator führt neben der JSON-Schema-Prüfung zusätzliche Konsistenz-
   Checks aus, z. B. dass die Längen von `arms`, `counts` und `values`
   übereinstimmen.

Bei Validierungsfehlern bricht der Befehl mit Exit-Code ≠ 0 ab und nennt das
betroffene Feld. Dadurch lassen sich Snapshots vor Deployments oder vor dem
Import aus Drittquellen absichern.

## 3. Snapshot anwenden

1. Validiertes JSON in die Policy laden, typischerweise beim Start eines
   Dienstes:
   ```rust
   let snapshot = serde_json::from_str::<serde_json::Value>(&json_str)?;
   bandit.load(snapshot);
   ```
2. Anschließend kann die Policy unmittelbar wieder Entscheidungen treffen und
   Feedback verarbeiten (`decide`/`feedback`).
3. Im CI/CD-Kontext empfiehlt es sich, nach erfolgreicher Validierung den
   Snapshot unter Versionskontrolle zu taggen oder in ein Artefakt-Repository
   hochzuladen, um den Stand nachverfolgen zu können.

## Validator-Workflow für JSONL-Samples

Beispiel-Events (z. B. `data/samples/aussensensor.jsonl`) enthalten mehrere
Dokumente im JSON-Lines-Format. Sie können direkt gegen den
[Außensensor-Contract](../contracts/aussen_event.schema.json) geprüft werden:

```bash
python scripts/validate_json.py contracts/aussen_event.schema.json data/samples/aussensensor.jsonl
```

Der Validator liest jede Zeile, validiert sie einzeln und gibt ein ✓ je Zeile
aus. Fehler werden mit Zeilennummer ausgewiesen, sodass defekte Events schnell
gefunden und korrigiert werden können. Anschließend lassen sich die gleichen
Samples z. B. über das Beispiel `ingest_events` einlesen:

```bash
cargo run -p heimlern-core --example ingest_events -- data/samples/aussensensor.jsonl
```

Damit steht für Sensorsamples der gleiche Qualitäts-Check zur Verfügung wie
für Policy-Snapshots.
```

