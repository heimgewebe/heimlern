# heimlern

`heimlern` ist ein kleines Experimentierfeld für lernfähige Policies im häuslichen Umfeld. Das Repository besteht aus einem schlanken Kern mit gemeinsam genutzten Traits sowie einer Beispiel-Implementierung eines Bandit-Agenten, die zusammen zeigen, wie Erinnerungs-Policies modelliert, ausgeführt und persistiert werden können.

## Inhaltsverzeichnis

1. [Schnellstart](#schnellstart)
2. [Architekturüberblick](#architekturüberblick)
3. [Crates](#crates)
4. [Beispiel ausführen](#beispiel-ausführen)
5. [Weiterführende Dokumentation](#weiterführende-dokumentation)

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

## Weiterführende Dokumentation

* [ADR-Index](docs/adr/README.md) – Übersicht und Motivation hinter den Architekturentscheidungen.
* Inline-Rustdocs in den Crates (`cargo doc --open`) erläutern Strukturen, Traits und das Snapshot-Format im Detail.
