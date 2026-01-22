#!/bin/bash
set -e

# Build the CLI
echo "Building heimlern-cli..."
cargo build -p heimlern-cli

# Use cargo run to ensure robust invocation
RUN_CMD="cargo run -p heimlern-cli -- ingest"

# Create a temporary directory for test data
TEST_DIR=$(mktemp -d)
trap 'rm -rf "$TEST_DIR"' EXIT

EVENTS_FILE="$TEST_DIR/events.jsonl"
STATE_FILE="$TEST_DIR/heimlern.ingest.file.state.json"
STATS_FILE="$TEST_DIR/heimlern.stats.json"

# Create sample events
cat <<EOF > "$EVENTS_FILE"
{"type": "test", "source": "smoke", "ts": "2023-01-01T10:00:00Z", "id": "1"}
{"type": "test", "source": "smoke", "ts": "2023-01-01T10:01:00Z", "id": "2"}
EOF

# Run ingest - pass 1
echo "Running ingest pass 1..."
$RUN_CMD file \
    --path "$EVENTS_FILE" \
    --state-file "$STATE_FILE" \
    --stats-file "$STATS_FILE"

# Verify state created
if [ ! -f "$STATE_FILE" ]; then
    echo "Error: State file not created."
    exit 1
fi

# Validate state JSON structure using python if available, else skip
if command -v python3 &> /dev/null; then
    python3 tests/validate_state.py "$STATE_FILE"
else
    echo "Python3 not found, skipping deep validation."
fi

# Verify cursor (should be 2) - using grep/awk for robustness
CURSOR=$(grep -o '"cursor": *[0-9]*' "$STATE_FILE" | head -n1 | awk -F': ' '{print $2}')
if [ "$CURSOR" != "2" ]; then
    echo "Error: Unexpected cursor value: $CURSOR"
    cat "$STATE_FILE"
    exit 1
fi

# Add new event
echo '{"type": "test", "source": "smoke", "ts": "2023-01-01T10:02:00Z", "id": "3"}' >> "$EVENTS_FILE"

# Run ingest - pass 2 (resume)
echo "Running ingest pass 2..."
$RUN_CMD file \
    --path "$EVENTS_FILE" \
    --state-file "$STATE_FILE" \
    --stats-file "$STATS_FILE"

# Verify cursor updated to 3
CURSOR=$(grep -o '"cursor": *[0-9]*' "$STATE_FILE" | head -n1 | awk -F': ' '{print $2}')
if [ "$CURSOR" != "3" ]; then
    echo "Error: Unexpected cursor value pass 2: $CURSOR"
    cat "$STATE_FILE"
    exit 1
fi

echo "Smoke test passed!"
