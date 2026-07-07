#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import sys

from jsonschema.exceptions import ValidationError

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from validate_json import ContractError, validate_single  # noqa: E402

SCHEMA = ROOT / "contracts" / "policy.feedback.schema.json"
POSITIVE = ROOT / "data" / "samples" / "policy.feedback.sample.json"
NEGATIVE = ROOT / "tests" / "fixtures" / "feedback" / "policy.feedback.bad.json"


def main() -> int:
    validate_single(SCHEMA, POSITIVE)

    try:
        validate_single(SCHEMA, NEGATIVE)
    except (ContractError, ValidationError):
        print("feedback negative fixture rejected")
        return 0

    print("feedback negative fixture was accepted", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
