# heimlern Optimierungsplan v1

Stand: 2026-07-06
Branch: `claude/heimlern-optimization-plan-jh0f3o`
Repo-HEAD bei Analyse: `9fef180`

## 0. Zweck und Abgrenzung

Dieses Dokument leitet aus dem **aktuellen, verifizierten Repo-Zustand** einen
engen Optimierungsplan ab. Es baut auf dem
[RepoBrief-Audit v1](heimlern-repobrief-audit-umbauplan-v1.md) auf, ersetzt
dessen Befundlage aber dort, wo sie inzwischen überholt ist.

Optimierung heißt hier **nicht** Feature-Ausbau. Sie heißt: Anschlusswahrheit
härten, Redundanz und Drift-Risiko senken, die eine echte Funktionslücke
schließen und bewusste Build-/Performance-Entscheidungen treffen. Die
`No-Auto-Apply`-Philosophie und die Rolle als reines
`learning_proposal_engine` bleiben unangetastet.

## 1. Verifizierter Ist-Zustand (2026-07-06)

Lokal auf der aktuellen Toolchain (`cargo 1.94.1`, `rustc 1.94.1`) geprüft:

| Prüfung | Ergebnis |
| --- | --- |
| `cargo test --workspace` | grün (67 Tests) |
| `cargo fmt --all --check` | grün |
| `cargo clippy --all-targets` | grün, keine Warnungen |

CI (`.github/workflows/rust.yml`) härtet bereits:

* `cargo fmt --all --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all --locked --workspace`
* Smoke: `decide`-Example
* Sample-Validierung gegen `contracts/`
* OPLEARN-Adapter-/Probe-Self-Tests
* **Fixture-Validierung gegen das gepinnte Metarepo-Contract**
  `policy.weight_adjustment.v1.schema.json`

Damit ist die Code-Gesundheit sehr gut. Die verbleibenden Optimierungen liegen
in Vertragsschärfe, Redundanz, einer Funktionslücke und bewussten Tuning-Entscheidungen.

## 2. Delta zum Audit v1 (was ist erledigt, was offen)

