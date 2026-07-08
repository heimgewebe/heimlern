//! Operator Learning Axis adapter invariants.
//!
//! This module is the Rust-owned contract for OLA adapter normalization. Python
//! scripts may call into it as probes or wrappers, but route-key sanitizing,
//! reward clamping and state normalization live here.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fmt;

pub const OUTCOME_VERSION: &str = "operator.routing_outcome.v1";
pub const DEFAULT_POLICY_ID: &str = "grabowski-routing-v0";
pub const VALID_COMPLETION_STATES: &[&str] =
    &["completed", "blocked", "deferred", "failed", "unknown"];
pub const VALID_CI_STATES: &[&str] = &["pass", "fail", "pending", "not_applicable", "unknown"];
pub const VALID_PR_STATES: &[&str] = &["merged", "open", "closed", "not_applicable", "unknown"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteDeltaKeyErrorKind {
    RouteDeltaKeyInvalid,
    RouteDeltaKeyCollision,
}

impl RouteDeltaKeyErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RouteDeltaKeyInvalid => "route_delta_key_invalid",
            Self::RouteDeltaKeyCollision => "route_delta_key_collision",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteDeltaKeyError {
    kind: RouteDeltaKeyErrorKind,
    message: String,
}

impl RouteDeltaKeyError {
    pub fn new(kind: RouteDeltaKeyErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn kind(&self) -> RouteDeltaKeyErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RouteDeltaKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for RouteDeltaKeyError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteDeltaKey {
    pub delta_key: String,
    pub route: String,
}

pub fn is_allowed_delta_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-')
}

pub fn safe_route(value: &str) -> String {
    let mapped: String = value
        .chars()
        .map(|ch| {
            if is_allowed_delta_key_char(ch) {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = mapped.trim_matches(|ch| matches!(ch, '.' | '_' | '-'));
    if trimmed.is_empty() {
        "unknown_route".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn route_delta_key(action: &str) -> Result<RouteDeltaKey, RouteDeltaKeyError> {
    let route = action.strip_prefix("route.").ok_or_else(|| {
        RouteDeltaKeyError::new(
            RouteDeltaKeyErrorKind::RouteDeltaKeyInvalid,
            format!("route action must start with 'route.': {action:?}"),
        )
    })?;

    if route.is_empty() {
        return Err(RouteDeltaKeyError::new(
            RouteDeltaKeyErrorKind::RouteDeltaKeyInvalid,
            format!("route action has empty route: {action:?}"),
        ));
    }

    Ok(RouteDeltaKey {
        delta_key: format!("route.{}.weight", safe_route(route)),
        route: route.to_string(),
    })
}

pub fn clamp_reward(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    ((value.clamp(-1.0, 1.0) * 1000.0).round()) / 1000.0
}

pub fn normalized_state(value: Option<&str>, allowed: &[&str], fallback: &str) -> String {
    match value {
        Some(candidate) if allowed.contains(&candidate) => candidate.to_string(),
        _ => fallback.to_string(),
    }
}

pub fn bool_from(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Bool(true)))
}

fn string_field(input: &Value, key: &str, fallback: &str) -> String {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn optional_limited_string(input: &Value, key: &str, max_chars: usize) -> Option<String> {
    input
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(max_chars).collect())
}

pub fn normalized_friction(raw_items: Option<&Value>) -> Vec<Value> {
    let Some(items) = raw_items.and_then(Value::as_array) else {
        return Vec::new();
    };

    let mut output = Vec::new();
    for raw in items.iter().filter(|item| item.is_object()) {
        let mut item = Map::new();
        item.insert(
            "kind".to_string(),
            Value::String(string_field(raw, "kind", "unknown")),
        );
        item.insert(
            "surface".to_string(),
            Value::String(string_field(raw, "surface", "unknown")),
        );
        item.insert(
            "resolved".to_string(),
            Value::Bool(bool_from(raw.get("resolved"))),
        );
        if let Some(operation) = optional_limited_string(raw, "operation", 160) {
            item.insert("operation".to_string(), Value::String(operation));
        }
        if let Some(fallback) = optional_limited_string(raw, "fallback", 500) {
            item.insert("fallback".to_string(), Value::String(fallback));
        }
        output.push(Value::Object(item));
    }
    output
}

pub fn classify_outcome(completion_state: &str, unresolved_friction: usize) -> String {
    match completion_state {
        "completed" => "success".to_string(),
        "failed" => "failure".to_string(),
        "blocked" | "deferred" => "partial".to_string(),
        _ if unresolved_friction > 0 => "partial".to_string(),
        _ => "unknown".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn compute_reward(
    completion_state: &str,
    friction_count: usize,
    unresolved_friction: usize,
    blocked_by_platform_filter: bool,
    manual_operator_needed: bool,
    ci_state: &str,
    pr_state: &str,
    rework_count: i64,
) -> f64 {
    let mut reward = match completion_state {
        "completed" => 0.7,
        "blocked" => -0.35,
        "deferred" => -0.1,
        "failed" => -0.75,
        _ => 0.0,
    };
    reward -= friction_count.min(5) as f64 * 0.05;
    reward -= unresolved_friction.min(5) as f64 * 0.1;
    if blocked_by_platform_filter {
        reward -= 0.1;
    }
    if manual_operator_needed {
        reward -= 0.15;
    }
    match ci_state {
        "pass" => reward += 0.1,
        "fail" => reward -= 0.2,
        _ => {}
    }
    match pr_state {
        "merged" => reward += 0.1,
        "closed" => reward -= 0.1,
        _ => {}
    }
    reward -= rework_count.clamp(0, 5) as f64 * 0.05;
    clamp_reward(reward)
}

pub fn adapt(input_record: &Value) -> Value {
    let friction = normalized_friction(input_record.get("friction"));
    let friction_count = friction.len();
    let unresolved_friction = friction
        .iter()
        .filter(|item| item.get("resolved") != Some(&Value::Bool(true)))
        .count();
    let completion_state = normalized_state(
        input_record.get("completion_state").and_then(Value::as_str),
        VALID_COMPLETION_STATES,
        "unknown",
    );
    let ci_state = normalized_state(
        input_record.get("ci_state").and_then(Value::as_str),
        VALID_CI_STATES,
        "unknown",
    );
    let pr_state = normalized_state(
        input_record.get("pr_state").and_then(Value::as_str),
        VALID_PR_STATES,
        "unknown",
    );
    let blocked_by_platform_filter = friction
        .iter()
        .any(|item| item.get("kind").and_then(Value::as_str) == Some("platform_filter"));
    let manual_operator_needed = bool_from(input_record.get("manual_operator_needed"))
        || friction
            .iter()
            .any(|item| item.get("kind").and_then(Value::as_str) == Some("user_input"));
    let rework_count = input_record
        .get("rework_count")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let mut metrics = Map::new();
    metrics.insert(
        "completion_state".to_string(),
        Value::String(completion_state.clone()),
    );
    metrics.insert("friction_count".to_string(), json!(friction_count));
    metrics.insert(
        "blocked_by_platform_filter".to_string(),
        Value::Bool(blocked_by_platform_filter),
    );
    metrics.insert(
        "manual_operator_needed".to_string(),
        Value::Bool(manual_operator_needed),
    );
    metrics.insert("ci_state".to_string(), Value::String(ci_state.clone()));
    metrics.insert("pr_state".to_string(), Value::String(pr_state.clone()));
    metrics.insert("rework_count".to_string(), json!(rework_count));

    if let Some(elapsed) = input_record.get("elapsed_seconds").and_then(Value::as_i64) {
        if elapsed >= 0 {
            metrics.insert("elapsed_seconds".to_string(), json!(elapsed));
        }
    }

    let outcome = classify_outcome(&completion_state, unresolved_friction);
    json!({
        "version": OUTCOME_VERSION,
        "decision_id": string_field(input_record, "decision_id", "unknown-decision"),
        "ts": string_field(input_record, "ts", "unknown-ts"),
        "task_class": string_field(input_record, "task_class", "unknown_task"),
        "route_used": string_field(input_record, "route_used", "unknown_route"),
        "outcome": outcome,
        "resolved": completion_state == "completed" && unresolved_friction == 0,
        "reward": compute_reward(
            &completion_state,
            friction_count,
            unresolved_friction,
            blocked_by_platform_filter,
            manual_operator_needed,
            &ci_state,
            &pr_state,
            rework_count,
        ),
        "metrics": Value::Object(metrics),
        "friction": friction,
        "does_not_establish": [
            "causal_route_superiority",
            "routing_policy_readiness",
            "auto_apply_permission"
        ]
    })
}

pub fn to_decision_outcome(routing_outcome: &Value, policy_id: &str) -> Value {
    let outcome = string_field(routing_outcome, "outcome", "unknown");
    let success = match outcome.as_str() {
        "success" => true,
        "partial" => bool_from(routing_outcome.get("resolved")),
        _ => false,
    };
    let route_used = string_field(routing_outcome, "route_used", "unknown_route");
    json!({
        "decision_id": string_field(routing_outcome, "decision_id", "unknown-decision"),
        "ts": string_field(routing_outcome, "ts", "unknown-ts"),
        "policy_id": policy_id,
        "action": format!("route.{route_used}"),
        "outcome": outcome,
        "success": success,
        "reward": routing_outcome.get("reward").cloned().unwrap_or(Value::Null),
        "context": {
            "task_class": routing_outcome.get("task_class").cloned().unwrap_or(Value::Null),
            "route_used": route_used
        },
        "metadata": {
            "source_version": routing_outcome.get("version").cloned().unwrap_or(Value::Null),
            "metrics": routing_outcome.get("metrics").cloned().unwrap_or_else(|| json!({})),
            "friction": routing_outcome.get("friction").cloned().unwrap_or_else(|| json!([])),
            "does_not_establish": [
                "routing_policy_readiness",
                "automatic_rule_change_permission"
            ]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn safe_route_matches_legacy_cases() {
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
        let route_key = match route_delta_key("route.direct:patch") {
            Ok(route_key) => route_key,
            Err(err) => panic!("unexpected route key error: {err}"),
        };
        assert_eq!(route_key.delta_key, "route.direct_patch.weight");

        let no_prefix = match route_delta_key("direct_patch") {
            Ok(route_key) => panic!("unexpected route key: {route_key:?}"),
            Err(err) => err,
        };
        assert_eq!(
            no_prefix.kind(),
            RouteDeltaKeyErrorKind::RouteDeltaKeyInvalid
        );

        let empty = match route_delta_key("route.") {
            Ok(route_key) => panic!("unexpected route key: {route_key:?}"),
            Err(err) => err,
        };
        assert_eq!(empty.kind(), RouteDeltaKeyErrorKind::RouteDeltaKeyInvalid);
    }

    #[test]
    fn reward_clamping_and_state_normalization_are_core_contracts() {
        assert_eq!(clamp_reward(2.5), 1.0);
        assert_eq!(clamp_reward(-2.5), -1.0);
        assert_eq!(clamp_reward(0.12349), 0.123);
        assert_eq!(
            normalized_state(Some("completed"), VALID_COMPLETION_STATES, "unknown"),
            "completed"
        );
        assert_eq!(
            normalized_state(Some("bogus"), VALID_COMPLETION_STATES, "unknown"),
            "unknown"
        );
        assert_eq!(
            normalized_state(None, VALID_COMPLETION_STATES, "unknown"),
            "unknown"
        );
    }

    #[test]
    fn adapt_normalizes_routing_outcome() {
        let input = json!({
            "decision_id": "gr-example-001",
            "ts": "2026-07-08T18:00:00Z",
            "task_class": "contract_slice",
            "route_used": "typed_tool",
            "completion_state": "blocked",
            "ci_state": "unknown",
            "pr_state": "open",
            "friction": [{
                "kind": "platform_filter",
                "surface": "chat_tool",
                "operation": "bounded_write",
                "resolved": false,
                "fallback": "narrowed scope and stopped before mutation"
            }]
        });

        let outcome = adapt(&input);
        assert_eq!(outcome["version"], OUTCOME_VERSION);
        assert_eq!(outcome["outcome"], "partial");
        assert_eq!(outcome["metrics"]["friction_count"], 1);
        assert_eq!(outcome["metrics"]["blocked_by_platform_filter"], true);
        assert_eq!(outcome["reward"].as_f64().unwrap_or_default(), -0.6);
    }

    #[test]
    fn decision_outcome_uses_rust_normalized_route_context() {
        let routing = json!({
            "version": OUTCOME_VERSION,
            "decision_id": "gr-example-001",
            "ts": "2026-07-08T18:00:00Z",
            "task_class": "contract_slice",
            "route_used": "direct:patch",
            "outcome": "success",
            "resolved": true,
            "reward": 0.8,
            "metrics": {},
            "friction": []
        });
        let decision = to_decision_outcome(&routing, DEFAULT_POLICY_ID);
        assert_eq!(decision["policy_id"], DEFAULT_POLICY_ID);
        assert_eq!(decision["action"], "route.direct:patch");
        assert!(decision["success"].as_bool().unwrap_or_default());
    }
}
