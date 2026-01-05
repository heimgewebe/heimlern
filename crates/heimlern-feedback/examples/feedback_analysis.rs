//! Example demonstrating feedback analysis and weight-tuning proposal generation.
//!
//! This example shows how heimlern analyzes decision outcomes retrospectively
//! and proposes weight adjustments without directly modifying live weights.
//!
//! Run with: cargo run -p heimlern-feedback --example feedback_analysis

use heimlern_feedback::{DecisionOutcome, FeedbackAnalyzer, OutcomeType};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== heimlern: Decision Feedback Analysis ===\n");

    // Simulate decision outcomes from hausKI
    let outcomes = vec![
        // Morning reminders mostly failed
        create_outcome("d1", "remind.morning", false, 0.0),
        create_outcome("d2", "remind.morning", false, 0.1),
        create_outcome("d3", "remind.morning", false, 0.0),
        create_outcome("d4", "remind.morning", false, 0.0),
        create_outcome("d5", "remind.morning", false, 0.2),
        create_outcome("d6", "remind.morning", false, 0.1),
        // Evening reminders also mostly failed
        create_outcome("d7", "remind.evening", false, 0.2),
        create_outcome("d8", "remind.evening", true, 0.9),
        create_outcome("d9", "remind.evening", false, 0.3),
        create_outcome("d10", "remind.evening", false, 0.1),
        create_outcome("d11", "remind.evening", false, 0.2),
        // Afternoon reminders mixed but mostly failing
        create_outcome("d12", "remind.afternoon", true, 0.7),
        create_outcome("d13", "remind.afternoon", false, 0.2),
        create_outcome("d14", "remind.afternoon", false, 0.1),
        create_outcome("d15", "remind.afternoon", false, 0.0),
    ];

    println!("📊 Analyzing {} decision outcomes...\n", outcomes.len());

    // Create analyzer with default thresholds
    let analyzer = FeedbackAnalyzer::default();

    // Aggregate outcomes by action
    println!("📈 Statistics by action:");
    let by_action = analyzer.aggregate_outcomes(&outcomes, |o| o.action.clone());
    for (action, stats) in &by_action {
        println!(
            "  {} → success: {}/{} ({:.1}%), avg reward: {:.2}",
            action,
            stats.successes,
            stats.total,
            stats.success_rate() * 100.0,
            stats.average_reward()
        );
    }
    println!();

    // Identify patterns
    println!("🔍 Identified patterns:");
    let patterns = analyzer.analyze_patterns(&outcomes);
    if patterns.is_empty() {
        println!("  (none detected with current thresholds)");
    } else {
        for pattern in &patterns {
            println!("  • {}", pattern);
        }
    }
    println!();

    // Generate weight adjustment proposal
    println!("💡 Generating weight adjustment proposal...");
    match analyzer.propose_adjustment("remind-bandit-v1", &outcomes) {
        Some(proposal) => {
            println!("\n✅ Proposal generated:");
            println!("  Policy: {}", proposal.basis_policy);
            println!("  Confidence: {:.2}", proposal.confidence);
            println!("  Status: {:?}", proposal.status);
            println!("\n  Deltas:");
            for (key, value) in &proposal.deltas {
                println!("    {}: {:?}", key, value);
            }
            println!("\n  Evidence:");
            println!(
                "    Decisions analyzed: {}",
                proposal.evidence.decisions_analyzed
            );
            if let Some(rate) = proposal.evidence.failure_rate_before {
                println!("    Failure rate (before): {:.1}%", rate * 100.0);
            }
            if let Some(rate) = proposal.evidence.failure_rate_after_sim {
                println!("    Failure rate (after sim): {:.1}%", rate * 100.0);
            }
            if let Some(patterns) = &proposal.evidence.patterns {
                println!("    Patterns:");
                for p in patterns {
                    println!("      • {}", p);
                }
            }
            if let Some(reasoning) = &proposal.reasoning {
                println!("\n  Reasoning:");
                for r in reasoning {
                    println!("    • {}", r);
                }
            }

            // Serialize to JSON (contract format)
            println!("\n📄 Proposal as JSON (policy.weight_adjustment.proposed.v1):");
            let json = serde_json::to_string_pretty(&proposal)?;
            println!("{}", json);

            // Simulate the adjustment
            println!("\n🔬 Simulating adjustment on historical data...");
            let simulated_success = analyzer.simulate_adjustment(&proposal, &outcomes);
            println!(
                "  Estimated success rate with adjustments: {:.1}%",
                simulated_success * 100.0
            );
        }
        None => {
            println!("\n⚠️  Insufficient data or confidence for proposal");
            println!("  (requires sufficient decisions with detectable patterns)");
        }
    }

    println!("\n=== Analysis complete ===");
    println!("\nℹ️  Note: heimlern proposes adjustments but does NOT apply them.");
    println!("   hausKI reviews and applies approved policy snapshots.");

    Ok(())
}

fn create_outcome(id: &str, action: &str, success: bool, reward: f32) -> DecisionOutcome {
    use time::{format_description::well_known::Rfc3339, OffsetDateTime};

    const FALLBACK_TIMESTAMP: &str = "1970-01-01T00:00:00Z";
    let ts = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| FALLBACK_TIMESTAMP.to_string());

    DecisionOutcome {
        decision_id: id.to_string(),
        ts,
        policy_id: Some("remind-bandit-v1".to_string()),
        action: Some(action.to_string()),
        outcome: if success {
            OutcomeType::Success
        } else {
            OutcomeType::Failure
        },
        success,
        reward: Some(reward),
        context: None,
        metadata: None,
    }
}
