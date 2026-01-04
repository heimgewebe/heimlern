# Learning Cycle Workflow

Dieses Dokument beschreibt den vollständigen Lernzyklus im Heimgewebe-Organismus zwischen **hausKI** (Entscheider) und **heimlern** (Lerner).

## Übersicht

```
hausKI → Entscheidung → Outcome → heimlern → Analyse → Vorschlag → hausKI → Anwendung
   ↑                                                                              ↓
   └──────────────────────────────────────────────────────────────────────────────┘
```

## Phasen

### 1. Entscheidung (hausKI)

hausKI nutzt eine Policy (z.B. `RemindBandit`) um Entscheidungen zu treffen:

```rust
let decision = policy.decide(&context);
```

Die Entscheidung wird dokumentiert:
- Format: `policy.decision.v1` (siehe `contracts/policy.decision.schema.json`)
- Enthält: Action, Score, Reasoning, Context
- Persistiert: In chronik oder lokalem Speicher

### 2. Outcome-Erfassung (hausKI/chronik)

Nach Ausführung der Entscheidung wird das Ergebnis dokumentiert:

```json
{
  "decision_id": "d123",
  "ts": "2026-01-04T12:00:00Z",
  "policy_id": "remind-bandit-v1",
  "action": "remind.morning",
  "outcome": "failure",
  "success": false,
  "reward": 0.0,
  "context": {...}
}
```

Format: `decision.outcome.v1` (Contract im metarepo: `heimgewebe/metarepo/contracts/decision.outcome.v1.schema.json`)

### 3. Aggregation & Analyse (heimlern)

heimlern konsumiert Outcomes und analysiert retrospektiv:

```rust
use heimlern_feedback::FeedbackAnalyzer;

let analyzer = FeedbackAnalyzer::default();
let outcomes = load_outcomes_from_chronik();

// Aggregiere nach verschiedenen Dimensionen
let by_action = analyzer.aggregate_outcomes(&outcomes, |o| o.action.clone());
let by_context = analyzer.aggregate_outcomes(&outcomes, |o| 
    o.context.and_then(|c| c.get("kind").and_then(|v| v.as_str().map(String::from)))
);

// Identifiziere Muster
let patterns = analyzer.analyze_patterns(&outcomes);
```

**Erkannte Muster (Heuristiken):**
- Wiederholte Fehlentscheidungen bei bestimmten Actions
- Hohe Failure-Rate in bestimmten Kontexten
- Systematische Übergewichtung alter Einträge
- Trust-Level-spezifische Probleme

### 4. Vorschlagsgenerierung (heimlern)

Basierend auf Analyse werden Weight-Adjustments vorgeschlagen:

```rust
if let Some(proposal) = analyzer.propose_adjustment("remind-bandit-v1", &outcomes) {
    // Proposal enthält:
    // - deltas: HashMap<String, DeltaValue>
    // - confidence: f32 (0.0..1.0)
    // - evidence: { decisions_analyzed, failure_rates, patterns }
    // - reasoning: Vec<String>
    // - status: Proposed
    
    // Exportiere als JSON
    let json = serde_json::to_string_pretty(&proposal)?;
    save_to_file("proposals/adjust-001.json", &json)?;
}
```

Format: `policy.weight_adjustment.proposed.v1` (Contract im metarepo: `heimgewebe/metarepo/contracts/policy.weight_adjustment.v1.schema.json`)

### 5. Simulation (heimlern)

Vor der Freigabe wird der Vorschlag simuliert:

```rust
let estimated_success = analyzer.simulate_adjustment(&proposal, &historical_outcomes);

if estimated_success < current_success_rate {
    proposal.status = ProposalStatus::Rejected;
    log_rejection(&proposal, "Simulation zeigt keine Verbesserung");
}
```

### 6. Review & Freigabe (Human/hausKI)

Vorschläge werden geprüft:

**Automatische Checks:**
- Confidence über Threshold (z.B. 0.6)
- Simulation zeigt Verbesserung
- Genug Evidenz (min. Decisions)

**Optionale menschliche Freigabe:**
- Bei Low-Confidence-Proposals
- Bei großen Deltas
- Bei widersprüchlichen Patterns

### 7. Anwendung (hausKI)

Nach Freigabe lädt hausKI die neue Policy-Version:

```rust
// Lade angepasste Policy-Parameter
let adjustment = load_approved_adjustment("proposals/adjust-001.json")?;

// Wende Deltas an
for (key, delta) in adjustment.deltas {
    match key.as_str() {
        "epsilon" => {
            if let DeltaValue::Numeric(d) = delta {
                policy.epsilon = (policy.epsilon + d).clamp(0.0, 1.0);
            }
        }
        // ... weitere Parameter
    }
}

// Persistiere als neuen Snapshot
let new_snapshot = policy.snapshot();
save_policy_version("remind-bandit-v2", &new_snapshot)?;
```

