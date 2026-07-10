#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

from jsonschema import Draft202012Validator, FormatChecker

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "contracts/learning.proposal.registration.v1.schema.json"
PROPOSALS = ROOT / "proposals"
PLACEHOLDER = "replace-with"


def _load(path: Path) -> dict:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: root must be object")
    return value


def validate(path: Path, *, now: datetime | None = None) -> dict:
    payload = _load(path)
    schema = _load(SCHEMA)
    Draft202012Validator(schema, format_checker=FormatChecker()).validate(payload)
    serialized = json.dumps(payload, sort_keys=True)
    if PLACEHOLDER in serialized:
        raise ValueError(f"{path}: unresolved template placeholder")
    expires = datetime.fromisoformat(payload["expires_at"].replace("Z", "+00:00"))
    review = datetime.fromisoformat(payload["closure"]["review_at"].replace("Z", "+00:00"))
    clock = now or datetime.now(timezone.utc)
    if expires <= clock:
        raise ValueError(f"{path}: proposal registration already expired")
    if review > expires:
        raise ValueError(f"{path}: review_at must not be after expires_at")
    expected = f"proposals/_archive/{payload['proposal_id']}.json"
    if payload["closure"]["archive_path"] != expected:
        raise ValueError(f"{path}: archive_path must equal {expected}")
    return payload


def validate_all(*, now: datetime | None = None) -> dict:
    files = sorted(path for path in PROPOSALS.glob("*.json") if not path.name.startswith("_"))
    ids: set[str] = set()
    for path in files:
        payload = validate(path, now=now)
        proposal_id = payload["proposal_id"]
        if proposal_id in ids:
            raise ValueError(f"duplicate proposal_id: {proposal_id}")
        if path.stem != proposal_id:
            raise ValueError(f"{path}: filename must match proposal_id")
        ids.add(proposal_id)
    return {"status": "valid", "registrations": len(files)}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("path", nargs="?", type=Path)
    args = parser.parse_args()
    result = validate(args.path.resolve()) if args.path else validate_all()
    print(json.dumps(result if args.path is None else {"status": "valid", "proposal_id": result["proposal_id"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
