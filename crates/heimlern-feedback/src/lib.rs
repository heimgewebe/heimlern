#![warn(clippy::unwrap_used, clippy::expect_used)]

//! Decision feedback analysis and policy weight tuning.
//!
//! This crate implements retrospective analysis of policy decisions and
//! generates weight adjustment proposals. It follows the principle:
//! **heimlern analyzes and proposes, never directly modifies live weights**.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

// Confidence calculation constants
/// Sample size at which confidence plateaus (smaller = more generous)
const CONFIDENCE_SAMPLE_SIZE_PLATEAU: f32 = 50.0;
/// Confidence level when 2+ patterns detected (high confidence)
const CONFIDENCE_HIGH_PATTERN: f32 = 0.7;
/// Confidence level when <2 patterns detected (moderate confidence)
const CONFIDENCE_LOW_PATTERN: f32 = 0.5;
/// Weight for sample size component in confidence calculation
const CONFIDENCE_SAMPLE_WEIGHT: f32 = 0.4;
/// Weight for pattern count component in confidence calculation
const CONFIDENCE_PATTERN_WEIGHT: f32 = 0.6;

// Simulation constants
/// Placeholder improvement estimate for simulations (15% improvement)
/// TODO: Replace with actual replay-based simulation
const SIMULATION_ESTIMATED_IMPROVEMENT: f32 = 0.15;

// Pattern detection thresholds
/// Minimum number of decisions for a specific action before analyzing patterns
const PATTERN_MIN_DECISIONS_PER_ACTION: usize = 5;
/// Failure rate threshold (60%) above which a pattern is flagged
const PATTERN_HIGH_FAILURE_THRESHOLD: f32 = 0.6;
/// Overall failure rate threshold (50%) for system-wide issues
const PATTERN_OVERALL_FAILURE_THRESHOLD: f32 = 0.5;

// Adjustment thresholds
/// Failure rate threshold (50%) that triggers exploration reduction
const ADJUSTMENT_FAILURE_THRESHOLD: f32 = 0.5;
/// Amount to reduce epsilon when failure rate is high
const ADJUSTMENT_EPSILON_DELTA: f32 = -0.05;

// Fallback constants
/// Fallback timestamp when formatting fails
const FALLBACK_TIMESTAMP: &str = "1970-01-01T00:00:00Z";

/// Outcome of a policy decision, used for retrospective analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutcome {
    /// Reference to the original decision
    pub decision_id: String,
    /// Timestamp when the outcome was recorded
    pub ts: String,
    /// Policy that made the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    /// Action that was taken
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    /// Classification of the outcome
    pub outcome: OutcomeType,
    /// Whether the decision was successful.
    ///
    /// For [`OutcomeType::Success`] and [`OutcomeType::Failure`], this should be
    /// consistent with `outcome`. For [`OutcomeType::Partial`] and
    /// [`OutcomeType::Unknown`], this flag drives success classification.
    pub success: bool,
    /// Numeric reward signal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reward: Option<f32>,
    /// Context in which the decision was made
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Classification of decision outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutcomeType {
    Success,
    Failure,
    Partial,
    Unknown,
}

/// Evidence supporting a weight adjustment proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Number of decisions analyzed
    pub decisions_analyzed: usize,
    /// Failure rate with current weights
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_rate_before: Option<f32>,
    /// Simulated failure rate with proposed weights
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_rate_after_sim: Option<f32>,
    /// Method used for simulation (e.g., "placeholder", "replay", "monte_carlo")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub simulation_method: Option<String>,
    /// Identified patterns that led to this proposal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patterns: Option<Vec<String>>,
}

/// Proposed weight adjustments based on decision feedback analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightAdjustmentProposal {
    /// Version of the proposal format
    pub version: String,
    /// Identifier of the base policy being adjusted
    pub basis_policy: String,
    /// Timestamp when the proposal was generated
    pub ts: String,
    /// Proposed weight adjustments as key-value pairs
    pub deltas: HashMap<String, DeltaValue>,
    /// Confidence in the proposed adjustments (0.0 to 1.0)
    pub confidence: f32,
    /// Evidence supporting the proposal
    pub evidence: Evidence,
    /// Human-readable explanations for the adjustments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<Vec<String>>,
    /// Current status of this proposal
    #[serde(default)]
    pub status: ProposalStatus,
}

/// Value type for weight deltas with explicit kind and unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum DeltaValue {
    /// Absolute numeric adjustment
    #[serde(rename = "absolute")]
    Absolute { value: f32 },
    /// Relative percentage adjustment
    #[serde(rename = "relative")]
    Relative { value: f32, unit: String },
}

