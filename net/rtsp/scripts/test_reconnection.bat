@echo off
echo Testing RTSP reconnection logic...
echo.
echo This test will:
echo 1. Connect to an RTSP stream
echo 2. Simulate connection loss
echo 3. Verify automatic reconnection
echo.
echo Make sure you have an RTSP server running or use a test URL.
echo.

REM Set GST_PLUGIN_PATH to include our plugin
set GST_PLUGIN_PATH=%cd%\target\debug;%GST_PLUGIN_PATH%

REM Enable debug output for our plugin
set GST_DEBUG=rtspsrc:6

REM Test with a sample RTSP URL (replace with your test server)
REM You can use a public test stream or your own RTSP server
set RTSP_URL=rtsp://localhost:8554/test

REM Run GStreamer pipeline with our plugin
echo Starting pipeline with reconnection enabled...
gst-launch-1.0 ^
    rtspsrc2 location=%RTSP_URL% ^
    protocols=tcp ^
    max-reconnection-attempts=5 ^
    reconnection-timeout=3000000000 ^
    retry-strategy=exponential ^
    ! decodebin ^
    ! fakesink

echo.
echo Test completed.
pause