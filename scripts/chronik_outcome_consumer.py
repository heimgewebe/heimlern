#!/usr/bin/env python3
"""Validate Chronik routing-outcome envelopes and analyze them review-only.

The consumer owns no transport or routing policy. It validates exact mirror
pins, both contract layers, deterministic identities, freshness and redaction
before delegating already-normalized routing outcomes to Heimlern's OLA probe.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

from jsonschema import Draft202012Validator, FormatChecker, ValidationError

from ola_probe import probe_routing_outcomes

ROOT = Path(__file__).resolve().parents[1]
ENVELOPE_SCHEMA_PATH = ROOT / "contracts/mirrors/chronik/operator-routing-outcome-export-v1.schema.json"
ENVELOPE_PIN_PATH = ROOT / "contracts/mirrors/chronik/operator-routing-outcome-export-v1.pin.json"
PAYLOAD_SCHEMA_PATH = ROOT / "contracts/operator.routing_outcome.v1.schema.json"
PROPOSAL_SCHEMA_PATH = ROOT / "contracts/mirrors/metarepo/policy.weight_adjustment.v1.schema.json"
PROPOSAL_PIN_PATH = ROOT / "contracts/mirrors/metarepo/policy.weight_adjustment.v1.pin.json"

_RAW_KEYS = {"raw_log", "raw_logs", "stdout", "stderr", "command_output", "log_excerpt"}
_PRIVATE_PATH_RE = re.compile(r"(?<![A-Za-z0-9._-])/(?:home|root|Users)/")
_SECRET_ASSIGNMENT_RE = re.compile(
    r"\b(?:api[_-]?key|token|secret|password)\s*[:=]\s*\S+", re.IGNORECASE
)
_SECRET_TOKEN_RE = re.compile(
    r"\b(?:bearer\s+\S+|gh[pousr]_[A-Za-z0-9_]{20,}|sk-[A-Za-z0-9_-]{20,}|AKIA[0-9A-Z]{16})\b",
    re.IGNORECASE,
)
_PRIVATE_KEY_RE = re.compile(
    r"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----", re.IGNORECASE
)
_HEX_REVISION_RE = re.compile(r"^[0-9a-f]{40}$")
_SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
_MAX_EXPORTS = 100
_MAX_EXPORT_BYTES = 1024 * 1024
_MAX_TOTAL_INPUT_BYTES = 16 * 1024 * 1024
_ALLOWED_ANALYSIS_STATUSES = {
    "insufficient_evidence",
    "proposal_candidate",
    "proposal_blocked",
}
_DOES_NOT_ESTABLISH = [
    "routing_policy_readiness",
    "automatic_application_permission",
    "production_sample_sufficiency",
    "chronik_runtime_readiness",
    "live_grabowski_producer",
    "causal_route_superiority",
]


class ConsumerError(ValueError):
    """Fail-closed typed consumer error."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class ContractIdentity:
    envelope_revision: str
    envelope_sha256: str
    payload_revision: str
    payload_sha256: str
    proposal_revision: str
    proposal_sha256: str


@dataclass(frozen=True)
class ValidatedEnvelope:
    event_id: str
    payload: dict[str, Any]
    observed_at: datetime
    exported_at: datetime
    canonical_sha256: str


def _reject_constant(value: str) -> None:
    raise ConsumerError("json_non_finite_number", f"non-finite JSON number {value} is forbidden")


def _object_without_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    value: dict[str, Any] = {}
    for key, item in pairs:
        if key in value:
            raise ConsumerError("json_duplicate_key", "duplicate JSON key")
        value[key] = item
    return value


def _strict_json_loads(text: str, *, label: str) -> Any:
    try:
        return json.loads(
            text,
            object_pairs_hook=_object_without_duplicates,
            parse_constant=_reject_constant,
        )
    except ConsumerError:
        raise
    except json.JSONDecodeError as exc:
        raise ConsumerError("json_invalid", f"{label}: {exc}") from exc


def _path_label(path: Path) -> str:
    return path.name or "<input>"


def _load_object(path: Path) -> dict[str, Any]:
    try:
        text = path.read_text(encoding="utf-8")
    except OSError as exc:
        raise ConsumerError("json_read_failed", f"{_path_label(path)}: read failed") from exc
    value = _strict_json_loads(text, label=_path_label(path))
    if not isinstance(value, dict):
        raise ConsumerError(
            "json_object_required", f"{_path_label(path)} must contain a JSON object"
        )
    return value


