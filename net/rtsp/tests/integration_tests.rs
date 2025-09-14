// Main integration test suite for RTSP plugin
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

#![cfg(feature = "integration-tests")]

mod integration;

use integration::{MediaMtxServer, RtspTestHarness};
use integration::server_helper::TestMode;

/// Run integration tests with:
/// cargo test -p gst-plugin-rtsp --features integration-tests integration -- --nocapture
///
/// Or specific scenario:
/// cargo test -p gst-plugin-rtsp --features integration-tests connection_limited -- --nocapture
///
/// Prerequisites:
/// - MediaMTX must be installed and in PATH (or already running on port 8554)
/// - GStreamer must be installed with gst-launch-1.0 available
///
/// The tests will:
/// 1. Check if an RTSP server is already running on port 8554
/// 2. If not, attempt to start MediaMTX
/// 3. Run the integration test scenarios
/// 4. Verify retry strategies, auto mode, HTTP tunneling, and adaptive persistence

#[test]
fn test_integration_suite_available() {
    // Basic test to verify the integration test infrastructure is available
    gst::init().unwrap();
    
    // Try to create a server (will use existing if available)
    match MediaMtxServer::new(TestMode::Normal) {
        Ok(server) => {
            println!("Integration test server available on port {}", server.port());
            assert!(server.port() > 0);
        }
        Err(e) => {
            eprintln!("Warning: Integration tests may not run fully: {}", e);
            eprintln!("Install MediaMTX or start an RTSP server on port 8554");
        }
    }
}

#[test]
fn test_basic_rtsp_connection() {
    // Basic end-to-end test
    gst::init().unwrap();
    
    let server = match MediaMtxServer::new(TestMode::Normal) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };
    
    let url = server.url("videotestsrc");
    
    let mut harness = RtspTestHarness::new(&url)
        .expect("Failed to create test harness");
    
    harness.start().unwrap();
    
    let connected = harness.wait_for_connection(std::time::Duration::from_secs(15)).unwrap();
    assert!(connected, "Should establish basic RTSP connection");
    
    assert!(harness.is_playing(), "Pipeline should be in playing state");
}

#[test]
fn test_basic_rtsps_connection() {
    // Basic RTSPS (secure) end-to-end test
    gst::init().unwrap();
    
    let server = match MediaMtxServer::new(TestMode::Normal) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };
    
    // Check if RTSPS is available
    if !MediaMtxServer::is_server_running(8322) {
        eprintln!("Skipping RTSPS test: No RTSPS server on port 8322");
        return;
    }
    
    let url = server.rtsps_url("videotestsrc");
    
    let mut harness = RtspTestHarness::new(&url)
        .expect("Failed to create test harness");
    
    // Allow self-signed certificates for testing
    harness.set_property("tls-validation-flags", 0u32).unwrap();
    
    harness.start().unwrap();
    
    let connected = harness.wait_for_connection(std::time::Duration::from_secs(15)).unwrap();
    assert!(connected, "Should establish basic RTSPS/TLS connection");
    
    assert!(harness.is_playing(), "Pipeline should be in playing state with RTSPS");
}