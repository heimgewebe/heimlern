use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use heimlern_core::event::AussenEvent;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
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
        /// Explicit cursor override (opaque string)
        #[arg(long)]
        cursor: Option<String>,

        /// Bootstrap timestamp (ISO 8601) - used only if no cursor exists
        #[arg(long)]
        since: Option<String>,

        /// Domain to fetch (default: aussen)
        #[arg(long, default_value = "aussen")]
        domain: String,

        /// Limit of events to fetch per batch (default: 100)
        #[arg(long, default_value = "100")]
        limit: u32,

        /// Maximum number of batches to consume in one run (default: 10)
        #[arg(long, default_value = "10")]
        max_batches: u32,

        /// Input file path (simulation mode)
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
    payload: AussenEvent,
}

#[derive(Deserialize, Debug)]
struct ChronikEventsResponse {
    events: Vec<ChronikEnvelope>,
    next_cursor: String,
    has_more: bool,
}

struct FetchResult {
    events: Vec<AussenEvent>,
    next_cursor: String,
    has_more: bool,
}

fn fetch_events(
    file: Option<&PathBuf>,
    cursor: Option<&String>,
    since: Option<&String>,
    domain: &str,
    limit: u32,
) -> Result<FetchResult> {
    if let Some(path) = file {
        let f = File::open(path).context("Failed to open input file")?;
        let reader = BufReader::new(f);
        let mut events = Vec::new();
        let mut last_ts = String::new();

        // File mode simulation: simplistic read-all or filter by TS
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let event: AussenEvent = serde_json::from_str(&line)?;

            // Simulation: treating cursor as a timestamp for file mode
            if let Some(c) = cursor {
                if let Some(ts) = &event.ts {
                    if ts <= c {
                        continue;
                    }
                }
            } else if let Some(s) = since {
                if let Some(ts) = &event.ts {
                    if ts <= s {
                        continue;
                    }
                }
            }

            if let Some(ts) = &event.ts {
                last_ts = ts.clone();
            }
            events.push(event);
        }

        let next_cursor = if !last_ts.is_empty() {
            last_ts
        } else {
            cursor.cloned().unwrap_or_else(|| "unknown".to_string())
        };

        return Ok(FetchResult {
            events,
            next_cursor,
            has_more: false, // Single pass for file mode
        });
    }

    // Chronik API Mode
    let base_url =
        env::var("CHRONIK_API_URL").unwrap_or_else(|_| "http://localhost:8080".to_string());
    let base = reqwest::Url::parse(&base_url).context("Invalid CHRONIK_API_URL")?;
    let events_url = base.join("v1/events").context("Failed to join URL path")?;

    let mut target_url = events_url;
    target_url
        .query_pairs_mut()
        .append_pair("domain", domain)
        .append_pair("limit", &limit.to_string());

    // Priority: Cursor > Since > Nothing (start from beginning)
    if let Some(c) = cursor {
        target_url.query_pairs_mut().append_pair("cursor", c);
    } else if let Some(s) = since {
        target_url.query_pairs_mut().append_pair("since", s);
    }

    let resp = reqwest::blocking::get(target_url.clone())
        .with_context(|| format!("Failed to fetch from {}", target_url))?;

    if !resp.status().is_success() {
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { source } => match source {
            IngestSource::Chronik {
                cursor: cursor_arg,
                since,
                domain,
                limit,
                max_batches,
                file,
                state_file,
                stats_file,
            } => {
                let mut batches_processed = 0;
                let mut current_cursor = cursor_arg.clone();

                // Load existing state if no explicit cursor arg
                if current_cursor.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file) {
                        current_cursor = Some(state.cursor);
                        println!(
                            "Resuming from state cursor: {}",
                            current_cursor.as_ref().unwrap()
                        );
                    }
                }

                // Initial bootstrap logging
                if current_cursor.is_none() && since.is_some() {
                    println!(
                        "Bootstrapping ingest from timestamp: {}",
                        since.as_ref().unwrap()
                    );
                }

                loop {
                    if batches_processed >= max_batches {
                        println!("Max batches ({}) reached. Stopping.", max_batches);
                        break;
                    }

                    match fetch_events(
                        file.as_ref(),
                        current_cursor.as_ref(),
                        since.as_ref(),
                        &domain,
                        limit,
                    ) {
                        Ok(fetch_result) => {
                            let mut stats = EventStats::load(&stats_file).unwrap_or_default();
                            let count = fetch_result.events.len();

                            for event in fetch_result.events {
                                stats.update(&event);
                            }

                            println!(
                                "Batch {}: Processed {} events.",
                                batches_processed + 1,
                                count
                            );

                            if count > 0 {
                                stats.save(&stats_file).context("Failed to save stats")?;
                            }

                            // Update state logic
                            let new_cursor = fetch_result.next_cursor.clone();
                            let cursor_advanced = if let Some(curr) = &current_cursor {
                                *curr != new_cursor
                            } else {
                                !new_cursor.is_empty()
                            };

                            if cursor_advanced {
                                let state = IngestState {
                                    cursor: new_cursor.clone(),
                                    last_ok: OffsetDateTime::now_utc(),
                                    last_error: None,
                                };
                                state.save(&state_file).context("Failed to save state")?;
                                current_cursor = Some(new_cursor);
                            }

                            batches_processed += 1;

                            if !fetch_result.has_more {
                                println!("No more events available. Stopping.");
                                break;
                            }
                        }
                        Err(e) => {
                            let err_msg = format!("{:?}", e);
                            eprintln!("Ingest failed: {}", err_msg);

                            // Try to update last_error in state
                            let existing_cursor = current_cursor.unwrap_or_else(|| "".to_string());
                            let state = IngestState {
                                cursor: existing_cursor,
                                last_ok: OffsetDateTime::now_utc(),
                                last_error: Some(err_msg),
                            };
                            let _ = state.save(&state_file);
                            std::process::exit(1);
                        }
                    }
                }
            }
        },
    }

    Ok(())
}
