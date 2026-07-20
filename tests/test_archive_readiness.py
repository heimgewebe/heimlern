from __future__ import annotations

import importlib.util
import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/validate_archive_readiness.py"


def _load_validator():
    spec = importlib.util.spec_from_file_location("validate_archive_readiness", SCRIPT)
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_current_archive_readiness_is_valid() -> None:
    module = _load_validator()
    result = module.validate()
    assert result["status"] == "valid"
    assert result["consumer_freeze_merge"] == "4af4d397c67acefd3dc385393f1ef55b88adc097"
    assert result["active_proposals"] == 0


def test_changed_bound_contract_fails_closed(tmp_path: Path, monkeypatch) -> None:
    module = _load_validator()
    manifest = json.loads(module.MANIFEST.read_text(encoding="utf-8"))
    manifest["local_contracts"][0]["local_sha256"] = "0" * 64
    altered = tmp_path / "archive-readiness.json"
    altered.write_text(json.dumps(manifest), encoding="utf-8")
    monkeypatch.setattr(module, "MANIFEST", altered)

    with pytest.raises(module.ArchiveReadinessError, match="bound file changed"):
        module.validate()


def test_unknown_proposal_file_fails_closed(tmp_path: Path) -> None:
    module = _load_validator()
    proposals = tmp_path / "proposals"
    proposals.mkdir()
    (proposals / "README.md").write_text("history", encoding="utf-8")
    (proposals / "_template.json").write_text("{}", encoding="utf-8")
    (proposals / "active.json").write_text("{}", encoding="utf-8")
    inventory = {
        "active_registration_count": 0,
        "allowed_files": ["proposals/README.md", "proposals/_template.json"],
    }

    with pytest.raises(module.ArchiveReadinessError, match="unexpected=.*active.json"):
        module._validate_proposal_inventory(tmp_path, inventory)
