use std::io::{self, Read};

use heimlern_bandits::RemindBandit;
use heimlern_core::{Chosen, Context, Decision, Policy};
use serde::Serialize;
use serde_json::{json, Value};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Serialize)]
struct PolicyDecisionRecord {
    ts: String,
    policy_id: String,
    policy: String,
    context: Context,
    decision: Decision,
}

fn iso8601_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let ctx = if input.trim().is_empty() {
        Context {
            kind: "reminder".into(),
            features: json!({}),
        }
    } else {
        match serde_json::from_str::<Context>(&input) {
            Ok(ctx) => ctx,
            Err(_) => match serde_json::from_str::<Value>(&input) {
                Ok(Value::Object(mut obj)) => {
                    let kind = obj
                        .remove("kind")
                        .and_then(|v| v.as_str().map(std::borrow::ToOwned::to_owned))
                        .unwrap_or_else(|| "reminder".to_string());
                    let features = obj.remove("features").unwrap_or_else(|| json!({}));
                    Context { kind, features }
                }
                Ok(Value::String(kind)) => Context {
                    kind,
                    features: json!({}),
                },
                _ => Context {
                    kind: input.trim().into(),
                    features: json!({}),
                },
            },
        }
    };

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
