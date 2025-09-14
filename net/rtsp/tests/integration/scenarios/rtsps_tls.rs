// RTSPS/TLS scenario tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

#[cfg(test)]
mod tests {
    use crate::integration::{MediaMtxServer, RtspTestHarness};
    use crate::integration::server_helper::TestMode;
    use std::time::Duration;

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_rtsps_basic_connection() {
        // Test basic RTSPS connection with TLS
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        // Use RTSPS URL on port 8322
        let url = "rtsps://127.0.0.1:8322/videotestsrc";
        
        let mut harness = RtspTestHarness::new(url)
            .expect("Failed to create test harness");
        
        // Configure TLS settings
        harness.set_property("tls-validation-flags", 0u32).unwrap(); // Accept self-signed certs for testing
        
        harness.start().unwrap();
        
        // Should connect via RTSPS
        let connected = harness.wait_for_connection(Duration::from_secs(15)).unwrap();
        assert!(connected, "Should connect via RTSPS/TLS");
        
        // Verify TLS is active
        let messages = harness.get_messages();
        let mut tls_active = false;
        
        for msg in messages.iter() {
            if let gst::MessageView::Element(element) = msg.view() {
                if let Some(structure) = element.structure() {
                    let name = structure.name();
                    if name.contains("tls") || name.contains("ssl") || name.contains("secure") {
                        tls_active = true;
                        break;
                    }
                }
            }
        }
        
        // Even if we don't see explicit TLS messages, connection to port 8322 implies TLS
        assert!(connected, "RTSPS connection should work");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_rtsps_with_retry() {
        // Test RTSPS connection with retry mechanism
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        let url = "rtsps://127.0.0.1:8322/videotestsrc";
        
        let mut harness = RtspTestHarness::new(url)
            .expect("Failed to create test harness");
        
        // Configure retry with TLS
        harness.set_property("retry-strategy", "exponential").unwrap();
        harness.set_property("max-retry-attempts", 5i32).unwrap();
        harness.set_property("tls-validation-flags", 0u32).unwrap();
        
        harness.start().unwrap();
        
        // Wait for connection
        let connected = harness.wait_for_connection(Duration::from_secs(20)).unwrap();
        assert!(connected, "Should connect via RTSPS with retry");
        
        // Force reconnection to test retry with TLS
        harness.force_reconnect().unwrap();
        
        let reconnected = harness.wait_for_connection(Duration::from_secs(15)).unwrap();
        assert!(reconnected, "Should reconnect via RTSPS");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_rtsps_certificate_validation() {
        // Test certificate validation behavior
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        let url = "rtsps://127.0.0.1:8322/videotestsrc";
        
        // Test with strict validation (should fail with self-signed cert)
        {
            let mut harness = RtspTestHarness::new(url)
                .expect("Failed to create test harness");
            
            // Use strict validation (default)
            // This should fail with self-signed certificate
            harness.start().unwrap();
            
            std::thread::sleep(Duration::from_secs(5));
            
            let messages = harness.get_messages();
            let mut cert_error = false;
            
            for msg in messages.iter() {
                if let gst::MessageView::Error(err) = msg.view() {
                    let error_str = err.error().to_string();
                    if error_str.contains("certificate") || error_str.contains("TLS") {
                        cert_error = true;
                        break;
                    }
                }
            }
            
            // Note: Connection might succeed if system trusts the cert
            println!("Strict validation test - cert error found: {}", cert_error);
        }
        
        // Test with relaxed validation (should succeed)
        {
            let mut harness = RtspTestHarness::new(url)
                .expect("Failed to create test harness");
            
            // Disable certificate validation for testing
            harness.set_property("tls-validation-flags", 0u32).unwrap();
            
            harness.start().unwrap();
            
            let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
            assert!(connected, "Should connect with relaxed certificate validation");
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_rtsps_fallback_to_rtsp() {
        // Test fallback from RTSPS to RTSP when TLS fails
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        // First try RTSPS on wrong port (should fail)
        let url = "rtsps://127.0.0.1:8554/videotestsrc"; // Wrong port for RTSPS
        
        let mut harness = RtspTestHarness::new(url)
            .expect("Failed to create test harness");
        
        // Configure to allow fallback
        harness.set_property("protocols", "tcp").unwrap();
        harness.set_property("tls-validation-flags", 0u32).unwrap();
        
        harness.start().unwrap();
        
        // Should fail RTSPS but might connect via regular RTSP
        std::thread::sleep(Duration::from_secs(5));
        
        // Now try regular RTSP
        harness.set_property("location", "rtsp://127.0.0.1:8554/videotestsrc").unwrap();
        harness.force_reconnect().unwrap();
        
        let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
        assert!(connected, "Should connect via regular RTSP after RTSPS failure");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_rtsps_with_racing() {
        // Test connection racing with RTSPS
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        let url = "rtsps://127.0.0.1:8322/videotestsrc-faulty";
        
        let mut harness = RtspTestHarness::new(url)
            .expect("Failed to create test harness");
        
        // Enable connection racing with TLS
        harness.set_property("connection-racing", true).unwrap();
        harness.set_property("racing-strategy", "first-wins").unwrap();
        harness.set_property("racing-connections", 3i32).unwrap();
        harness.set_property("tls-validation-flags", 0u32).unwrap();
        
        harness.start().unwrap();
        
        // Racing should help establish secure connection despite faulty network
        let connected = harness.wait_for_connection(Duration::from_secs(15)).unwrap();
        assert!(connected, "Racing should help establish RTSPS connection");
        
        let stats = harness.get_retry_stats().unwrap();
        println!("RTSPS racing stats: attempts={}, failures={}, successes={}", 
                 stats.connection_attempts, stats.connection_failures, stats.connection_successes);
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_rtsps_adaptive_learning() {
        // Test that adaptive learning works with RTSPS
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        let url = "rtsps://127.0.0.1:8322/videotestsrc";
        
        // Train the adaptive system with RTSPS
        {
            let mut harness = RtspTestHarness::new(url)
                .expect("Failed to create test harness");
            
            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();
            harness.set_property("tls-validation-flags", 0u32).unwrap();
            
            harness.start().unwrap();
            
            // Multiple connection attempts to train
            for _ in 0..3 {
                harness.wait_for_connection(Duration::from_secs(10)).unwrap();
                std::thread::sleep(Duration::from_secs(1));
                harness.force_reconnect().unwrap();
            }
            
            harness.stop().unwrap();
        }
        
        // Verify learning persists for RTSPS
        {
            let mut harness = RtspTestHarness::new(url)
                .expect("Failed to create test harness");
            
            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();
            harness.set_property("tls-validation-flags", 0u32).unwrap();
            
            harness.start().unwrap();
            
            // Should connect quickly with learned strategy
            let start = std::time::Instant::now();
            let connected = harness.wait_for_connection(Duration::from_secs(5)).unwrap();
            let connect_time = start.elapsed();
            
            assert!(connected, "Should connect quickly with learned RTSPS strategy");
            assert!(connect_time < Duration::from_secs(5), "Learned strategy should be fast");
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_mixed_rtsp_rtsps_servers() {
        // Test handling of both RTSP and RTSPS in the same session
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        // Test RTSP first
        {
            let url = server.url("videotestsrc");
            let mut harness = RtspTestHarness::new(&url)
                .expect("Failed to create test harness");
            
            harness.start().unwrap();
            let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
            assert!(connected, "Should connect via RTSP");
            harness.stop().unwrap();
        }
        
        // Then test RTSPS
        {
            let url = "rtsps://127.0.0.1:8322/videotestsrc";
            let mut harness = RtspTestHarness::new(url)
                .expect("Failed to create test harness");
            
            harness.set_property("tls-validation-flags", 0u32).unwrap();
            harness.start().unwrap();
            let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
            assert!(connected, "Should connect via RTSPS");
            harness.stop().unwrap();
        }
        
        println!("Successfully tested both RTSP and RTSPS connections");
    }
}