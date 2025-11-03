### 📄 scripts/examples.py

**Größe:** 1 KB | **md5:** `7847620050da59acbf01323e4d6fe208`

```python
#!/usr/bin/env python3
"""
Schreibt minimal-belegte Beispiel-Dokumente in /tmp für schnelles Testen.
"""
from __future__ import annotations

import json
import time
from datetime import datetime, timezone
from pathlib import Path


def iso_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def write(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        json.dump(obj, handle, ensure_ascii=False, indent=2)
    print("→", path)


def main() -> None:
    snapshot = {
        "version": "0.1.0",
        "policy_id": "example-policy",
        "ts": iso_now(),
        "arms": ["a", "b", "c"],
        "counts": [10, 5, 2],
        "values": [0.12, 0.18, 0.05],
        "epsilon": 0.1,
        "seed": 42,
    }
    feedback = {
        "version": "0.1.0",
        "policy_id": "example-policy",
        "ts": iso_now(),
        "decision_id": "dec-" + str(int(time.time())),
        "reward": 1.0,
        "notes": "first feedback",
    }

    write(Path("/tmp/heimlern_snapshot.json"), snapshot)
    write(Path("/tmp/heimlern_feedback.json"), feedback)


if __name__ == "__main__":
    main()
```

### 📄 scripts/validate_json.py

**Größe:** 3 KB | **md5:** `57248893a205fc3d7d2b83bb71f5da30`

```python
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

    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # pragma: no cover - CLI helper
        print(f"\u274c Validation failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
```

