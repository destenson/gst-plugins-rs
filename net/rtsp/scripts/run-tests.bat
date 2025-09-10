@echo off
REM RTSP Test Runner Script for Windows
REM Launches RTSP test server and runs validation tests

setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set PROJECT_DIR=%SCRIPT_DIR%..
set SERVER_PORT=8554
set SERVER_LOG=%SCRIPT_DIR%rtsp-server.log
set SERVER_PID_FILE=%SCRIPT_DIR%rtsp-server.pid
set TEST_VIDEO=%SCRIPT_DIR%test-video.mp4
set SERVER_PID=

REM Colors for output (Windows 10+)
set RED=[91m
set GREEN=[92m
set YELLOW=[93m
set NC=[0m

echo %GREEN%=== RTSP Test Suite ===%NC%
echo Working directory: %PROJECT_DIR%
echo.

REM Check requirements
:check_requirements
echo Checking requirements...

where gst-launch-1.0 >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo %RED%Error: gst-launch-1.0 not found. Please install GStreamer.%NC%
    echo Download from: https://gstreamer.freedesktop.org/download/
    goto :error_exit
)

where gst-inspect-1.0 >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo %RED%Error: gst-inspect-1.0 not found. Please install GStreamer.%NC%
    goto :error_exit
)

REM Check for RTSP server capability
gst-inspect-1.0 rtspsink >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo Using rtspsink for RTSP server
    set SERVER_TYPE=rtspsink
    goto :requirements_ok
)

where gst-rtsp-server-1.0 >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo Using gst-rtsp-server-1.0
    set SERVER_TYPE=rtsp-server
    goto :requirements_ok
)

gst-inspect-1.0 rtspclientsink >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo Using rtspclientsink for RTSP server
    set SERVER_TYPE=rtspclientsink
    goto :requirements_ok
)

echo %RED%Error: No RTSP server found. Install gst-plugins-bad or gst-rtsp-server.%NC%
goto :error_exit

:requirements_ok
echo %GREEN%All requirements met!%NC%

REM Parse command line arguments
set MODE=%1
if "%MODE%"=="" set MODE=live

REM Create test video if needed
:create_test_video
if not exist "%TEST_VIDEO%" (
    echo Creating test video...
    gst-launch-1.0 -e ^
        videotestsrc num-buffers=300 ! ^
        video/x-raw,width=640,height=480,framerate=30/1 ! ^
        x264enc ! ^
        mp4mux ! ^
        filesink location="%TEST_VIDEO%" 2>nul
    echo Test video created: %TEST_VIDEO%
) else (
    echo Using existing test video: %TEST_VIDEO%
)

REM Main execution based on mode
if "%MODE%"=="live" goto :mode_live
if "%MODE%"=="vod" goto :mode_vod
if "%MODE%"=="all" goto :mode_all
if "%MODE%"=="quick" goto :mode_quick
goto :usage

:mode_live
echo %YELLOW%Mode: Live streaming tests%NC%
call :start_rtsp_server
if %ERRORLEVEL% NEQ 0 goto :cleanup
call :run_tests %2
goto :cleanup

:mode_vod
echo %YELLOW%Mode: VOD/Seeking tests%NC%
call :start_vod_server
if %ERRORLEVEL% NEQ 0 goto :cleanup
call :run_tests seek
goto :cleanup

:mode_all
echo %YELLOW%Mode: All tests%NC%
REM Run live tests first
call :start_rtsp_server
if %ERRORLEVEL% NEQ 0 goto :cleanup
call :run_tests
call :stop_server
REM Then VOD tests
call :start_vod_server
if %ERRORLEVEL% NEQ 0 goto :cleanup
call :run_tests seek
goto :cleanup

:mode_quick
echo %YELLOW%Mode: Quick validation%NC%
call :start_rtsp_server
if %ERRORLEVEL% NEQ 0 goto :cleanup
echo Testing RTSP connection...
timeout /t 1 >nul
gst-launch-1.0 ^
    rtspsrc2 location="rtsp://localhost:%SERVER_PORT%/test" ! ^
    fakesink num-buffers=100
echo Test completed
goto :cleanup

:start_rtsp_server
echo Starting RTSP server on port %SERVER_PORT%...

REM Kill any existing process on port
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":%SERVER_PORT%"') do (
    taskkill /F /PID %%a >nul 2>&1
)

if "%SERVER_TYPE%"=="rtspsink" (
    REM Live test pattern with rtspsink
    start /B gst-launch-1.0 -e ^
        videotestsrc is-live=true ! ^
        video/x-raw,width=640,height=480,framerate=30/1 ! ^
        x264enc tune=zerolatency ! ^
        rtph264pay config-interval=1 ! ^
        rtspsink location="rtsp://localhost:%SERVER_PORT%/test" ^
        > "%SERVER_LOG%" 2>&1
) else if "%SERVER_TYPE%"=="rtsp-server" (
    REM Use gst-rtsp-server
    start /B gst-rtsp-server-1.0 ^
        -p %SERVER_PORT% ^
        "( videotestsrc ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtph264pay name=pay0 pt=96 )" ^
        > "%SERVER_LOG%" 2>&1
) else (
    REM Alternative using rtspclientsink
    start /B gst-launch-1.0 -e ^
        videotestsrc is-live=true ! ^
        video/x-raw,width=640,height=480,framerate=30/1 ! ^
        x264enc tune=zerolatency ! ^
        rtph264pay config-interval=1 ! ^
        rtspclientsink location="rtsp://localhost:%SERVER_PORT%/test" ^
        > "%SERVER_LOG%" 2>&1
)

