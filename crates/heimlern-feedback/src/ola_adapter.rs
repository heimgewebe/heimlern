//! Rust-owned OPERATOR-LEARNING adapter invariants.
//!
//! The Python OLA scripts remain probe and compatibility surfaces. This module
//! keeps the safety-critical adapter transformations in the tested Rust core so
//! route delta key construction, state normalization and reward bounds are not
//! owned primarily by script-local helper functions.

use crate::OutcomeType;

const REWARD_MIN: f32 = -1.0;
const REWARD_MAX: f32 = 1.0;

/// Route delta key validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteDeltaKeyError {
    InvalidActionPrefix,
    EmptyRoute,
}

/// Completion states accepted by the OLA adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionState {
    Completed,
    Blocked,
    Deferred,
    Failed,
    Unknown,
}

/// CI states accepted by the OLA adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiState {
    Pass,
    Fail,
    Pending,
    NotApplicable,
    Unknown,
}

/// Pull request states accepted by the OLA adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrState {
    Merged,
    Open,
    Closed,
    NotApplicable,
    Unknown,
}

/// Numeric reward calculation inputs used by the OLA adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RewardInputs {
    pub completion_state: CompletionState,
    pub friction_count: u8,
    pub unresolved_friction: u8,
    pub blocked_by_platform_filter: bool,
    pub manual_operator_needed: bool,
    pub ci_state: CiState,
    pub pr_state: PrState,
    pub rework_count: u8,
}

/// Convert any route label to the safe key fragment used in v1 delta keys.
#[must_use]
pub fn safe_route(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    let normalized = result.trim_matches(['.', '_', '-']);
    if normalized.is_empty() {
        "unknown_route".to_string()
    } else {
        normalized.to_string()
    }
}

/// Build the contract-bound route weight delta key from a `route.*` action.
pub fn route_delta_key(action: &str) -> Result<(String, String), RouteDeltaKeyError> {
    let Some(route) = action.strip_prefix("route.") else {
        return Err(RouteDeltaKeyError::InvalidActionPrefix);
    };
    if route.is_empty() {
        return Err(RouteDeltaKeyError::EmptyRoute);
    }
    Ok((
        format!("route.{}.weight", safe_route(route)),
        route.to_string(),
    ))
}

/// Clamp finite rewards to the contract boundary and round to three decimals.
#[must_use]
pub fn clamp_reward(value: f32) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    let rounded = (value * 1000.0).round() / 1000.0;
    rounded.clamp(REWARD_MIN, REWARD_MAX)
}

/// Normalize completion state strings with a fail-closed unknown fallback.
#[must_use]
pub fn normalize_completion_state(value: Option<&str>) -> CompletionState {
    match value {
        Some("completed") => CompletionState::Completed,
        Some("blocked") => CompletionState::Blocked,
        Some("deferred") => CompletionState::Deferred,
        Some("failed") => CompletionState::Failed,
        Some("unknown") => CompletionState::Unknown,
        _ => CompletionState::Unknown,
    }
}

/// Normalize CI state strings with a fail-closed unknown fallback.
#[must_use]
pub fn normalize_ci_state(value: Option<&str>) -> CiState {
    match value {
        Some("pass") => CiState::Pass,
        Some("fail") => CiState::Fail,
        Some("pending") => CiState::Pending,
        Some("not_applicable") => CiState::NotApplicable,
        Some("unknown") => CiState::Unknown,
        _ => CiState::Unknown,
    }
}

/// Normalize pull request state strings with a fail-closed unknown fallback.
#[must_use]
pub fn normalize_pr_state(value: Option<&str>) -> PrState {
    match value {
        Some("merged") => PrState::Merged,
        Some("open") => PrState::Open,
        Some("closed") => PrState::Closed,
        Some("not_applicable") => PrState::NotApplicable,
        Some("unknown") => PrState::Unknown,
        _ => PrState::Unknown,
    }
}

/// Classify a normalized OLA completion state into a decision outcome.
#[must_use]
pub fn classify_outcome(completion_state: CompletionState, unresolved_friction: u8) -> OutcomeType {
    match completion_state {
        CompletionState::Completed => OutcomeType::Success,
        CompletionState::Failed => OutcomeType::Failure,
        CompletionState::Blocked | CompletionState::Deferred => OutcomeType::Partial,
        CompletionState::Unknown if unresolved_friction > 0 => OutcomeType::Partial,
        CompletionState::Unknown => OutcomeType::Unknown,
    }
}

