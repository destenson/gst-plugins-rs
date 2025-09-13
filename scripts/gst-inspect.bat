
set "SCRIPT_DIR=%~dp0"
set "GST_PLUGIN_PATH=%SCRIPT_DIR%..\target\release;%GST_PLUGIN_PATH%"

gst-inspect-1.0.exe %*

