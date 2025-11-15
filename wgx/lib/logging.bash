#!/usr/bin/env bash

wgx_log() {
  echo "[wgx] $*"
}

wgx_log_error() {
  echo "[wgx] ERROR: $*" >&2
}
