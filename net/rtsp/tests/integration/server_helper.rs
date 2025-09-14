// MediaMTX server helper for integration tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{fs, thread};

/// MediaMTX server wrapper for integration testing
pub struct MediaMtxServer {
    process: Option<Child>,
    port: u16,
    config_path: PathBuf,
    is_running: Arc<AtomicBool>,
    test_mode: TestMode,
}

#[derive(Debug, Clone)]
pub enum TestMode {
    /// Normal operation
    Normal,
    /// Simulates connection-limited device (drops connections after timeout)
    ConnectionLimited { timeout_secs: u64 },
    /// Simulates lossy network with packet loss
    LossyNetwork { loss_percent: f32 },
    /// Blocks RTSP port to force HTTP tunneling
    HttpTunnelingOnly,
}

impl MediaMtxServer {
    /// Create a new MediaMTX server with test mode
    pub fn new(test_mode: TestMode) -> Result<Self, Box<dyn std::error::Error>> {
        // First check if an RTSP server is already running on default port
        let default_port = 8554;
        let rtsps_port = 8322;
        
        if Self::is_server_running(default_port) {
            eprintln!("RTSP server already running on port {}, using existing server", default_port);
            
            // Also check for RTSPS
            if Self::is_server_running(rtsps_port) {
                eprintln!("RTSPS server also available on port {}", rtsps_port);
            }
            
            // Use existing server without managing process
            let config_path = PathBuf::new(); // Empty path since we're not managing config
            let is_running = Arc::new(AtomicBool::new(true));
            
            return Ok(Self {
                process: None,
                port: default_port,
                config_path,
                is_running,
                test_mode,
            });
        }
        
        let port = Self::find_available_port()?;
        let config_path = Self::create_test_config(port, &test_mode)?;
        let is_running = Arc::new(AtomicBool::new(false));

        let mut server = Self {
            process: None,
            port,
            config_path,
            is_running: is_running.clone(),
            test_mode,
        };

        server.start()?;
        Ok(server)
    }
    
    /// Check if a server is already running on the given port
    pub fn is_server_running(port: u16) -> bool {
        TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok()
    }

    /// Start the MediaMTX server
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // Launch MediaMTX with custom config
        let mediamtx_cmd = if cfg!(windows) { "mediamtx.exe" } else { "mediamtx" };
        
        let process = Command::new(mediamtx_cmd)
            .arg(&self.config_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start MediaMTX: {}. Ensure mediamtx is in PATH", e))?;

        self.process = Some(process);
        self.wait_for_ready()?;
        self.is_running.store(true, Ordering::SeqCst);
        
        Ok(())
    }

    /// Stop the MediaMTX server
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        
        // Only kill process if we started it ourselves
        if let Some(mut process) = self.process.take() {
            let _ = process.kill();
            let _ = process.wait();
        }
        
