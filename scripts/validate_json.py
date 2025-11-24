#!/usr/bin/env python3
"""
Kleiner Validator für heimlern-Contracts.
Verwendung:
  # Einzelne Datei validieren:
  python scripts/validate_json.py contracts/policy_snapshot.schema.json /path/to/doc.json

  # Batch-Validierung (CI-Modus):
  python scripts/validate_json.py --schemas contracts/ --samples data/samples/
"""
from __future__ import annotations

import argparse
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


def validate_single(schema_path: pathlib.Path, doc_path: pathlib.Path) -> None:
    """Validate a single document against a schema."""
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    resolver = RefResolver.from_schema(schema)
    validator = Draft202012Validator(schema, resolver=resolver)

    def validate_payload(payload: Any, label: str) -> None:
        validator.validate(payload)
        _extra_checks(schema_path, payload)
        print(f"\u2713 {label} valid against {schema_path.name}")

    if doc_path.suffix == ".jsonl":
        with doc_path.open("r", encoding="utf-8") as handle:
            for idx, raw in enumerate(handle, start=1):
                stripped = raw.strip()
                if not stripped:
                    continue
                try:
                    data = json.loads(stripped)
                except json.JSONDecodeError as exc:  # pragma: no cover - CLI helper
                    raise ContractError(
                        f"line {idx} ist kein valides JSON: {exc}"
                    ) from exc
                validate_payload(data, f"{doc_path.name}:{idx}")
    else:
        data = json.loads(doc_path.read_text(encoding="utf-8"))
        validate_payload(data, doc_path.name)


# Mapping from sample file names to their expected schema
SCHEMA_MAPPING = {
    "aussensensor.jsonl": "aussen_event.schema.json",
}


def validate_batch(schemas_dir: pathlib.Path, samples_dir: pathlib.Path) -> None:
    """Batch validate samples against schemas using naming convention."""
    validated = 0
    for sample_file in samples_dir.iterdir():
        if sample_file.name.startswith("."):
            continue
        if sample_file.name not in SCHEMA_MAPPING:
            print(f"⚠ Skipping {sample_file.name}: no schema mapping defined")
            continue

        schema_name = SCHEMA_MAPPING[sample_file.name]
        schema_path = schemas_dir / schema_name
        if not schema_path.exists():
            raise ContractError(f"Schema {schema_name} not found for {sample_file.name}")

        validate_single(schema_path, sample_file)
        validated += 1

    if validated == 0:
        print("⚠ No samples validated")
    else:
        print(f"\n✓ {validated} sample file(s) validated successfully")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Validate JSON documents against heimlern contracts"
    )
    parser.add_argument(
        "--schemas",
        type=pathlib.Path,
        help="Directory containing schema files (batch mode)",
    )
    parser.add_argument(
        "--samples",
        type=pathlib.Path,
        help="Directory containing sample files to validate (batch mode)",
    )
    parser.add_argument(
        "positional",
        nargs="*",
        help="Schema and document paths (single file mode)",
    )

    args = parser.parse_args()

    # Batch mode: --schemas and --samples
    if args.schemas and args.samples:
        validate_batch(args.schemas.resolve(), args.samples.resolve())
        return 0

    # Single file mode: two positional arguments
    if len(args.positional) == 2:
        schema_path = pathlib.Path(args.positional[0]).resolve()
        doc_path = pathlib.Path(args.positional[1]).resolve()
        validate_single(schema_path, doc_path)
        return 0

    parser.print_help()
    return 2


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover - CLI helper
        print(f"\u274c Validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
