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
# TS 1 and 2
cat <<EOF > "$EVENTS_FILE"
{"type": "test", "source": "smoke", "ts": "2023-01-01T10:00:00Z", "id": "1"}
{"type": "test", "source": "smoke", "ts": "2023-01-01T10:01:00Z", "id": "2"}
EOF

# Run ingest - pass 1 (bootstrap simulation)
echo "Running ingest pass 1..."
./target/debug/heimlern ingest chronik \
    --file "$EVENTS_FILE" \
    --state-file "$STATE_FILE" \
    --stats-file "$STATS_FILE" \
    --max-batches 1

# Verify state created
if [ ! -f "$STATE_FILE" ]; then
    echo "Error: State file not created."
    exit 1
fi

# Verify cursor (should be last TS in file mode)
CURSOR=$(grep -o '"cursor": *"[^"]*"' "$STATE_FILE" | cut -d'"' -f4)
if [ "$CURSOR" != "2023-01-01T10:01:00Z" ]; then
    echo "Error: Unexpected cursor value: $CURSOR"
    cat "$STATE_FILE"
    exit 1
fi

# Add new event
echo '{"type": "test", "source": "smoke", "ts": "2023-01-01T10:02:00Z", "id": "3"}' >> "$EVENTS_FILE"

# Run ingest - pass 2 (resume)
echo "Running ingest pass 2..."
./target/debug/heimlern ingest chronik \
    --file "$EVENTS_FILE" \
    --state-file "$STATE_FILE" \
    --stats-file "$STATS_FILE" \
    --max-batches 1

# Verify cursor updated
CURSOR=$(grep -o '"cursor": *"[^"]*"' "$STATE_FILE" | cut -d'"' -f4)
if [ "$CURSOR" != "2023-01-01T10:02:00Z" ]; then
    echo "Error: Unexpected cursor value pass 2: $CURSOR"
    cat "$STATE_FILE"
    exit 1
fi

# Verify stats
# Total processed should be 2 (pass 1) + 1 (pass 2) = 3
TOTAL=$(grep -o '"total_processed": *[0-9]*' "$STATS_FILE" | awk -F': ' '{print $2}')
if [ "$TOTAL" != "3" ]; then
    echo "Error: Unexpected total_processed value: $TOTAL"
    cat "$STATS_FILE"
    exit 1
fi

echo "Smoke test passed!"
