use std::io::{self, Read};

use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let ctx = if input.trim().is_empty() {
        Context {
            kind: "reminder".into(),
            features: json!({}),
        }
    } else {
        serde_json::from_str::<Context>(&input).or_else(|_| {
            serde_json::from_value(json!({
                "kind": input.trim(),
                "features": json!({}),
            }))
        })?
    };

    let mut policy = RemindBandit::default();
    let decision = policy.decide(&ctx);

    serde_json::to_writer_pretty(io::stdout(), &decision)?;
    println!();

    Ok(())
}
