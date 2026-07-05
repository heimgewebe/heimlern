# heimlern RepoBrief/rLens Audit und Umbauplan v1

Stand: 2026-07-05
Branch: `audit/repobrief-umbau-plan-v1`
Repo-HEAD beim Bundle-Lauf: `e19ae9de7c43c296aa57ae568dc7f48caaa00950`

## 0. Zweck

Dieses Dokument bindet den aktuellen heimlern-Audit an ein frisch erzeugtes RepoBrief/rLens-Bundle und leitet daraus einen engen Umbauplan ab.

RepoBrief/rLens ist hier Evidenz- und Navigationsorgan. Es liefert keine Review-Freigabe, keine Runtime-Korrektheit und keine inhaltliche Wahrheit jenseits des kanonischen Bundle-Inhalts.

## 1. Bundle-Receipt

Erzeugt wurden zwei Bundles unter `/home/alex/repos/merges/`:

- `heimlern-overview-260705-0119_*`
- `heimlern-max-260705-0119_*`

Maßgebliches Audit-Bundle:

- Stem: `heimlern-max-260705-0119`
- Manifest: `/home/alex/repos/merges/heimlern-max-260705-0119_merge.bundle.manifest.json`
- Manifest-SHA256: `8e3ee3607aef7e4e94a56f38cb566e76f151941de78cbd723a49a0dd8ece000e`
- Canonical MD: `/home/alex/repos/merges/heimlern-max-260705-0119_merge.md`
- Canonical MD-SHA256 laut Manifest: `8c234b42cf73e11abf682dde89939e33a56642b4aeaa1a9c689171b69eb418b0`
- Agent Reading Pack: `/home/alex/repos/merges/heimlern-max-260705-0119_merge.agent_reading_pack.md`
- Post-Emit Health: `/home/alex/repos/merges/heimlern-max-260705-0119_merge.bundle_health.post.json`
- Bundle Surface Validation: `/home/alex/repos/merges/heimlern-max-260705-0119_merge.bundle_surface_validation.json`

Bundle-Status:

- `post_emit_health.status = pass`
- `output_health.verdict = pass`
- `range_ref_resolution_status = ok`
- `bundle_surface_validation.status = pass`
- Artifact-Rollen im Manifest: 16
- Generator-Provenance vollständig; Generator-Commit: `a8528e0209d27a2d7283ae20d1e486b6f9ba0e19`; Generator-Tree sauber.

Nicht etabliert durch das Bundle:

- Repo verstanden
- Claims wahr
- Review vollständig
- Runtime korrekt
- Testabdeckung ausreichend
- Regressionsfreiheit
- Forensikbereitschaft

## 2. These / Antithese / Synthese

### These

`heimlern` ist als Lernorgan nützlich: Es kann Entscheidungen, Outcomes, Feedback und Vorschläge voneinander trennen. Der Code besitzt einen stabilen Rust-Kern, nachvollziehbare Tests und eine klare No-Auto-Apply-Philosophie.

### Antithese

Die aktuelle Integration ist nicht tragfähig genug, um `heimlern` schon als aktives Organ einzusetzen. Contract-Drift, rote Metrics-CI, fehlende Claim-Evidence-Registry und fehlende echte chronik/hausKI-End-to-End-Belege machen die operative Nutzbarkeit brüchig.

### Synthese

Nicht erweitern, bevor die Anschlusswahrheit repariert ist. Der erste Umbau muss Contracts, CI und Evidence-Surfaces stabilisieren. Erst danach lohnt ein Policy- oder Bandit-Ausbau.

## 3. Belegte Auditbefunde

### A1 — Bundle ist erzeugt und gesund

Das `max`-Bundle wurde erzeugt und enthält Manifest, Canonical MD, Agent Reading Pack, Citation Map, Chunk Index, SQLite Index, Lens Cards, Concept Cards, Architekturgraph, Entry Points, Output Health, Post-Emit Health und Surface Validation.

Bewertung: gut. Das Repo ist nun rLens-/RepoBrief-fähig inspizierbar.

### A2 — Rust-Kern ist aktuell lokal gesund

Lokal auf heim-pc ausgeführt:

- `cargo test --workspace`: 67 Tests bestanden
- `cargo test --workspace --features telemetry`: 67 Tests bestanden
- `cargo fmt --all --check`: bestanden
- `cargo clippy --all-targets -- -D warnings`: bestanden
- `cargo clippy --all-targets --features telemetry -- -D warnings`: bestanden

Bewertung: guter Code-Gesundheitsbefund, aber keine Systemintegration.

### A3 — Metrics-Workflow ist kaputt durch Schema-Pfad-Drift

