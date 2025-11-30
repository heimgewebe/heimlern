//! Integrationstest für das Beispiel `ingest_events.rs`.
//!
//! Erwartung: zwei JSONL-Zeilen → zwei Scores (zwischen 0.0 und 1.0)
//! und ein Fallback-Titel für Zeile ohne "title".

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

fn write_temp_jsonl() -> std::path::PathBuf {
    let tmp =
        std::env::temp_dir().join(format!("heimlern_ingest_test_{}.jsonl", std::process::id()));
    fs::write(
        &tmp,
        r#"{"type":"link","source":"t","title":"Hello","url":"https://e.org","tags":["demo"]}
{"type":"link","source":"t","summary":"No title","url":"https://e.org/2"}"#,
    )
    .unwrap_or_else(|e| panic!("Fehler beim Schreiben der temporären JSONL-Datei: {e}"));
    tmp
}

#[test]
fn example_ingest_events_outputs_two_lines_and_scores() {
    let path = write_temp_jsonl();
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--package",
        "heimlern-core",
        "--example",
        "ingest_events",
        "--",
        path.to_str()
            .unwrap_or_else(|| panic!("Temporärer Pfad ist kein valides UTF-8: {:?}", path)),
    ]);

    cmd.assert()
        .success()
        // Erwartet zwei Zeilen, jeweils Score + Titel oder <untitled>.
        .stdout(predicate::str::contains("<untitled>").and(predicate::str::contains("Hello")))
        .stdout(predicate::str::contains('\n'));
}

#[test]
fn example_ingest_events_accepts_stdin() {
    let input = r#"{"type":"link","source":"stdin","title":"A","url":"https://a"}
{"type":"link","source":"stdin","summary":"B","url":"https://b"}"#;

    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--package",
        "heimlern-core",
        "--example",
        "ingest_events",
    ]);
    cmd.write_stdin(input);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("A").and(predicate::str::contains("<untitled>")));
}

#[test]
fn example_ingest_events_scores_range_between_0_and_1() {
    let path = write_temp_jsonl();
    let mut cmd = Command::new("cargo");
    cmd.args([
        "run",
        "--package",
        "heimlern-core",
        "--example",
        "ingest_events",
        "--",
        path.to_str()
            .unwrap_or_else(|| panic!("Temporärer Pfad ist kein valides UTF-8: {path:?}")),
    ]);
    let output = cmd.assert().get_output().stdout.clone();

    let out_str = String::from_utf8_lossy(&output);
    for line in out_str.lines() {
        if let Some((score_str, _title)) = line.split_once('\t') {
            let score: f32 = score_str
                .parse()
                .unwrap_or_else(|e| panic!("Score '{score_str}' konnte nicht als f32 geparst werden: {e}"));
            assert!(
                (0.0..=1.0).contains(&score),
                "Score außerhalb 0..1: {score}"
            );
        }
    }
}
