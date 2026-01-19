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

        /// Input file path (for testing/file-based ingest)
        #[arg(long)]
        file: Option<PathBuf>,

        /// Path to the state file
        #[arg(long, default_value = "data/heimlern.cursor")]
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

fn get_reader(
    url: Option<&String>,
    file: Option<&PathBuf>,
    since: Option<&String>,
) -> Result<Box<dyn BufRead>> {
    if let Some(path) = file {
        let f = File::open(path).context("Failed to open input file")?;
        return Ok(Box::new(BufReader::new(f)));
    }

    if let Some(u) = url {
        // Build URL with query params
        let mut target_url = reqwest::Url::parse(u).context("Invalid URL")?;
        if let Some(s) = since {
            target_url.query_pairs_mut().append_pair("since", s);
        }

        let resp = reqwest::blocking::get(target_url.clone())
            .with_context(|| format!("Failed to fetch from {}", target_url))?;

        // Ensure success
        if !resp.status().is_success() {
            anyhow::bail!("Request failed: {}", resp.status());
        }

        // We assume the response is the JSONL stream directly
        let reader = BufReader::new(resp);
        return Ok(Box::new(reader));
    }

    println!("Reading from stdin...");
    Ok(Box::new(BufReader::new(std::io::stdin())))
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Ingest { source } => match source {
            IngestSource::Chronik {
                since,
                url,
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

                // Determine input source
                let reader = get_reader(url.as_ref(), file.as_ref(), current_cursor.as_ref())?;

                let mut count = 0;
                let mut last_ts = String::new();
                let mut stats = EventStats::load(&stats_file).unwrap_or_default();

                for line in reader.lines() {
                    let line = line?;
                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<AussenEvent>(&line) {
                        Ok(event) => {
                            // Filter by cursor if needed (simple string comparison for ISO TS)
                            if let (Some(cursor), Some(ts)) = (&current_cursor, &event.ts) {
                                if ts <= cursor {
                                    continue; // Skip already processed
                                }
                            }

                            // Process event: Update statistics
                            stats.update(&event);

                            if let Some(ts) = event.ts {
                                last_ts = ts;
                            } else if let Some(id) = event.id {
                                last_ts = id; // Fallback to ID as cursor
                            }
                            count += 1;
                        }
                        Err(e) => {
                            eprintln!("Skipping invalid event: {}", e);
                        }
                    }
                }

                println!("Processed {} new events.", count);

                if count > 0 {
                    stats.save(&stats_file).context("Failed to save stats")?;
                    println!("Stats updated.");
                }

                if !last_ts.is_empty() {
                    let state = IngestState {
                        cursor: last_ts,
                        last_ok: OffsetDateTime::now_utc(),
                        last_error: None,
                    };
                    state.save(&state_file).context("Failed to save state")?;
                    println!("State updated.");
                } else {
                    println!("No new events with timestamps found to update state.");
                }
            }
        },
    }

    Ok(())
}
