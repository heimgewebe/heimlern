use std::io::{self, Read};

use heimlern_bandits::RemindBandit;
use heimlern_core::{Chosen, Context, Decision, Policy};
use serde::Serialize;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn iso8601_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn parse_context(input: &str) -> Context {
    if input.trim().is_empty() {
        return Context {
            kind: "reminder".into(),
            features: json!({}),
        };
    }

    if let Ok(ctx) = serde_json::from_str::<Context>(input) {
        return ctx;
    }

    if let Ok(Value::Object(mut obj)) = serde_json::from_str::<Value>(input) {
        let kind = obj
            .remove("kind")
            .and_then(|v| v.as_str().map(std::borrow::ToOwned::to_owned))
            .unwrap_or_else(|| "reminder".to_string());

        let features = match obj.remove("features") {
            Some(value) => value,
            None if obj.is_empty() => json!({}),
            None => Value::Object(obj),
        };

        return Context { kind, features };
    }

    if let Ok(Value::String(kind)) = serde_json::from_str::<Value>(input) {
        return Context {
            kind,
            features: json!({}),
        };
    }

    Context {
        kind: input.trim().into(),
        features: json!({}),
    }
}

#[derive(Serialize)]
struct PolicyDecisionRecord {
    ts: String,
    policy_id: String,
    policy: String,
    context: Context,
    decision: Decision,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let ctx = parse_context(&input);

    let mut policy = RemindBandit::default();
    let mut decision = policy.decide(&ctx);

    // Populate the 'chosen' field for strict schema compliance
    if decision.chosen.is_none() {
        decision.chosen = Some(Chosen {
            action: decision.action.clone(),
        });
    }

    let record = PolicyDecisionRecord {
        ts: iso8601_now(),
        policy_id: "remind-bandit".to_string(), // Matches RemindBandit snapshot ID
        policy: "heimlern-bandits".to_string(),
        context: ctx.clone(),
        decision,
    };

    serde_json::to_writer_pretty(io::stdout(), &record)?;
    println!();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_remaining_fields_as_features_when_no_features_key() {
        let ctx = parse_context(r#"{"kind":"reminder","foo":1,"bar":"baz"}"#);

        assert_eq!(ctx.kind, "reminder");
        assert_eq!(ctx.features["foo"], 1); // preserved from top-level map
        assert_eq!(ctx.features["bar"], "baz");
    }

    #[test]
    fn prefers_explicit_features_over_remaining_fields() {
        let ctx = parse_context(r#"{"kind":"custom","features":{"x":true},"foo":1}"#);

        assert_eq!(ctx.kind, "custom");
        assert_eq!(ctx.features["x"], true); // features key wins
        assert_eq!(ctx.features.get("foo"), None); // not duplicated
    }
}
