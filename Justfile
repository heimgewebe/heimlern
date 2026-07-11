set shell := ["bash","-eu","-o","pipefail","-c"]

default: schema-validate

# ---- Python venv für Tools (jsonschema) ----
venv:
	@test -d .venv || python3 -m venv .venv
	. .venv/bin/activate && python -m pip install --upgrade pip
	. .venv/bin/activate && pip install -r requirements-tools.txt

# ---- Beispiele schreiben ----
snapshot-example:
	. .venv/bin/activate || true
	python3 scripts/examples.py
	@ls -l /tmp/heimlern_snapshot.json

feedback-example:
	. .venv/bin/activate || true
	python3 scripts/examples.py
	@ls -l /tmp/heimlern_feedback.json

# ---- Validierung ----
schema-validate: venv
	. .venv/bin/activate && python scripts/examples.py
	. .venv/bin/activate && python scripts/validate_json.py contracts/policy.snapshot.schema.json /tmp/heimlern_snapshot.json
	. .venv/bin/activate && python scripts/validate_json.py contracts/policy.feedback.schema.json /tmp/heimlern_feedback.json
	python3 -m json.tool contracts/operator.routing_decision.v1.schema.json >/dev/null
	python3 -m json.tool contracts/operator.routing_outcome.v1.schema.json >/dev/null
	. .venv/bin/activate && python -m pytest -q tests/test_doc_freshness_registry.py
	. .venv/bin/activate && python -m pytest -q tests/test_chronik_outcome_consumer.py
	. .venv/bin/activate && python scripts/chronik_outcome_consumer.py --review-time 2026-07-10T23:00:00Z --max-age-seconds 7200 tests/fixtures/chronik-outcome/operator-routing-outcome-export.v1.json >/tmp/heimlern_chronik_outcome_report.json
	@echo "✓ alle Beispiel-Dokumente und das RepoBrief-Register sind valide"

# Lokaler Helper: Schnelltests & Linter – sicher mit Null-Trennung und Quoting
lint:
    @set -euo pipefail; \
    mapfile -d '' files < <(git ls-files -z -- '*.sh' '*.bash' || true); \
    if [ "${#files[@]}" -eq 0 ]; then echo "keine Shell-Dateien"; exit 0; fi; \
    printf '%s\0' "${files[@]}" | xargs -0 bash -n; \
    shfmt -d -i 2 -ci -sr -- "${files[@]}"; \
    shellcheck -S style -- "${files[@]}"

# ---- Release-Profil Benchmark ----
release-profile-bench:
	python3 scripts/benchmark_release_profiles.py --out docs/benchmarks/release-profile-comparison.latest.json