| Audit-Befund | Status heute | Beleg |
| --- | --- | --- |
| A3 Metrics-Workflow durch Schema-Pfad-Drift kaputt | **erledigt** | `metrics.yml` nutzt gepinnte `METRICS_SCHEMA_URL` (Commit-SHA statt `contracts-v1`) |
| A4 `policy.weight_adjustment.v1` driftet | **für Emission/CI stabilisiert; additive-Vertragsfrage offen** | `version="v1"`, `reasoning: String`, Delta-Varianten `absolute`/`relative`/`additive`; CI validiert Fixture gegen Metarepo-Contract. **Aber:** Bureau führt `OPERATOR-LEARNING-AXIS-V1-T002` („Decide additive contract follow-up path", state `planned`) noch offen — siehe §2.1 |
| A6 Sample-Mapping unvollständig | **erledigt** | `foreign-aussensensor.jsonl` ist in `SCHEMA_MAPPING` gemappt |
| A8 Doc-Namensdrift `policy_snapshot` | **effektiv erledigt** | Verweis existiert nur noch im Audit-Doc selbst |
| **A5 `policy.feedback.schema.json` fast leer** | **offen** | Schema ist nur `{"type":"object"}` — akzeptiert alles |
| **A7 Claim-Evidence-Registry fehlt** | **offen** | `docs/doc-freshness-registry.yml` existiert nicht |

Neu seit v1 (nicht im Audit adressiert):

* OPLEARN-Friction-Adapter und -Probe in **Python** (`scripts/ola_adapter.py`,
  `scripts/ola_probe.py`).
* Operator-Routing-Contracts (`operator.routing_decision.v1`,
  `operator.routing_outcome.v1`).
* Operator-Role-Contract-Overlay in den Docs.

### 2.1 Offene Anschlusswahrheit: A4 ist nicht vollständig abgeschlossen

A4 ist **für die aktuelle Emission und CI stabilisiert** — der Code erzeugt
`v1`-konforme Proposals und die CI validiert sie gegen das gepinnte
Metarepo-Contract. Die **grundsätzliche `additive`-Vertragsfrage** ist damit
aber **nicht** geschlossen: Bureau führt weiterhin den Task

> `OPERATOR-LEARNING-AXIS-V1-T002` — „Decide additive contract follow-up path"
> (state `planned`). Akzeptanz: Entscheidung, ob `additive`-Deltas in den
> Shared Contract aufgenommen oder aus der contract-bound Emission
> herausgehalten werden.

Solange dieser Task offen steht, gilt: **entweder** ist Bureau veraltet und
T002 muss als erledigt/obsolet markiert werden, **oder** der Abschluss von A4
wird überschätzt. Dieser Plan behauptet deshalb bewusst **nicht**, dass A4
endgültig erledigt ist. Der Abgleich mit Bureau (T002 schließen oder explizit
offen halten) ist Vorbedingung, bevor irgendeine additive-bezogene
Contract-Änderung erfolgt — siehe Reihenfolge §4.

## 3. Optimierungsachsen

### O1 — Vertragsschärfe: `policy.feedback.schema.json` (Audit A5)

**Befund.** Das Schema ist `{"type":"object"}`. Jede Validierung dagegen ist
grün, aber inhaltsleer — ein „höflicher Türsteher".

**Optimierung.** Schema auf die real produzierten Felder konkretisieren:
`feedback_id`, `decision_id`, `reward`, optional `comment`, optional `ts`;
`required` und `additionalProperties: false` bewusst setzen. Bestehendes
Sample `data/samples/policy.feedback.sample.json` als Positiv-Fixture
verankern, ein Negativ-Fixture ergänzen.

**Risiko.** Externe Produzenten könnten brechen. Deshalb klein halten,
zuerst mit Fixtures, `additionalProperties` erst nach Sichtung realer Records
schließen.

**Receipt.** `validate_json.py` lehnt ein invalides Feedback ab; gültiges
Sample bleibt grün; CI unverändert grün.

### O2 — Redundanz im Feedback-Kern senken

Drei konkrete, testgedeckte Vereinfachungen in
`crates/heimlern-feedback/src/lib.rs`:

1. **Doppelte Zähllogik.** `collect_strategy_stats` wiederholt den
   `total/successes/failures`-Increment-Block dreimal; `aggregate_outcomes`
   und `summarize_outcomes` wiederholen ihn erneut. Ein
   `OutcomeStatistics::record(is_success, reward)` bündelt das an einer Stelle.

2. **Sim-Logik doppelt gepflegt.** `propose_adjustment` leitet
   `failure_rate_after_sim` über einen eigenen Inline-`match` auf
   `Relative { percent | factor }` her — eine Teilkopie von
   `simulate_adjustment`. Fällt die Delta-Art künftig anders aus (z. B.
   `absolute`), rechnet `propose_adjustment` still am Simulator vorbei. Fix:
   `simulate_adjustment` als einzige Sim-Quelle aufrufen.

3. **`summarize_outcomes` ⊂ `aggregate_outcomes`.** `summarize_outcomes` ist
   der Spezialfall „ein Bucket". Über einen konstanten Key ausdrückbar oder
   klar als dünner Wrapper markieren.

**Receipt.** Netto weniger Code, identisches Verhalten, alle 67 Tests grün;
ein neuer Test fixiert, dass `propose_adjustment` und `simulate_adjustment`
für dasselbe Proposal denselben Wert liefern.

### O3 — Funktionslücke schließen: End-to-End-Lernpfad als Rust-CLI

**Befund.** Die CLI (`heimlern-cli`) kann heute **nur** `ingest`. Der
eigentliche Lernpfad `outcomes → proposal → schema-validate` existiert nur in
Rust-Unit-Tests und — parallel — in `scripts/ola_probe.py`. Es gibt kein
Kommando, das den Kreis als belegbares Artefakt schließt (Audit-PR 5 offen).

**Optimierung.** Subcommand `heimlern propose`:
liest `DecisionOutcome`-JSONL, ruft `FeedbackAnalyzer::propose_adjustment`,
gibt ein `policy.weight_adjustment.v1`-Proposal auf stdout aus. Review-only,
kein Auto-Apply. Ein Integrationstest beweist
`outcome-fixture → proposal-json → schema pass` gegen das gepinnte
Metarepo-Contract.

**Nutzen.** Ein einziger, auditierbarer Lernpfad in Rust; Grundlage, um die
Python-Doppelung (O4) später abzulösen.

### O4 — Python/Rust-Kohärenz: Doppel-Implementierung von „Proposal-Emission"

**Befund.** „Outcomes → `weight_adjustment`-Proposal" existiert zweimal:
in `heimlern-feedback` (Rust) und in `scripts/ola_probe.py` (Python,
`--emit proposal`). Zwei Implementierungen derselben Contract-Semantik driften
erfahrungsgemäß auseinander.

**Optimierung (nach O3).** Entscheidung dokumentieren und umsetzen:
Pfad A — Python-Probe ruft die Rust-CLI (`heimlern propose`) als Backend;
Pfad B — Python bleibt eigenständig, aber ein gemeinsamer Contract-Test
prüft beide Ausgaben gegen dasselbe Schema **und** gegeneinander.
Empfehlung: Pfad A, sobald O3 steht. Bis dahin mindestens Pfad B als
Drift-Wächter.

### O5 — Build-/Performance-Profil bewusst wählen

**Befund.** `Cargo.toml` setzt `[profile.release] opt-level = "z"` (Optimierung
auf **Codegröße**) zusammen mit `lto = true`, `codegen-units = 1`. Für ein
Analyse-Organ, das viele `DecisionOutcome`s replayt/reweightet, ist `"z"` eine
Geschwindigkeits-für-Größe-Wette, die nie explizit begründet wurde.

**Optimierung.** Keine Blindänderung. Stattdessen `criterion`-Benchmark auf
Basis des vorhandenen `bench_feedback`-Examples, dann `opt-level` `"z"` vs
`"s"` vs `3` messen. Wahl dokumentieren (ADR-Kurznotiz). Bei kleinen
Binaries für Edge-Deploy kann `"z"` korrekt bleiben — dann bewusst so belassen.

**Receipt.** Reproduzierbare Benchmark-Zahl + einzeiliger ADR mit Entscheidung.

### O6 — Evidence-Surface: Claim-Registry (Audit A7)

**Befund.** `docs/doc-freshness-registry.yml` fehlt; RepoBrief meldet
`claim_evidence_map_json = no_registry`.

**Optimierung.** Minimal-Registry mit den tragenden Claims (Contract Ownership,
Learning Cycle, No-Auto-Apply, Weight-Adjustment-Output, Chronik-Ingest,
Metrics-Workflow). Bewusst schlank halten — nur Claims, die wirklich auditiert
werden, sonst wird die Registry selbst Ballast.

**Priorität.** Niedriger als O1–O3; nur sinnvoll, wenn heimlern regelmäßig
per RepoBrief auditiert wird.

## 4. Reihenfolge und Schnitt (PR-groß)

| # | Titel | Kern | Aufwand | Abhängigkeit |
| --- | --- | --- | --- | --- |
| PR 1 | Feedback-Schema härten | O1 | S | — |
| PR 2 | Feedback-Kern entdoppeln | O2 | S–M | — |
| PR 3 | `heimlern propose` CLI + E2E-Test | O3 | M | PR 2 hilfreich |
| PR 4 | Build-Profil-Benchmark + ADR | O5 | S | — |
| PR 5 | Python/Rust-Kohärenz | O4 | M | PR 3 |
| PR 6 | Claim-Evidence-Registry | O6 | S | — |

**Empfohlene Reihenfolge — sequenziell, nicht gebündelt:**

1. **O1 allein zuerst** (PR 1). Billig, schließt A5, hoher Sicherheitsgewinn.
   Contract-Härtung wird **nicht** mit einem Refactor in denselben
   Implementierungsgriff gemischt — sonst vermischen sich Verhaltens- und
   Vertragsänderung in einem Diff und der Blast-Radius wird unnötig groß.
2. **Dann O2** (PR 2), rein testgedeckte Simplification, isoliert reviewbar.
3. **Dann O3** (PR 3), der größte strukturelle Nutzen.

Vor jeder additive-bezogenen Contract-Änderung steht der Bureau-Abgleich zu
`OPERATOR-LEARNING-AXIS-V1-T002` (§2.1).

## 5. Nicht-Ziele (bewusst nicht zuerst)

* Neue Bandit-Algorithmen oder ML-/Embedding-Schicht.
* Dauerlaufender Service statt CLI-Aufruf.
* Automatische Policy-Anwendung (`No-Auto-Apply` bleibt).
* Änderung der Scoring-Semantik ohne Version-Bump (siehe `.ai-context.yml`).

## 6. Risiko und Gegenmaßnahme

| Risiko | Gegenmaßnahme |
| --- | --- |
| Schema-Härtung bricht externe Produzenten (O1) | Fixture-first, `additionalProperties` gestuft schließen |
| Refactor ändert Simulationswerte (O2) | Gleichheits-Test `propose == simulate`, alle 67 Tests als Regressionsnetz |
| Zwei Proposal-Pfade driften weiter (O4) | Gemeinsamer Contract-Test als Drift-Wächter, bis Pfad A steht |
| Blinde Profil-Umstellung verschlechtert Größe/Speed (O5) | Erst messen, dann entscheiden, ADR-dokumentiert |

## 7. Entscheidung

heimlern ist code-seitig gesund und contract-seitig deutlich stabiler als zum
Audit v1. Der beste nächste Griff ist **O1 allein** (PR 1, Vertragsschärfe
schließt A5) — **nicht** gebündelt mit O2. Danach **O2** (Entdopplung senkt
Drift-Risiko im Kern) und **O3** (schließt den Lernkreis erstmals als
auditierbares Rust-Artefakt).

A4 gilt als für Emission/CI stabilisiert, **nicht** als endgültig erledigt:
die additive-Vertragsfrage bleibt an Bureau `OPERATOR-LEARNING-AXIS-V1-T002`
gebunden (§2.1) und ist dort zu schließen oder explizit offen zu halten,
bevor eine additive-bezogene Contract-Änderung erfolgt.