/// Status of a weight adjustment proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ProposalStatus {
    #[default]
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

/// Statistics aggregated from decision outcomes.
#[derive(Debug, Default, Clone)]
pub struct OutcomeStatistics {
    /// Total number of outcomes (successes + failures).
    pub total: usize,
    pub successes: usize,
    pub failures: usize,
    pub total_reward: f32,
}

impl OutcomeStatistics {
    /// Calculate success rate (0.0 to 1.0).
    #[must_use]
    pub fn success_rate(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            self.successes as f32 / self.total as f32
        }
    }

    /// Calculate failure rate (0.0 to 1.0).
    #[must_use]
    pub fn failure_rate(&self) -> f32 {
        debug_assert!(
            self.successes + self.failures == self.total,
            "OutcomeStatistics totals are inconsistent"
        );
        if self.total == 0 {
            return 0.0;
        }
        1.0 - self.success_rate()
    }

    /// Calculate average reward.
    #[must_use]
    pub fn average_reward(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        #[allow(clippy::cast_precision_loss)]
        {
            self.total_reward / self.total as f32
        }
    }
}

/// Analyzes decision outcomes and generates weight adjustment proposals.
#[derive(Debug)]
pub struct FeedbackAnalyzer {
    /// Minimum number of decisions required before proposing adjustments
    min_decisions: usize,
    /// Minimum confidence threshold for proposals
    min_confidence: f32,
}

impl Default for FeedbackAnalyzer {
    fn default() -> Self {
        Self {
            min_decisions: 10,
            min_confidence: 0.5,
        }
    }
}

impl FeedbackAnalyzer {
    /// Create a new feedback analyzer with custom thresholds.
    #[must_use]
    pub fn new(min_decisions: usize, min_confidence: f32) -> Self {
        Self {
            min_decisions,
            min_confidence: min_confidence.clamp(0.0, 1.0),
        }
    }

    /// Aggregate outcomes by a grouping key (e.g., action, context type).
    #[must_use]
    pub fn aggregate_outcomes(
        &self,
        outcomes: &[DecisionOutcome],
        key_fn: impl Fn(&DecisionOutcome) -> Option<String>,
    ) -> HashMap<String, OutcomeStatistics> {
        let mut stats: HashMap<String, OutcomeStatistics> = HashMap::new();

        for outcome in outcomes {
            if let Some(key) = key_fn(outcome) {
                let entry = stats.entry(key).or_default();
                entry.total += 1;
                if outcome_is_success(outcome) {
                    entry.successes += 1;
                } else {
                    entry.failures += 1;
                }
                if let Some(reward) = outcome.reward {
                    if reward.is_finite() {
                        entry.total_reward += reward;
                    }
                }
            }
        }

        stats
    }

    fn summarize_outcomes(&self, outcomes: &[DecisionOutcome]) -> OutcomeStatistics {
        let mut stats = OutcomeStatistics::default();

        for outcome in outcomes {
            stats.total += 1;
            if outcome_is_success(outcome) {
                stats.successes += 1;
            } else {
                stats.failures += 1;
            }
            if let Some(reward) = outcome.reward {
                if reward.is_finite() {
                    stats.total_reward += reward;
                }
            }
        }

        stats
    }

    /// Analyze outcomes and identify patterns requiring weight adjustments.
    ///
    /// This is a heuristic-based analysis (not ML-based initially).
    #[must_use]
    pub fn analyze_patterns(&self, outcomes: &[DecisionOutcome]) -> Vec<String> {
        let mut patterns = Vec::new();

        if outcomes.len() < self.min_decisions {
            return patterns;
        }

        // Aggregate by action
        let by_action = self.aggregate_outcomes(outcomes, |o| o.action.clone());

        // Pattern 1: Repeated failures for specific actions
        for (action, stats) in &by_action {
            if stats.total >= PATTERN_MIN_DECISIONS_PER_ACTION
                && stats.failure_rate() > PATTERN_HIGH_FAILURE_THRESHOLD
            {
                patterns.push(format!(
                    "High failure rate ({:.1}%) for action '{}'",
                    stats.failure_rate() * 100.0,
                    action
                ));
            }
        }

        // Pattern 2: Overall poor performance
        let overall_stats = self.summarize_outcomes(outcomes);

        if overall_stats.total >= self.min_decisions
            && overall_stats.failure_rate() > PATTERN_OVERALL_FAILURE_THRESHOLD
        {
            patterns.push(format!(
                "Overall failure rate is high ({:.1}%)",
                overall_stats.failure_rate() * 100.0
            ));
        }

        patterns
    }

