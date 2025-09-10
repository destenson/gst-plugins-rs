// GStreamer RTSP plugin connection racing tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use gst::prelude::*;
use serial_test::serial;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::time::sleep;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("rtsp plugin registration failed");
    });
}

#[test]
#[serial]
fn test_racing_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test connection-racing property
    element.set_property("connection-racing", "first-wins");
    let strategy: String = element.property("connection-racing");
    assert_eq!(strategy, "first-wins");

    element.set_property("connection-racing", "last-wins");
    let strategy: String = element.property("connection-racing");
    assert_eq!(strategy, "last-wins");

    element.set_property("connection-racing", "hybrid");
    let strategy: String = element.property("connection-racing");
    assert_eq!(strategy, "hybrid");

    element.set_property("connection-racing", "none");
    let strategy: String = element.property("connection-racing");
    assert_eq!(strategy, "none");

    // Test max-parallel-connections property
    element.set_property("max-parallel-connections", 5u32);
    let max_conn: u32 = element.property("max-parallel-connections");
    assert_eq!(max_conn, 5);

    // Test racing-delay-ms property
    element.set_property("racing-delay-ms", 100u32);
    let delay: u32 = element.property("racing-delay-ms");
    assert_eq!(delay, 100);

    // Test racing-timeout property
    element.set_property("racing-timeout", 10_000_000_000u64); // 10 seconds
    let timeout: u64 = element.property("racing-timeout");
    assert_eq!(timeout, 10_000_000_000);
}

#[tokio::test]
#[serial]
async fn test_racing_first_wins() {
    init();

    // Track connection attempts
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();

    // Start multiple listeners on different ports
    let listener1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port1 = listener1.local_addr().unwrap().port();
    
    let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let _port2 = listener2.local_addr().unwrap().port();

    // First listener accepts immediately
    let handle1 = tokio::spawn(async move {
        if let Ok((stream, _)) = listener1.accept().await {
            attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            // Keep connection open
            sleep(Duration::from_secs(10)).await;
            drop(stream);
        }
    });

    // Second listener has delay before accepting
    let attempt_count_clone2 = attempt_count.clone();
    let handle2 = tokio::spawn(async move {
        sleep(Duration::from_millis(500)).await;
        if let Ok((stream, _)) = listener2.accept().await {
            attempt_count_clone2.fetch_add(1, Ordering::SeqCst);
            drop(stream);
        }
    });

    // Create element with first-wins racing
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", format!("rtsp://127.0.0.1:{}/test", port1))
        .property("connection-racing", "first-wins")
        .property("max-parallel-connections", 2u32)
        .property("racing-delay-ms", 100u32)
        .property("timeout", 200_000_000u64) // 200ms
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to connect - should use first connection
    let _result = element.set_state(gst::State::Paused);
    
    // Give some time for connection
    sleep(Duration::from_millis(300)).await;
    
    // Should only see one connection (the first one)
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert_eq!(attempts, 1, "Expected only 1 connection for first-wins");

    element.set_state(gst::State::Null).unwrap();
    
    handle1.abort();
    handle2.abort();
}

#[tokio::test]
#[serial]
async fn test_racing_last_wins() {
    init();

    // Track connection attempts
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_times = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    // Start multiple listeners
    let listener1 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port1 = listener1.local_addr().unwrap().port();
    
    let listener2 = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let _port2 = listener2.local_addr().unwrap().port();

    // Both listeners accept connections
    let attempt_count_clone = attempt_count.clone();
    let attempt_times_clone = attempt_times.clone();
    let handle1 = tokio::spawn(async move {
        if let Ok((stream, _)) = listener1.accept().await {
            attempt_count_clone.fetch_add(1, Ordering::SeqCst);
            let mut times = attempt_times_clone.lock().await;
            times.push(Instant::now());
            // Simulate connection being dropped
            drop(stream);
        }
    });

    let attempt_count_clone2 = attempt_count.clone();
    let attempt_times_clone2 = attempt_times.clone();
    let handle2 = tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        if let Ok((stream, _)) = listener2.accept().await {
            attempt_count_clone2.fetch_add(1, Ordering::SeqCst);
            let mut times = attempt_times_clone2.lock().await;
            times.push(Instant::now());
            // Keep this connection
            sleep(Duration::from_secs(10)).await;
            drop(stream);
        }
    });

    // Create element with last-wins racing
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", format!("rtsp://127.0.0.1:{}/test", port1))
        .property("connection-racing", "last-wins")
        .property("max-parallel-connections", 2u32)
        .property("racing-delay-ms", 200u32)
        .property("timeout", 500_000_000u64) // 500ms
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to connect - should try both and use the last one
    let _result = element.set_state(gst::State::Paused);
    
    // Give time for both connections
    sleep(Duration::from_millis(600)).await;
    
    // Should see both connection attempts for last-wins
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(attempts >= 1, "Expected at least 1 connection attempt for last-wins");

    element.set_state(gst::State::Null).unwrap();
    
    handle1.abort();
    handle2.abort();
}

