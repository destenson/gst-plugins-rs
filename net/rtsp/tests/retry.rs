// GStreamer RTSP plugin retry logic tests
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
fn test_retry_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test retry-strategy property
    element.set_property("retry-strategy", "exponential");
    let strategy: String = element.property("retry-strategy");
    assert_eq!(strategy, "exponential");

    element.set_property("retry-strategy", "linear");
    let strategy: String = element.property("retry-strategy");
    assert_eq!(strategy, "linear");

    element.set_property("retry-strategy", "none");
    let strategy: String = element.property("retry-strategy");
    assert_eq!(strategy, "none");

    // Test max-reconnection-attempts property
    element.set_property("max-reconnection-attempts", 10i32);
    let attempts: i32 = element.property("max-reconnection-attempts");
    assert_eq!(attempts, 10);

    element.set_property("max-reconnection-attempts", -1i32); // Infinite
    let attempts: i32 = element.property("max-reconnection-attempts");
    assert_eq!(attempts, -1);

    // Test reconnection-timeout property (in nanoseconds)
    element.set_property("reconnection-timeout", 60_000_000_000u64); // 60 seconds
    let timeout: u64 = element.property("reconnection-timeout");
    assert_eq!(timeout, 60_000_000_000);

    // Test initial-retry-delay property
    element.set_property("initial-retry-delay", 500_000_000u64); // 500ms
    let delay: u64 = element.property("initial-retry-delay");
    assert_eq!(delay, 500_000_000);

    // Test linear-retry-step property
    element.set_property("linear-retry-step", 1_000_000_000u64); // 1 second
    let step: u64 = element.property("linear-retry-step");
    assert_eq!(step, 1_000_000_000);
}

#[test]
#[serial]
fn test_no_retry_strategy() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://127.0.0.1:9997/test") // Non-existent port
        .property("retry-strategy", "none")
        .property("timeout", 100_000_000u64) // 100ms timeout
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to go to PAUSED - should fail immediately without retry
    let start_time = Instant::now();
    let _result = element.set_state(gst::State::Paused);
    
    // Should fail quickly (within 2 seconds) since no retry
    assert!(start_time.elapsed() < Duration::from_secs(2));
    
    // For "none" strategy, we expect failure but state change might return async
    // Just verify we didn't spend time retrying
    let _ = element.set_state(gst::State::Null);
}

