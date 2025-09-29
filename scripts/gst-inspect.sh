#!/bin/sh

SCRIPT_DIR="$(dirname "$0")"
export GST_PLUGIN_PATH="$SCRIPT_DIR/../target/release:$GST_PLUGIN_PATH"

gst-inspect-1.0 "$SCRIPT_DIR/../mediamtx.yml"
