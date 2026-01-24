//! CLI for heimlern.
//!
//! Provides commands for ingesting events from Chronik or local files, managing state and stats,
//! and performing drift checks. It serves as the operational interface for the policy framework.

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
enum IngestMode {
    Chronik,
    File,
}

#[derive(Serialize, Deserialize, Debug)]
struct IngestState {
    cursor: u64, // Strictly u64
    mode: IngestMode,
    #[serde(with = "time::serde::iso8601::option")]
    last_ok: Option<OffsetDateTime>,
    last_error: Option<String>,
}

impl IngestState {
    fn load(path: &Path, expected_mode: IngestMode) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let file = File::open(path)?;
        let state: IngestState = serde_json::from_reader(file)?;

        if state.mode != expected_mode {
            anyhow::bail!(
                "State file mode mismatch: expected {:?}, found {:?}",
                expected_mode,
                state.mode
            );
        }

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
    next_cursor: Option<u64>, // Relaxed to Option<u64> to handle EOF null
    has_more: bool,
    #[allow(dead_code)]
    meta: Option<BatchMeta>,
}

struct FetchResult {
    events: Vec<AussenEvent>,
    next_cursor: Option<u64>, // Relaxed to Option<u64>
    has_more: bool,
}

/// Validates an event domain/namespace identifier.
///
/// This validates event namespace identifiers (e.g., "aussen", "sensor.v1"), not DNS domains.
/// Single-label identifiers like "aussen" are valid by design for internal event routing.
///
/// Rules (similar to DNS hostname rules but applied to namespace identifiers):
/// - Labels separated by dots, each 1-63 chars, total ≤253 chars
/// - Each label: starts/ends with alphanumeric, may contain hyphens in middle
/// - No whitespace, underscores, or leading/trailing dots
/// - No IDN/Unicode (ASCII alphanumeric + hyphens only)
///
/// Note: If future requirements need different characters (e.g., underscores, slashes),
/// this validation should be relaxed or the semantic meaning of "domain" clarified
/// with respect to the Chronik API contract.
fn is_valid_event_domain(domain: &str) -> bool {
    let domain = domain.trim();
    if domain.is_empty() || domain.len() > 253 {
        return false;
    }
    if domain.contains(char::is_whitespace) {
        return false;
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return false;
    }

    for label in domain.split('.') {
        if label.is_empty() || label.len() > 63 {
            return false;
        }
        let mut chars = label.chars();
        // First char must be alphanumeric
        if !chars.next().unwrap().is_alphanumeric() {
            return false;
        }
        // If there's more than one char, the last must be alphanumeric
        if label.len() > 1 && !label.chars().last().unwrap().is_alphanumeric() {
            return false;
        }
        // All chars must be alphanumeric or hyphen
        if !label.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return false;
        }
    }

    true
}

fn record_state_error(
    state_file: &Path,
    mode: IngestMode,
    cursor: u64,
    err_msg: &str,
) -> Result<()> {
    // Attempt to load old state to preserve last_ok
    let old_last_ok = if let Ok(Some(s)) = IngestState::load(state_file, mode) {
        s.last_ok
    } else {
        None
    };

    let state = IngestState {
        cursor,
        mode,
        last_ok: old_last_ok,
        last_error: Some(err_msg.to_string()),
    };

    if let Err(e) = state.save(state_file) {
        eprintln!(
            "CRITICAL: Failed to save error state to {:?}. Original error: {}. Save error: {}",
            state_file, err_msg, e
        );
        return Err(e);
    }

    Ok(())
}

