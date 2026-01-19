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
    cursor: u64, // Strictly u64 (byte offset or line number)
    #[serde(with = "time::serde::iso8601")]
    last_ok: OffsetDateTime,
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

// Assuming Chronik returns a wrapper. If strictly list, we'd change this.
// "events": [ { "payload": ... }, ... ]
#[derive(Deserialize, Debug)]
struct ChronikEnvelope {
    payload: AussenEvent,
}

#[derive(Deserialize, Debug)]
struct ChronikEventsResponse {
    events: Vec<ChronikEnvelope>,
    next_cursor: u64, // Int based cursor
    has_more: bool,
}

struct FetchResult {
    events: Vec<AussenEvent>,
    next_cursor: u64,
    has_more: bool,
}

fn fetch_chronik(cursor: Option<u64>, domain: &str, limit: u32) -> Result<FetchResult> {
    let base_url =
        env::var("CHRONIK_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());

    // URL Normalization
    let mut target_url = reqwest::Url::parse(&base_url).context("Invalid CHRONIK_API_URL")?;
    if !target_url.path().ends_with("/v1/events") {
        target_url = target_url
            .join("v1/events")
            .context("Failed to join URL path")?;
    }

    target_url
        .query_pairs_mut()
        .append_pair("domain", domain)
        .append_pair("limit", &limit.to_string());

    if let Some(c) = cursor {
        target_url
            .query_pairs_mut()
            .append_pair("cursor", &c.to_string());
    }

    // Auth & Client
    let token = env::var("CHRONIK_TOKEN").context("CHRONIK_TOKEN env var is required")?;

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    let resp = client
        .get(target_url.clone())
        .header("X-Auth", token)
        .send()
        .with_context(|| format!("Failed to fetch from {}", target_url))?;

    if !resp.status().is_success() {
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED
            || resp.status() == reqwest::StatusCode::FORBIDDEN
        {
            anyhow::bail!("Access denied: {}", resp.status());
        }
        anyhow::bail!("Chronik request failed: {}", resp.status());
    }

    let response_body: ChronikEventsResponse = resp.json()?;

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

fn fetch_file(path: &PathBuf, offset: u64) -> Result<FetchResult> {
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
    state_file: &PathBuf,
    stats_file: &PathBuf,
    current_cursor: &mut Option<u64>,
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
            // Only update if cursor advanced
            let new_cursor = fetch_result.next_cursor;

            // For file mode, new_cursor is offset + lines_read. If lines_read > 0, we advanced.
            // For API, next_cursor is returned.
            if Some(new_cursor) != *current_cursor {
                let state = IngestState {
                    cursor: new_cursor,
                    last_ok: OffsetDateTime::now_utc(),
                    last_error: None,
                };
                state.save(state_file).context("Failed to save state")?;
                *current_cursor = Some(new_cursor);
                println!("State updated to cursor: {}", new_cursor);
            }

            Ok(fetch_result.has_more)
        }
        Err(e) => {
            let err_msg = format!("{:?}", e);
            eprintln!("Ingest failed: {}", err_msg);

            // Try to update last_error in state
            let existing_cursor = current_cursor.unwrap_or(0);
            let state = IngestState {
                cursor: existing_cursor,
                last_ok: OffsetDateTime::now_utc(),
                last_error: Some(err_msg),
            };
            let _ = state.save(state_file);
            std::process::exit(1);
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
                let mut current_cursor = cursor;

                if current_cursor.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file) {
                        current_cursor = Some(state.cursor);
                        println!("Resuming from state cursor: {}", state.cursor);
                    }
                }

                loop {
                    if batches_processed >= max_batches {
                        println!("Max batches ({}) reached. Stopping.", max_batches);
                        break;
                    }

                    let has_more = process_ingest(
                        fetch_chronik(current_cursor, &domain, limit),
                        &state_file,
                        &stats_file,
                        &mut current_cursor,
                    )?;

                    batches_processed += 1;
                    if !has_more {
                        break;
                    }
                }
            }
            IngestSource::File {
                path,
                line_offset,
                state_file,
                stats_file,
            } => {
                let mut current_cursor = line_offset;
                if current_cursor.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file) {
                        current_cursor = Some(state.cursor);
                        println!("Resuming from file offset: {}", state.cursor);
                    }
                }

                // File mode is single pass
                process_ingest(
                    fetch_file(&path, current_cursor.unwrap_or(0)),
                    &state_file,
                    &stats_file,
                    &mut current_cursor,
                )?;
            }
        },
    }

    Ok(())
}
