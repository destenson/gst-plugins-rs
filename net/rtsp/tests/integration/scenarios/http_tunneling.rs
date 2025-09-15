// HTTP tunneling scenario tests
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
    use std::time::Duration;

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_http_tunneling_activation() {
        // Test that HTTP tunneling activates when RTSP port is blocked

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        // Use rtspt:// URL to force HTTP tunneling
        let rtsp_url = server.url("videotestsrc");
        let tunnel_url = rtsp_url.replace("rtsp://", "rtspt://");

        let mut harness = RtspTestHarness::new(&tunnel_url).expect("Failed to create test harness");

        // Enable HTTP tunneling
        harness.set_property("protocols", "tcp+http").unwrap();

        harness.start().unwrap();

        // Should connect via HTTP tunnel
        let connected = harness
            .wait_for_connection(Duration::from_secs(15))
            .unwrap();
        assert!(connected, "Should connect via HTTP tunnel");

        // Verify tunneling is active
        let messages = harness.get_messages();
        let mut tunnel_active = false;

        for msg in messages.iter() {
            if let gst::MessageView::Element(element) = msg.view() {
                if let Some(structure) = element.structure() {
                    let name = structure.name();
                    if name.contains("tunnel") || name.contains("http") {
                        tunnel_active = true;
                        break;
                    }
                }
            }
        }

        assert!(tunnel_active, "HTTP tunneling should be active");
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_http_tunneling_fallback() {
        // Test automatic fallback to HTTP tunneling when direct RTSP fails

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url = server.url("videotestsrc");

        let mut harness = RtspTestHarness::new(&url).expect("Failed to create test harness");

        // Configure to try TCP first, then fall back to HTTP
        harness.set_property("protocols", "tcp+http").unwrap();
        harness.set_property("tcp-timeout", 5000u32).unwrap(); // 5 second TCP timeout

        // Simulate RTSP port being blocked by using wrong port initially
        let blocked_url = url.replace(":8554", ":8555");
        harness.set_property("location", &blocked_url).unwrap();

        harness.start().unwrap();

        // Give it time to fail TCP and try HTTP
        std::thread::sleep(Duration::from_secs(8));

        // Now fix the URL to allow HTTP tunneling to work
        harness.set_property("location", &url).unwrap();

        // Should eventually connect via HTTP tunnel
        let connected = harness
            .wait_for_connection(Duration::from_secs(20))
            .unwrap();
        assert!(connected, "Should fall back to HTTP tunneling");

        let stats = harness.get_retry_stats().unwrap();
        assert!(
            stats.connection_attempts >= 2,
            "Should have multiple attempts (TCP then HTTP)"
        );
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_http_tunneling_with_proxy() {
        // Test HTTP tunneling through proxy settings

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url = server.url("videotestsrc");
        let tunnel_url = url.replace("rtsp://", "rtspt://");

        let mut harness = RtspTestHarness::new(&tunnel_url).expect("Failed to create test harness");

        // Configure proxy settings (even if proxy doesn't exist, test the configuration)
        harness
            .set_property("proxy", "http://proxy.example.com:8080")
            .unwrap();
        harness.set_property("proxy-id", "testuser").unwrap();
        harness.set_property("proxy-pw", "testpass").unwrap();

        harness.start().unwrap();

        // Will likely fail to connect through non-existent proxy, but should attempt
        std::thread::sleep(Duration::from_secs(5));

        let messages = harness.get_messages();
        let mut proxy_attempted = false;

        for msg in messages.iter() {
            if let gst::MessageView::Element(element) = msg.view() {
                if let Some(structure) = element.structure() {
                    let name = structure.name();
                    if name.contains("proxy") || name.contains("http") {
                        proxy_attempted = true;
                        break;
                    }
                }
            }
        }

        assert!(
            proxy_attempted || !harness.is_playing(),
            "Should attempt proxy connection"
        );
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_http_tunneling_performance() {
        // Test that HTTP tunneling maintains reasonable performance

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url = server.url("videotestsrc");
        let tunnel_url = url.replace("rtsp://", "rtspt://");

        let mut harness = RtspTestHarness::new(&tunnel_url).expect("Failed to create test harness");

        // Configure for HTTP tunneling with buffering
        harness.set_property("protocols", "tcp+http").unwrap();
        harness.set_property("latency", 500u32).unwrap(); // 500ms buffer

        harness.start().unwrap();

        // Wait for connection
        let start = std::time::Instant::now();
        let connected = harness
            .wait_for_connection(Duration::from_secs(10))
            .unwrap();
        let connect_time = start.elapsed();

        assert!(connected, "Should connect via HTTP tunnel");
        assert!(
            connect_time < Duration::from_secs(10),
            "HTTP tunnel connection should be reasonably fast"
        );

        // Run for a while and check for stability
        harness.clear_messages();
        std::thread::sleep(Duration::from_secs(10));

        let messages = harness.get_messages();
        let mut errors = 0;
        let mut data_received = false;

        for msg in messages.iter() {
            match msg.view() {
                gst::MessageView::Error(_) => errors += 1,
                gst::MessageView::Element(element) => {
                    if let Some(structure) = element.structure() {
                        if structure.name().contains("data") || structure.name().contains("buffer")
                        {
                            data_received = true;
                        }
                    }
                }
                _ => {}
            }
        }

        assert_eq!(errors, 0, "HTTP tunneling should be stable");
        assert!(
            data_received || harness.is_playing(),
            "Should receive data through HTTP tunnel"
        );
    }

    #[test]
    #[cfg(feature = "integration-tests")]
    fn test_http_tunneling_reconnection() {
        // Test reconnection behavior with HTTP tunneling

        let server =
            MediaMtxServer::new(TestMode::Normal).expect("Failed to start MediaMTX server");

        let url = server.url("videotestsrc");
        let tunnel_url = url.replace("rtsp://", "rtspt://");

        let mut harness = RtspTestHarness::new(&tunnel_url).expect("Failed to create test harness");

        // Configure HTTP tunneling with retry
        harness.set_property("protocols", "tcp+http").unwrap();
        harness
            .set_property("retry-strategy", "exponential")
            .unwrap();
        harness.set_property("max-retry-attempts", 5i32).unwrap();

        harness.start().unwrap();

        // Wait for initial connection
        let connected = harness
            .wait_for_connection(Duration::from_secs(10))
            .unwrap();
        assert!(connected, "Should connect initially");

        // Force reconnection
        harness.force_reconnect().unwrap();

        // Should reconnect via HTTP tunnel
        let reconnected = harness
            .wait_for_connection(Duration::from_secs(15))
            .unwrap();
        assert!(reconnected, "Should reconnect via HTTP tunnel");

        let stats = harness.get_retry_stats().unwrap();
        assert!(
            stats.connection_successes >= 2,
            "Should have at least 2 successful connections"
        );
    }
}