REM Get the PID of the last started process
for /f "tokens=2" %%a in ('tasklist /FI "IMAGENAME eq gst-launch-1.0.exe" /FO LIST ^| findstr "PID:"') do (
    set SERVER_PID=%%a
)
echo %SERVER_PID% > "%SERVER_PID_FILE%"

REM Wait for server to start
echo Waiting for server to start...
set COUNT=0
:wait_server
timeout /t 1 /nobreak >nul
netstat -an | findstr ":%SERVER_PORT%" >nul
if %ERRORLEVEL% EQU 0 (
    echo %GREEN%RTSP server started successfully%NC%
    exit /b 0
)
set /a COUNT+=1
if %COUNT% LSS 10 goto :wait_server

echo %RED%Failed to start RTSP server%NC%
type "%SERVER_LOG%"
exit /b 1

:start_vod_server
echo Starting VOD RTSP server on port %SERVER_PORT%...

call :create_test_video

REM Kill any existing process on port
for /f "tokens=5" %%a in ('netstat -aon ^| findstr ":%SERVER_PORT%"') do (
    taskkill /F /PID %%a >nul 2>&1
)

if "%SERVER_TYPE%"=="rtsp-server" (
    REM VOD server with seeking support
    start /B gst-rtsp-server-1.0 ^
        -p %SERVER_PORT% ^
        "( filesrc location=%TEST_VIDEO% ! qtdemux ! h264parse ! rtph264pay name=pay0 pt=96 )" ^
        > "%SERVER_LOG%" 2>&1
) else (
    REM Fallback to filesrc
    start /B gst-launch-1.0 -e ^
        filesrc location="%TEST_VIDEO%" ! ^
        qtdemux ! h264parse ! ^
        rtph264pay config-interval=1 ! ^
        rtspsink location="rtsp://localhost:%SERVER_PORT%/vod" ^
        > "%SERVER_LOG%" 2>&1
)

REM Get the PID
for /f "tokens=2" %%a in ('tasklist /FI "IMAGENAME eq gst*" /FO LIST ^| findstr "PID:"') do (
    set SERVER_PID=%%a
)
echo %SERVER_PID% > "%SERVER_PID_FILE%"

REM Wait for server to start
echo Waiting for VOD server to start...
set COUNT=0
:wait_vod_server
timeout /t 1 /nobreak >nul
netstat -an | findstr ":%SERVER_PORT%" >nul
if %ERRORLEVEL% EQU 0 (
    echo %GREEN%VOD RTSP server started successfully%NC%
    exit /b 0
)
set /a COUNT+=1
if %COUNT% LSS 10 goto :wait_vod_server

echo %RED%Failed to start VOD server%NC%
type "%SERVER_LOG%"
exit /b 1

:run_tests
echo.
echo %YELLOW%Running tests...%NC%

cd /d "%PROJECT_DIR%"

if "%1"=="" (
    echo Running all RTSP tests...
    cargo test -p gst-plugin-rtsp --lib -- --nocapture
    
    echo.
    echo %YELLOW%Running seeking tests...%NC%
    cargo test -p gst-plugin-rtsp seek -- --nocapture --test-threads=1
    
    echo.
    echo %YELLOW%Running integration tests...%NC%
    cargo test -p gst-plugin-rtsp integration -- --nocapture --test-threads=1
) else (
    echo Running specific tests: %1
    cargo test -p gst-plugin-rtsp %1 -- --nocapture
)
exit /b 0

:stop_server
echo Stopping RTSP server...
if exist "%SERVER_PID_FILE%" (
    set /p PID=<"%SERVER_PID_FILE%"
    taskkill /F /PID !PID! >nul 2>&1
    del "%SERVER_PID_FILE%" 2>nul
)
REM Also kill by name as backup
taskkill /F /IM gst-launch-1.0.exe >nul 2>&1
taskkill /F /IM gst-rtsp-server-1.0.exe >nul 2>&1
exit /b 0

:cleanup
echo.
echo %YELLOW%Cleaning up...%NC%
call :stop_server
if exist "%SERVER_LOG%" del "%SERVER_LOG%" 2>nul
echo %GREEN%=== Test Suite Complete ===%NC%
exit /b 0

:usage
echo Usage: %0 [live^|vod^|all^|quick] [test_filter]
echo   live  - Run tests with live RTSP server (default)
echo   vod   - Run VOD/seeking tests
echo   all   - Run all test suites
echo   quick - Quick validation test
echo.
echo Example: %0 live
echo Example: %0 vod
echo Example: %0 all
exit /b 1

:error_exit
pause
exit /b 1