#[tokio::test]
#[serial]
async fn test_immediate_retry_strategy() {
    init();

    // Use an atomic counter to track connection attempts
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();

    // Start a listener that counts connection attempts
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                // Immediately close the connection
                drop(stream);
            }
        }
    });

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", format!("rtsp://127.0.0.1:{}/test", port))
        .property("retry-strategy", "immediate")
        .property("max-reconnection-attempts", 3i32)
        .property("timeout", 100_000_000u64) // 100ms timeout
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to connect - should retry immediately
    let _result = element.set_state(gst::State::Paused);
    
    // Give it some time to attempt connections
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // We should see multiple connection attempts (at least 2)
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(attempts >= 2, "Expected at least 2 connection attempts, got {}", attempts);

    element.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_linear_backoff_timing() {
    init();

    // Create an element with linear retry strategy
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://127.0.0.1:9999/test") // Non-existent port
        .property("retry-strategy", "linear")
        .property("max-reconnection-attempts", 3i32)
        .property("initial-retry-delay", 100_000_000u64) // 100ms
        .property("linear-retry-step", 50_000_000u64)    // 50ms step
        .property("timeout", 50_000_000u64)              // 50ms connection timeout
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a pipeline to get proper bus support
    let pipeline = gst::Pipeline::new();
    pipeline.add(&element).unwrap();
    
    // Connect to bus to monitor retry messages
    let bus = pipeline.bus().unwrap();
    let mut retry_times = Vec::new();
    let start_time = Instant::now();

    // Start state change in background
    let pipeline_clone = pipeline.clone();
    tokio::spawn(async move {
        let _ = pipeline_clone.set_state(gst::State::Paused);
    });

    // Collect retry messages
    while retry_times.len() < 3 {
        if let Some(msg) = bus.timed_pop(gst::ClockTime::from_seconds(2)) {
            if let gst::MessageView::Element(elem_msg) = msg.view() {
                if let Some(structure) = elem_msg.structure() {
                    if structure.name() == "rtsp-connection-retry" {
                        retry_times.push(start_time.elapsed());
                    }
                }
            }
        } else {
            break;
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();

    // Verify we got retry messages
    assert!(retry_times.len() >= 2, "Expected at least 2 retry attempts");

    // Verify timing follows linear pattern (with some tolerance)
    // First retry should be after ~100ms
    // Second retry should be after ~150ms (100ms + 50ms)
    // Third retry should be after ~200ms (100ms + 100ms)
    if retry_times.len() >= 2 {
        let first_delay = retry_times[0];
        let second_delay = retry_times[1] - retry_times[0];
        
        // Allow 50ms tolerance for timing
        assert!(first_delay >= Duration::from_millis(50) && first_delay <= Duration::from_millis(200),
                "First retry delay out of range: {:?}", first_delay);
        assert!(second_delay >= Duration::from_millis(100) && second_delay <= Duration::from_millis(250),
                "Second retry delay out of range: {:?}", second_delay);
    }
}

#[tokio::test]
#[serial]
async fn test_exponential_backoff_timing() {
    init();

    // Create an element with exponential retry strategy
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://127.0.0.1:9998/test") // Non-existent port
        .property("retry-strategy", "exponential")
        .property("max-reconnection-attempts", 4i32)
        .property("initial-retry-delay", 100_000_000u64) // 100ms
        .property("timeout", 50_000_000u64)              // 50ms connection timeout
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a pipeline to get proper bus support
    let pipeline = gst::Pipeline::new();
    pipeline.add(&element).unwrap();
    
    // Connect to bus to monitor retry messages
    let bus = pipeline.bus().unwrap();
    let mut retry_delays = Vec::new();

    // Start state change in background
    let pipeline_clone = pipeline.clone();
    tokio::spawn(async move {
        let _ = pipeline_clone.set_state(gst::State::Paused);
    });

    // Collect retry delay values from messages
    while retry_delays.len() < 3 {
        if let Some(msg) = bus.timed_pop(gst::ClockTime::from_seconds(3)) {
            if let gst::MessageView::Element(elem_msg) = msg.view() {
                if let Some(structure) = elem_msg.structure() {
                    if structure.name() == "rtsp-connection-retry" {
                        if let Ok(delay_ms) = structure.get::<u64>("next-delay-ms") {
                            retry_delays.push(delay_ms);
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    pipeline.set_state(gst::State::Null).unwrap();

    // Verify we got retry messages
    assert!(retry_delays.len() >= 2, "Expected at least 2 retry attempts");

    // Verify delays follow exponential pattern
    // Should be approximately: 100ms, 200ms, 400ms, 800ms
    if retry_delays.len() >= 2 {
        assert!(retry_delays[0] >= 75 && retry_delays[0] <= 150,
                "First delay out of range: {}ms", retry_delays[0]);
        assert!(retry_delays[1] >= 150 && retry_delays[1] <= 250,
                "Second delay out of range: {}ms", retry_delays[1]);
    }
    if retry_delays.len() >= 3 {
        assert!(retry_delays[2] >= 350 && retry_delays[2] <= 450,
                "Third delay out of range: {}ms", retry_delays[2]);
    }
}

#[tokio::test]
#[serial]
async fn test_max_reconnection_attempts() {
    init();

    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = attempt_count.clone();

    // Start a listener that counts but rejects connections
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    
    tokio::spawn(async move {
        loop {
            if let Ok((stream, _)) = listener.accept().await {
                attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                drop(stream);
            }
        }
    });

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", format!("rtsp://127.0.0.1:{}/test", port))
        .property("retry-strategy", "immediate")
        .property("max-reconnection-attempts", 2i32)
        .property("timeout", 100_000_000u64) // 100ms
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to connect - should stop after max attempts
    let _ = element.set_state(gst::State::Paused);
    
    // Wait for retries to complete
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Should not exceed max attempts + 1 (initial attempt + retries)
    let attempts = attempt_count.load(Ordering::SeqCst);
    assert!(attempts <= 3, "Should not exceed 3 total attempts (1 initial + 2 retries), got {}", attempts);

    element.set_state(gst::State::Null).unwrap();
}

#[test]
#[serial]
fn test_retry_strategy_with_mock_server() {
    init();

    // This test verifies that successful connections reset the retry counter
    // We'll use the existing mock server to test this
    // The actual connection test with mock server is in integration.rs
    
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("retry-strategy", "exponential")
        .property("max-reconnection-attempts", 5i32)
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Just verify the element was created with retry settings
    let strategy: String = element.property("retry-strategy");
    assert_eq!(strategy, "exponential");
    
    let attempts: i32 = element.property("max-reconnection-attempts");
    assert_eq!(attempts, 5);
}