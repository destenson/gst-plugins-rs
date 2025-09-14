// Connection-limited device scenario tests
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
    fn test_connection_limited_device_auto_mode() {
        // This test verifies that auto mode detects connection drops and switches to last-wins racing
        
        // Start server that drops connections after 20 seconds
        let server = MediaMtxServer::new(TestMode::ConnectionLimited { timeout_secs: 20 })
            .expect("Failed to start MediaMTX server");
        
        let url = server.url("test");
        
        // Create harness with auto retry mode
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Enable auto retry mode
        harness.set_property("retry-strategy", "auto").unwrap();
        harness.set_property("max-retry-attempts", 10i32).unwrap();
        
        // Start streaming
        harness.start().unwrap();
        
        // Wait for initial connection
        let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
        assert!(connected, "Should connect initially");
        
        // Wait for connection to drop (should happen after ~20 seconds)
        std::thread::sleep(Duration::from_secs(22));
        
        // Check that we're attempting to reconnect
        let stats = harness.get_retry_stats().unwrap();
        assert!(stats.connection_attempts > 1, "Should have multiple connection attempts");
        
        // Verify auto mode detected the pattern
        let racing_strategy = harness.get_property::<String>("racing-strategy").unwrap_or_default();
        assert_eq!(racing_strategy, "last-wins", "Auto mode should switch to last-wins for connection-limited devices");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_connection_limited_retry_behavior() {
        // This test verifies retry behavior with connection drops
        
        let server = MediaMtxServer::new(TestMode::ConnectionLimited { timeout_secs: 5 })
            .expect("Failed to start MediaMTX server");
        
        let url = server.url("test");
        
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Configure retry settings
        harness.set_property("retry-strategy", "exponential").unwrap();
        harness.set_property("max-retry-attempts", 5i32).unwrap();
        harness.set_property("retry-initial-delay", 1000u32).unwrap(); // 1 second
        
        harness.start().unwrap();
        
        // Wait for initial connection
        let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
        assert!(connected, "Should connect initially");
        
        // Clear messages to track new ones
        harness.clear_messages();
        
        // Wait for connection drop
        std::thread::sleep(Duration::from_secs(6));
        
        // Collect retry attempts over next 10 seconds
        std::thread::sleep(Duration::from_secs(10));
        
        let stats = harness.get_retry_stats().unwrap();
        assert!(stats.connection_attempts >= 2, "Should have at least 2 connection attempts");
        assert!(stats.connection_failures >= 1, "Should have at least 1 connection failure");
        
        // Verify exponential backoff is working
        let messages = harness.get_messages();
        let mut retry_timestamps = Vec::new();
        
        for msg in messages.iter() {
            if let gst::MessageView::Element(element) = msg.view() {
                if let Some(structure) = element.structure() {
                    if structure.name().contains("retry") {
                        retry_timestamps.push(harness.elapsed());
                    }
                }
            }
        }
        
        // Check that retry delays are increasing
        if retry_timestamps.len() >= 2 {
            for i in 1..retry_timestamps.len() {
                let delay = retry_timestamps[i].as_millis() - retry_timestamps[i-1].as_millis();
                assert!(delay > 900, "Retry delay should be at least 900ms");
            }
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_connection_limited_with_keepalive() {
        // Test that keep-alive helps maintain connections
        
        let server = MediaMtxServer::new(TestMode::ConnectionLimited { timeout_secs: 30 })
            .expect("Failed to start MediaMTX server");
        
        let url = server.url("test");
        
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Enable keep-alive with short interval
        harness.set_property("do-rtsp-keep-alive", true).unwrap();
        harness.set_property("keep-alive-interval", 10u32).unwrap(); // 10 seconds
        
        harness.start().unwrap();
        
        // Wait for connection
        let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
        assert!(connected, "Should connect");
        
        // Wait for 25 seconds (less than timeout but multiple keep-alive intervals)
        std::thread::sleep(Duration::from_secs(25));
        
        // Should still be playing thanks to keep-alive
        assert!(harness.is_playing(), "Should still be playing with keep-alive");
        
        let stats = harness.get_retry_stats().unwrap();
        assert_eq!(stats.connection_attempts, 1, "Should maintain single connection with keep-alive");
    }
}