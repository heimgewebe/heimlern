#!/usr/bin/env python3
"""Validate the immutable Heimlern archive-readiness declaration."""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs/archive-readiness.v1.json"


class ArchiveReadinessError(RuntimeError):
    """Raised when the declared archive boundary no longer holds."""


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _require_file(root: Path, path_text: str, expected_sha256: str) -> None:
    path = root / path_text
    if not path.is_file():
        raise ArchiveReadinessError(f"missing bound file: {path_text}")
    if _sha256(path) != expected_sha256:
        raise ArchiveReadinessError(f"bound file changed: {path_text}")


def _validate_proposal_inventory(root: Path, inventory: dict[str, Any]) -> None:
    proposal_files = {
        path.relative_to(root).as_posix()
        for path in (root / "proposals").rglob("*")
        if path.is_file()
    }
    expected = set(inventory["allowed_files"])
    if proposal_files != expected:
        unexpected = sorted(proposal_files - expected)
        missing = sorted(expected - proposal_files)
        raise ArchiveReadinessError(
            f"proposal inventory changed: unexpected={unexpected}, missing={missing}"
        )
    if inventory["active_registration_count"] != 0:
        raise ArchiveReadinessError("active proposal registrations are declared")


def _validate_mirror(root: Path, entry: dict[str, Any]) -> None:
    _require_file(root, entry["local_schema"], entry["local_sha256"])
    pin = json.loads((root / entry["pin"]).read_text(encoding="utf-8"))
    authority = entry["authority"]
    fields = {
        "source_repository": "repository",
        "source_revision": "revision",
        "source_path": "path",
        "sha256": "sha256",
    }
    for pin_key, authority_key in fields.items():
        if pin.get(pin_key) != authority[authority_key]:
            raise ArchiveReadinessError(
                f"mirror pin mismatch: {entry['pin']}:{pin_key}"
            )


def validate() -> dict[str, Any]:
    data = json.loads(MANIFEST.read_text(encoding="utf-8"))
    if data.get("schema_version") != 1:
        raise ArchiveReadinessError("unsupported archive-readiness schema")
    if data.get("status") != "ready_for_archive_effect":
        raise ArchiveReadinessError("archive readiness status is invalid")

    snapshot = data["prearchive_github_snapshot"]
    if snapshot["is_archived"] is not False:
        raise ArchiveReadinessError("prearchive snapshot is not prearchive")
    if snapshot["open_pull_requests"] != 0 or snapshot["open_issues"] != 0:
        raise ArchiveReadinessError("prearchive snapshot contains open work")
    if data["consumer_freeze"]["status"] != "merged_and_green":
        raise ArchiveReadinessError("HausKI consumer freeze is not merged and green")

    for entry in data["normative_mirrors"]:
        _validate_mirror(ROOT, entry)
    for entry in data["local_contracts"]:
        _require_file(ROOT, entry["local_path"], entry["local_sha256"])

    expected_top = {
        Path(entry["local_path"]).as_posix()
        for entry in data["local_contracts"]
        if Path(entry["local_path"]).parent == Path("contracts")
    }
    actual_top = {
        path.relative_to(ROOT).as_posix()
        for path in (ROOT / "contracts").glob("*.json")
    }
    if actual_top != expected_top:
        raise ArchiveReadinessError("top-level contract inventory changed")

    if list(ROOT.rglob("*.proto")):
        raise ArchiveReadinessError("protobuf definitions unexpectedly present")

    boundary = data["offline_proposal_only_boundary"]
    if boundary["status"] != "preserved":
        raise ArchiveReadinessError("offline proposal-only boundary is not preserved")
    for entry in boundary["file_bindings"]:
        _require_file(ROOT, entry["path"], entry["sha256"])

    _validate_proposal_inventory(ROOT, data["proposal_inventory"])

    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    historical = (ROOT / "docs/historical-status.md").read_text(encoding="utf-8")
    if "frozen as implementation and design history" not in readme:
        raise ArchiveReadinessError("README historical boundary missing")
    if "frozen historical reference" not in historical:
        raise ArchiveReadinessError("historical status boundary missing")

    return {
        "status": "valid",
        "kind": data["kind"],
        "normative_mirrors": len(data["normative_mirrors"]),
        "historical_contracts": len(data["local_contracts"]),
        "protobuf_count": 0,
        "active_proposals": 0,
        "consumer_freeze_merge": data["consumer_freeze"]["merge_commit"],
    }


def main() -> int:
    try:
        result = validate()
    except (ArchiveReadinessError, KeyError, OSError, ValueError, json.JSONDecodeError) as exc:
        print(json.dumps({"status": "invalid", "error": str(exc)}, sort_keys=True))
        return 1
    print(json.dumps(result, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
