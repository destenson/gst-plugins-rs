#!/bin/sh

SCRIPT_DIR="$(cd $(dirname "$0") && pwd)"
export GST_PLUGIN_PATH="$(cd $SCRIPT_DIR/../target/ && pwd)/release:$GST_PLUGIN_PATH"

gst-inspect-1.0 "$@"

