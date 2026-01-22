use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use heimlern_core::event::AussenEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;
use time::OffsetDateTime;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Ingest events from Chronik or a file
    Ingest {
        #[command(subcommand)]
        source: IngestSource,
    },
}

#[derive(Subcommand)]
enum IngestSource {
    /// Ingest from Chronik (via HTTP)
    Chronik {
        /// Explicit cursor start (byte offset) - overrides state
        #[arg(long)]
        cursor: Option<u64>,

        /// Domain to fetch (default: aussen)
        #[arg(long, default_value = "aussen")]
        domain: String,

        /// Limit of events to fetch per batch (default: 100)
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Maximum number of batches to consume in one run (default: 10)
        #[arg(long, default_value = "10")]
        max_batches: u32,

        /// Path to the state file
        #[arg(long, default_value = "data/heimlern.ingest.state.json")]
        state_file: PathBuf,

        /// Path to the stats file
        #[arg(long, default_value = "data/heimlern.stats.json")]
        stats_file: PathBuf,
    },
    /// Ingest from local file (Simulation mode)
    File {
        /// Input file path
        #[arg(long)]
        path: PathBuf,

        /// Start from line number (0-based)
        #[arg(long)]
        line_offset: Option<u64>,

        /// Path to the state file
        #[arg(long, default_value = "data/heimlern.ingest.file.state.json")]
        state_file: PathBuf,

        /// Path to the stats file
        #[arg(long, default_value = "data/heimlern.stats.json")]
        stats_file: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct IngestState {
    cursor: u64, // Strictly u64
    #[serde(with = "time::serde::iso8601::option")]
    last_ok: Option<OffsetDateTime>,
    last_error: Option<String>,
}

impl IngestState {
    fn load(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let file = File::open(path)?;
        let state = serde_json::from_reader(file)?;
        Ok(Some(state))
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct EventStats {
    total_processed: u64,
    by_type: HashMap<String, u64>,
    by_source: HashMap<String, u64>,
    #[serde(with = "time::serde::iso8601")]
    last_updated: OffsetDateTime,
}

impl Default for EventStats {
    fn default() -> Self {
        Self {
            total_processed: 0,
            by_type: HashMap::new(),
            by_source: HashMap::new(),
            last_updated: OffsetDateTime::now_utc(),
        }
    }
}

impl EventStats {
    fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let file = File::open(path)?;
        let stats = serde_json::from_reader(file)?;
        Ok(stats)
    }

    fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        serde_json::to_writer_pretty(file, self)?;
        Ok(())
    }

    fn update(&mut self, event: &AussenEvent) {
        self.total_processed += 1;
        *self.by_type.entry(event.r#type.clone()).or_insert(0) += 1;
        *self.by_source.entry(event.source.clone()).or_insert(0) += 1;
        self.last_updated = OffsetDateTime::now_utc();
    }
}

#[derive(Deserialize, Debug)]
struct ChronikEvent {
    #[allow(dead_code)]
    r#type: Option<String>,
    payload: AussenEvent,
}

#[derive(Deserialize, Debug)]
struct BatchMeta {
    #[allow(dead_code)]
    count: Option<u32>,
    #[allow(dead_code)]
    generated_at: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ChronikEventsResponse {
    events: Vec<ChronikEvent>,
    next_cursor: u64, // Strictly u64
    has_more: bool,
    #[allow(dead_code)]
    meta: Option<BatchMeta>,
}

struct FetchResult {
    events: Vec<AussenEvent>,
    next_cursor: u64,
    has_more: bool,
}

fn fetch_chronik(cursor: Option<u64>, domain: &str, limit: u32) -> Result<FetchResult> {
    if domain.trim().is_empty()
        || !domain
            .chars()
            .all(|c| c.is_alphanumeric() || c == '.' || c == '-')
    {
        anyhow::bail!("Invalid domain: {}", domain);
    }

    // Require CHRONIK_BASE_URL (or fallback to legacy CHRONIK_API_URL for compat)
    let base_env = env::var("CHRONIK_BASE_URL")
        .or_else(|_| env::var("CHRONIK_API_URL"))
        .context("CHRONIK_BASE_URL or CHRONIK_API_URL env var is required")?;

    // Robust URL normalization
    // Goal: Cleanly append /v1/events to the base root
    let mut target_url = url::Url::parse(&base_env).context("Invalid CHRONIK_BASE_URL")?;

    // Strip existing suffix fragments to ensure clean base
    // e.g. http://host/v1/events -> http://host/
    // e.g. http://host/v1 -> http://host/
    // This allows users to paste whatever URL they have and we fix it.
    {
        let mut path = target_url.path().to_string();
        if path.ends_with("/v1/events") {
            path = path.replace("/v1/events", "");
        } else if path.ends_with("/v1/events/") {
            path = path.replace("/v1/events/", "");
        } else if path.ends_with("/v1") {
            path = path.replace("/v1", "");
        } else if path.ends_with("/v1/") {
            path = path.replace("/v1/", "");
        }
        // Also strip trailing slash
        if path.ends_with('/') {
            path.pop();
        }
        target_url.set_path(&path);
    }

    // Now cleanly append the target path
    if let Ok(mut segments) = target_url.path_segments_mut() {
        segments.pop_if_empty().push("v1").push("events");
    }

    let token = env::var("CHRONIK_TOKEN").context("CHRONIK_TOKEN env var is required")?;

    let mut req = ureq::get(target_url.as_str())
        .set("X-Auth", &token)
        .query("domain", domain)
        .query("limit", &limit.to_string())
        .timeout(Duration::from_secs(10));

    if let Some(c) = cursor {
        req = req.query("cursor", &c.to_string());
    }

    let resp = req
        .call()
        .with_context(|| format!("Failed to fetch from {}", target_url))?;

    let response_body: ChronikEventsResponse = resp.into_json()?;

    let events = response_body
        .events
        .into_iter()
        .map(|env| env.payload)
        .collect();

    Ok(FetchResult {
        events,
        next_cursor: response_body.next_cursor,
        has_more: response_body.has_more,
    })
}

fn fetch_file(path: &Path, offset: u64) -> Result<FetchResult> {
    // Note: 'offset' here refers to line-offset (0-based index of lines),
    // which differs from the API's byte-offset cursor.
    // Ideally, state files should be kept separate to avoid confusion.
    let f = File::open(path).context("Failed to open input file")?;
    let reader = BufReader::new(f);
    let mut events = Vec::new();
    let mut lines_read = 0;

    for (idx, line) in reader.lines().enumerate() {
        if (idx as u64) < offset {
            continue;
        }
        let line = line?;
        if line.trim().is_empty() {
            lines_read += 1;
            continue;
        }
        let event: AussenEvent = serde_json::from_str(&line)?;
        events.push(event);
        lines_read += 1;
    }

    let next_offset = offset + lines_read; // next line number

    Ok(FetchResult {
        events,
        next_cursor: next_offset,
        has_more: false,
    })
}

fn process_ingest(
    source_result: Result<FetchResult>,
    state_file: &Path,
    stats_file: &Path,
    current_cursor: &mut u64,
) -> Result<bool> {
    match source_result {
        Ok(fetch_result) => {
            let mut stats = EventStats::load(stats_file).unwrap_or_default();
            let count = fetch_result.events.len();

            for event in fetch_result.events {
                stats.update(&event);
            }

            if count > 0 {
                println!("Processed {} events.", count);
                stats.save(stats_file).context("Failed to save stats")?;
            } else {
                println!("No new events.");
            }

            // Update state logic
            let new_cursor = fetch_result.next_cursor;

            // Advance cursor
            if new_cursor != *current_cursor {
                *current_cursor = new_cursor;
            }

            // Always save state on success to update last_ok
            let state = IngestState {
                cursor: *current_cursor,
                last_ok: Some(OffsetDateTime::now_utc()),
                last_error: None,
            };
            state.save(state_file).context("Failed to save state")?;
            println!("State updated to cursor: {}", *current_cursor);

            Ok(fetch_result.has_more)
        }
        Err(e) => {
            let err_msg = format!("{:?}", e);
            eprintln!("Ingest failed: {}", err_msg);

            let existing_cursor = *current_cursor;

            // Preserve old last_ok
            let old_last_ok = if let Ok(Some(s)) = IngestState::load(state_file) {
                s.last_ok
            } else {
                None // Semantically correct: never successful yet
            };

            let state = IngestState {
                cursor: existing_cursor,
                last_ok: old_last_ok,
                last_error: Some(err_msg),
            };
            let _ = state.save(state_file);
            Err(anyhow::anyhow!("Ingestion cycle failed"))
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { source } => match source {
            IngestSource::Chronik {
                cursor,
                domain,
                limit,
                max_batches,
                state_file,
                stats_file,
            } => {
                let mut batches_processed = 0;
                let mut current_cursor = cursor.unwrap_or(0); // Default to 0

                if cursor.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file) {
                        current_cursor = state.cursor;
                        println!("Resuming from state cursor: {}", current_cursor);
                    }
                }

                loop {
                    if batches_processed >= max_batches {
                        println!("Max batches ({}) reached. Stopping.", max_batches);
                        break;
                    }

                    if process_ingest(
                        fetch_chronik(Some(current_cursor), &domain, limit),
                        &state_file,
                        &stats_file,
                        &mut current_cursor,
                    )
                    .is_err()
                    {
                        std::process::exit(1);
                    }

                    // fetch_chronik does not return has_more directly in main loop flow currently?
                    // Wait, process_ingest returns Result<bool> (has_more)
                    // I need to capture has_more properly.

                    // Fixed logic:
                    match process_ingest(
                        fetch_chronik(Some(current_cursor), &domain, limit),
                        &state_file,
                        &stats_file,
                        &mut current_cursor,
                    ) {
                        Ok(has_more) => {
                            batches_processed += 1;
                            if !has_more {
                                break;
                            }
                        }
                        Err(_) => std::process::exit(1),
                    }
                }
            }
            IngestSource::File {
                path,
                line_offset,
                state_file,
                stats_file,
            } => {
                let mut current_cursor = line_offset.unwrap_or(0);

                // Fallback to loading state if CLI arg missing
                if line_offset.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file) {
                        current_cursor = state.cursor;
                        println!("Resuming from file offset: {}", current_cursor);
                    }
                }

                // File mode is single pass
                if process_ingest(
                    fetch_file(&path, current_cursor),
                    &state_file,
                    &stats_file,
                    &mut current_cursor,
                )
                .is_err()
                {
                    std::process::exit(1);
                }
            }
        },
    }

    Ok(())
}
