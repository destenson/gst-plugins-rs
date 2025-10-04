#!/usr/bin/env bash
#
# Helper script to exercise the rtspsrc2 removal/recreation flow while forcing
# reconnects or packet loss on a real RTSP server.
#
# Usage:
#   ./test_reconnection_cleanup.sh [rtsp-url] [restart-interval-seconds] [restart-jitter]
#
# Defaults:
#   RTSP URL: rtsp://192.168.12.38:8554/test-h264
#   Restart interval: 25 seconds
#   Restart jitter: 0.25 (±25%)
#
# Environment variables:
#   GST_PLUGIN_PATH – should point to the build output containing the plugin.
#   GST_DEBUG       – optional, defaults to "rtspsrc2:6" for detailed logs.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RTSP_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$RTSP_DIR")"

# echo "Using PROJECT_ROOT: $PROJECT_ROOT"
# echo "Using RTSP_DIR: $RTSP_DIR"
# echo "Using SCRIPT_DIR: $SCRIPT_DIR"
# echo

RTSP_URL="${1:-rtsp://192.168.12.38:8554/test-h264}"
RESTART_INTERVAL="${2:-25}"
RESTART_JITTER="${3:-0.25}"

if [[ -n "${GST_DEBUG:-}" ]]; then
  echo "Using existing GST_DEBUG=${GST_DEBUG}"
  export GST_DEBUG
else
  # echo "GST_DEBUG not set; relying on example status output"
  unset GST_DEBUG || true
fi
export GST_PLUGIN_PATH="${PROJECT_ROOT}/net/rtsp/target/debug:${GST_PLUGIN_PATH:-}"

pushd "${PROJECT_ROOT}/net/rtsp" >/dev/null

cargo build -p gst-plugin-rtsp --example rtspsrc_cleanup

printf '\nRunning rtspsrc_cleanup example against %s (interval %ss, jitter %.2f)\n' "${RTSP_URL}" "${RESTART_INTERVAL}" "${RESTART_JITTER}" >&2
cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- \
  --url "${RTSP_URL}" \
  --restart-interval "${RESTART_INTERVAL}" \
  --restart-jitter "${RESTART_JITTER}"

popd >/dev/null
