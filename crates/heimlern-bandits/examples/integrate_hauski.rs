use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

fn main() {
    let mut p = RemindBandit::default();
    let ctx = Context {
        kind: "reminder".into(),
        features: serde_json::json!({"load": 0.3}),
    };
    let d = p.decide(&ctx);

    let ts = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string());

    let record = serde_json::json!({
        "ts": ts,
        "policy_id": "remind-bandit",
        "policy": "heimlern-bandits",
        "context": ctx,
        "decision": d
    });

    println!("{}", serde_json::to_string_pretty(&record).unwrap());
}
