from __future__ import annotations

import importlib.util
import json
from datetime import datetime, timezone
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("proposal_gate", ROOT / "scripts/validate_proposal_registrations.py")
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
NOW = datetime(2026, 7, 10, tzinfo=timezone.utc)


def _valid(tmp_path: Path) -> Path:
    payload = json.loads((ROOT / "proposals/_template.json").read_text(encoding="utf-8"))
    payload["proposal_id"] = "routing-friction-v1"
    payload["consumer"] = {"organ": "grabowski", "use": "Review whether routing friction falls after one bounded weight change."}
    payload["decision_target"] = {"question": "Should the reviewed route weight be changed?", "owner": "bureau"}
    payload["success_metric"] = {
        "name": "blocked_run_rate",
        "measure": "Compare blocked-run rate across equivalent pre/post windows.",
        "success": "At least 10 percent lower.",
        "falsification": "No reduction or a higher rate."
    }
    payload["closure"]["archive_path"] = "proposals/_archive/routing-friction-v1.json"
    path = tmp_path / "routing-friction-v1.json"
    path.write_text(json.dumps(payload), encoding="utf-8")
    return path


def test_valid_registration(tmp_path: Path) -> None:
    payload = MODULE.validate(_valid(tmp_path), now=NOW)
    assert payload["boundary"]["no_auto_policy"] is True


def test_missing_consumer_fails(tmp_path: Path) -> None:
    path = _valid(tmp_path)
    payload = json.loads(path.read_text())
    del payload["consumer"]
    path.write_text(json.dumps(payload))
    with pytest.raises(Exception):
        MODULE.validate(path, now=NOW)


def test_expired_registration_fails(tmp_path: Path) -> None:
    path = _valid(tmp_path)
    payload = json.loads(path.read_text())
    payload["expires_at"] = "2026-07-01T00:00:00Z"
    path.write_text(json.dumps(payload))
    with pytest.raises(ValueError, match="expired"):
        MODULE.validate(path, now=NOW)


def test_archive_path_is_deterministic(tmp_path: Path) -> None:
    path = _valid(tmp_path)
    payload = json.loads(path.read_text())
    payload["closure"]["archive_path"] = "proposals/_archive/other.json"
    path.write_text(json.dumps(payload))
    with pytest.raises(ValueError, match="archive_path"):
        MODULE.validate(path, now=NOW)