GitHub Actions zeigt wiederholte Failures im Workflow `Metrics Snapshot & Validation`. Ursache des geprüften Runs: `curl` auf `https://raw.githubusercontent.com/heimgewebe/metarepo/contracts-v1/contracts/wgx/metrics.json` liefert 404.

Der lokal erzeugte Metrics-Snapshot validiert dagegen gegen `/home/alex/repos/metarepo/contracts/metrics.snapshot.schema.json`.

Bewertung: CI-Fehler ist wahrscheinlich kein Datenfehler, sondern Contract-Pfad-Drift.

### A4 — `policy.weight_adjustment.v1` driftet zwischen Code und Metarepo

`heimlern-feedback` erzeugt Vorschläge mit:

- `version = "0.1.0"`
- `deltas.epsilon.kind = "additive"`
- `reasoning` als Liste

Das Metarepo-Schema `policy.weight_adjustment.v1.schema.json` erwartet im geprüften Stand:

- `version = "v1"`
- Delta-Varianten nur `absolute` oder `relative`
- `reasoning` als String
- bei simulierter Failure-Rate ein gebundenes `simulation_method`

Bewertung: harter Integrationsbruch. Das ist P1.

### A5 — `policy.feedback.schema.json` ist fast leer

Das lokale Schema fordert nur ein JSON-Objekt. Damit ist Validierung grün, aber wenig aussagekräftig.

Bewertung: Scheinvertragsrisiko. Ein Vertrag, der fast alles erlaubt, ist eher ein höflicher Türsteher als ein Gate.

### A6 — Sample-Mapping ist unvollständig

`python3 scripts/validate_json.py --schemas contracts/ --samples data/samples/` validiert drei Sample-Dateien, überspringt aber `foreign-aussensensor.jsonl`, weil kein Mapping definiert ist.

Bewertung: kleine, klare Lücke. P2, aber billig zu schließen.

### A7 — RepoBrief-Claim-Evidence-Map fehlt strukturell

Bundle Surface Validation meldet `claim_evidence_map_json` als abwesend mit maschinenlesbarem Grund `no_registry`: im Quellrepo fehlt `docs/doc-freshness-registry.yml`.

Bewertung: Für einfache Repo-Fragen akzeptabel. Für Roadmap-/Status-/Umbauclaims fehlt ein starkes Evidence-Surface.

### A8 — Dokumentation hat Namensdrift

Es gibt Verweise auf `contracts/policy_snapshot.schema.json`, tatsächlich existiert `contracts/policy.snapshot.schema.json`.

Bewertung: niedriges Risiko, aber symptomatisch für Contract-Namensdrift.

## 4. Resonanz- und Kontrastprüfung

### Deutung 1: heimlern ist bereits brauchbar

Diese Deutung ist teilweise wahr: Die Rust-Crates sind testbar, der Bandit- und Feedback-Kern ist robust, das RepoBrief-Bundle ist gesund.

### Deutung 2: heimlern ist noch nicht anschlussfähig

Diese Deutung ist für Systembetrieb stärker: Ohne Contract-Gleichlauf mit Metarepo und ohne End-to-End-Outcome-Fluss bleibt `heimlern` ein lokales Experiment, kein aktives Lernorgan.

### Einordnung

Für Codequalität: Deutung 1 überwiegt.
Für Organismusnutzen: Deutung 2 überwiegt.

## 5. Epistemische Leere

Folgendes fehlt, nötig für harte Freigaben:

- Echte chronik/hausKI-Outcome-Daten, nötig für die Aussage: `heimlern` lernt aus realen Folgen.
- Ein End-to-End-Test `chronik outcomes -> heimlern proposal -> hausKI review/apply`, nötig für aktive Organrolle.
- Metarepo-Contract-Entscheidung zu `additive` vs. `absolute/relative`, nötig für P1-Fix.
- Claim-Evidence-Registry, nötig für belastbare Roadmap-/Statusclaims im RepoBrief.
- Runtime-/Service-Konzept für heimlern, nötig für Dauerbetrieb statt CLI-Simulation.

## 6. Umbauplan

### PR 1 — Contract Realignment für Weight Adjustments

Ziel: `heimlern-feedback` erzeugt metarepo-kompatible `policy.weight_adjustment.v1`-Vorschläge.

Änderungen:

1. `version` auf `v1` setzen oder Schema bewusst versioniert erweitern.
2. Delta-Semantik entscheiden:
   - Pfad A: `additive` im Metarepo-Schema ergänzen.
   - Pfad B: Code auf vorhandene `absolute`/`relative`-Semantik umbauen.