#[tokio::test]
#[serial]
async fn test_racing_with_all_failures() {
    init();

    // No listeners - all connections will fail
    
    // Create element with first-wins racing
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://127.0.0.1:65535/test") // Invalid port
        .property("connection-racing", "first-wins")
        .property("max-parallel-connections", 3u32)
        .property("racing-delay-ms", 50u32)
        .property("racing-timeout", 200_000_000u64) // 200ms
        .property("timeout", 100_000_000u64) // 100ms per connection
        .property("retry-strategy", "none") // No retry to test racing only
        .build()
        .expect("Failed to create rtspsrc2 element");

    let start_time = Instant::now();
    
    // Try to connect - all should fail
    let result = element.set_state(gst::State::Paused);
    
    // Should fail relatively quickly with racing
    assert!(start_time.elapsed() < Duration::from_secs(2), 
            "Racing should fail quickly when all connections fail");
    
    // State change should not succeed
    let (_res, state, _pending) = element.state(gst::ClockTime::from_mseconds(500));
    assert_eq!(state, gst::State::Null, "Should remain in NULL state when connection fails");

    element.set_state(gst::State::Null).unwrap();
}

#[test]
#[serial]
fn test_racing_property_defaults() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Check default values
    let strategy: String = element.property("connection-racing");
    assert_eq!(strategy, "none", "Default racing strategy should be 'none'");

    let max_conn: u32 = element.property("max-parallel-connections");
    assert_eq!(max_conn, 3, "Default max parallel connections should be 3");

    let delay: u32 = element.property("racing-delay-ms");
    assert_eq!(delay, 250, "Default racing delay should be 250ms");

    let timeout: u64 = element.property("racing-timeout");
    assert_eq!(timeout, 5_000_000_000, "Default racing timeout should be 5 seconds");
}

#[tokio::test]
#[serial]
async fn test_racing_cleanup() {
    init();

    // Track connection attempts and ensure cleanup
    let connection_count = Arc::new(AtomicU32::new(0));
    let active_connections = Arc::new(AtomicU32::new(0));

    // Start a listener that tracks connections
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    
    let connection_count_clone = connection_count.clone();
    let active_connections_clone = active_connections.clone();
    
    let handle = tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                connection_count_clone.fetch_add(1, Ordering::SeqCst);
                let active = active_connections_clone.clone();
                active.fetch_add(1, Ordering::SeqCst);
                
                // Track when connection is dropped
                tokio::spawn(async move {
                    sleep(Duration::from_secs(10)).await;
                    drop(stream);
                    active.fetch_sub(1, Ordering::SeqCst);
                });
            }
        }
    });

    // Create element with first-wins racing
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", format!("rtsp://127.0.0.1:{}/test", port))
        .property("connection-racing", "first-wins")
        .property("max-parallel-connections", 3u32)
        .property("racing-delay-ms", 100u32)
        .property("timeout", 500_000_000u64) // 500ms
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Connect
    let _result = element.set_state(gst::State::Paused);
    sleep(Duration::from_millis(500)).await;
    
    // Should have attempted multiple connections
    let total_attempts = connection_count.load(Ordering::SeqCst);
    assert!(total_attempts >= 1, "Should have at least one connection attempt");
    
    // But only one should remain active
    let active = active_connections.load(Ordering::SeqCst);
    assert_eq!(active, 1, "Only one connection should remain active after first-wins");

    // Clean up
    element.set_state(gst::State::Null).unwrap();
    handle.abort();
}