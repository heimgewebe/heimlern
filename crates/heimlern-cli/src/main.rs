use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use heimlern_core::event::AussenEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
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
    /// Ingest from Chronik
    Chronik {
        /// Start ingesting from this cursor (timestamp/ID)
        #[arg(long)]
        since: Option<String>,

        /// Chronik API URL
        #[arg(long)]
        url: Option<String>,

        /// Domain to fetch (default: aussen)
        #[arg(long, default_value = "aussen")]
        domain: String,

        /// Limit of events to fetch (default: 100)
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Input file path (for testing/file-based ingest)
        #[arg(long)]
        file: Option<PathBuf>,

        /// Path to the state file
        #[arg(long, default_value = "data/heimlern.ingest.state.json")]
        state_file: PathBuf,

        /// Path to the stats file
        #[arg(long, default_value = "data/heimlern.stats.json")]
        stats_file: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct IngestState {
    cursor: String,
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

#[derive(Deserialize, Debug)]
struct ChronikEnvelope {
    // domain: String,
    received_at: String,
    payload: AussenEvent,
}

struct FetchedEvent {
    event: AussenEvent,
    cursor: String,
}

fn fetch_events(
    url: Option<&String>,
    file: Option<&PathBuf>,
    since: Option<&String>,
    domain: &str,
    limit: u32,
) -> Result<Vec<FetchedEvent>> {
    if let Some(path) = file {
        let f = File::open(path).context("Failed to open input file")?;
        let reader = BufReader::new(f);
        let mut results = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            // In file mode, we assume raw AussenEvents for simplicity/legacy support
            let event: AussenEvent = serde_json::from_str(&line)?;

            // Derive cursor from TS
            let cursor = event
                .ts
                .clone()
                .or_else(|| event.id.clone())
                .unwrap_or_else(|| "unknown".to_string());

            results.push(FetchedEvent { event, cursor });
        }
        return Ok(results);
    }

    if let Some(u) = url {
        // Construct /v1/tail URL
        // Base URL + /v1/tail
        let base = reqwest::Url::parse(u).context("Invalid base URL")?;
        let tail_url = base.join("v1/tail").context("Failed to join URL path")?;

        let mut target_url = tail_url;
        target_url
            .query_pairs_mut()
            .append_pair("domain", domain)
            .append_pair("limit", &limit.to_string());

        if let Some(s) = since {
            target_url.query_pairs_mut().append_pair("since", s);
        }

        let resp = reqwest::blocking::get(target_url.clone())
            .with_context(|| format!("Failed to fetch from {}", target_url))?;

        if !resp.status().is_success() {
            anyhow::bail!("Chronik request failed: {}", resp.status());
        }

        let envelopes: Vec<ChronikEnvelope> = resp.json()?;
        let results = envelopes
            .into_iter()
            .map(|env| FetchedEvent {
                event: env.payload,
                cursor: env.received_at,
            })
            .collect();

        return Ok(results);
    }

    // Stdin fallback - similar to file mode
    println!("Reading from stdin...");
    let reader = BufReader::new(std::io::stdin());
    let mut results = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let event: AussenEvent = serde_json::from_str(&line)?;
        let cursor = event.ts.clone().unwrap_or_else(|| "unknown".to_string());
        results.push(FetchedEvent { event, cursor });
    }
    Ok(results)
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { source } => match source {
            IngestSource::Chronik {
                since,
                url,
                domain,
                limit,
                file,
                state_file,
                stats_file,
            } => {
                let mut current_cursor = since.clone();

                // Load existing state if cursor not provided via args
                if current_cursor.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file) {
                        current_cursor = Some(state.cursor);
                        println!("Resuming from cursor: {}", current_cursor.as_ref().unwrap());
                    }
                }

                // Fetch events
                match fetch_events(
                    url.as_ref(),
                    file.as_ref(),
                    current_cursor.as_ref(),
                    &domain,
                    limit,
                ) {
                    Ok(fetched_events) => {
                        let mut count = 0;
                        let mut last_processed_cursor = String::new();
                        let mut stats = EventStats::load(&stats_file).unwrap_or_default();

                        for item in fetched_events {
                            let event = item.event;
                            let cursor = item.cursor;

                            // Filter by cursor (double check, though API should handle it)
                            if let Some(current) = &current_cursor {
                                if cursor <= *current {
                                    continue;
                                }
                            }

                            // Process
                            stats.update(&event);
                            last_processed_cursor = cursor;
                            count += 1;
                        }

                        println!("Processed {} new events.", count);

                        if count > 0 {
                            stats.save(&stats_file).context("Failed to save stats")?;
                            println!("Stats updated.");
                        }

                        if !last_processed_cursor.is_empty() {
                            let state = IngestState {
                                cursor: last_processed_cursor,
                                last_ok: OffsetDateTime::now_utc(),
                                last_error: None,
                            };
                            state.save(&state_file).context("Failed to save state")?;
                            println!("State updated.");
                        } else {
                            println!("No new events found to update state.");
                        }
                    }
                    Err(e) => {
                        let err_msg = format!("{:?}", e);
                        eprintln!("Ingest failed: {}", err_msg);

                        // Try to update last_error in state, preserving cursor
                        let existing_cursor =
                            current_cursor.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
                        let state = IngestState {
                            cursor: existing_cursor,
                            last_ok: OffsetDateTime::now_utc(), // We update TS to indicate when we tried
                            last_error: Some(err_msg),
                        };
                        let _ = state.save(&state_file); // Ignore save error during error handling
                        std::process::exit(1);
                    }
                }
            }
        },
    }

    Ok(())
}
