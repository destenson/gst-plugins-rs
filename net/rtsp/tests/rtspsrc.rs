// GStreamer RTSP plugin unit tests
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
fn test_element_registration() {
    init();

    // Check that the element factory is available
    let factory = gst::ElementFactory::find("rtspsrc2");
    assert!(factory.is_some(), "rtspsrc2 element factory not found");

    // Check element metadata
    let factory = factory.unwrap();
    assert_eq!(factory.element_type().name(), "GstRtspSrc2");

    // Check that it's in the right category
    let klass = factory.metadata("klass").unwrap();
    assert!(klass.contains("Source"));
    assert!(klass.contains("Network"));
}

#[test]
#[serial]
fn test_element_creation() {
    init();

    // Create an element instance
    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify element name
    assert_eq!(element.factory().unwrap().name(), "rtspsrc2");

    // Check that it's a GstBin (rtspsrc2 is implemented as a bin)
    assert!(element.is::<gst::Bin>());
}

#[test]
#[serial]
fn test_property_defaults() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test location property (should be None by default)
    let location: Option<String> = element.property("location");
    assert_eq!(location, None);

    // Test protocols property
    let protocols: String = element.property("protocols");
    assert!(!protocols.is_empty());

    // Test port-start property (not port-range)
    let port_start: u32 = element.property("port-start");
    assert_eq!(port_start, 0);

    // Test timeout property (in nanoseconds)
    let timeout: u64 = element.property("timeout");
    assert_eq!(timeout, 5_000_000_000); // 5 seconds in nanoseconds

    // Test receive-mtu property
    let receive_mtu: u32 = element.property("receive-mtu");
    assert_eq!(receive_mtu, 1508); // Default MTU + 8

    // Note: is-live is a property of GstBin, not specific to rtspsrc2
}

#[test]
#[serial]
fn test_jitterbuffer_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2").build().unwrap();

    // Test that the properties exist and can be read/written
    assert!(element.property_type("latency").is_some());
    assert!(element.property_type("drop-on-latency").is_some());
    assert!(element.property_type("probation").is_some());

    // Test default values
    assert_eq!(element.property::<u32>("latency"), 2000);
    assert_eq!(element.property::<bool>("drop-on-latency"), false);
    assert_eq!(element.property::<u32>("probation"), 2);

    // Test setting values
    element.set_property("latency", 5000u32);
    assert_eq!(element.property::<u32>("latency"), 5000);

    element.set_property("drop-on-latency", true);
    assert_eq!(element.property::<bool>("drop-on-latency"), true);

    element.set_property("probation", 10u32);
    assert_eq!(element.property::<u32>("probation"), 10);

    // Test edge values
    element.set_property("latency", 0u32);
    assert_eq!(element.property::<u32>("latency"), 0);

    element.set_property("latency", u32::MAX);
    assert_eq!(element.property::<u32>("latency"), u32::MAX);
}

#[test]
#[serial]
fn test_buffer_mode_property() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2").build().unwrap();

    // Test that buffer-mode property exists and has correct default
    assert!(element.property_type("buffer-mode").is_some());
    assert_eq!(
        element.property::<Option<String>>("buffer-mode"),
        Some("auto".to_string())
    );

    // Test setting all valid buffer modes
    let valid_modes = ["none", "slave", "buffer", "auto", "synced"];
    for mode in &valid_modes {
        element.set_property("buffer-mode", mode);
        assert_eq!(
            element.property::<Option<String>>("buffer-mode"),
            Some(mode.to_string())
        );
    }

    // Test setting invalid mode should not crash (but may emit warnings)
    element.set_property("buffer-mode", "invalid-mode");
    // Element should keep previous valid value or reset to default
    let current_value = element.property::<Option<String>>("buffer-mode");
    assert!(
        current_value.is_some(),
        "Property should have some value even after invalid input"
    );
}

#[test]
#[serial]
fn test_rtcp_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2").build().unwrap();

    // Test that RTCP properties exist and have correct defaults
    assert!(element.property_type("do-rtcp").is_some());
    assert!(element.property_type("do-retransmission").is_some());
    assert!(element.property_type("max-rtcp-rtp-time-diff").is_some());

    // Test default values (matching original rtspsrc)
    assert_eq!(element.property::<bool>("do-rtcp"), true);
    assert_eq!(element.property::<bool>("do-retransmission"), true);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), -1);

    // Test setting boolean RTCP properties
    element.set_property("do-rtcp", false);
    assert_eq!(element.property::<bool>("do-rtcp"), false);

    element.set_property("do-retransmission", false);
    assert_eq!(element.property::<bool>("do-retransmission"), false);

    // Test setting max-rtcp-rtp-time-diff with different values
    element.set_property("max-rtcp-rtp-time-diff", 1000i32);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), 1000);

    // Test disabled value (-1)
    element.set_property("max-rtcp-rtp-time-diff", -1i32);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), -1);

    // Test maximum value
    element.set_property("max-rtcp-rtp-time-diff", i32::MAX);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), i32::MAX);
}

