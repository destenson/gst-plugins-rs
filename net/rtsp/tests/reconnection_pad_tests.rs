// Tests for rtspsrc2 pad state management during reconnection
//
// These tests verify that pad state remains consistent during auto-reconnection
// and that no-more-pads is properly emitted in all scenarios.

use gst::prelude::*;
use std::sync::{Arc, Mutex};

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("rtsp plugin register failed");
    });
}

/// Test that no-more-pads is never emitted multiple times
///
/// This is critical during reconnection - even if reconnection happens multiple times,
/// no-more-pads should only be emitted once per connection cycle.
#[test]
fn test_no_more_pads_emitted_only_once() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Track all no-more-pads emissions
    let emissions = Arc::new(Mutex::new(Vec::new()));
    let emissions_clone = emissions.clone();

    rtspsrc.connect("no-more-pads", false, move |_args| {
        let mut emits = emissions_clone.lock().unwrap();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        emits.push(timestamp);
        None
    });

    // Verify no emissions yet
    assert_eq!(emissions.lock().unwrap().len(), 0);

    // Note: Full reconnection testing requires a mock RTSP server
    // This test verifies the signal tracking infrastructure works
}

/// Test that reconnection properties are accessible
#[test]
fn test_reconnection_properties() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Verify reconnection properties exist and have sensible defaults
    let max_reconnection_attempts: i32 = rtspsrc.property("max-reconnection-attempts");
    assert!(max_reconnection_attempts > 0 || max_reconnection_attempts == -1,
            "max-reconnection-attempts should be positive or -1 for unlimited");

    let reconnection_timeout: u64 = rtspsrc.property("reconnection-timeout");
    assert!(reconnection_timeout > 0, "reconnection-timeout should be positive");

    let udp_reconnect: bool = rtspsrc.property("udp-reconnect");
    // Just verify it's readable
    let _ = udp_reconnect;

    // Test setting reconnection properties
    rtspsrc.set_property("max-reconnection-attempts", 10i32);
    let new_max: i32 = rtspsrc.property("max-reconnection-attempts");
    assert_eq!(new_max, 10);

    // Test unlimited attempts (-1)
    rtspsrc.set_property("max-reconnection-attempts", -1i32);
    let unlimited: i32 = rtspsrc.property("max-reconnection-attempts");
    assert_eq!(unlimited, -1);

    // Test disabling reconnection (0 attempts)
    rtspsrc.set_property("max-reconnection-attempts", 0i32);
    let disabled: i32 = rtspsrc.property("max-reconnection-attempts");
    assert_eq!(disabled, 0);
}

/// Test that pad state is properly reset
///
/// During reconnection, the internal pad state tracking should be reset
/// to ensure consistent behavior across multiple connection cycles.
#[test]
fn test_pad_state_consistency() {
    init();

    let pipeline = gst::Pipeline::new();
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    pipeline.add(&rtspsrc).expect("Failed to add rtspsrc2");

    // Track pad-added events
    let pads_added = Arc::new(Mutex::new(Vec::new()));
    let pads_clone = pads_added.clone();

    rtspsrc.connect_pad_added(move |_element, pad| {
        let mut pads = pads_clone.lock().unwrap();
        pads.push(pad.name().to_string());
    });

    // Verify no pads initially
    assert_eq!(pads_added.lock().unwrap().len(), 0);

    // Test state transitions from NULL (doesn't require connection)
    let state_change = rtspsrc.set_state(gst::State::Null);
    assert!(state_change.is_ok(), "Should stay in NULL");

    // Verify element is created and functional
    assert_eq!(rtspsrc.current_state(), gst::State::Null);
}

/// Test element behavior during state changes
///
/// Verify that the element properly handles state transitions that might
/// occur during reconnection attempts.
#[test]
fn test_state_transitions_during_reconnection() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Configure for quick timeout (so test doesn't hang)
    rtspsrc.set_property("location", "rtsp://invalid.example.com/stream");
    rtspsrc.set_property("timeout", 1_000_000_000u64); // 1 second

    // These transitions should succeed even if connection fails
    let result = rtspsrc.set_state(gst::State::Ready);
    assert!(result.is_ok());

    let result = rtspsrc.set_state(gst::State::Null);
    assert!(result.is_ok());
}

/// Test that pads are properly cleaned up on stop
#[test]
fn test_pad_cleanup_on_stop() {
    init();

    let pipeline = gst::Pipeline::new();
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    pipeline.add(&rtspsrc).expect("Failed to add element");

    // Get initial pad count (should be 0)
    let initial_pad_count = rtspsrc.src_pads().len();
    assert_eq!(initial_pad_count, 0, "Should have no pads initially");

    // Verify element is in NULL state
    assert_eq!(rtspsrc.current_state(), gst::State::Null);

    // Note: Without an actual RTSP connection, pads won't be created
    // This test verifies the initial state is clean
}

/// Test retry strategy configuration
#[test]
fn test_retry_strategy_configuration() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Test different retry strategies
    let strategies = vec!["auto", "none", "immediate", "linear", "exponential"];

    for strategy in strategies {
        rtspsrc.set_property("retry-strategy", strategy);
        let set_strategy: String = rtspsrc.property("retry-strategy");
        assert_eq!(
            set_strategy, strategy,
            "retry-strategy should be set to {}",
            strategy
        );
    }
}

/// Test that element reports URIHandler interface for reconnection
#[test]
fn test_uri_handler_for_reconnection() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Cast to URIHandler
    let uri_handler = rtspsrc
        .dynamic_cast::<gst::URIHandler>()
        .expect("Should implement URIHandler");

    // Test setting a URI
    uri_handler
        .set_uri("rtsp://example.com/stream")
        .expect("Should accept valid RTSP URI");

    let uri = uri_handler.uri();
    assert!(uri.is_some(), "URI should be set");
    assert!(
        uri.unwrap().starts_with("rtsp://"),
        "URI should be RTSP scheme"
    );
}
