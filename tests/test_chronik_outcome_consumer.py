from __future__ import annotations

import copy
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPTS = ROOT / "scripts"
if str(SCRIPTS) not in sys.path:
    sys.path.insert(0, str(SCRIPTS))

import chronik_outcome_consumer as consumer  # noqa: E402

FIXTURE = ROOT / "tests/fixtures/chronik-outcome/operator-routing-outcome-export.v1.json"
REVIEW_TIME = datetime(2026, 7, 10, 23, 0, 0, tzinfo=timezone.utc)


def load_fixture() -> dict:
    return json.loads(FIXTURE.read_text(encoding="utf-8"))


def rehash(export: dict) -> dict:
    export["payload_sha256"] = consumer.sha256_json(export["payload"])
    export["event_id"] = consumer.event_id_for(export)
    return export


def consume(exports: list[dict], **kwargs) -> dict:
    return consumer.consume_exports(
        exports,
        review_time=kwargs.get("review_time", REVIEW_TIME),
        max_age_seconds=kwargs.get("max_age_seconds", 7200),
        min_decisions=kwargs.get("min_decisions", 10),
    )


def assert_error(code: str, export: dict) -> None:
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([export])
    assert caught.value.code == code


def failed_export(index: int) -> dict:
    export = load_fixture()
    export["source"]["run_id"] = f"outcome-export-failed-{index:03d}"
    payload = export["payload"]
    payload["decision_id"] = f"grabowski-route-failed-{index:03d}"
    evidence_ref = f"grabowski-task:failed-{index:03d}"
    payload["evidence_refs"][0]["ref"] = evidence_ref
    export["evidence_refs"][0]["ref"] = evidence_ref
    payload["route_used"] = "direct:patch"
    payload["outcome"] = "failure"
    payload["resolved"] = False
    payload["reward"] = -0.8
    payload["metrics"].update(
        {
            "completion_state": "failed",
            "friction_count": 1,
            "ci_state": "fail",
            "pr_state": "closed",
            "rework_count": 2,
        }
    )
    payload["friction"] = [
        {
            "kind": "operator_bug",
            "surface": "local_tool",
            "operation": "patch_apply",
            "resolved": False,
            "fallback": "stopped without mutation",
        }
    ]
    return rehash(export)


def test_valid_fixture_is_review_only_and_insufficient() -> None:
    report = consume([load_fixture()])
    assert report["status"] == "insufficient_evidence"
    assert report["review_only"] is True
    assert report["input"] == {
        "received": 1,
        "unique_events": 1,
        "duplicate_events_ignored": 0,
        "duplicate_event_ids": [],
        "unique_decisions": 1,
        "duplicate_decisions_ignored": 0,
        "duplicate_decision_ids": [],
        "fresh": 1,
        "stale_excluded": 0,
    }
    assert report["analysis"]["summary"]["total"] == 1
    assert report["accepted"][0]["event_id"] == load_fixture()["event_id"]
    assert report["proposal_validation"]["status"] == "not_applicable"
    assert report["writes"] == []


def test_duplicate_event_is_ignored_deterministically(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        consumer,
        "probe_routing_outcomes",
        lambda payloads, min_decisions, proposal_ts=None: {
            "schema_version": 1,
            "kind": "ola_analyzer_probe",
            "status": "insufficient_evidence",
            "summary": {"total": len(payloads)},
            "proposal": None,
            "does_not_establish": [],
        },
    )
    export = load_fixture()
    report = consume([copy.deepcopy(export), copy.deepcopy(export)])
    assert report["input"]["received"] == 2
    assert report["input"]["unique_events"] == 1
    assert report["input"]["duplicate_events_ignored"] == 1
    assert report["input"]["duplicate_event_ids"] == [export["event_id"]]
    assert report["analysis"]["summary"]["total"] == 1


def test_stale_event_is_excluded_and_freshness_is_recomputed() -> None:
    report = consume(
        [load_fixture()],
        review_time=datetime(2026, 7, 12, 0, 0, 0, tzinfo=timezone.utc),
        max_age_seconds=60,
    )
    assert report["status"] == "insufficient_evidence"
    assert report["input"]["fresh"] == 0
    assert report["input"]["stale_excluded"] == 1
    assert report["freshness_policy"]["consumer_recomputed"] is True
    assert report["analysis"]["summary"]["total"] == 0


def test_payload_contract_drift_is_rejected() -> None:
    export = load_fixture()
    export["payload_contract"]["source_revision"] = "0" * 40
    export["event_id"] = consumer.event_id_for(export)
    assert_error("payload_contract_mismatch", export)


def test_payload_digest_mismatch_is_rejected() -> None:
    export = load_fixture()
    export["payload"]["reward"] = 0.7
    assert_error("payload_digest_mismatch", export)


def test_event_identity_mismatch_is_rejected() -> None:
    export = load_fixture()
    export["source"]["run_id"] = "changed-run"
    assert_error("event_identity_mismatch", export)