#[test]
#[serial]
fn test_property_setting() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set and verify location property
    let test_url = "rtsp://example.com/test";
    element.set_property("location", test_url);
    let location: Option<String> = element.property("location");
    assert_eq!(location, Some(test_url.to_string()));

    // Set and verify port-start property
    element.set_property("port-start", 5000u32);
    let port_start: u32 = element.property("port-start");
    assert_eq!(port_start, 5000);

    // Set and verify timeout property
    element.set_property("timeout", 30_000_000u64);
    let timeout: u64 = element.property("timeout");
    assert_eq!(timeout, 30_000_000);

    // Set and verify receive-mtu property
    element.set_property("receive-mtu", 2000u32);
    let receive_mtu: u32 = element.property("receive-mtu");
    assert_eq!(receive_mtu, 2000);
}

#[test]
#[serial]
fn test_state_changes() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid location first
    element.set_property("location", "rtsp://example.com/test");

    // NULL -> READY state change should succeed with a valid location
    let result = element.set_state(gst::State::Ready);
    assert!(
        result.is_ok(),
        "Failed to change state to READY: {:?}",
        result
    );

    // Wait for state change to complete
    let (result, state, _pending) = element.state(gst::ClockTime::from_seconds(1));
    assert_eq!(result, Ok(gst::StateChangeSuccess::Success));
    assert_eq!(state, gst::State::Ready);

    // Going back to NULL should also work
    let result = element.set_state(gst::State::Null);
    assert!(matches!(result, Ok(_)));
}

#[test]
#[serial]
fn test_state_change_without_location_fails() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to go to PAUSED without setting location - should fail
    let result = element.set_state(gst::State::Paused);
    assert!(matches!(result, Err(_)));

    // Wait to confirm element is in NULL state
    let (_result, state, _pending) = element.state(gst::ClockTime::from_seconds(1));
    assert_eq!(state, gst::State::Null);
}

#[test]
#[serial]
fn test_signal_connection() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Currently rtspsrc2 doesn't have custom signals
    // This test is a placeholder for when signals are added
    // Check that the element is a proper GstBin
    assert!(element.is::<gst::Bin>());
}

#[test]
#[serial]
fn test_protocols_property_parsing() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test various protocol combinations
    // The property accepts comma-separated values
    let test_cases = vec![
        ("tcp", "tcp"),
        ("udp", "udp"),
        ("udp-mcast", "udp-mcast"),
        ("tcp,udp", "tcp,udp"),
        ("udp,tcp", "udp,tcp"),
        ("udp-mcast,udp,tcp", "udp-mcast,udp,tcp"),
    ];

    for (input, expected) in test_cases {
        element.set_property("protocols", input);
        let protocols: String = element.property("protocols");
        assert_eq!(protocols, expected, "Failed for input: {}", input);
    }
}

#[test]
#[serial]
fn test_invalid_location_handling() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set various invalid locations
    let invalid_urls = vec![
        "",                   // Empty string
        "not-a-url",          // Invalid scheme
        "http://example.com", // Wrong protocol
        "rtsp://",            // Missing host
    ];

    for invalid_url in invalid_urls {
        element.set_property("location", invalid_url);

        // Try to go to PAUSED - should fail for invalid URLs
        let result = element.set_state(gst::State::Paused);
        assert!(
            matches!(result, Err(_)),
            "Expected state change to fail for URL: {}",
            invalid_url
        );

        // Reset to NULL for next test
        element.set_state(gst::State::Null).unwrap();
    }
}

// Test harness for future mock server tests (preparation for PRP-02)
mod mock_server_prep {
    use super::*;

    #[test]
    #[serial]
    fn test_harness_preparation() {
        init();

        // This test prepares for future mock server implementation
        // For now, just verify we can create elements that will be used in testing
        let src = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2 element");

        let sink = gst::ElementFactory::make("fakesink")
            .build()
            .expect("Failed to create fakesink element");

        // Create a pipeline for future testing
        let pipeline = gst::Pipeline::new();
        pipeline.add(&src).unwrap();
        pipeline.add(&sink).unwrap();

        // Note: We can't link them directly since rtspsrc2 has dynamic pads
        // This will be handled properly with mock server in PRP-02

        assert_eq!(pipeline.children().len(), 2);
    }
}
