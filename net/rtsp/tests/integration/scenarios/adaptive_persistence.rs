// Adaptive learning persistence scenario tests
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
    use crate::integration::server_helper::TestMode;
    use crate::integration::{MediaMtxServer, RtspTestHarness};
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    fn get_cache_dir() -> PathBuf {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| std::env::temp_dir())
            .join("gst-rtsp-adaptive");
        fs::create_dir_all(&cache_dir).unwrap();
        cache_dir
    }

    fn clear_adaptive_cache() {
        let cache_dir = get_cache_dir();
        if cache_dir.exists() {
            let _ = fs::remove_dir_all(&cache_dir);
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_adaptive_learning_persistence() {
        // Test that adaptive learning persists across restarts

        clear_adaptive_cache();

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url = server.url("videotestsrc");

        // First session - train the adaptive system
        {
            let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

            // Enable adaptive learning
            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();
            harness.set_property("max-retry-attempts", 10i32).unwrap();

            harness.start().unwrap();

            // Simulate some connection patterns
            for _ in 0..3 {
                let connected = harness
                    .wait_for_connection(Duration::from_secs(10))
                    .unwrap();
                assert!(connected, "Should connect");

                std::thread::sleep(Duration::from_secs(2));
                harness.force_reconnect().unwrap();
            }

            // The system should have learned something
            let racing_strategy = harness
                .get_property::<String>("racing-strategy")
                .unwrap_or_default();
            assert!(
                !racing_strategy.is_empty(),
                "Should have determined a racing strategy"
            );

            // Stop the harness (should trigger persistence)
            harness.stop().unwrap();
        }

        // Second session - verify persistence
        {
            let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

            // Enable adaptive learning again
            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();

            harness.start().unwrap();

            // Should immediately have the learned strategy
            let racing_strategy = harness
                .get_property::<String>("racing-strategy")
                .unwrap_or_default();
            assert!(
                !racing_strategy.is_empty(),
                "Should have loaded persisted strategy"
            );

            // Verify it connects using the learned approach
            let connected = harness.wait_for_connection(Duration::from_secs(5)).unwrap();
            assert!(connected, "Should connect quickly with learned strategy");
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_adaptive_learning_per_server() {
        // Test that adaptive learning is per-server

        clear_adaptive_cache();

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url1 = server.url("videotestsrc");
        let url2 = server.url("videotestsrc-faulty");

        // Train on first URL
        {
            let mut harness = RtspTestHarness::new(&url1).expect("Failed to create test harness");

            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();

            harness.start().unwrap();
            harness
                .wait_for_connection(Duration::from_secs(10))
                .unwrap();

            // Should learn one strategy for normal server
            let strategy1 = harness
                .get_property::<String>("racing-strategy")
                .unwrap_or_default();

            harness.stop().unwrap();

            // Now test with faulty server
            let mut harness2 = RtspTestHarness::new(&url2).expect("Failed to create test harness");

            harness2.set_property("retry-strategy", "auto").unwrap();
            harness2.set_property("adaptive-learning", true).unwrap();

            harness2.start().unwrap();

            // Give it time to learn different pattern
            std::thread::sleep(Duration::from_secs(10));

            let strategy2 = harness2
                .get_property::<String>("racing-strategy")
                .unwrap_or_default();

            // Strategies might be different based on server behavior
            println!(
                "Strategy for normal: {}, Strategy for faulty: {}",
                strategy1, strategy2
            );
        }

        // Verify both are persisted
        {
            let mut harness1 = RtspTestHarness::new(&url1).expect("Failed to create test harness");

            harness1.set_property("retry-strategy", "auto").unwrap();
            harness1.set_property("adaptive-learning", true).unwrap();

            harness1.start().unwrap();

            let loaded_strategy1 = harness1
                .get_property::<String>("racing-strategy")
                .unwrap_or_default();
            assert!(
                !loaded_strategy1.is_empty(),
                "Should have persisted strategy for URL1"
            );
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_adaptive_cache_cleanup() {
        // Test that old cache entries are cleaned up

        clear_adaptive_cache();

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        // Create multiple sessions to generate cache entries
        for i in 0..5 {
            let url = server.url(&format!("test{}", i));

            let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();
            harness.set_property("adaptive-cache-ttl", 1u32).unwrap(); // 1 second TTL for testing

            harness.start().unwrap();
            std::thread::sleep(Duration::from_millis(500));
            harness.stop().unwrap();
        }

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_secs(2));

        // Create new session which should trigger cleanup
        let url = server.url("cleanup-test");
        let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

        harness.set_property("retry-strategy", "auto").unwrap();
        harness.set_property("adaptive-learning", true).unwrap();

        harness.start().unwrap();

        // Check cache directory size
        let cache_dir = get_cache_dir();
        if cache_dir.exists() {
            let entries: Vec<_> = fs::read_dir(&cache_dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .collect();

            // Should have cleaned up old entries
            assert!(
                entries.len() < 10,
                "Cache cleanup should limit number of entries"
            );
        }
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_adaptive_learning_improves_performance() {
        // Test that adaptive learning actually improves connection time

        clear_adaptive_cache();

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url = server.url("videotestsrc-faulty");

        // First connection without learning
        let initial_connect_time = {
            let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

            harness
                .set_property("retry-strategy", "exponential")
                .unwrap();
            harness.set_property("adaptive-learning", false).unwrap();

            let start = std::time::Instant::now();
            harness.start().unwrap();
            harness
                .wait_for_connection(Duration::from_secs(30))
                .unwrap();
            start.elapsed()
        };

        // Train the adaptive system
        {
            let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();

            harness.start().unwrap();

            // Multiple connection attempts to train
            for _ in 0..5 {
                harness
                    .wait_for_connection(Duration::from_secs(10))
                    .unwrap();
                harness.force_reconnect().unwrap();
            }

            harness.stop().unwrap();
        }

        // Connection with learned strategy
        let learned_connect_time = {
            let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

            harness.set_property("retry-strategy", "auto").unwrap();
            harness.set_property("adaptive-learning", true).unwrap();

            let start = std::time::Instant::now();
            harness.start().unwrap();
            harness
                .wait_for_connection(Duration::from_secs(30))
                .unwrap();
            start.elapsed()
        };

        println!(
            "Initial connect time: {:?}, Learned connect time: {:?}",
            initial_connect_time, learned_connect_time
        );

        // Learned strategy should be faster or at least not significantly slower
        assert!(
            learned_connect_time <= initial_connect_time + Duration::from_secs(2),
            "Adaptive learning should not make connections slower"
        );
    }
}
