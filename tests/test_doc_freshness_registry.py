from __future__ import annotations

from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = ROOT / "docs" / "doc-freshness-registry.yml"
EXPECTED_IDS = {
    "contract-ownership-boundary",
    "learning-cycle-proposal-gate",
    "no-auto-apply-boundary",
    "weight-adjustment-output",
    "chronik-routing-outcome-ingest",
    "metrics-workflow-local-contract",
}
ALLOWED_EVIDENCE_KINDS = {
    "file",
    "fixture",
    "sample",
    "schema",
    "script",
    "symbol",
    "test",
    "text",
    "workflow",
}


def _load_registry() -> dict:
    assert REGISTRY.is_file()
    data = yaml.safe_load(REGISTRY.read_text(encoding="utf-8"))
    assert isinstance(data, dict)
    return data


def test_doc_freshness_registry_shape_and_scope() -> None:
    data = _load_registry()

    assert data["kind"] == "heimlern.doc_freshness_registry"
    assert data["version"] == "1.0"
    assert data["authority"] == "diagnostic_signal"
    assert data["risk_class"] == "diagnostic"
    assert data["scope"] == "audited_agent_relevant_claims_only"
    assert "RepoBrief or rLens is a source of task truth" in data["does_not_prove"]

    entries = data["entries"]
    assert isinstance(entries, list)
    assert {entry["id"] for entry in entries} == EXPECTED_IDS
    assert len(entries) == len(EXPECTED_IDS), "registry must stay minimal, not a general doc index"


def test_doc_freshness_registry_entries_are_bounded_and_resolvable() -> None:
    data = _load_registry()

    for entry in data["entries"]:
        assert entry["status"] in {"done", "partial", "stale", "historical"}
        assert entry["doc"].endswith(('.md', '.yml', '.yaml'))
        doc_path = ROOT / entry["doc"]
        assert doc_path.is_file(), entry["doc"]
        assert entry["claim"].strip()
        assert entry["notes"].strip()

        evidence = entry["evidence"]
        assert isinstance(evidence, list) and evidence
        for item in evidence:
            assert item["kind"] in ALLOWED_EVIDENCE_KINDS
            target = item["target"]
            assert isinstance(target, str) and target.strip()
            path_part = target.split("::", 1)[0]
            assert not path_part.startswith("/")
            assert ".." not in Path(path_part).parts
            assert (ROOT / path_part).exists(), target
