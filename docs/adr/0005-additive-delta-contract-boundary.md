# ADR-0005: Additive Deltas bleiben außerhalb von `policy.weight_adjustment.v1`

Status: Accepted
Date: 2026-07-08
Bureau task: `OPERATOR-LEARNING-AXIS-V1-T002`

## Kontext

`heimlern` erzeugt Vorschläge zur Gewichtsanpassung, wendet sie aber nicht selbst
an. Der gemeinsame externe Contract dafür ist
`policy.weight_adjustment.v1.schema.json` im Metarepo. Dieser v1-Contract erlaubt
für `deltas` aktuell nur zwei Formen:

- `absolute`: Zielwert setzen;
- `relative`: Prozent- oder Faktoränderung relativ zum aktuellen Wert.

Der Rust-Core kennt zusätzlich `DeltaValue::Additive`. Diese Variante hat
interne Delta-Semantik: der Wert wird rechnerisch auf den aktuellen Parameter
addiert. Sie ist nützlich für Simulationen und alte In-Memory-Tests, ist aber
nicht Teil der validierten v1-Emissionsfläche.

Das Problem: Eine additive Änderung kann fachlich präzise sein, wäre aber in
`policy.weight_adjustment.v1` ein Contract-Bruch. Würde `heimlern` sie trotzdem
emittieren, drifteten Rust-Code, Python-OLA-Probe und Metarepo-Schema auseinander.

## Entscheidung

`additive` bleibt **außerhalb** der contract-bound `policy.weight_adjustment.v1`
Emission.

Für v1 gilt:

1. `heimlern` darf `DeltaValue::Additive` intern für Simulationen und
   Kompatibilität behalten.
2. `heimlern-feedback` und `scripts/ola_probe.py` dürfen additive Deltas nicht
   als v1-Proposals emittieren.
3. OLA-Routingvorschläge bleiben relative Prozent-Deltas, solange sie gegen den
   gepinnten v1-Contract validiert werden.
4. `absolute` darf nicht als umetikettiertes Additiv verwendet werden. Absolute
   Deltas bedeuten Set-to-Semantik.
5. Es wird für T002 **kein** Metarepo-Contract-Follow-up eröffnet, weil die
   aktuelle OLA-Emission ohne additive Deltas auskommt.

## Follow-up-Pfad

Kein unmittelbarer Cross-Repository-Contract-Follow-up ist nötig.

Ein späterer Contract-Follow-up wäre erst gerechtfertigt, wenn mindestens eine
geprüfte Proposal-Quelle additive Semantik wirklich braucht und `relative` oder
`absolute` die Bedeutung nicht sauber ausdrücken. Dann wäre der saubere Pfad:

1. separates Metarepo-Contract-Design für eine v2- oder kompatibel erweiterte
   Delta-Semantik;
2. Consumer-/Producer-Migrationsplan für hausKI, chronik und heimlern;
3. Fixtures für additive, absolute und relative Deltas;
4. erst danach Anpassung der `heimlern`-Emission.

Dieser spätere Pfad darf nicht in T002 stillschweigend begonnen werden.

## Konsequenzen

Positive Folgen:

- Heimlern bleibt schema-konform zum gepinnten Metarepo-v1-Contract.
- Python-OLA-Probe und Rust-Feedback-Pfad bleiben durch
  `scripts/validate_weight_adjustment_coherence.py` gegen denselben Contract
  prüfbar.
- Die nächste OLA-Aufgabe kann sich auf Rust-Invarianten konzentrieren, statt
  gleichzeitig einen Cross-Repo-Contract zu ändern.

Nachteile:

- In-Memory-Simulation und externe Proposal-Emission haben bewusst nicht dieselbe
  Delta-Variantenmenge.
- Wer additive Semantik will, braucht später einen expliziten Contract-Entwurf.

## Prüfbare Evidenz

- `crates/heimlern-feedback/src/lib.rs` dokumentiert `DeltaValue::Additive` als
  simulation-only und emittiert in `propose_adjustment` relative Prozent-Deltas.
- `scripts/ola_probe.py` erzeugt OLA-Vorschläge mit `kind: "relative"` und
  `unit: "percent"`.
- `scripts/validate_weight_adjustment_coherence.py` validiert Rust-Fixture und
  Python-OLA-Proposal gegen dieselbe gepinnte v1-Schema-Datei und blockiert
  additive Python-Probe-Emission.
