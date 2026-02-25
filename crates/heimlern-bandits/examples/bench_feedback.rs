use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};
use std::time::Instant;

fn main() {
    let mut bandit = RemindBandit::default();
    let ctx = Context {
        kind: "bench".into(),
        features: serde_json::json!({}),
    };

    let iterations = 1_000_000;
    let action = "remind.morning";
    let reward = 1.0;

    // Warmup
    for _ in 0..1000 {
        bandit.feedback(&ctx, action, reward);
    }

    let start = Instant::now();
    for _ in 0..iterations {
        bandit.feedback(&ctx, action, reward);
    }
    let duration = start.elapsed();

    println!("Feedback for existing slot took: {:?}", duration);
    println!("Average per call: {:?}", duration / iterations as u32);

    let mut bandit = RemindBandit::default();
    let start = Instant::now();
    for i in 0..iterations {
        let action = format!("remind.slot_{}", i);
        bandit.feedback(&ctx, &action, reward);
    }
    let duration = start.elapsed();
    println!("Feedback for NEW slots took: {:?}", duration);
    println!("Average per call: {:?}", duration / iterations as u32);
}
