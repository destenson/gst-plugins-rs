// Lossy network scenario tests
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
    fn test_lossy_network_auto_mode() {
        // This test verifies that auto mode detects high failure rate and switches to first-wins racing
        
        // Use the faulty network source from mediamtx.yml
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        // Use the videotestsrc-faulty path which has 2% packet loss
        let url = server.url("videotestsrc-faulty");
        
        // Create harness with auto retry mode
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Enable auto retry mode
        harness.set_property("retry-strategy", "auto").unwrap();
        harness.set_property("max-retry-attempts", 15i32).unwrap();
        
        // Start streaming
        harness.start().unwrap();
        
        // Allow time for multiple connection attempts in lossy conditions
        std::thread::sleep(Duration::from_secs(15));
        
        let stats = harness.get_retry_stats().unwrap();
        
        // With 2% packet loss, we should see some failures
        if stats.connection_failures > 3 {
            // Verify auto mode detected the lossy pattern
            let racing_strategy = harness.get_property::<String>("racing-strategy").unwrap_or_default();
            assert_eq!(racing_strategy, "first-wins", "Auto mode should switch to first-wins for lossy networks");
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_lossy_network_retry_resilience() {
        // Test that retry mechanism handles packet loss gracefully
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        // Use the more aggressive lossy source
        let url = server.url("videotestsrc-bad");
        
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Configure aggressive retry for lossy conditions
        harness.set_property("retry-strategy", "linear").unwrap();
        harness.set_property("max-retry-attempts", 20i32).unwrap();
        harness.set_property("retry-initial-delay", 500u32).unwrap(); // 500ms
        harness.set_property("retry-linear-step", 500u32).unwrap(); // 500ms increments
        
        harness.start().unwrap();
        
        // Give it time to establish connection despite packet loss
        let connected = harness.wait_for_connection(Duration::from_secs(30)).unwrap();
        assert!(connected, "Should eventually connect despite packet loss");
        
        // Run for a while to see if connection is maintained
        std::thread::sleep(Duration::from_secs(20));
        
        let stats = harness.get_retry_stats().unwrap();
        println!("Lossy network stats: attempts={}, failures={}, successes={}", 
                 stats.connection_attempts, stats.connection_failures, stats.connection_successes);
        
        // We should have at least one successful connection
        assert!(stats.connection_successes >= 1, "Should have at least one successful connection");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_lossy_network_with_racing() {
        // Test connection racing in lossy conditions
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        let url = server.url("videotestsrc-faulty");
        
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Enable connection racing
        harness.set_property("connection-racing", true).unwrap();
        harness.set_property("racing-strategy", "first-wins").unwrap();
        harness.set_property("racing-connections", 3i32).unwrap(); // Try 3 parallel connections
        
        harness.start().unwrap();
        
        // First-wins racing should help establish connection faster
        let start = std::time::Instant::now();
        let connected = harness.wait_for_connection(Duration::from_secs(10)).unwrap();
        let connect_time = start.elapsed();
        
        assert!(connected, "Racing should help establish connection");
        assert!(connect_time < Duration::from_secs(8), "Racing should connect faster than serial attempts");
        
        // Check that racing is actually being used
        let messages = harness.get_messages();
        let mut racing_attempts = 0;
        
        for msg in messages.iter() {
            if let gst::MessageView::Element(element) = msg.view() {
                if let Some(structure) = element.structure() {
                    if structure.name().contains("racing") || structure.name().contains("parallel") {
                        racing_attempts += 1;
                    }
                }
            }
        }
        
        assert!(racing_attempts > 0, "Should see evidence of connection racing");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_lossy_network_buffering() {
        // Test that buffering helps with packet loss
        
        let server = MediaMtxServer::new(TestMode::Normal)
            .expect("Failed to start MediaMTX server");
        
        let url = server.url("videotestsrc-faulty");
        
        let mut harness = RtspTestHarness::new(&url)
            .expect("Failed to create test harness");
        
        // Configure buffering to handle packet loss
        harness.set_property("buffer-mode", "auto").unwrap();
        harness.set_property("latency", 1000u32).unwrap(); // 1 second buffer
        
        harness.start().unwrap();
        
        // Wait for connection and buffering
        let connected = harness.wait_for_connection(Duration::from_secs(15)).unwrap();
        assert!(connected, "Should connect with buffering");
        
        // Let it run and check for buffer underruns
        harness.clear_messages();
        std::thread::sleep(Duration::from_secs(10));
        
        let messages = harness.get_messages();
        let mut underruns = 0;
        
        for msg in messages.iter() {
            match msg.view() {
                gst::MessageView::Buffering(buffering) => {
                    let percent = buffering.percent();
                    if percent < 100 {
                        underruns += 1;
                    }
                }
                _ => {}
            }
        }
        
        // With proper buffering, underruns should be minimal
        assert!(underruns < 5, "Buffering should minimize underruns in lossy conditions");
    }
}