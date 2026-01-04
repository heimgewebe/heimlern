# heimlern

[![rust (cached)](https://github.com/heimgewebe/heimlern/actions/workflows/rust.yml/badge.svg)](https://github.com/heimgewebe/heimlern/actions/workflows/rust.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Siehe auch: **Policy-Lifecycle**: `docs/policy-lifecycle.md` und **Contracts** in `contracts/`.

`heimlern` ist ein kleines Experimentierfeld für lernfähige Policies im häuslichen Umfeld. Das Repository besteht aus einem schlanken Kern mit gemeinsam genutzten Traits sowie einer Beispiel-Implementierung eines Bandit-Agenten, die zusammen zeigen, wie Erinnerungs-Policies modelliert, ausgeführt und persistiert werden können.

## Inhaltsverzeichnis

1. [Schnellstart](#schnellstart)
2. [Architekturüberblick](#architekturüberblick)
3. [Crates](#crates)
4. [Beispiel ausführen](#beispiel-ausführen)
5. [Weiterführende Dokumentation](#weiterführende-dokumentation)
6. [Installation / Entwicklung](#installation--entwicklung)
7. [Beitragen](#beitragen)
8. [Optional: Telemetrie-Logging](#optional-telemetrie-logging)

## Schnellstart

Voraussetzungen:

* [Rust](https://www.rust-lang.org/tools/install) ab Version 1.70

Repository klonen und Beispiel ausführen:

```bash
git clone https://github.com/heimgewebe/heimlern.git
cd heimlern
cargo run --example integrate_hauski
```

## Architekturüberblick

* **Policies** sind Strategien, die auf Basis eines `Context` Entscheidungen (`Decision`) treffen und über Feedback lernen.
* **Snapshots** sichern und laden den internen Zustand einer Policy als JSON, sodass Agenten zwischen Sessions fortgeführt werden können.
* **Bandit-Implementierungen** kombinieren Exploration (Zufall) und Exploitation (Heuristik), um passende Erinnerungs-Slots zu wählen.

Die zentralen Entwurfsentscheidungen sind in den [Architecture Decision Records](docs/adr/README.md) dokumentiert.

## Crates

| Crate | Zweck |
| --- | --- |
| [`heimlern-core`](crates/heimlern-core) | Definiert die Basistypen `Context`, `Decision` sowie das `Policy`-Trait und beschreibt das JSON-basierte Snapshot-Interface. |
| [`heimlern-bandits`](crates/heimlern-bandits) | Enthält den Beispielagenten `RemindBandit`, der über ε-greedy Exploration Erinnerungs-Slots auswählt. |
| [`heimlern-feedback`](crates/heimlern-feedback) | Retrospektive Feedback-Analyse und Weight-Tuning. Analysiert Entscheidungs-Outcomes und erzeugt auditierbare Gewichtsanpassungsvorschläge. |

## Beispiel ausführen

Das Beispiel `integrate_hauski` simuliert einen einfachen Ablauf: Es erstellt Kontextdaten, ruft den Bandit zur Entscheidungsfindung auf und demonstriert den Snapshot-Mechanismus.

```bash
cargo run --example integrate_hauski
```

Die Ausgabe enthält die gewählte Aktion, den Grund (Exploration oder Heuristik) und den in JSON serialisierten Kontext.

Für schnelle Smoke-Tests des Banditen eignet sich außerdem das Beispiel `decide`. Es liest einen Kontext aus `stdin` (oder verwendet Defaults) und gibt die Entscheidung als JSON im [Policy-Schema](crates/heimlern-core/src/policy.rs) aus:

```bash
echo '{}' | cargo run -p heimlern-bandits --example decide
```

Ersetze `{}` durch einen gewünschten Kontext, um andere Slots oder Heuristiken zu prüfen.

### Beispiel: Außensensor-Events grob scoren

```bash
# kompiliert und liest JSONL aus Datei oder stdin
cargo run -p heimlern-core --example ingest_events -- data/samples/foreign-aussensensor.jsonl
```
Die Ausgabe listet pro Zeile einen Score (0..1) und den Titel (falls vorhanden).

## Tests / CI
`cargo test` führt die Policy-Unit- und Integrationstests aus.

### CI: Validierung von Policy-Entscheidungen
Fixtures unter `tests/fixtures/decision/*.json` werden in CI gegen
`contracts/policy.decision.schema.json` (metarepo `@contracts-v1`) mit **ajv-cli** geprüft.
Ungültige Beispiele lassen die Pipeline fehlschlagen.

## Weiterführende Dokumentation

* [ADR-Index](docs/adr/README.md) – Übersicht und Motivation hinter den Architekturentscheidungen.
* Policy-Lifecycle: `docs/policy-lifecycle.md`
* Inline-Rustdocs in den Crates (`cargo doc --open`) erläutern Strukturen, Traits und das Snapshot-Format im Detail.

## Installation / Entwicklung

### Anforderungen
- Rust (stable)
- Cargo

### Schnellstart
```bash
git clone https://github.com/heimgewebe/heimlern.git
cd heimlern
cargo build --workspace
cargo test  --workspace
```

## Beitragen
Siehe [CONTRIBUTING.md](CONTRIBUTING.md).

## Optional: Telemetrie-Logging

Das Feature `telemetry` aktiviert strukturiertes Logging via [`tracing`](https://docs.rs/tracing).
Ohne das Feature wird zu `stderr` (`eprintln!`) geloggt.

### Aktivieren
```bash
cargo run -p heimlern-bandits --features telemetry --example decide
```

### Beispiel: Subscriber konfigurieren
In einem Binary kann ein einfacher Subscriber gesetzt werden:
```rust
#[cfg(feature = "telemetry")]
{
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();
}
```

## Organismus-Kontext

Dieses Repository ist Teil des **Heimgewebe-Organismus**.

Die übergeordnete Architektur, Achsen, Rollen und Contracts sind zentral beschrieben im  
👉 [`metarepo/docs/heimgewebe-organismus.md`](https://github.com/heimgewebe/metarepo/blob/main/docs/heimgewebe-organismus.md)  
sowie im Zielbild  
👉 [`metarepo/docs/heimgewebe-zielbild.md`](https://github.com/heimgewebe/metarepo/blob/main/docs/heimgewebe-zielbild.md).

Alle Rollen-Definitionen, Datenflüsse und Contract-Zuordnungen dieses Repos
sind dort verankert.