def test_evidence_digest_identity_mismatch_is_rejected() -> None:
    export = load_fixture()
    export["evidence_refs"][0]["sha256"] = "2" * 64
    export["event_id"] = consumer.event_id_for(export)
    assert_error("evidence_identity_mismatch", export)


def test_secret_shaped_text_is_rejected_before_analysis() -> None:
    export = load_fixture()
    export["payload"]["metrics"]["friction_count"] = 1
    export["payload"]["friction"] = [
        {
            "kind": "unknown",
            "surface": "runtime",
            "resolved": False,
            "fallback": "Authorization: " + "Bearer " + "example-value",
        }
    ]
    assert_error("secret_material_forbidden", rehash(export))


def test_private_absolute_path_is_rejected_before_analysis() -> None:
    export = load_fixture()
    export["payload"]["metrics"]["friction_count"] = 1
    export["payload"]["friction"] = [
        {
            "kind": "unknown",
            "surface": "runtime",
            "resolved": False,
            "fallback": "/home/example/private/receipt.json",
        }
    ]
    assert_error("private_path_forbidden", rehash(export))


def test_event_newer_than_review_time_is_rejected() -> None:
    export = load_fixture()
    with pytest.raises(consumer.ConsumerError) as caught:
        consume(
            [export],
            review_time=datetime(2026, 7, 10, 22, 0, 0, tzinfo=timezone.utc),
        )
    assert caught.value.code == "future_export"


def test_failed_corpus_produces_schema_valid_review_only_proposal() -> None:
    report = consume([failed_export(index) for index in range(10)])
    assert report["status"] == "proposal_candidate"
    assert report["analysis"]["proposal"] is not None
    assert report["proposal_validation"]["status"] == "valid"
    assert report["analysis"]["proposal"]["status"] == "proposed"
    assert report["analysis"]["proposal"]["evidence"]["decisions_analyzed"] == 10
    assert report["writes"] == []
    assert "automatic_application_permission" in report["does_not_establish"]


