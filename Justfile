set shell := ["bash","-eu","-o","pipefail","-c"]

default: schema:validate

# ---- Python venv für Tools (jsonschema) ----
venv:
	@test -d .venv || python3 -m venv .venv
	. .venv/bin/activate && python -m pip install --upgrade pip
	. .venv/bin/activate && pip install -r requirements-tools.txt

# ---- Beispiele schreiben ----
snapshot:example:
	. .venv/bin/activate || true
	python3 scripts/examples.py
	@ls -l /tmp/heimlern_snapshot.json

feedback:example:
	. .venv/bin/activate || true
	python3 scripts/examples.py
	@ls -l /tmp/heimlern_feedback.json

# ---- Validierung ----
schema:validate: venv
	. .venv/bin/activate && python scripts/examples.py
	. .venv/bin/activate && python scripts/validate_json.py contracts/policy_snapshot.schema.json /tmp/heimlern_snapshot.json
	. .venv/bin/activate && python scripts/validate_json.py contracts/policy_feedback.schema.json /tmp/heimlern_feedback.json
	@echo "✓ alle Beispiel-Dokumente sind valide"
