# RTSP Test Runner Script for Windows (PowerShell)
# Launches RTSP test server and runs validation tests

param(
    [Parameter(Position=0)]
    [ValidateSet("live", "vod", "all", "quick", "continuous")]
    [string]$Mode = "live",
    
    [Parameter(Position=1)]
    [string]$TestFilter = "",
    
    [int]$Port = 8554,
    [switch]$KeepServer,
    [switch]$Verbose
)

$ErrorActionPreference = "Stop"

# Script configuration
$Script:Config = @{
    ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    ProjectDir = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
    ServerPort = $Port
    ServerLog = Join-Path $PSScriptRoot "rtsp-server.log"
    ServerPidFile = Join-Path $PSScriptRoot "rtsp-server.pid"
    TestVideo = Join-Path $PSScriptRoot "test-video.mp4"
    ServerProcess = $null
    ServerType = $null
}

# Color definitions
$Script:Colors = @{
    Red = "`e[91m"
    Green = "`e[92m"
    Yellow = "`e[93m"
    Blue = "`e[94m"
    Reset = "`e[0m"
}

function Write-ColorOutput {
    param(
        [string]$Message,
        [string]$Color = "Reset"
    )
    Write-Host "$($Script:Colors[$Color])$Message$($Script:Colors.Reset)"
}

function Test-Requirements {
    Write-ColorOutput "Checking requirements..." "Yellow"
    
    # Check for GStreamer
    $gstLaunch = Get-Command "gst-launch-1.0" -ErrorAction SilentlyContinue
    if (-not $gstLaunch) {
        Write-ColorOutput "Error: gst-launch-1.0 not found. Please install GStreamer." "Red"
        Write-Host "Download from: https://gstreamer.freedesktop.org/download/"
        exit 1
    }
    
    # Check for gst-inspect
    $gstInspect = Get-Command "gst-inspect-1.0" -ErrorAction SilentlyContinue
    if (-not $gstInspect) {
        Write-ColorOutput "Error: gst-inspect-1.0 not found." "Red"
        exit 1
    }
    
    # Determine available RTSP server type
    $rtspSink = & gst-inspect-1.0 rtspsink 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Using rtspsink for RTSP server"
        $Script:Config.ServerType = "rtspsink"
        return
    }
    
    $rtspServer = Get-Command "gst-rtsp-server-1.0" -ErrorAction SilentlyContinue
    if ($rtspServer) {
        Write-Host "Using gst-rtsp-server-1.0"
        $Script:Config.ServerType = "rtsp-server"
        return
    }
    
    $rtspClientSink = & gst-inspect-1.0 rtspclientsink 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "Using rtspclientsink for RTSP server"
        $Script:Config.ServerType = "rtspclientsink"
        return
    }
    
    Write-ColorOutput "Error: No RTSP server capability found." "Red"
    Write-Host "Install gst-plugins-bad or gst-rtsp-server"
    exit 1
}

function New-TestVideo {
    if (-not (Test-Path $Script:Config.TestVideo)) {
        Write-Host "Creating test video..."
        $cmd = @"
gst-launch-1.0 -e 
    videotestsrc num-buffers=300 ! 
    video/x-raw,width=640,height=480,framerate=30/1 ! 
    x264enc ! 
    mp4mux ! 
    filesink location="$($Script:Config.TestVideo)"
"@ -replace "`r`n", " "
        
        & cmd /c $cmd 2>$null
        Write-ColorOutput "Test video created: $($Script:Config.TestVideo)" "Green"
    } else {
        Write-Host "Using existing test video: $($Script:Config.TestVideo)"
    }
}