def test_cli_emits_machine_readable_invalid_input(tmp_path: Path) -> None:
    export = load_fixture()
    export["event_id"] = "sha256:" + "0" * 64
    path = tmp_path / "invalid.json"
    path.write_text(json.dumps(export), encoding="utf-8")
    completed = subprocess.run(
        [
            sys.executable,
            str(ROOT / "scripts/chronik_outcome_consumer.py"),
            "--review-time",
            "2026-07-10T23:00:00Z",
            str(path),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    assert completed.returncode == 1
    report = json.loads(completed.stdout)
    assert report["status"] == "invalid_input"
    assert report["error"]["code"] == "event_identity_mismatch"
    assert report["writes"] == []


def test_empty_input_is_rejected() -> None:
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([])
    assert caught.value.code == "input_empty"


def test_future_export_is_rejected_even_when_observation_is_older() -> None:
    export = load_fixture()
    export["ts"] = "2026-07-10T23:01:00Z"
    export["freshness"]["exported_at"] = "2026-07-10T23:01:00Z"
    export["event_id"] = consumer.event_id_for(export)
    assert_error("future_export", export)


def test_payload_timestamp_must_match_observation_timestamp() -> None:
    export = load_fixture()
    export["payload"]["ts"] = "2026-07-10T22:39:00Z"
    assert_error("payload_observation_mismatch", rehash(export))


def test_mirror_digest_drift_is_rejected(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    drifted = tmp_path / "chronik-envelope.schema.json"
    drifted.write_bytes(consumer.ENVELOPE_SCHEMA_PATH.read_bytes() + b"\n")
    monkeypatch.setattr(consumer, "ENVELOPE_SCHEMA_PATH", drifted)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "mirror_digest_mismatch"


def test_mirror_source_identity_is_fail_closed(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    pin = json.loads(consumer.ENVELOPE_PIN_PATH.read_text(encoding="utf-8"))
    pin["source_repository"] = "heimgewebe/other"
    bad_pin = tmp_path / "chronik-envelope.pin.json"
    bad_pin.write_text(json.dumps(pin), encoding="utf-8")
    monkeypatch.setattr(consumer, "ENVELOPE_PIN_PATH", bad_pin)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "mirror_repository_invalid"


def test_missing_mirror_is_reported_as_typed_error(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    monkeypatch.setattr(consumer, "ENVELOPE_SCHEMA_PATH", tmp_path / "missing.json")
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "mirror_read_failed"


def test_rust_analysis_failure_is_reported_as_typed_error(
    monkeypatch: pytest.MonkeyPatch
) -> None:
    def fail_analysis(payloads, min_decisions, proposal_ts=None):
        raise RuntimeError("heimlern-ola unavailable")

    monkeypatch.setattr(consumer, "probe_routing_outcomes", fail_analysis)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "analysis_failed"


def test_duplicate_decision_is_not_double_counted() -> None:
    first = load_fixture()
    second = copy.deepcopy(first)
    second["source"]["run_id"] = "outcome-export-replay-002"
    second["event_id"] = consumer.event_id_for(second)
    report = consume([first, second])
    assert report["input"]["unique_events"] == 2
    assert report["input"]["unique_decisions"] == 1
    assert report["input"]["duplicate_decisions_ignored"] == 1
    assert report["input"]["duplicate_decision_ids"] == [
        first["payload"]["decision_id"]
    ]
    assert report["analysis"]["summary"]["total"] == 1


def test_conflicting_duplicate_decision_is_rejected() -> None:
    first = load_fixture()
    second = copy.deepcopy(first)
    second["source"]["run_id"] = "outcome-export-conflict-002"
    second["payload"]["reward"] = 0.7
    rehash(second)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([first, second])
    assert caught.value.code == "decision_identity_conflict"


def test_redaction_runs_before_schema_error_and_does_not_echo_value() -> None:
    export = load_fixture()
    export["unexpected"] = "Authorization: " + "Bearer " + "sensitive-example"
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([export])
    assert caught.value.code == "secret_material_forbidden"
    assert "sensitive-example" not in str(caught.value)


def test_input_count_is_bounded(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(consumer, "_MAX_EXPORTS", 1)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture(), load_fixture()])
    assert caught.value.code == "input_count_exceeded"


def test_duplicate_json_keys_are_rejected(tmp_path: Path) -> None:
    path = tmp_path / "duplicate.json"
    path.write_text('{"schema_version":1,"schema_version":2}', encoding="utf-8")
    with pytest.raises(consumer.ConsumerError) as caught:
        consumer._read_exports([path])
    assert caught.value.code == "json_duplicate_key"
    assert "schema_version" not in str(caught.value)


def test_non_finite_json_numbers_are_rejected(tmp_path: Path) -> None:
    path = tmp_path / "nan.json"
    path.write_text('{"value":NaN}', encoding="utf-8")
    with pytest.raises(consumer.ConsumerError) as caught:
        consumer._read_exports([path])
    assert caught.value.code == "json_non_finite_number"


def test_analysis_result_shape_is_fail_closed(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(consumer, "probe_routing_outcomes", lambda *_args, **_kwargs: {})
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "analysis_invalid"


def test_pin_revision_and_payload_identity_are_fail_closed(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path
) -> None:
    canonical_pin_path = consumer.ENVELOPE_PIN_PATH
    canonical_pin = json.loads(canonical_pin_path.read_text(encoding="utf-8"))

    bad_revision = dict(canonical_pin)
    bad_revision["source_revision"] = "main"
    bad_pin = tmp_path / "chronik.pin.json"
    bad_pin.write_text(json.dumps(bad_revision), encoding="utf-8")
    monkeypatch.setattr(consumer, "ENVELOPE_PIN_PATH", bad_pin)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "mirror_revision_invalid"

    bad_identity = json.loads(canonical_pin_path.read_text(encoding="utf-8"))
    bad_identity["expected_payload_contract"]["owner"] = "heimgewebe/other"
    bad_pin.write_text(json.dumps(bad_identity), encoding="utf-8")
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "payload_contract_identity_invalid"


def test_review_time_requires_whole_seconds() -> None:
    with pytest.raises(consumer.ConsumerError) as caught:
        consumer._parse_utc("2026-07-10T23:00:00.500Z", label="review_time")
    assert caught.value.code == "timestamp_precision_invalid"


def test_redaction_catches_case_variants_uri_paths_and_sensitive_keys() -> None:
    values = [
        "authorization: bearer example-value",
        "file:///home/example/private.json",
        "-----BEGIN RSA " + "PRIVATE KEY-----",
    ]
    expected = ["secret_material_forbidden", "private_path_forbidden", "secret_material_forbidden"]
    for value, code in zip(values, expected):
        export = load_fixture()
        export["payload"]["metrics"]["friction_count"] = 1
        export["payload"]["friction"] = [{
            "kind": "unknown",
            "surface": "runtime",
            "resolved": False,
            "fallback": value,
        }]
        with pytest.raises(consumer.ConsumerError) as caught:
            consume([rehash(export)])
        assert caught.value.code == code
        assert value not in str(caught.value)

    export = load_fixture()
    export["Authorization: Bearer example-value"] = "x"
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([export])
    assert caught.value.code == "sensitive_key_forbidden"
    assert "example-value" not in str(caught.value)


def test_proposal_timestamp_is_bound_to_review_time() -> None:
    report = consume([failed_export(index) for index in range(10)])
    assert report["analysis"]["proposal"]["ts"] == "2026-07-10T23:00:00Z"


def test_rust_timeout_is_converted_to_typed_analysis_error(
    monkeypatch: pytest.MonkeyPatch
) -> None:
    import subprocess
    import ola_adapter

    def timeout(*_args, **_kwargs):
        raise subprocess.TimeoutExpired(cmd=["heimlern-ola"], timeout=1)

    monkeypatch.setattr(ola_adapter.subprocess, "run", timeout)
    with pytest.raises(consumer.ConsumerError) as caught:
        consume([load_fixture()])
    assert caught.value.code == "analysis_failed"
    assert "timed out" in str(caught.value)
