use heimlern_feedback::{
    DecisionOutcome, DeltaValue, Evidence, FeedbackAnalyzer, OutcomeType, ProposalStatus,
    WeightAdjustmentProposal,
};
use std::collections::HashMap;

fn create_outcome(
    decision_id: &str,
    action: &str,
    success: bool,
    reward: f32,
    strategy: Option<&str>,
) -> DecisionOutcome {
    let metadata = strategy.map(|s| {
        serde_json::json!({
            "why": [s]
        })
    });

    DecisionOutcome {
        decision_id: decision_id.to_string(),
        ts: "2026-01-04T12:00:00Z".to_string(),
        policy_id: Some("test-policy".to_string()),
        action: Some(action.to_string()),
        outcome: if success {
            OutcomeType::Success
        } else {
            OutcomeType::Failure
        },
        success,
        reward: Some(reward),
        context: None,
        metadata,
    }
}

fn additive_epsilon_proposal(value: f32) -> WeightAdjustmentProposal {
    let mut deltas = HashMap::new();
    deltas.insert("epsilon".to_string(), DeltaValue::Additive { value });
    WeightAdjustmentProposal {
        version: "legacy-simulation".to_string(),
        basis_policy: "test".to_string(),
        ts: "2026-01-04T12:00:00Z".to_string(),
        deltas,
        confidence: 0.7,
        evidence: Evidence::default(),
        reasoning: None,
        status: ProposalStatus::Proposed,
    }
}

#[test]
fn simulation_handles_unknown_strategies() {
    let analyzer = FeedbackAnalyzer::default();
    let outcomes: Vec<DecisionOutcome> = (0..10)
        .map(|i| {
            create_outcome(
                &i.to_string(),
                "action",
                i % 2 == 0,
                if i % 2 == 0 { 1.0 } else { 0.0 },
                None,
            )
        })
        .collect();

    let simulated_rate =
        analyzer.simulate_adjustment(&additive_epsilon_proposal(-0.1), &outcomes);
    assert!((simulated_rate - 0.5).abs() < 1e-5);
}

#[test]
fn simulation_handles_mixed_known_unknown() {
    let analyzer = FeedbackAnalyzer::default();
    let mut outcomes = Vec::new();

    for i in 0..10 {
        let is_exploit = i % 2 == 0;
        outcomes.push(create_outcome(
            &i.to_string(),
            "a",
            is_exploit,
            if is_exploit { 1.0 } else { 0.0 },
            Some(if is_exploit { "exploit" } else { "explore ε" }),
        ));
    }

    for i in 10..20 {
        outcomes.push(create_outcome(&i.to_string(), "b", true, 1.0, None));
    }

    let simulated_rate =
        analyzer.simulate_adjustment(&additive_epsilon_proposal(-0.1), &outcomes);
    assert!(
        (simulated_rate - 0.8).abs() < 1e-5,
        "Expected 0.8, got {simulated_rate}"
    );
}
