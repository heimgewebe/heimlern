#!/usr/bin/env python3
"""
Kleiner Validator für heimlern-Contracts.
Verwendung:
  python scripts/validate_json.py contracts/policy_snapshot.schema.json /path/to/doc.json
"""
from __future__ import annotations

import json
import pathlib
import sys
from typing import Any, Sequence

from jsonschema import Draft202012Validator, RefResolver


class ContractError(ValueError):
    pass


def _extra_checks(schema_path: pathlib.Path, data: Any) -> None:
    if schema_path.name == "policy_snapshot.schema.json":
        arms = data.get("arms")
        counts = data.get("counts")
        values = data.get("values")
        if isinstance(arms, Sequence) and not isinstance(arms, (str, bytes)):
            expected = len(arms)
            if isinstance(counts, Sequence) and not isinstance(counts, (str, bytes)):
                if len(counts) != expected:
                    raise ContractError("counts length must match arms length")
            if isinstance(values, Sequence) and not isinstance(values, (str, bytes)):
                if len(values) != expected:
                    raise ContractError("values length must match arms length")


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: validate_json.py <schema.json> <document.json>", file=sys.stderr)
        return 2

    schema_path = pathlib.Path(sys.argv[1]).resolve()
    doc_path = pathlib.Path(sys.argv[2]).resolve()

    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    data = json.loads(doc_path.read_text(encoding="utf-8"))

    resolver = RefResolver.from_schema(schema)
    Draft202012Validator(schema, resolver=resolver).validate(data)

    _extra_checks(schema_path, data)

    print(f"\u2713 {doc_path.name} valid against {schema_path.name}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover - CLI helper
        print(f"\u274c Validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