/// Compute the bounded OLA reward from normalized adapter inputs.
#[must_use]
pub fn compute_reward(inputs: RewardInputs) -> f32 {
    let mut reward = match inputs.completion_state {
        CompletionState::Completed => 0.7,
        CompletionState::Blocked => -0.35,
        CompletionState::Deferred => -0.1,
        CompletionState::Failed => -0.75,
        CompletionState::Unknown => 0.0,
    };
    reward -= f32::from(inputs.friction_count.min(5)) * 0.05;
    reward -= f32::from(inputs.unresolved_friction.min(5)) * 0.1;
    if inputs.blocked_by_platform_filter {
        reward -= 0.1;
    }
    if inputs.manual_operator_needed {
        reward -= 0.15;
    }
    match inputs.ci_state {
        CiState::Pass => reward += 0.1,
        CiState::Fail => reward -= 0.2,
        CiState::Pending | CiState::NotApplicable | CiState::Unknown => {}
    }
    match inputs.pr_state {
        PrState::Merged => reward += 0.1,
        PrState::Closed => reward -= 0.1,
        PrState::Open | PrState::NotApplicable | PrState::Unknown => {}
    }
    reward -= f32::from(inputs.rework_count.min(5)) * 0.05;
    clamp_reward(reward)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_route_matches_ola_probe_contract() {
        assert_eq!(safe_route(""), "unknown_route");
        assert_eq!(safe_route("direct:patch"), "direct_patch");
        assert_eq!(safe_route("direct/patch/v2"), "direct_patch_v2");
        assert_eq!(safe_route("direct_patch"), "direct_patch");
        assert_eq!(safe_route("foo__bar"), "foo__bar");
        assert_eq!(
            safe_route("route.with.dots-and-dashes"),
            "route.with.dots-and-dashes"
        );
        assert_eq!(safe_route("röute"), "r_ute");
        assert_eq!(safe_route("中"), "unknown_route");
    }

    #[test]
    fn route_delta_key_fails_closed() {
        assert_eq!(
            route_delta_key("route.direct:patch"),
            Ok((
                "route.direct_patch.weight".to_string(),
                "direct:patch".to_string()
            ))
        );
        assert_eq!(
            route_delta_key("direct_patch"),
            Err(RouteDeltaKeyError::InvalidActionPrefix)
        );
        assert_eq!(route_delta_key("route."), Err(RouteDeltaKeyError::EmptyRoute));
    }

    #[test]
    fn reward_is_rounded_and_clamped() {
        assert_eq!(clamp_reward(1.2345), 1.0);
        assert_eq!(clamp_reward(-2.0), -1.0);
        assert_eq!(clamp_reward(0.1236), 0.124);
        assert_eq!(clamp_reward(f32::NAN), 0.0);
    }

    #[test]
    fn states_normalize_with_unknown_fallback() {
        assert_eq!(
            normalize_completion_state(Some("completed")),
            CompletionState::Completed
        );
        assert_eq!(
            normalize_completion_state(Some("bogus")),
            CompletionState::Unknown
        );
        assert_eq!(normalize_ci_state(Some("pass")), CiState::Pass);
        assert_eq!(normalize_ci_state(None), CiState::Unknown);
        assert_eq!(normalize_pr_state(Some("merged")), PrState::Merged);
        assert_eq!(normalize_pr_state(Some("draft")), PrState::Unknown);
    }

    #[test]
    fn outcome_classification_matches_probe_boundary() {
        assert_eq!(
            classify_outcome(CompletionState::Completed, 0),
            OutcomeType::Success
        );
        assert_eq!(
            classify_outcome(CompletionState::Failed, 0),
            OutcomeType::Failure
        );
        assert_eq!(
            classify_outcome(CompletionState::Blocked, 0),
            OutcomeType::Partial
        );
        assert_eq!(
            classify_outcome(CompletionState::Unknown, 1),
            OutcomeType::Partial
        );
        assert_eq!(
            classify_outcome(CompletionState::Unknown, 0),
            OutcomeType::Unknown
        );
    }

    #[test]
    fn compute_reward_matches_documented_probe_formula() {
        let reward = compute_reward(RewardInputs {
            completion_state: CompletionState::Completed,
            friction_count: 1,
            unresolved_friction: 0,
            blocked_by_platform_filter: false,
            manual_operator_needed: false,
            ci_state: CiState::Pass,
            pr_state: PrState::Merged,
            rework_count: 0,
        });
        assert!((reward - 0.85).abs() < f32::EPSILON);

        let blocked = compute_reward(RewardInputs {
            completion_state: CompletionState::Blocked,
            friction_count: 9,
            unresolved_friction: 9,
            blocked_by_platform_filter: true,
            manual_operator_needed: true,
            ci_state: CiState::Fail,
            pr_state: PrState::Closed,
            rework_count: 9,
        });
        assert_eq!(blocked, -1.0);
    }
}