fn build_chronik_url(base: &str) -> Result<url::Url> {
    let mut target_url = url::Url::parse(base).context("Invalid base URL")?;

    let mut segments: Vec<String> = target_url
        .path_segments()
        .map(|iter| iter.map(String::from).collect())
        .unwrap_or_default();

    if let Some(last) = segments.last() {
        if last.is_empty() {
            segments.pop();
        }
    }

    if segments.ends_with(&["v1".to_string(), "events".to_string()]) {
        segments.pop();
        segments.pop();
    } else if segments.ends_with(&["v1".to_string()]) {
        segments.pop();
    }

    target_url
        .path_segments_mut()
        .map_err(|()| anyhow::anyhow!("URL cannot be used as a base (e.g., 'data:' or 'mailto:' schemes are not supported)"))?
        .clear()
        .extend(segments)
        .push("v1")
        .push("events");

    Ok(target_url)
}

fn fetch_chronik(cursor: Option<u64>, domain: &str, limit: u32) -> Result<FetchResult> {
    if !is_valid_event_domain(domain) {
        anyhow::bail!("Invalid domain: {}", domain);
    }

    let base_env = env::var("CHRONIK_BASE_URL")
        .or_else(|_| env::var("CHRONIK_API_URL"))
        .context("CHRONIK_BASE_URL or CHRONIK_API_URL env var is required")?;

    let target_url = build_chronik_url(&base_env).context("Failed to build Chronik URL")?;

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

    let next_offset = offset.checked_add(lines_read).context("Cursor overflow")?;

    Ok(FetchResult {
        events,
        next_cursor: Some(next_offset),
        has_more: false,
    })
}