def canonical_json_bytes(value: Any) -> bytes:
    try:
        text = json.dumps(
            value,
            allow_nan=False,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
    except (TypeError, ValueError) as exc:
        raise ConsumerError("json_not_canonicalizable", str(exc)) from exc
    return text.encode("utf-8")


def sha256_json(value: Any) -> str:
    return hashlib.sha256(canonical_json_bytes(value)).hexdigest()


def event_id_for(export: dict[str, Any]) -> str:
    without_id = dict(export)
    without_id.pop("event_id", None)
    return "sha256:" + sha256_json(without_id)


def _file_sha256(path: Path) -> str:
    try:
        content = path.read_bytes()
    except OSError as exc:
        raise ConsumerError("mirror_read_failed", f"{_path_label(path)}: read failed") from exc
    return hashlib.sha256(content).hexdigest()


def _validate_pin(
    pin_path: Path,
    mirror_path: Path,
    *,
    expected_owner: str,
    expected_repository: str,
    expected_path: str,
    expected_non_claims: set[str],
) -> dict[str, Any]:
    pin = _load_object(pin_path)
    if pin.get("schema_version") != 1:
        raise ConsumerError("mirror_pin_version_invalid", "mirror pin schema_version must be 1")
    if not _HEX_REVISION_RE.fullmatch(str(pin.get("source_revision") or "")):
        raise ConsumerError("mirror_revision_invalid", "mirror source_revision must be a commit SHA")
    if not _SHA256_RE.fullmatch(str(pin.get("sha256") or "")):
        raise ConsumerError("mirror_sha256_invalid", "mirror sha256 must be lowercase hex")
    if pin.get("authority") != "mirror_only":
        raise ConsumerError("mirror_authority_invalid", f"{pin_path} must remain mirror_only")
    if pin.get("owner") != expected_owner:
        raise ConsumerError("mirror_owner_invalid", f"{pin_path} owner is not {expected_owner}")
    if pin.get("source_repository") != expected_repository:
        raise ConsumerError(
            "mirror_repository_invalid",
            f"{pin_path} source repository is not {expected_repository}",
        )
    if pin.get("source_path") != expected_path:
        raise ConsumerError(
            "mirror_path_invalid", f"{pin_path} source path is not {expected_path}"
        )
    if set(pin.get("does_not_establish", [])) != expected_non_claims:
        raise ConsumerError("mirror_non_claims_invalid", f"{pin_path} non-claims are incomplete")
    actual = _file_sha256(mirror_path)
    if actual != pin.get("sha256"):
        raise ConsumerError(
            "mirror_digest_mismatch",
            f"{mirror_path} sha256 {actual} does not match pin {pin.get('sha256')}",
        )
    return pin


def load_contracts() -> tuple[ContractIdentity, dict[str, Any], dict[str, Any], dict[str, Any], dict[str, Any]]:
    envelope_pin = _validate_pin(
        ENVELOPE_PIN_PATH,
        ENVELOPE_SCHEMA_PATH,
        expected_owner="heimgewebe/chronik",
        expected_repository="heimgewebe/chronik",
        expected_path="docs/chronik/operator-routing-outcome-export-v1.schema.json",
        expected_non_claims={
            "heimlern_owns_chronik_envelope",
            "automatic_mirror_refresh",
            "chronik_runtime_readiness",
        },
    )
    proposal_pin = _validate_pin(
        PROPOSAL_PIN_PATH,
        PROPOSAL_SCHEMA_PATH,
        expected_owner="heimgewebe/metarepo",
        expected_repository="heimgewebe/metarepo",
        expected_path="contracts/policy.weight_adjustment.v1.schema.json",
        expected_non_claims={
            "heimlern_owns_policy_weight_contract",
            "automatic_mirror_refresh",
            "proposal_application_permission",
        },
    )
    payload_sha256 = _file_sha256(PAYLOAD_SCHEMA_PATH)
    expected_payload = envelope_pin.get("expected_payload_contract")
    expected_payload_identity = {
        "owner": "heimgewebe/heimlern",
        "source_repository": "heimgewebe/heimlern",
        "source_path": "contracts/operator.routing_outcome.v1.schema.json",
    }
    if not isinstance(expected_payload, dict) or any(
        expected_payload.get(key) != value
        for key, value in expected_payload_identity.items()
    ):
        raise ConsumerError(
            "payload_contract_identity_invalid",
            "Chronik pin does not identify the canonical Heimlern payload contract",
        )
    if not _HEX_REVISION_RE.fullmatch(str(expected_payload.get("source_revision") or "")):
        raise ConsumerError(
            "payload_contract_revision_invalid",
            "Heimlern payload contract revision must be a commit SHA",
        )
    if not _SHA256_RE.fullmatch(str(expected_payload.get("sha256") or "")):
        raise ConsumerError(
            "payload_contract_sha256_invalid",
            "Heimlern payload contract sha256 must be lowercase hex",
        )
    if payload_sha256 != expected_payload.get("sha256"):
        raise ConsumerError(
            "payload_contract_drift",
            "local operator.routing_outcome.v1 schema does not match the Chronik mirror pin",
        )

    envelope_schema = _load_object(ENVELOPE_SCHEMA_PATH)
    payload_schema = _load_object(PAYLOAD_SCHEMA_PATH)
    proposal_schema = _load_object(PROPOSAL_SCHEMA_PATH)
    for label, schema in (
        ("envelope", envelope_schema),
        ("payload", payload_schema),
        ("proposal", proposal_schema),
    ):
        try:
            Draft202012Validator.check_schema(schema)
        except Exception as exc:
            raise ConsumerError("schema_invalid", f"{label} schema is invalid: {exc}") from exc

    identity = ContractIdentity(
        envelope_revision=str(envelope_pin["source_revision"]),
        envelope_sha256=str(envelope_pin["sha256"]),
        payload_revision=str(expected_payload["source_revision"]),
        payload_sha256=payload_sha256,
        proposal_revision=str(proposal_pin["source_revision"]),
        proposal_sha256=str(proposal_pin["sha256"]),
    )
    return identity, envelope_pin, envelope_schema, payload_schema, proposal_schema


def _parse_utc(value: str, *, label: str) -> datetime:
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (AttributeError, ValueError) as exc:
        raise ConsumerError("timestamp_invalid", f"{label} must be an RFC3339 timestamp") from exc
    if parsed.tzinfo is None or parsed.utcoffset() != timezone.utc.utcoffset(parsed):
        raise ConsumerError("timestamp_not_utc", f"{label} must be UTC")
    if parsed.microsecond != 0:
        raise ConsumerError("timestamp_precision_invalid", f"{label} must use whole seconds")
    return parsed.astimezone(timezone.utc)


def _walk(value: Any, *, path: str = "$") -> Iterable[tuple[str, str, Any]]:
    if isinstance(value, dict):
        for key, item in value.items():
            yield path, str(key), item
            yield from _walk(item, path=f"{path}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            yield from _walk(item, path=f"{path}[{index}]")


def _contains_sensitive_text(value: str) -> bool:
    return bool(
        _PRIVATE_PATH_RE.search(value)
        or _PRIVATE_KEY_RE.search(value)
        or _SECRET_TOKEN_RE.search(value)
        or _SECRET_ASSIGNMENT_RE.search(value)
    )


def _validate_redaction(export: dict[str, Any]) -> None:
    for path, key, value in _walk(export):
        if _contains_sensitive_text(key):
            raise ConsumerError(
                "sensitive_key_forbidden", "sensitive material in JSON key is forbidden"
            )
        if key.lower() in _RAW_KEYS:
            raise ConsumerError("raw_output_forbidden", f"raw output field at {path}.{key}")
        if not isinstance(value, str):
            continue
        if _PRIVATE_PATH_RE.search(value):
            raise ConsumerError("private_path_forbidden", "private absolute path is forbidden")
        if _PRIVATE_KEY_RE.search(value) or _SECRET_TOKEN_RE.search(value) or _SECRET_ASSIGNMENT_RE.search(value):
            raise ConsumerError("secret_material_forbidden", "secret-shaped text is forbidden")


def _validate_evidence_identity(export: dict[str, Any]) -> None:
    outer = export.get("evidence_refs")
    inner = export.get("payload", {}).get("evidence_refs")
    if not isinstance(outer, list) or not isinstance(inner, list):
        raise ConsumerError("evidence_refs_missing", "outer and payload evidence_refs are required")

    def normalize(items: list[Any]) -> list[tuple[str, str, str]]:
        normalized: list[tuple[str, str, str]] = []
        seen: dict[tuple[str, str], str] = {}
        for item in items:
            if not isinstance(item, dict):
                raise ConsumerError("evidence_ref_invalid", "evidence ref must be an object")
            key = (str(item.get("kind") or ""), str(item.get("ref") or ""))
            digest = str(item.get("sha256") or "")
            previous = seen.get(key)
            if previous is not None:
                if previous != digest:
                    raise ConsumerError(
                        "evidence_digest_conflict", "conflicting digest for evidence ref"
                    )
                raise ConsumerError("evidence_ref_duplicate", "duplicate evidence ref")
            seen[key] = digest
            normalized.append((key[0], key[1], digest))
        return sorted(normalized)

    if normalize(outer) != normalize(inner):
        raise ConsumerError(
            "evidence_identity_mismatch",
            "transport evidence refs do not match payload evidence refs",
        )


def validate_envelope(
    export: dict[str, Any],
    *,
    envelope_pin: dict[str, Any],
    envelope_schema: dict[str, Any],
    payload_schema: dict[str, Any],
) -> ValidatedEnvelope:
    canonical_size = len(canonical_json_bytes(export))
    if canonical_size > _MAX_EXPORT_BYTES:
        raise ConsumerError(
            "export_too_large",
            f"canonical export exceeds {_MAX_EXPORT_BYTES} bytes",
        )
    _validate_redaction(export)

    format_checker = FormatChecker()
    try:
        Draft202012Validator(envelope_schema, format_checker=format_checker).validate(export)
    except ValidationError as exc:
        raise ConsumerError("envelope_schema_invalid", exc.message) from exc

    if export.get("payload_contract") != envelope_pin.get("expected_payload_contract"):
        raise ConsumerError(
            "payload_contract_mismatch",
            "envelope payload_contract does not match the pinned Heimlern contract",
        )

    payload = export.get("payload")
    if not isinstance(payload, dict):
        raise ConsumerError("payload_object_required", "payload must be an object")
    try:
        Draft202012Validator(payload_schema, format_checker=format_checker).validate(payload)
    except ValidationError as exc:
        raise ConsumerError("payload_schema_invalid", exc.message) from exc

    payload_sha256 = sha256_json(payload)
    if export.get("payload_sha256") != payload_sha256:
        raise ConsumerError("payload_digest_mismatch", "payload_sha256 does not match canonical payload bytes")
    expected_event_id = event_id_for(export)
    if export.get("event_id") != expected_event_id:
        raise ConsumerError("event_identity_mismatch", "event_id does not match canonical export bytes")

    _validate_evidence_identity(export)

    observed_at = _parse_utc(export["freshness"]["observed_at"], label="freshness.observed_at")
    exported_at = _parse_utc(export["freshness"]["exported_at"], label="freshness.exported_at")
    event_ts = _parse_utc(export["ts"], label="ts")
    if exported_at < observed_at:
        raise ConsumerError("freshness_inverted", "exported_at precedes observed_at")
    if event_ts != exported_at:
        raise ConsumerError("event_timestamp_mismatch", "ts must equal freshness.exported_at")
    payload_ts = _parse_utc(payload["ts"], label="payload.ts")
    if payload_ts != observed_at:
        raise ConsumerError(
            "payload_observation_mismatch",
            "payload.ts must equal freshness.observed_at",
        )

    return ValidatedEnvelope(
        event_id=str(export["event_id"]),
        payload=payload,
        observed_at=observed_at,
        exported_at=exported_at,
        canonical_sha256=hashlib.sha256(canonical_json_bytes(export)).hexdigest(),
    )


def consume_exports(
    exports: list[dict[str, Any]],
    *,
    review_time: datetime,
    max_age_seconds: int,
    min_decisions: int,
) -> dict[str, Any]:
    if not exports:
        raise ConsumerError("input_empty", "at least one Chronik export is required")
    if len(exports) > _MAX_EXPORTS:
        raise ConsumerError(
            "input_count_exceeded", f"at most {_MAX_EXPORTS} exports are accepted"
        )
    if max_age_seconds < 0:
        raise ConsumerError("freshness_policy_invalid", "max_age_seconds must be non-negative")
    if min_decisions < 1:
        raise ConsumerError("analysis_policy_invalid", "min_decisions must be at least 1")
    if review_time.tzinfo is None:
        raise ConsumerError("review_time_invalid", "review_time must be timezone-aware")
    review_time = review_time.astimezone(timezone.utc)

    identity, envelope_pin, envelope_schema, payload_schema, proposal_schema = load_contracts()
    validated = [
        validate_envelope(
            export,
            envelope_pin=envelope_pin,
            envelope_schema=envelope_schema,
            payload_schema=payload_schema,
        )
        for export in exports
    ]

    unique: dict[str, ValidatedEnvelope] = {}
    duplicate_event_ids: list[str] = []
    for item in sorted(validated, key=lambda value: (value.event_id, value.canonical_sha256)):
        previous = unique.get(item.event_id)
        if previous is None:
            unique[item.event_id] = item
            continue
        if previous.canonical_sha256 != item.canonical_sha256:
            raise ConsumerError("event_id_collision", f"different exports share {item.event_id}")
        duplicate_event_ids.append(item.event_id)

    decisions: dict[str, ValidatedEnvelope] = {}
    duplicate_decision_ids: list[str] = []
    for item in sorted(unique.values(), key=lambda value: (value.payload["decision_id"], value.event_id)):
        decision_id = str(item.payload["decision_id"])
        previous = decisions.get(decision_id)
        if previous is None:
            decisions[decision_id] = item
            continue
        if sha256_json(previous.payload) != sha256_json(item.payload):
            raise ConsumerError(
                "decision_identity_conflict",
                f"different payloads share decision_id {decision_id}",
            )
        duplicate_decision_ids.append(decision_id)

    fresh: list[ValidatedEnvelope] = []
    stale: list[dict[str, Any]] = []
    for item in sorted(decisions.values(), key=lambda value: (value.observed_at, value.event_id)):
        age_seconds = int((review_time - item.observed_at).total_seconds())
        if item.exported_at > review_time:
            raise ConsumerError("future_export", f"{item.event_id} was exported after review_time")
        if age_seconds < 0:
            raise ConsumerError("future_observation", f"{item.event_id} is newer than review_time")
        if age_seconds > max_age_seconds:
            stale.append({"event_id": item.event_id, "age_seconds": age_seconds, "reason": "stale"})
        else:
            fresh.append(item)

    try:
        analysis = probe_routing_outcomes(
            [item.payload for item in fresh],
            min_decisions=min_decisions,
            proposal_ts=review_time.replace(microsecond=0).isoformat().replace("+00:00", "Z"),
        )
    except (OSError, RuntimeError, ValueError, json.JSONDecodeError) as exc:
        raise ConsumerError("analysis_failed", f"OLA analysis failed: {exc}") from exc
    if not isinstance(analysis, dict) or analysis.get("status") not in _ALLOWED_ANALYSIS_STATUSES:
        raise ConsumerError("analysis_invalid", "OLA analysis returned an invalid status")
    if not isinstance(analysis.get("summary"), dict):
        raise ConsumerError("analysis_invalid", "OLA analysis returned no summary")

    proposal = analysis.get("proposal")
    proposal_validation: dict[str, Any]
    if proposal is None:
        proposal_validation = {
            "status": "not_applicable",
            "contract_revision": identity.proposal_revision,
            "contract_sha256": identity.proposal_sha256,
        }
    else:
        try:
            Draft202012Validator(
                proposal_schema, format_checker=FormatChecker()
            ).validate(proposal)
        except ValidationError as exc:
            raise ConsumerError("proposal_schema_invalid", exc.message) from exc
        proposal_validation = {
            "status": "valid",
            "contract_revision": identity.proposal_revision,
            "contract_sha256": identity.proposal_sha256,
        }

    return {
        "schema_version": 1,
        "kind": "chronik_operator_outcome_consumer_report",
        "status": analysis["status"],
        "review_only": True,
        "input": {
            "received": len(exports),
            "unique_events": len(unique),
            "duplicate_events_ignored": len(duplicate_event_ids),
            "duplicate_event_ids": sorted(duplicate_event_ids),
            "unique_decisions": len(decisions),
            "duplicate_decisions_ignored": len(duplicate_decision_ids),
            "duplicate_decision_ids": sorted(duplicate_decision_ids),
            "fresh": len(fresh),
            "stale_excluded": len(stale),
        },
        "accepted": [
            {
                "event_id": item.event_id,
                "envelope_sha256": item.canonical_sha256,
                "observed_at": item.observed_at.isoformat().replace("+00:00", "Z"),
                "exported_at": item.exported_at.isoformat().replace("+00:00", "Z"),
                "age_seconds": int((review_time - item.observed_at).total_seconds()),
            }
            for item in fresh
        ],
        "freshness_policy": {
            "review_time": review_time.replace(microsecond=0).isoformat().replace("+00:00", "Z"),
            "max_age_seconds": max_age_seconds,
            "consumer_recomputed": True,
        },
        "stale": stale,
        "analysis_policy": {
            "min_decisions": min_decisions,
            "review_only": True,
        },
        "contracts": {
            "chronik_envelope": {
                "revision": identity.envelope_revision,
                "sha256": identity.envelope_sha256,
                "authority": "mirror_only",
            },
            "heimlern_payload": {
                "revision": identity.payload_revision,
                "sha256": identity.payload_sha256,
                "authority": "canonical_local_contract",
            },
            "weight_adjustment_proposal": {
                "revision": identity.proposal_revision,
                "sha256": identity.proposal_sha256,
                "authority": "mirror_only",
            },
        },
        "analysis": analysis,
        "proposal_validation": proposal_validation,
        "writes": [],
        "does_not_establish": _DOES_NOT_ESTABLISH,
    }


def _read_exports(paths: list[Path]) -> list[dict[str, Any]]:
    exports: list[dict[str, Any]] = []
    total_bytes = 0
    for path in paths:
        try:
            content = path.read_bytes()
        except OSError as exc:
            raise ConsumerError(
                "json_read_failed", f"{_path_label(path)}: read failed"
            ) from exc
        total_bytes += len(content)
        if total_bytes > _MAX_TOTAL_INPUT_BYTES:
            raise ConsumerError(
                "input_bytes_exceeded",
                f"input exceeds {_MAX_TOTAL_INPUT_BYTES} bytes",
            )
        try:
            text = content.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise ConsumerError(
                "json_encoding_invalid", f"{_path_label(path)}: UTF-8 required"
            ) from exc
        value = _strict_json_loads(text, label=_path_label(path))
        if isinstance(value, dict):
            exports.append(value)
        elif isinstance(value, list) and all(isinstance(item, dict) for item in value):
            exports.extend(value)
        else:
            raise ConsumerError(
                "json_exports_required",
                f"{_path_label(path)} must contain an object or array of objects",
            )
        if len(exports) > _MAX_EXPORTS:
            raise ConsumerError(
                "input_count_exceeded", f"at most {_MAX_EXPORTS} exports are accepted"
            )
    return exports


def _invalid_report(exc: ConsumerError) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "kind": "chronik_operator_outcome_consumer_report",
        "status": "invalid_input",
        "review_only": True,
        "error": {"code": exc.code, "message": str(exc)},
        "analysis": None,
        "proposal_validation": {"status": "not_run"},
        "writes": [],
        "does_not_establish": _DOES_NOT_ESTABLISH,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inputs", nargs="+", type=Path)
    parser.add_argument("--review-time", help="RFC3339 UTC timestamp; defaults to current UTC")
    parser.add_argument("--max-age-seconds", type=int, default=86400)
    parser.add_argument("--min-decisions", type=int, default=10)
    args = parser.parse_args()
    try:
        review_time = (
            _parse_utc(args.review_time, label="review_time")
            if args.review_time
            else datetime.now(timezone.utc).replace(microsecond=0)
        )
        report = consume_exports(
            _read_exports(args.inputs),
            review_time=review_time,
            max_age_seconds=args.max_age_seconds,
            min_decisions=args.min_decisions,
        )
    except ConsumerError as exc:
        print(json.dumps(_invalid_report(exc), ensure_ascii=False, indent=2, sort_keys=True))
        return 1
    print(json.dumps(report, ensure_ascii=False, indent=2, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
