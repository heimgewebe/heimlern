# heimlern-feedback

Decision feedback analysis and policy weight tuning for heimlern.

## Overview

`heimlern-feedback` implements retrospective analysis of policy decisions and generates weight adjustment proposals. It follows the core principle:

**heimlern analyzes and proposes, never directly modifies live weights.**

## Key Features

- **Feedback Analysis**: Aggregate and analyze decision outcomes by action, context, and other dimensions
- **Pattern Detection**: Identify systematic issues like repeated failures or biased weighting
- **Weight Adjustment Proposals**: Generate evidence-based adjustment recommendations
- **Simulation**: Test proposed adjustments on historical data before applying
- **Audit Trail**: Full transparency with reasoning, evidence, and confidence scores

## Architecture

### Input Pipeline

heimlern-feedback consumes:
- `decision.outcome.v1` artifacts (from hausKI/chronik)
- Historical decision snapshots for simulation

### Analysis Flow

1. **Aggregation**: Group outcomes by intent, context profile, trust level, etc.
2. **Pattern Detection**: Identify systematic issues using heuristics (no ML initially)
3. **Proposal Generation**: Calculate weight deltas with confidence and evidence
4. **Simulation**: Test proposals on historical data
5. **Export**: Version and serialize as `policy.weight_adjustment.proposed.v1`

### Output

Weight adjustment proposals contain:
- Proposed deltas (e.g., `epsilon: -0.1`)
- Confidence score (0.0 to 1.0)
- Evidence (decisions analyzed, failure rates, patterns)
- Reasoning (human-readable explanations)
- Status (proposed/accepted/rejected/superseded)

## Usage

```rust
use heimlern_feedback::{DecisionOutcome, FeedbackAnalyzer, OutcomeType};

// Create analyzer with default thresholds
let analyzer = FeedbackAnalyzer::default();

// Or customize thresholds
let analyzer = FeedbackAnalyzer::new(
    20,   // min_decisions
    0.6,  // min_confidence
);

// Collect decision outcomes
let outcomes = vec![
    DecisionOutcome {
        decision_id: "d1".to_string(),
        ts: "2026-01-04T12:00:00Z".to_string(),
        policy_id: Some("remind-bandit".to_string()),
        action: Some("remind.morning".to_string()),
        outcome: OutcomeType::Failure,
        success: false,
        reward: Some(0.0),
        context: None,
        metadata: None,
    },
    // ... more outcomes
];

// Analyze and propose adjustments
if let Some(proposal) = analyzer.propose_adjustment("remind-bandit-v1", &outcomes) {
    println!("Confidence: {:.2}", proposal.confidence);
    println!("Evidence: {} decisions analyzed", proposal.evidence.decisions_analyzed);
    
    // Serialize to JSON (contract format)
    let json = serde_json::to_string_pretty(&proposal)?;
    
    // Simulate before applying
    let estimated_success = analyzer.simulate_adjustment(&proposal, &outcomes);
    println!("Estimated improvement: {:.1}%", estimated_success * 100.0);
}
```

## Example

Run the feedback analysis example:

```bash
cargo run -p heimlern-feedback --example feedback_analysis
```

This demonstrates:
- Outcome aggregation and statistics
- Pattern identification
- Proposal generation with evidence
- Simulation of proposed adjustments
- JSON serialization for hausKI consumption

## Safety Guarantees

1. **No Direct Weight Modification**: heimlern only proposes, never applies
2. **Versioned Proposals**: Every adjustment is versioned and traceable
3. **Evidence-Based**: All proposals backed by analyzed decisions
4. **Simulation First**: Test before applying via historical replay
5. **Audit Trail**: Full logging of proposals, acceptances, and rejections

## Observability

Metrics to track:
- `learning_cycles_total`: Number of analysis cycles completed
- `weight_adjustments_proposed_total`: Total proposals generated
- `weight_adjustments_accepted_total`: Proposals approved by hausKI
- `weight_adjustments_rejected_total`: Proposals rejected

Logs include:
- Pattern detection reasoning
- Proposal generation details
- Simulation results
- Acceptance/rejection decisions

## Contracts

See [contracts/](../../contracts/) for JSON schemas:
- `decision.outcome.schema.json`: Input format for outcomes
- `policy.weight_adjustment.schema.json`: Output format for proposals

## Design Philosophy

From the issue description:

> hausKI handelt.  
> heimlern bewertet.  
> Und Lernen entsteht erst dort,  
> wo Fehler nicht verborgen, sondern ausgewertet werden.

Learning is:
- **Time-delayed**: Outcomes observed over time
- **Statistical**: Based on aggregated evidence
- **Consequence-sensitive**: Driven by real decision outcomes
- **Auditable**: Every adjustment is transparent and reversible

## Future Enhancements

- Meta-learning: Detect when NOT to learn
- Advanced simulation: Replay with modified weights
- Context-aware adjustments: Different deltas per context
- Drift detection: Identify when policies diverge from intended behavior