    /// Generate a weight adjustment proposal based on analyzed outcomes.
    ///
    /// Returns `None` if insufficient data or confidence is too low.
    #[must_use]
    pub fn propose_adjustment(
        &self,
        basis_policy: &str,
        outcomes: &[DecisionOutcome],
    ) -> Option<WeightAdjustmentProposal> {
        if outcomes.len() < self.min_decisions {
            return None;
        }

        let patterns = self.analyze_patterns(outcomes);
        if patterns.is_empty() {
            return None;
        }

        let overall_stats = self.summarize_outcomes(outcomes);

        // Calculate confidence based on sample size and consistency
        #[allow(clippy::cast_precision_loss)]
        let confidence = {
            let sample_confidence =
                (outcomes.len() as f32 / CONFIDENCE_SAMPLE_SIZE_PLATEAU).min(1.0);
            let pattern_confidence = if patterns.len() >= 2 {
                CONFIDENCE_HIGH_PATTERN
            } else {
                CONFIDENCE_LOW_PATTERN
            };
            (sample_confidence * CONFIDENCE_SAMPLE_WEIGHT
                + pattern_confidence * CONFIDENCE_PATTERN_WEIGHT)
                .clamp(0.0, 1.0)
        };

        if confidence < self.min_confidence {
            return None;
        }

        // Generate heuristic deltas
        let mut deltas = HashMap::new();
        let mut reasoning = Vec::new();

        // If overall failure rate is high, suggest reducing exploration
        if overall_stats.failure_rate() > ADJUSTMENT_FAILURE_THRESHOLD {
            deltas.insert(
                "epsilon".to_string(),
                DeltaValue::Absolute {
                    value: ADJUSTMENT_EPSILON_DELTA,
                },
            );
            reasoning.push("Reduce exploration due to high failure rate".to_string());
        }

        // Simulate improvement (placeholder - real simulation would replay decisions)
        let failure_rate_after_sim =
            (overall_stats.failure_rate() - SIMULATION_ESTIMATED_IMPROVEMENT).max(0.0);

        Some(WeightAdjustmentProposal {
            version: "0.1.0".to_string(),
            basis_policy: basis_policy.to_string(),
            ts: iso8601_now(),
            deltas,
            confidence,
            evidence: Evidence {
                decisions_analyzed: outcomes.len(),
                failure_rate_before: Some(overall_stats.failure_rate()),
                failure_rate_after_sim: Some(failure_rate_after_sim),
                simulation_method: Some("placeholder_constant".to_string()),
                patterns: Some(patterns),
            },
            reasoning: Some(reasoning),
            status: ProposalStatus::Proposed,
        })
    }

    /// Simulate applying proposed adjustments to historical outcomes.
    ///
    /// Returns estimated success rate with the proposed adjustments.
    /// This is a simplified simulation - a real implementation would replay
    /// decisions with modified weights.
    #[must_use]
    pub fn simulate_adjustment(
        &self,
        _proposal: &WeightAdjustmentProposal,
        outcomes: &[DecisionOutcome],
    ) -> f32 {
        if outcomes.is_empty() {
            return 0.0;
        }

        // Simple simulation: calculate baseline success rate
        let successes = outcomes.iter().filter(|o| outcome_is_success(o)).count();
        #[allow(clippy::cast_precision_loss)]
        let baseline = successes as f32 / outcomes.len() as f32;

        // Estimate improvement using placeholder constant
        // TODO: Replace with actual replay-based simulation
        (baseline + SIMULATION_ESTIMATED_IMPROVEMENT).min(1.0)
    }
}

fn outcome_is_success(outcome: &DecisionOutcome) -> bool {
    match outcome.outcome {
        OutcomeType::Success => {
            debug_assert!(outcome.success, "Success outcome marked as unsuccessful");
            true
        }
        OutcomeType::Failure => {
            debug_assert!(!outcome.success, "Failure outcome marked as successful");
            false
        }
        OutcomeType::Partial | OutcomeType::Unknown => outcome.success,
    }
}

