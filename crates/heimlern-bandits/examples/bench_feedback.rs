use heimlern_bandits::RemindBandit;
use heimlern_core::{Context, Policy};
use serde::Serialize;
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

/// Must be kept in sync with `MAX_ARMS` in `crates/heimlern-bandits/src/lib.rs`.
const CAP: usize = 1000;

#[derive(Debug, Clone, Copy)]
struct BenchConfig {
    iterations: u64,
    replay_iterations: u64,
    fill_cap: usize,
    warmup: u64,
    samples: u32,
    json: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            iterations: 1_000_000,
            replay_iterations: 10_000,
            fill_cap: CAP,
            warmup: 1_000,
            samples: 1,
            json: false,
        }
    }
}

#[derive(Debug, Serialize)]
struct BenchReport {
    benchmark: &'static str,
    cap: usize,
    iterations: u64,
    replay_iterations: u64,
    fill_cap: usize,
    warmup: u64,
    samples: Vec<BenchSample>,
}

#[derive(Debug, Serialize)]
struct BenchSample {
    sample: u32,
    existing_slot_total_ns: u128,
    existing_slot_ns_per_call: f64,
    filling_slots_total_ns: u128,
    filling_slots_ns_per_call: f64,
    snapshot_replay_total_ns: u128,
    snapshot_replay_ns_per_call: f64,
}

fn parse_u64_arg(args: &[String], index: &mut usize, name: &str) -> Result<u64, String> {
    *index += 1;
    let value = args
        .get(*index)
        .ok_or_else(|| format!("{name} requires a numeric value"))?;
    value
        .parse::<u64>()
        .map_err(|err| format!("invalid value for {name}: {err}"))
}

fn parse_usize_arg(args: &[String], index: &mut usize, name: &str) -> Result<usize, String> {
    *index += 1;
    let value = args
        .get(*index)
        .ok_or_else(|| format!("{name} requires a numeric value"))?;
    value
        .parse::<usize>()
        .map_err(|err| format!("invalid value for {name}: {err}"))
}

fn parse_u32_arg(args: &[String], index: &mut usize, name: &str) -> Result<u32, String> {
    *index += 1;
    let value = args
        .get(*index)
        .ok_or_else(|| format!("{name} requires a numeric value"))?;
    value
        .parse::<u32>()
        .map_err(|err| format!("invalid value for {name}: {err}"))
}

fn parse_config() -> Result<BenchConfig, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut config = BenchConfig::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => config.json = true,
            "--iterations" => config.iterations = parse_u64_arg(&args, &mut index, "--iterations")?,
            "--replay-iterations" => {
                config.replay_iterations = parse_u64_arg(&args, &mut index, "--replay-iterations")?;
            }
            "--fill-cap" => config.fill_cap = parse_usize_arg(&args, &mut index, "--fill-cap")?,
            "--warmup" => config.warmup = parse_u64_arg(&args, &mut index, "--warmup")?,
            "--samples" => config.samples = parse_u32_arg(&args, &mut index, "--samples")?,
            "--help" | "-h" => {
                println!(
                    "Usage: bench_feedback [--json] [--iterations N] [--replay-iterations N] [--fill-cap N] [--warmup N] [--samples N]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }

    if config.iterations == 0 {
        return Err("--iterations must be greater than 0".to_string());
    }
    if config.replay_iterations == 0 {
        return Err("--replay-iterations must be greater than 0".to_string());
    }
    if config.fill_cap == 0 || config.fill_cap > CAP {
        return Err(format!("--fill-cap must be between 1 and {CAP}"));
    }
    if config.samples == 0 {
        return Err("--samples must be greater than 0".to_string());
    }
    Ok(config)
}

fn ns_per_call(total_ns: u128, calls: u64) -> f64 {
    total_ns as f64 / calls as f64
}

fn run_one_sample(config: BenchConfig, sample: u32) -> BenchSample {
    let ctx = Context {
        kind: "bench".into(),
        features: serde_json::json!({}),
    };
    let action = "remind.bench_existing";
    let reward = 1.0;

    let mut bandit = RemindBandit::default();
    for _ in 0..config.warmup {
        bandit.feedback(black_box(&ctx), black_box(action), black_box(reward));
    }

    let start = Instant::now();
    for _ in 0..config.iterations {
        bandit.feedback(black_box(&ctx), black_box(action), black_box(reward));
    }
    let existing_slot_total_ns = start.elapsed().as_nanos();

    let mut bandit = RemindBandit::default();
    let mut action_buf = String::with_capacity(32);
    let start = Instant::now();
    for i in 0..config.fill_cap {
        action_buf.clear();
        let _ = write!(&mut action_buf, "remind.slot_{i}");
        bandit.feedback(black_box(&ctx), black_box(&action_buf), black_box(reward));
    }
    let filling_slots_total_ns = start.elapsed().as_nanos();

    let snapshot = black_box(bandit.snapshot());
    let start = Instant::now();
    for _ in 0..config.replay_iterations {
        let mut replayed = RemindBandit::default();
        replayed.load(black_box(snapshot.clone()));
        black_box(replayed.snapshot());
    }
    let snapshot_replay_total_ns = start.elapsed().as_nanos();

    BenchSample {
        sample,
        existing_slot_total_ns,
        existing_slot_ns_per_call: ns_per_call(existing_slot_total_ns, config.iterations),
        filling_slots_total_ns,
        filling_slots_ns_per_call: ns_per_call(filling_slots_total_ns, config.fill_cap as u64),
        snapshot_replay_total_ns,
        snapshot_replay_ns_per_call: ns_per_call(
            snapshot_replay_total_ns,
            config.replay_iterations,
        ),
    }
}

fn run_benchmark(config: BenchConfig) -> BenchReport {
    let samples = (1..=config.samples)
        .map(|sample| run_one_sample(config, sample))
        .collect();
    BenchReport {
        benchmark: "heimlern-bandits.feedback_paths.v1",
        cap: CAP,
        iterations: config.iterations,
        replay_iterations: config.replay_iterations,
        fill_cap: config.fill_cap,
        warmup: config.warmup,
        samples,
    }
}

fn print_text(report: &BenchReport) {
    println!("Benchmark: {}", report.benchmark);
    println!("CAP: {}", report.cap);
    println!("Iterations: {}", report.iterations);
    println!("Replay iterations: {}", report.replay_iterations);
    println!("Fill cap: {}", report.fill_cap);
    for sample in &report.samples {
        println!("Sample {}", sample.sample);
        println!(
            "  existing slot: {} ns total, {:.3} ns/call",
            sample.existing_slot_total_ns, sample.existing_slot_ns_per_call
        );
        println!(
            "  filling slots: {} ns total, {:.3} ns/call",
            sample.filling_slots_total_ns, sample.filling_slots_ns_per_call
        );
        println!(
            "  snapshot replay: {} ns total, {:.3} ns/call",
            sample.snapshot_replay_total_ns, sample.snapshot_replay_ns_per_call
        );
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = parse_config().map_err(|err| format!("configuration error: {err}"))?;
    let report = run_benchmark(config);
    if config.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_text(&report);
    }
    Ok(())
}