        // Clean up config file only if we created one
        if !self.config_path.as_os_str().is_empty() {
            let _ = fs::remove_file(&self.config_path);
        }
    }

    /// Get the RTSP URL for a given path
    pub fn url(&self, path: &str) -> String {
        format!("rtsp://127.0.0.1:{}/{}", self.port, path)
    }
    
    /// Get the RTSPS (secure) URL for a given path
    pub fn rtsps_url(&self, path: &str) -> String {
        format!("rtsps://127.0.0.1:8322/{}", path)
    }

    /// Get the server port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Create a test-specific MediaMTX configuration
    fn create_test_config(port: u16, test_mode: &TestMode) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join(format!("mediamtx_test_{}.yml", port));
        
        let mut config = String::from("logLevel: warn\n");
        
        match test_mode {
            TestMode::Normal => {
                config.push_str(&format!("rtspAddress: :{}\n", port));
            }
            TestMode::ConnectionLimited { timeout_secs } => {
                config.push_str(&format!("rtspAddress: :{}\n", port));
                config.push_str(&format!("readTimeout: {}s\n", timeout_secs));
                config.push_str(&format!("writeTimeout: {}s\n", timeout_secs));
            }
            TestMode::LossyNetwork { loss_percent } => {
                config.push_str(&format!("rtspAddress: :{}\n", port));
                // Note: MediaMTX doesn't directly support packet loss simulation
                // We'll need to use netsim in the GStreamer pipeline
            }
            TestMode::HttpTunnelingOnly => {
                // Block RTSP port, only allow HTTP
                config.push_str(&format!("rtspAddress: :{}\n", port + 1000)); // Use different port
                config.push_str(&format!("hlsAddress: :{}\n", port)); // Use HLS port for HTTP
            }
        }
        
        // Add path configuration
        config.push_str("pathDefaults:\n");
        config.push_str("  record: false\n");
        config.push_str("\npaths:\n");
        
        // Test source with configurable behavior
        match test_mode {
            TestMode::LossyNetwork { loss_percent } => {
                // Use netsim for packet loss simulation
                config.push_str(&format!(
                    "  test:\n    runOnDemand: gst-launch-1.0 videotestsrc pattern=smpte is-live=true ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! mpegtsmux ! netsim drop-probability={} ! rtspclientsink location=rtsp://localhost:$RTSP_PORT/$MTX_PATH\n",
                    loss_percent / 100.0
                ));
            }
            TestMode::ConnectionLimited { timeout_secs } => {
                // Add a source that will timeout
                config.push_str(&format!(
                    "  test:\n    runOnDemand: gst-launch-1.0 videotestsrc pattern=ball is-live=true num-buffers={} ! video/x-raw,width=640,height=480,framerate=1/1 ! x264enc tune=zerolatency ! rtspclientsink location=rtsp://localhost:$RTSP_PORT/$MTX_PATH\n",
                    timeout_secs
                ));
            }
            _ => {
                // Normal test source
                config.push_str("  test:\n    runOnDemand: gst-launch-1.0 videotestsrc pattern=smpte is-live=true ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtspclientsink location=rtsp://localhost:$RTSP_PORT/$MTX_PATH\n");
            }
        }
        
        config.push_str("    runOnDemandStartTimeout: 10s\n");
        config.push_str("    runOnDemandCloseAfter: 10s\n");
        
        // Allow any path for testing
        config.push_str("  ~^.*:\n");
        
        // Disable other protocols
        config.push_str("\nhlsAddress: \"\"\n");
        config.push_str("webrtcAddress: \"\"\n");
        
        let mut file = fs::File::create(&config_path)?;
        file.write_all(config.as_bytes())?;
        
        Ok(config_path)
    }

    /// Find an available port for the server
    fn find_available_port() -> Result<u16, Box<dyn std::error::Error>> {
        for port in 8554..8654 {
            if let Ok(listener) = std::net::TcpListener::bind(format!("127.0.0.1:{}", port)) {
                drop(listener);
                return Ok(port);
            }
        }
        Err("No available port found".into())
    }

    /// Wait for the server to be ready
    fn wait_for_ready(&self) -> Result<(), Box<dyn std::error::Error>> {
        let start = Instant::now();
        let timeout = Duration::from_secs(10);
        
        while start.elapsed() < timeout {
            if TcpStream::connect(format!("127.0.0.1:{}", self.port)).is_ok() {
                thread::sleep(Duration::from_millis(500)); // Give server time to fully initialize
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }
        
        Err("MediaMTX server failed to start within timeout".into())
    }

    /// Restart the server with a new test mode
    pub fn restart_with_mode(&mut self, test_mode: TestMode) -> Result<(), Box<dyn std::error::Error>> {
        self.stop();
        thread::sleep(Duration::from_millis(500));
        
        self.test_mode = test_mode;
        self.config_path = Self::create_test_config(self.port, &self.test_mode)?;
        self.start()?;
        
        Ok(())
    }
}

impl Drop for MediaMtxServer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_lifecycle() {
        // Try to create and start server
        match MediaMtxServer::new(TestMode::Normal) {
            Ok(server) => {
                assert!(server.port() >= 8554 && server.port() <= 8654);
                let url = server.url("test");
                assert!(url.starts_with("rtsp://127.0.0.1:"));
                assert!(url.ends_with("/test"));
            }
            Err(e) => {
                eprintln!("Server creation failed (expected if MediaMTX not installed): {}", e);
            }
        }
    }

    #[test]
    fn test_config_creation() {
        let config_path = MediaMtxServer::create_test_config(8554, &TestMode::Normal);
        assert!(config_path.is_ok());
        
        if let Ok(path) = config_path {
            assert!(path.exists());
            let content = fs::read_to_string(&path).unwrap();
            assert!(content.contains("rtspAddress: :8554"));
            let _ = fs::remove_file(path);
        }
    }
}