### 8. Monitoring (hausKI/heimlern)

Nach Anwendung wird die Wirksamkeit überwacht:

**Metriken:**
- `learning_cycles_total`: Anzahl Analysezyklen
- `weight_adjustments_proposed_total`: Generierte Vorschläge
- `weight_adjustments_accepted_total`: Angewendete Vorschläge
- `weight_adjustments_rejected_total`: Abgelehnte Vorschläge
- `policy_success_rate_before`: Success-Rate vor Adjustment
- `policy_success_rate_after`: Success-Rate nach Adjustment

**Drift-Detection:**
- Vergleiche aktuelle Performance mit historischer Baseline
- Warne bei unerwarteter Verschlechterung
- Triggere Rollback bei kritischer Drift

## Beispiel-Ablauf

1. **Tag 1-7:** hausKI trifft 100 Entscheidungen, dokumentiert Outcomes
2. **Tag 8:** heimlern analysiert 100 Outcomes
   - Erkennt: 65% Failure-Rate
   - Muster: "High failure rate for remind.morning"
   - Vorschlag: `epsilon: -0.05` (weniger Exploration)
   - Confidence: 0.68
3. **Tag 8:** Simulation zeigt Verbesserung auf 50% Failure-Rate
4. **Tag 8:** Vorschlag wird automatisch freigegeben (Confidence > 0.6)
5. **Tag 9:** hausKI wendet Anpassung an, erstellt `remind-bandit-v2`
6. **Tag 9-15:** hausKI nutzt v2, monitort Performance
7. **Tag 16:** heimlern validiert Wirksamkeit
   - Neue Failure-Rate: 48%
   - Adjustment erfolgreich ✓

## Sicherheitsmechanismen

### 1. No Auto-Apply
heimlern ändert **nie** direkt live Gewichte. Alle Änderungen gehen über Proposals.

### 2. Versionierung
Jeder Snapshot ist versioniert und kann zurückgeladen werden:
```bash
# Rollback bei Problemen
hausKI load_policy "remind-bandit-v1"
```

### 3. Evidence-Threshold
Proposals werden nur bei genug Daten generiert:
- Min. 10-20 Decisions (konfigurierbar)
- Min. Confidence 0.5 (konfigurierbar)

### 4. Simulation-Gate
Vorschläge, die in Simulation schlechter performen, werden automatisch abgelehnt.

### 5. Audit-Trail
Alle Proposals, Freigaben und Ablehnungen werden geloggt:
```
2026-01-08 12:00:00 [heimlern] Proposal generated: adjust-001 (confidence: 0.68)
2026-01-08 12:05:00 [hausKI] Proposal accepted: adjust-001
2026-01-09 08:00:00 [hausKI] Policy updated: remind-bandit-v1 → v2
```

## Fallstricke vermeiden

### ❌ Online-Learning
Nie direkt während der Entscheidung anpassen:
```rust
// FALSCH:
fn decide(&mut self, ctx: &Context) -> Decision {
    let decision = self.compute_decision(ctx);
    self.adjust_weights_immediately(&decision); // ❌ Gefährlich!
    decision
}
```

### ✅ Offline-Learning
Immer retrospektiv und kontrolliert:
```rust
// RICHTIG:
let outcomes = collect_outcomes_over_time();
let proposal = analyzer.propose_adjustment("policy-v1", &outcomes);
review_and_apply(proposal); // Mit Simulation & Gates
```

### ❌ Confidence ohne Evidenz
```rust
// FALSCH:
WeightAdjustmentProposal {
    confidence: 0.9, // Zu hoch!
    evidence: Evidence {
        decisions_analyzed: 3, // Zu wenig!
        ...
    },
    ...
}
```

### ✅ Evidenz-basierte Confidence
```rust
// RICHTIG:
let confidence = calculate_confidence(
    sample_size,
    pattern_strength,
    simulation_improvement
);
```

## Tools

### Analyse-Beispiel ausführen
```bash
cargo run -p heimlern-feedback --example feedback_analysis
```

### Manuelle Proposal-Generierung
```bash
# Outcomes aus chronik laden
chronik export-outcomes --since 7d > outcomes.jsonl

# Analyse durchführen
heimlern-feedback analyze outcomes.jsonl > proposal.json

# Proposal prüfen
cat proposal.json | jq '.confidence, .evidence'
```

## Weiterführend

- [ADR-0003: Decision Feedback Analysis](adr/0003-decision-feedback-analysis.md)
- [Policy Lifecycle](policy-lifecycle.md)
- [Contracts README](../contracts/README.md)
- [heimlern-feedback README](../crates/heimlern-feedback/README.md)

## Philosophie

> hausKI handelt.  
> heimlern bewertet.  
> Und Lernen entsteht erst dort,  
> wo Fehler nicht verborgen, sondern ausgewertet werden.

Lernen ist **zeitversetzt, statistisch und konsequenzsensibel** – genau dafür existiert heimlern.
