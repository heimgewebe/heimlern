use heimlern_feedback::DecisionOutcome;
use heimlern_feedback::DeltaValue;
use heimlern_feedback::Evidence;
use heimlern_feedback::FeedbackAnalyzer;
use heimlern_feedback::OutcomeType;
use heimlern_feedback::ProposalStatus;
use heimlern_feedback::WeightAdjustmentProposal;
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
            let success = i % 2 == 0;
            let reward = if success { 1.0 } else { 0.0 };
            create_outcome(&i.to_string(), "action", success, reward, None)
        })
        .collect();

    let proposal = additive_epsilon_proposal(-0.1);
    let simulated_rate = analyzer.simulate_adjustment(&proposal, &outcomes);
    assert!((simulated_rate - 0.5).abs() < 1e-5);
}

#[test]
fn simulation_handles_mixed_known_unknown() {
    let analyzer = FeedbackAnalyzer::default();
    let mut outcomes = Vec::new();

    for i in 0..10 {
        let is_exploit = i % 2 == 0;
        let strategy = if is_exploit { "exploit" } else { "explore ε" };
        let reward = if is_exploit { 1.0 } else { 0.0 };
        outcomes.push(create_outcome(
            &i.to_string(),
            "a",
            is_exploit,
            reward,
            Some(strategy),
        ));
    }

    for i in 10..20 {
        outcomes.push(create_outcome(&i.to_string(), "b", true, 1.0, None));
    }

    let proposal = additive_epsilon_proposal(-0.1);
    let simulated_rate = analyzer.simulate_adjustment(&proposal, &outcomes);
    assert!((simulated_rate - 0.8).abs() < 1e-5);
}
