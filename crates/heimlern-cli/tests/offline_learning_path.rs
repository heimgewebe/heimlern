use assert_cmd::Command;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;

#[allow(deprecated)]
#[test]
fn offline_learning_path_emits_schema_valid_read_only_artifact() {
    let temp = tempfile::tempdir().expect("tempdir");
    let output = Command::cargo_bin("heimlern")
        .expect("binary")
        .current_dir(temp.path())
        .args(["learning-path", "offline"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let artifact: Value = serde_json::from_slice(&output).expect("json artifact");
    let schema = learning_path_schema();
    validate_artifact_against_checked_in_schema(&schema, &artifact);

    assert_eq!(artifact["artifact"], "heimlern.offline_learning_path.v1");
    assert_eq!(artifact["mode"], "offline");
    assert_eq!(artifact["writes_production"], false);
    assert!(artifact["steps"].as_array().expect("steps").len() >= 3);
    assert!(artifact["steps"]
        .as_array()
        .expect("steps")
        .iter()
        .all(|step| step["read_only"] == true));

    let entries = fs::read_dir(temp.path()).expect("read tempdir");
    assert_eq!(entries.count(), 0, "CLI must write only to stdout");
}

fn learning_path_schema() -> Value {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root");
    let schema_path = repo_root.join("contracts/learning_path.schema.json");
    serde_json::from_str(&fs::read_to_string(schema_path).expect("schema file"))
        .expect("schema json")
}

fn validate_artifact_against_checked_in_schema(schema: &Value, artifact: &Value) {
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["additionalProperties"], false);

    let object = artifact.as_object().expect("artifact object");
    let required = schema["required"].as_array().expect("required array");
    for field in required {
        let field = field.as_str().expect("required field name");
        assert!(object.contains_key(field), "missing required field {field}");
    }

    let allowed: BTreeSet<_> = schema["properties"]
        .as_object()
        .expect("schema properties")
        .keys()
        .cloned()
        .collect();
    for key in object.keys() {
        assert!(allowed.contains(key), "unexpected artifact field {key}");
    }

    assert_const(schema, artifact, "schema_version");
    assert_const(schema, artifact, "artifact");
    assert_const(schema, artifact, "mode");
    assert_const(schema, artifact, "generated_by");
    assert_const(schema, artifact, "writes_production");

    let steps = artifact["steps"].as_array().expect("steps array");
    let min_items = schema["properties"]["steps"]["minItems"]
        .as_u64()
        .expect("steps minItems") as usize;
    assert!(steps.len() >= min_items);

    let step_schema = &schema["properties"]["steps"]["items"];
    assert_eq!(step_schema["type"], "object");
    assert_eq!(step_schema["additionalProperties"], false);
    let step_allowed: BTreeSet<_> = step_schema["properties"]
        .as_object()
        .expect("step properties")
        .keys()
        .cloned()
        .collect();
    let step_required = step_schema["required"].as_array().expect("step required");
    for step in steps {
        let step_object = step.as_object().expect("step object");
        for field in step_required {
            let field = field.as_str().expect("step required field");
            assert!(
                step_object.contains_key(field),
                "missing step field {field}"
            );
        }
        for key in step_object.keys() {
            assert!(step_allowed.contains(key), "unexpected step field {key}");
        }
        assert_eq!(
            step["read_only"],
            step_schema["properties"]["read_only"]["const"]
        );
        for field in ["id", "title", "command"] {
            assert!(step[field].as_str().is_some_and(|value| !value.is_empty()));
        }
    }
}

fn assert_const(schema: &Value, artifact: &Value, field: &str) {
    let expected = &schema["properties"][field]["const"];
    assert!(
        !expected.is_null(),
        "schema field {field} must define const"
    );
    assert_eq!(&artifact[field], expected, "const mismatch for {field}");
}