function Stop-RtspServer {
    Write-Host "Stopping RTSP server..."
    
    # Stop tracked process
    if ($Script:Config.ServerProcess -and -not $Script:Config.ServerProcess.HasExited) {
        $Script:Config.ServerProcess.Kill()
        $Script:Config.ServerProcess.WaitForExit(2000)
    }
    
    # Clean up PID file
    if (Test-Path $Script:Config.ServerPidFile) {
        $pid = Get-Content $Script:Config.ServerPidFile
        try {
            Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
        } catch {}
        Remove-Item $Script:Config.ServerPidFile -Force
    }
    
    # Kill any process on the port
    $netstat = netstat -ano | Select-String ":$($Script:Config.ServerPort)"
    foreach ($line in $netstat) {
        if ($line -match '\s+(\d+)$') {
            $pid = $matches[1]
            try {
                Stop-Process -Id $pid -Force -ErrorAction SilentlyContinue
            } catch {}
        }
    }
    
    # Kill GStreamer processes as backup
    Get-Process | Where-Object { $_.Name -match "gst-" } | Stop-Process -Force -ErrorAction SilentlyContinue
}

function Start-RtspServer {
    param([switch]$VOD)
    
    Write-ColorOutput "Starting RTSP server on port $($Script:Config.ServerPort)..." "Yellow"
    
    # Stop any existing server
    Stop-RtspServer
    
    # Prepare server command based on type and mode
    if ($VOD) {
        New-TestVideo
        $serverCmd = switch ($Script:Config.ServerType) {
            "rtsp-server" {
                "gst-rtsp-server-1.0 -p $($Script:Config.ServerPort) `"( filesrc location=$($Script:Config.TestVideo) ! qtdemux ! h264parse ! rtph264pay name=pay0 pt=96 )`""
            }
            default {
                "gst-launch-1.0 -e filesrc location=`"$($Script:Config.TestVideo)`" ! qtdemux ! h264parse ! rtph264pay config-interval=1 ! rtspsink location=`"rtsp://localhost:$($Script:Config.ServerPort)/vod`""
            }
        }
    } else {
        $serverCmd = switch ($Script:Config.ServerType) {
            "rtspsink" {
                "gst-launch-1.0 -e videotestsrc is-live=true ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtph264pay config-interval=1 ! rtspsink location=`"rtsp://localhost:$($Script:Config.ServerPort)/test`""
            }
            "rtsp-server" {
                "gst-rtsp-server-1.0 -p $($Script:Config.ServerPort) `"( videotestsrc ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtph264pay name=pay0 pt=96 )`""
            }
            default {
                "gst-launch-1.0 -e videotestsrc is-live=true ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtph264pay config-interval=1 ! rtspclientsink location=`"rtsp://localhost:$($Script:Config.ServerPort)/test`""
            }
        }
    }
    
    # Start the server process
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = "cmd.exe"
    $psi.Arguments = "/c `"$serverCmd > `"$($Script:Config.ServerLog)`" 2>&1`""
    $psi.UseShellExecute = $false
    $psi.CreateNoWindow = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    
    $Script:Config.ServerProcess = [System.Diagnostics.Process]::Start($psi)
    $Script:Config.ServerProcess.Id | Out-File -FilePath $Script:Config.ServerPidFile
    
    # Wait for server to be ready
    Write-Host -NoNewline "Waiting for server to start"
    $timeout = 10
    $started = $false
    
    for ($i = 0; $i -lt $timeout; $i++) {
        Write-Host -NoNewline "."
        Start-Sleep -Seconds 1
        
        $connection = Test-NetConnection -ComputerName localhost -Port $Script:Config.ServerPort -WarningAction SilentlyContinue -ErrorAction SilentlyContinue
        if ($connection.TcpTestSucceeded) {
            $started = $true
            break
        }
    }
    
    Write-Host ""
    
    if ($started) {
        Write-ColorOutput "RTSP server started successfully (PID: $($Script:Config.ServerProcess.Id))" "Green"
        return $true
    } else {
        Write-ColorOutput "Failed to start RTSP server" "Red"
        if (Test-Path $Script:Config.ServerLog) {
            Write-Host "Server log:"
            Get-Content $Script:Config.ServerLog | Select-Object -Last 20
        }
        return $false
    }
}

