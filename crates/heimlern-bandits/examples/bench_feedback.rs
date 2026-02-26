use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};
use std::fmt::Write;
use std::time::Instant;

/// Must be kept in sync with `MAX_ARMS` in `crates/heimlern-bandits/src/lib.rs`.
const CAP: usize = 1000;

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

    println!("Feedback for EXISTING slot took: {:?}", duration);
    println!("Average per call: {:?}", duration / iterations as u32);

    // Measure filling up to capacity
    let mut bandit = RemindBandit::default();
    let start = Instant::now();
    for i in 0..CAP {
        let action = format!("remind.slot_{}", i);
        bandit.feedback(&ctx, &action, reward);
    }
    let duration = start.elapsed();
    println!(
        "Feedback for FILLING slots (0..{}) took: {:?}",
        CAP, duration
    );
    println!(
        "Average per call: {:?}",
        duration / u32::try_from(CAP).unwrap()
    );

    // Measure rejection overhead (slots full)
    // We reuse the bandit which is now full (len == CAP)
    let reject_iterations = 1_000_000;
    // Reuse buffer to avoid allocation noise in the benchmark loop
    let mut action_buf = String::with_capacity(32);
    let start = Instant::now();
    for i in 0..reject_iterations {
        // These are new slot names, so they trigger the "new slot" path,
        // but get rejected because len >= MAX_ARMS.
        action_buf.clear();
        write!(&mut action_buf, "remind.reject_{}", i).unwrap();
        bandit.feedback(&ctx, &action_buf, reward);
    }
    let duration = start.elapsed();
    println!(
        "Feedback for REJECTING slots (over capacity) took: {:?}",
        duration
    );
    println!(
        "Average per call: {:?}",
        duration / reject_iterations as u32
    );
}
