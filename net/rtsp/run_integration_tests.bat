@echo off
REM Run RTSP Integration Tests
REM
REM This script runs the integration test suite for the RTSP plugin.
REM Prerequisites:
REM - MediaMTX must be installed or an RTSP server running on port 8554
REM - GStreamer must be installed

echo.
echo =====================================================
echo    RTSP Plugin Integration Test Suite
echo =====================================================
echo.

REM Check if MediaMTX or RTSP server is available
echo Checking for RTSP server on port 8554...
powershell -Command "Test-NetConnection -ComputerName localhost -Port 8554 -InformationLevel Quiet" >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [OK] RTSP server found on port 8554
) else (
    echo [INFO] No RTSP server found on port 8554
    echo [INFO] Tests will attempt to start MediaMTX if available
)

echo Checking for RTSPS server on port 8322...
powershell -Command "Test-NetConnection -ComputerName localhost -Port 8322 -InformationLevel Quiet" >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [OK] RTSPS server found on port 8322 (TLS/secure)
) else (
    echo [INFO] No RTSPS server found on port 8322
)

echo.
echo Running integration tests...
echo.

REM Run the tests with the integration-tests feature
cargo test -p gst-plugin-rtsp --features integration-tests integration -- --nocapture

if %ERRORLEVEL% EQU 0 (
    echo.
    echo =====================================================
    echo    Integration tests completed successfully!
    echo =====================================================
) else (
    echo.
    echo =====================================================
    echo    Some integration tests failed
    echo =====================================================
    echo.
    echo To run specific scenarios:
    echo   cargo test -p gst-plugin-rtsp --features integration-tests connection_limited -- --nocapture
    echo   cargo test -p gst-plugin-rtsp --features integration-tests lossy_network -- --nocapture
    echo   cargo test -p gst-plugin-rtsp --features integration-tests http_tunneling -- --nocapture
    echo   cargo test -p gst-plugin-rtsp --features integration-tests adaptive_persistence -- --nocapture
    echo   cargo test -p gst-plugin-rtsp --features integration-tests rtsps_tls -- --nocapture
)

echo.