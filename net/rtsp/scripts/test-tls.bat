@echo off
echo Testing RTSP TLS connection...
echo.
echo Make sure MediaMTX is running with the provided config (mediamtx.yml)
echo.

echo Testing rtsps:// connection to MediaMTX on port 8322...
gst-launch-1.0 rtspsrc2 location=rtsps://myuser:mypass@127.0.0.1:8322/videotestsrc-priv protocols=tcp name=rtspsrc ! decodebin ! fakesink -v

echo.
echo Test complete!
pause