fn iso8601_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| FALLBACK_TIMESTAMP.to_string())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn create_outcome(
        decision_id: &str,
        action: &str,
        success: bool,
        reward: f32,
    ) -> DecisionOutcome {
        DecisionOutcome {
            decision_id: decision_id.to_string(),
            ts: iso8601_now(),
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
            metadata: None,
        }
    }

    #[test]
    fn outcome_statistics_calculates_rates_correctly() {
        let stats = OutcomeStatistics {
            total: 10,
            successes: 7,
            failures: 3,
            total_reward: 5.0,
        };

        #[allow(clippy::float_cmp)]
        {
            assert_eq!(stats.success_rate(), 0.7);
            assert_eq!(stats.failure_rate(), 0.3);
            assert_eq!(stats.average_reward(), 0.5);
        }
    }

    #[test]
    fn outcome_statistics_handles_empty_set() {
        let stats = OutcomeStatistics::default();

        #[allow(clippy::float_cmp)]
        {
            assert_eq!(stats.success_rate(), 0.0);
            assert_eq!(stats.failure_rate(), 0.0);
            assert_eq!(stats.average_reward(), 0.0);
        }
    }

    #[test]
    fn analyzer_aggregates_outcomes_by_action() {
        let analyzer = FeedbackAnalyzer::default();
        let outcomes = vec![
            create_outcome("1", "remind.morning", true, 1.0),
            create_outcome("2", "remind.morning", false, 0.0),
            create_outcome("3", "remind.evening", true, 1.0),
        ];

        let by_action = analyzer.aggregate_outcomes(&outcomes, |o| o.action.clone());

        assert_eq!(by_action.len(), 2);
        let morning_stats = by_action
            .get("remind.morning")
            .expect("morning stats should exist");
        assert_eq!(morning_stats.total, 2);
        assert_eq!(morning_stats.successes, 1);
    }

    #[test]
    fn analyzer_identifies_high_failure_patterns() {
        let analyzer = FeedbackAnalyzer::default();
        let outcomes: Vec<DecisionOutcome> = (0..10)
            .map(|i| create_outcome(&i.to_string(), "remind.night", false, 0.0))
            .collect();

        let patterns = analyzer.analyze_patterns(&outcomes);

        assert!(!patterns.is_empty());
        assert!(patterns.iter().any(|p| p.contains("High failure rate")));
    }

    #[test]
    fn analyzer_uses_outcomes_without_action_for_overall_stats() {
        let analyzer = FeedbackAnalyzer::default();
        let mut outcomes: Vec<DecisionOutcome> = (0..9)
            .map(|i| DecisionOutcome {
                decision_id: i.to_string(),
                ts: iso8601_now(),
                policy_id: Some("test-policy".to_string()),
                action: None,
                outcome: OutcomeType::Failure,
                success: false,
                reward: None,
                context: None,
                metadata: None,
            })
            .collect();

        outcomes.push(create_outcome("10", "remind.morning", true, 1.0));

        let patterns = analyzer.analyze_patterns(&outcomes);

        assert!(patterns
            .iter()
            .any(|p| p.contains("Overall failure rate is high")));
    }

    #[test]
    fn analyzer_requires_minimum_decisions() {
        let analyzer = FeedbackAnalyzer::new(10, 0.5);
        let outcomes = vec![
            create_outcome("1", "remind.morning", false, 0.0),
            create_outcome("2", "remind.morning", false, 0.0),
        ];

        let proposal = analyzer.propose_adjustment("test-policy", &outcomes);
        assert!(proposal.is_none());
    }

    #[test]
    fn analyzer_generates_proposal_with_sufficient_data() {
        let analyzer = FeedbackAnalyzer::new(10, 0.5);
        let outcomes: Vec<DecisionOutcome> = (0..15)
            .map(|i| {
                let success = i % 3 == 0; // 33% success rate
                create_outcome(
                    &i.to_string(),
                    "remind.morning",
                    success,
                    if success { 1.0 } else { 0.0 },
                )
            })
            .collect();

        let proposal = analyzer.propose_adjustment("test-policy", &outcomes);
        assert!(proposal.is_some());

        let proposal = proposal.expect("proposal should exist");
        assert_eq!(proposal.basis_policy, "test-policy");
        assert_eq!(proposal.evidence.decisions_analyzed, 15);
        assert!(proposal.confidence >= 0.5);
    }

    #[test]
    fn proposal_serializes_to_valid_json() {
        let proposal = WeightAdjustmentProposal {
            version: "0.1.0".to_string(),
            basis_policy: "test-policy".to_string(),
            ts: iso8601_now(),
            deltas: {
                let mut map = HashMap::new();
                map.insert("epsilon".to_string(), DeltaValue::Absolute { value: -0.1 });
                map
            },
            confidence: 0.68,
            evidence: Evidence {
                decisions_analyzed: 100,
                failure_rate_before: Some(0.42),
                failure_rate_after_sim: Some(0.31),
                simulation_method: None,
                patterns: Some(vec!["Test pattern".to_string()]),
            },
            reasoning: Some(vec!["Test reasoning".to_string()]),
            status: ProposalStatus::Proposed,
        };

        let json = serde_json::to_string_pretty(&proposal).expect("should serialize");
        assert!(json.contains("test-policy"));
        assert!(json.contains("epsilon"));

        // Verify it can be deserialized
        let _deserialized: WeightAdjustmentProposal =
            serde_json::from_str(&json).expect("should deserialize");
    }

    #[test]
    fn simulation_returns_reasonable_improvement() {
        let analyzer = FeedbackAnalyzer::default();
        let outcomes: Vec<DecisionOutcome> = (0..20)
            .map(|i| {
                let success = i % 2 == 0; // 50% success rate
                create_outcome(
                    &i.to_string(),
                    "action",
                    success,
                    if success { 1.0 } else { 0.0 },
                )
            })
            .collect();

        let proposal = WeightAdjustmentProposal {
            version: "0.1.0".to_string(),
            basis_policy: "test".to_string(),
            ts: iso8601_now(),
            deltas: HashMap::new(),
            confidence: 0.7,
            evidence: Evidence {
                decisions_analyzed: 20,
                failure_rate_before: None,
                failure_rate_after_sim: None,
                simulation_method: None,
                patterns: None,
            },
            reasoning: None,
            status: ProposalStatus::Proposed,
        };

        let simulated_rate = analyzer.simulate_adjustment(&proposal, &outcomes);
        assert!(simulated_rate > 0.5); // Should show improvement
        assert!(simulated_rate <= 1.0);
    }

    #[test]
    fn fixtures_decision_outcome_deserializes() {
        // Test that the example fixture format works
        let json = r#"{
            "decision_id": "d123",
            "ts": "2026-01-04T12:00:00Z",
            "policy_id": "remind-bandit-v1",
            "action": "remind.morning",
            "outcome": "failure",
            "success": false,
            "reward": 0.0
        }"#;

        let outcome: DecisionOutcome =
            serde_json::from_str(json).expect("should deserialize outcome");
        assert_eq!(outcome.decision_id, "d123");
        assert!(!outcome.success);
        assert_eq!(outcome.outcome, OutcomeType::Failure);
    }

    #[test]
    fn fixtures_weight_adjustment_deserializes() {
        // Test that the example fixture format works with both delta types
        let json = r#"{
            "version": "0.1.0",
            "basis_policy": "remind-bandit-v1",
            "ts": "2026-01-04T12:00:00Z",
            "deltas": {
                "epsilon": {
                    "kind": "absolute",
                    "value": -0.05
                },
                "recency.half_life": {
                    "kind": "relative",
                    "value": -20.0,
                    "unit": "percent"
                }
            },
            "confidence": 0.68,
            "evidence": {
                "decisions_analyzed": 143,
                "failure_rate_before": 0.42,
                "failure_rate_after_sim": 0.31
            },
            "status": "proposed"
        }"#;

        let proposal: WeightAdjustmentProposal =
            serde_json::from_str(json).expect("should deserialize proposal");
        assert_eq!(proposal.basis_policy, "remind-bandit-v1");
        assert!((proposal.confidence - 0.68).abs() < 1e-6);
        assert_eq!(proposal.evidence.decisions_analyzed, 143);
        assert_eq!(proposal.status, ProposalStatus::Proposed);

        // Verify both delta types deserialize correctly
        assert_eq!(proposal.deltas.len(), 2);
        if let Some(DeltaValue::Absolute { value }) = proposal.deltas.get("epsilon") {
            assert!((value + 0.05).abs() < 1e-6);
        } else {
            panic!("Expected Absolute delta for epsilon");
        }
        if let Some(DeltaValue::Relative { value, unit }) = proposal.deltas.get("recency.half_life")
        {
            assert!((value + 20.0).abs() < 1e-6);
            assert_eq!(unit, "percent");
        } else {
            panic!("Expected Relative delta for recency.half_life");
        }
    }

    #[test]
    fn fixtures_full_adjustment_file_deserializes() {
        // Test that the actual fixture file deserializes correctly
        let json = include_str!("../../../tests/fixtures/feedback/adjustment.ok.json");
        let proposal: WeightAdjustmentProposal =
            serde_json::from_str(json).expect("should deserialize fixture");

        assert_eq!(proposal.basis_policy, "remind-bandit-v1");
        assert_eq!(proposal.deltas.len(), 2);
        assert!(proposal.reasoning.as_ref().is_some_and(|r| r.len() >= 2));
        assert!(proposal
            .evidence
            .patterns
            .as_ref()
            .is_some_and(|p| p.len() >= 2));
    }
}