3. `reasoning` kompatibel machen.
4. Fixture `tests/fixtures/feedback/adjustment.ok.json` gegen Metarepo-Schema validieren.
5. Rust-Test ergänzen: serialisiertes Proposal ist contract-konform.

Empfehlung: Pfad A, weil additive Epsilon-Änderung fachlich präziser ist als ein umetikettiertes `absolute`.

Risiko: Metarepo-Contract-Änderung betrifft Konsumenten. Deshalb zuerst Contract-PR oder synchroner Cross-Repo-Slice.

### PR 2 — Metrics Workflow Repair

Ziel: grüner Scheduled Metrics Workflow.

Änderungen:

1. `METRICS_SCHEMA_URL` auf aktuellen kanonischen Contract ändern oder lokale Contract-Kopie verwenden.
2. Fallback: Wenn Remote-Schema nicht verfügbar ist, fail-closed mit klarer Diagnose, nicht 404-Noise.
3. Optional: Workflow-Name/Artifact `metrics.snapshot` angleichen.

Receipt:

- Workflow `Metrics Snapshot & Validation` läuft grün.
- Lokale Validierung gegen `metarepo/contracts/metrics.snapshot.schema.json` bleibt grün.

### PR 3 — Contract Validation Tightening

Ziel: Scheinvalidierung reduzieren.

Änderungen:

1. `policy.feedback.schema.json` konkretisieren: `feedback_id`, `decision_id`, `reward`, optional `comment`, optionale `ts`.
2. `foreign-aussensensor.jsonl` in `SCHEMA_MAPPING` aufnehmen.
3. Tests für JSONL-Mapping und leere/invalid Feedbacks ergänzen.
4. Doc-Namensdrift `policy_snapshot` -> `policy.snapshot` korrigieren.

Risiko: Bestehende Samples oder externe Produzenten können brechen. Deshalb klein halten und mit Fixtures beginnen.

### PR 4 — RepoBrief Evidence Surface für heimlern

Ziel: Roadmap-/Statusclaims werden besser belegbar.

Änderungen:

1. `docs/doc-freshness-registry.yml` einführen.
2. Wichtige Claims registrieren:
   - Contract Ownership
   - Learning Cycle
   - No Auto-Apply
   - Weight Adjustment Output
   - Chronik Ingest CLI
   - Metrics Workflow
3. Neues RepoBrief-Bundle erzeugen und prüfen: `claim_evidence_map_json` vorhanden.

Risiko: Mehr Pflegeaufwand. Nutzen nur hoch, wenn heimlern regelmäßig auditiert wird.

### PR 5 — End-to-End Learning Smoke

Ziel: Minimaler belegbarer Lernkreis.

Ablauf:

1. Fixture-Outcomes im `decision.outcome.v1`-Format.
2. CLI oder Test erzeugt Proposal.
3. Proposal validiert gegen Metarepo-Contract.
4. Anwendung bleibt simuliert oder review-only; kein Auto-Apply.

Receipt:

- Ein Test oder Script beweist `outcome fixture -> proposal json -> schema pass`.
- Explizite Non-Claims: keine echte hausKI-Anwendung, kein Runtime-Betrieb, keine Lernqualität.

## 7. Reihenfolge

1. PR 1 Contract Realignment
2. PR 2 Metrics Workflow Repair
3. PR 3 Validation Tightening
4. PR 4 RepoBrief Evidence Surface
5. PR 5 End-to-End Learning Smoke

Nicht zuerst bauen:

- neue Bandit-Algorithmen
- dauerlaufender Service
- automatische Policy-Anwendung
- ML/Embedding-Schicht

Begründung: Ohne Contract-Stabilität würde jede Intelligenz nur schneller falsch integriert.

## 8. Nutzen-/Risikoabschätzung

Nutzen:

- heimlern wird als Organismus-Lernorgan anschlussfähig.
- CI-Signal wird wieder wahrer.
- RepoBrief kann künftige Audits besser stützen.
- Grabowski-/Bureau-Outcome-Lernen bekommt ein Zielorgan.

Risiken:

- Cross-Repo-Contract-Änderungen können Konsumenten brechen.
- Zu frühe End-to-End-Automatisierung kann falsches Lernen operationalisieren.
- Zu viel Evidence-Registry kann Dokumentationsballast werden.

Risikoreduktion:

- Kleine PRs.
- Contract-first.
- No-Auto-Apply beibehalten.
- Jede Aussage als Vorschlag/Evidenz markieren, nicht als Freigabe.

## 9. Entscheidung

`heimlern` bleibt nützlich, aber erst nach Umbau der Anschlussflächen. Der beste nächste Griff ist PR 1: Contract Realignment für `policy.weight_adjustment.v1`.