function Invoke-Tests {
    param([string]$Filter = "")
    
    Write-ColorOutput "`nRunning tests..." "Yellow"
    
    Push-Location $Script:Config.ProjectDir
    try {
        if ($Filter) {
            Write-Host "Running specific tests: $Filter"
            & cargo test -p gst-plugin-rtsp $Filter -- --nocapture
        } else {
            Write-Host "Running all RTSP tests..."
            
            # Library tests
            & cargo test -p gst-plugin-rtsp --lib -- --nocapture
            
            # Seeking tests
            Write-ColorOutput "`nRunning seeking tests..." "Yellow"
            & cargo test -p gst-plugin-rtsp seek -- --nocapture --test-threads=1
            
            # Integration tests
            Write-ColorOutput "`nRunning integration tests..." "Yellow"
            & cargo test -p gst-plugin-rtsp integration -- --nocapture --test-threads=1
        }
    } finally {
        Pop-Location
    }
}

function Invoke-QuickTest {
    Write-Host "Testing RTSP connection..."
    $testCmd = "gst-launch-1.0 rtspsrc2 location=`"rtsp://localhost:$($Script:Config.ServerPort)/test`" ! fakesink num-buffers=100"
    
    $process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", $testCmd -NoNewWindow -PassThru
    $process.WaitForExit(5000)
    
    if ($process.HasExited) {
        if ($process.ExitCode -eq 0) {
            Write-ColorOutput "Quick test passed!" "Green"
        } else {
            Write-ColorOutput "Quick test failed with exit code: $($process.ExitCode)" "Red"
        }
    } else {
        $process.Kill()
        Write-ColorOutput "Quick test timed out" "Yellow"
    }
}

function Start-ContinuousMode {
    Write-ColorOutput "Starting continuous test mode (Press Ctrl+C to stop)" "Blue"
    Write-Host "Server will remain running for manual testing"
    Write-Host ""
    
    if (Start-RtspServer) {
        Write-Host "RTSP URL: rtsp://localhost:$($Script:Config.ServerPort)/test"
        Write-Host ""
        Write-Host "Test with: gst-launch-1.0 rtspsrc2 location=rtsp://localhost:$($Script:Config.ServerPort)/test ! decodebin ! autovideosink"
        Write-Host ""
        
        # Keep running until interrupted
        try {
            while ($true) {
                Start-Sleep -Seconds 1
                if ($Script:Config.ServerProcess.HasExited) {
                    Write-ColorOutput "Server process died unexpectedly" "Red"
                    break
                }
            }
        } catch {
            Write-Host "Interrupted by user"
        }
    }
}

# Main execution
try {
    Write-ColorOutput "=== RTSP Test Suite ===" "Green"
    Write-Host "Working directory: $($Script:Config.ProjectDir)"
    Write-Host ""
    
    Test-Requirements
    
    $success = $true
    
    switch ($Mode) {
        "live" {
            Write-ColorOutput "Mode: Live streaming tests" "Yellow"
            if (Start-RtspServer) {
                Invoke-Tests $TestFilter
            } else {
                $success = $false
            }
        }
        
        "vod" {
            Write-ColorOutput "Mode: VOD/Seeking tests" "Yellow"
            if (Start-RtspServer -VOD) {
                Invoke-Tests "seek"
            } else {
                $success = $false
            }
        }
        
        "all" {
            Write-ColorOutput "Mode: All tests" "Yellow"
            
            # Live tests
            if (Start-RtspServer) {
                Invoke-Tests
                Stop-RtspServer
            }
            
            # VOD tests
            if (Start-RtspServer -VOD) {
                Invoke-Tests "seek"
            }
        }
        
        "quick" {
            Write-ColorOutput "Mode: Quick validation" "Yellow"
            if (Start-RtspServer) {
                Invoke-QuickTest
            } else {
                $success = $false
            }
        }
        
        "continuous" {
            Start-ContinuousMode
        }
    }
    
    if ($success) {
        Write-ColorOutput "`n=== Test Suite Complete ===" "Green"
    } else {
        Write-ColorOutput "`n=== Test Suite Failed ===" "Red"
        exit 1
    }
    
} finally {
    # Cleanup
    if (-not $KeepServer) {
        Write-ColorOutput "`nCleaning up..." "Yellow"
        Stop-RtspServer
        
        if (Test-Path $Script:Config.ServerLog) {
            Remove-Item $Script:Config.ServerLog -Force -ErrorAction SilentlyContinue
        }
    } else {
        Write-Host "`nServer kept running at rtsp://localhost:$($Script:Config.ServerPort)/test"
    }
}