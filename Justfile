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
	@echo "✓ alle Beispiel-Dokumente sind valide"

# Lokaler Helper: Schnelltests & Linter – sicher mit Null-Trennung und Quoting
lint:
    @set -euo pipefail; \
    mapfile -d '' files < <(git ls-files -z -- '*.sh' '*.bash' || true); \
    if [ "${#files[@]}" -eq 0 ]; then echo "keine Shell-Dateien"; exit 0; fi; \
    printf '%s\0' "${files[@]}" | xargs -0 bash -n; \
    shfmt -d -i 2 -ci -sr -- "${files[@]}"; \
    shellcheck -S style -- "${files[@]}"
