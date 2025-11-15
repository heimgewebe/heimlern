#!/usr/bin/env bash

wgx_cmd_metrics() {
  wgx_log "Running metrics task..."
  if [ -x scripts/wgx-metrics-snapshot.sh ]; then
    scripts/wgx-metrics-snapshot.sh --json
  else
    echo "no metrics script defined for heimlern; skipping"
  fi
}
