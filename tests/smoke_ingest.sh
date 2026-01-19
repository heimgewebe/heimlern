#!/bin/bash
set -e

# Build the CLI
echo "Building heimlern-cli..."
cargo build -p heimlern-cli

# Create a temporary directory for test data
TEST_DIR=$(mktemp -d)
trap 'rm -rf "$TEST_DIR"' EXIT

EVENTS_FILE="$TEST_DIR/events.jsonl"
STATE_FILE="$TEST_DIR/heimlern.ingest.state.json"
STATS_FILE="$TEST_DIR/heimlern.stats.json"

# Create sample events (Raw AussenEvents for file mode)
cat <<EOF > "$EVENTS_FILE"
{"type": "test", "source": "smoke", "ts": "2023-01-01T10:00:00Z", "id": "1"}
{"type": "test", "source": "smoke", "ts": "2023-01-01T10:01:00Z", "id": "2"}
EOF

# Run ingest
echo "Running ingest..."
./target/debug/heimlern ingest chronik \
    --file "$EVENTS_FILE" \
    --state-file "$STATE_FILE" \
    --stats-file "$STATS_FILE"

# Verify state file exists
if [ ! -f "$STATE_FILE" ]; then
    echo "Error: State file not created."
    exit 1
fi

# Verify stats file exists
if [ ! -f "$STATS_FILE" ]; then
    echo "Error: Stats file not created."
    exit 1
fi

# Verify cursor (should be the timestamp of the last event in file mode)
CURSOR=$(grep -o '"cursor": *"[^"]*"' "$STATE_FILE" | cut -d'"' -f4)
if [ "$CURSOR" != "2023-01-01T10:01:00Z" ]; then
    echo "Error: Unexpected cursor value: $CURSOR"
    cat "$STATE_FILE"
    exit 1
fi

# Verify stats
# We expect total_processed = 2
TOTAL=$(grep -o '"total_processed": *[0-9]*' "$STATS_FILE" | awk -F': ' '{print $2}')
if [ "$TOTAL" != "2" ]; then
    echo "Error: Unexpected total_processed value: $TOTAL"
    cat "$STATS_FILE"
    exit 1
fi

echo "Smoke test passed!"
