use heimlern_core::Decision;
use std::fs;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct DecisionRecord {
    #[allow(dead_code)]
    decision: Decision,
}

#[test]
fn test_deserialize_decision_from_fixture() {
    let content = fs::read_to_string("../../tests/fixtures/decision/sample.ok.json")
        .expect("Failed to read fixture file");

    // This should succeed now that Decision.why supports array or string
    let _record: DecisionRecord = serde_json::from_str(&content)
        .expect("Failed to deserialize decision fixture with array in 'why' field");
}
