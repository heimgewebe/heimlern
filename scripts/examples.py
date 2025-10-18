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