fn process_ingest(
    source_result: Result<FetchResult>,
    state_file: &Path,
    stats_file: &Path,
    current_cursor: &mut u64,
    mode: IngestMode,
) -> Result<bool> {
    match source_result {
        Ok(fetch_result) => {
            let mut stats = EventStats::load(stats_file).unwrap_or_else(|e| {
                eprintln!(
                    "Warning: failed to read stats from {:?}; starting fresh: {}",
                    stats_file, e
                );
                EventStats::default()
            });
            let count = fetch_result.events.len();

            for event in fetch_result.events {
                stats.update(&event);
            }

            // Always update last_updated to reflect the check time
            stats.last_updated = OffsetDateTime::now_utc();

            println!(
                "Processed {} events. (Stats updated at {})",
                count, stats.last_updated
            );
            stats.save(stats_file).context("Failed to save stats")?;

            // Safety Protocol: If next_cursor is MISSING but has_more=true, it's a protocol error.
            if fetch_result.next_cursor.is_none() && fetch_result.has_more {
                let err_msg = "Protocol Error: has_more=true but next_cursor is missing.";
                eprintln!("{}", err_msg);

                // Record error, preserve old last_ok
                if let Err(e) = record_state_error(state_file, mode, *current_cursor, err_msg) {
                    eprintln!("Failed to record error state: {}", e);
                }

                return Err(anyhow::anyhow!(err_msg));
            }

            let new_cursor_opt = fetch_result.next_cursor;

            // Advance cursor if valid and changed
            if let Some(nc) = new_cursor_opt {
                // Check if stalled: next_cursor same as current AND has_more=true
                if nc == *current_cursor && fetch_result.has_more {
                    let err_msg = format!(
                        "Protocol Error: Stalled cursor {} with has_more=true",
                        *current_cursor
                    );
                    eprintln!("{}", err_msg);
                    if let Err(e) = record_state_error(state_file, mode, *current_cursor, &err_msg)
                    {
                        eprintln!("Failed to record error state: {}", e);
                    }
                    return Err(anyhow::anyhow!(err_msg));
                }

                if nc != *current_cursor {
                    *current_cursor = nc;
                }
            } else {
                // If next_cursor is None, we keep current cursor (EOF state)
            }

            // Always save state on success to update last_ok
            IngestState {
                cursor: *current_cursor,
                mode,
                last_ok: Some(OffsetDateTime::now_utc()),
                last_error: None,
            }
            .save(state_file)
            .context("Failed to save state")?;

            println!("State updated to cursor: {}", *current_cursor);

            Ok(fetch_result.has_more)
        }
        Err(e) => {
            let err_msg = e.to_string();
            eprintln!("Ingest failed: {}", err_msg);

            if let Err(e) = record_state_error(state_file, mode, *current_cursor, &err_msg) {
                eprintln!("Failed to record error state: {}", e);
            }
            Err(e.context("Ingestion cycle failed"))
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
                let mut current_cursor = cursor.unwrap_or(0);

                if cursor.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file, IngestMode::Chronik) {
                        current_cursor = state.cursor;
                        println!("Resuming from state cursor: {}", current_cursor);
                    }
                }

                loop {
                    if batches_processed >= max_batches {
                        println!("Max batches ({}) reached. Stopping.", max_batches);
                        break;
                    }

                    let has_more = process_ingest(
                        fetch_chronik(Some(current_cursor), &domain, limit),
                        &state_file,
                        &stats_file,
                        &mut current_cursor,
                        IngestMode::Chronik,
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
                let mut current_cursor = line_offset.unwrap_or(0);

                if line_offset.is_none() {
                    if let Ok(Some(state)) = IngestState::load(&state_file, IngestMode::File) {
                        current_cursor = state.cursor;
                        println!("Resuming from file offset: {}", current_cursor);
                    }
                }

                process_ingest(
                    fetch_file(&path, current_cursor),
                    &state_file,
                    &stats_file,
                    &mut current_cursor,
                    IngestMode::File,
                )?;
            }
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_event_domain() {
        assert!(is_valid_event_domain("example.com"));
        assert!(is_valid_event_domain("a.b.c"));
        assert!(is_valid_event_domain("my-domain.com"));
        assert!(is_valid_event_domain("x"));

        assert!(!is_valid_event_domain(""));
        assert!(!is_valid_event_domain(" "));
        assert!(!is_valid_event_domain(".start"));
        assert!(!is_valid_event_domain("end."));
        assert!(!is_valid_event_domain("my..domain"));
        assert!(!is_valid_event_domain("bad_char"));
        assert!(!is_valid_event_domain("-start"));
        assert!(!is_valid_event_domain("end-"));
    }

    #[test]
    fn test_build_chronik_url() {
        let cases = vec![
            ("http://host", "http://host/v1/events"),
            ("http://host/", "http://host/v1/events"),
            ("http://host/v1", "http://host/v1/events"),
            ("http://host/v1/events", "http://host/v1/events"),
            ("http://host/prefix", "http://host/prefix/v1/events"),
            ("http://host/prefix/", "http://host/prefix/v1/events"),
        ];

        for (input, expected) in cases {
            let url = build_chronik_url(input).unwrap();
            assert_eq!(url.as_str(), expected);
        }
    }

    #[test]
    fn test_process_ingest_protocol_error_missing_cursor() {
        let dir = std::env::temp_dir().join("heimlern_test_missing_cursor");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let state_file = dir.join("state.json");
        let stats_file = dir.join("stats.json");

        let fetch_result = FetchResult {
            events: vec![],
            next_cursor: None,
            has_more: true,
        };
        let mut cursor = 0;

        let res = process_ingest(
            Ok(fetch_result),
            &state_file,
            &stats_file,
            &mut cursor,
            IngestMode::Chronik,
        );
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("next_cursor is missing"));

        // Check state recorded
        let state = IngestState::load(&state_file, IngestMode::Chronik)
            .unwrap()
            .unwrap();
        assert!(state.last_error.is_some());
    }

    #[test]
    fn test_process_ingest_protocol_error_stalled() {
        let dir = std::env::temp_dir().join("heimlern_test_stalled");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let state_file = dir.join("state.json");
        let stats_file = dir.join("stats.json");

        let fetch_result = FetchResult {
            events: vec![],
            next_cursor: Some(10),
            has_more: true,
        };
        let mut cursor = 10; // Same as next

        let res = process_ingest(
            Ok(fetch_result),
            &state_file,
            &stats_file,
            &mut cursor,
            IngestMode::Chronik,
        );
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("Stalled cursor"));

        let state = IngestState::load(&state_file, IngestMode::Chronik)
            .unwrap()
            .unwrap();
        assert!(state.last_error.is_some());
    }

    #[test]
    fn test_process_ingest_normal() {
        let dir = std::env::temp_dir().join("heimlern_test_normal");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let state_file = dir.join("state.json");
        let stats_file = dir.join("stats.json");

        let fetch_result = FetchResult {
            events: vec![],
            next_cursor: Some(20),
            has_more: true,
        };
        let mut cursor = 10;

        let res = process_ingest(
            Ok(fetch_result),
            &state_file,
            &stats_file,
            &mut cursor,
            IngestMode::Chronik,
        );
        assert!(res.is_ok());
        assert!(res.unwrap()); // has_more
        assert_eq!(cursor, 20);

        let state = IngestState::load(&state_file, IngestMode::Chronik)
            .unwrap()
            .unwrap();
        assert_eq!(state.cursor, 20);
        assert!(state.last_ok.is_some());
        assert!(state.last_error.is_none());
    }

    #[test]
    #[cfg(unix)]
    fn test_process_ingest_save_error_does_not_mask_protocol_error() {
        // Note: This test uses filesystem permissions to simulate save failure.
        // It is Unix-only and includes a skip mechanism for filesystems that don't
        // support permission restrictions (e.g., some CI environments).
        // An alternative would be to inject a filesystem abstraction for testing,
        // but this approach is simpler and sufficient for catching the error-masking bug.
        use std::os::unix::fs::PermissionsExt;

        // Setup: Create a directory that we can make read-only to force a save error
        let dir = std::env::temp_dir().join("heimlern_test_save_error");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);

        // Remove write permissions from the directory to prevent creating files in it.
        // We set mode to 500 (r-x --- ---).
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_mode(0o500);
        std::fs::set_permissions(&dir, perms).unwrap();

        // Verify permissions were actually set (some filesystems may not support this)
        let actual_perms = std::fs::metadata(&dir).unwrap().permissions();
        let actual_mode = actual_perms.mode() & 0o777;
        if actual_mode != 0o500 {
            // Skip test if filesystem doesn't support permission changes
            eprintln!("Warning: Skipping test - filesystem doesn't support permission restriction (mode: {:o})", actual_mode);
            // Cleanup and return early
            let mut perms = std::fs::metadata(&dir).unwrap().permissions();
            perms.set_mode(0o700);
            let _ = std::fs::set_permissions(&dir, perms);
            let _ = std::fs::remove_dir_all(&dir);
            return;
        }

        let state_file = dir.join("state.json");

        // Use a different stats file location that IS writable, because process_ingest
        // tries to save stats BEFORE checking protocol errors. If stats save fails,
        // it returns early. We want to test record_state_error failure specifically.
        // So we need a separate writable dir for stats.
        let writable_dir = std::env::temp_dir().join("heimlern_test_save_error_writable");
        let _ = std::fs::remove_dir_all(&writable_dir);
        let _ = std::fs::create_dir_all(&writable_dir);
        let valid_stats_file = writable_dir.join("stats.json");

        let fetch_result = FetchResult {
            events: vec![],
            next_cursor: None,
            has_more: true, // Protocol error condition
        };
        let mut cursor = 0;

        let res = process_ingest(
            Ok(fetch_result),
            &state_file,       // This save should fail
            &valid_stats_file, // This save should succeed
            &mut cursor,
            IngestMode::Chronik,
        );

        // Cleanup permissions so we can delete the dir
        let mut perms = std::fs::metadata(&dir).unwrap().permissions();
        perms.set_mode(0o700);
        std::fs::set_permissions(&dir, perms).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&writable_dir);

        // Assertions
        assert!(res.is_err());
        let err_str = res.unwrap_err().to_string();

        // We expect the Protocol Error, NOT the Permission Denied error from saving state
        assert!(err_str.contains("Protocol Error"));
        assert!(!err_str.contains("Permission denied"));
    }
}
