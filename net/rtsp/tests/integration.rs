#![allow(unused)]
// GStreamer RTSP plugin integration tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

mod mock_server;

use gst::prelude::*;
use serial_test::serial;
use std::time::Duration;

use mock_server::MockRtspServer;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("rtsp plugin registration failed");
    });
}

// NOTE: These tests are disabled because GStreamer cannot be used with tokio runtime
// The mock server uses tokio, but GStreamer elements have their own event loop
// TODO: Rewrite these tests to use std::thread or GStreamer's async mechanisms

/*
#[tokio::test]
#[serial]
async fn test_connect_to_mock_server() {
    init();

    // Start mock server
    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    // Create rtspsrc2 element
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Try to go to READY state
    let result = element.set_state(gst::State::Ready);
    assert!(result.is_ok(), "Failed to set state to READY");

    // Clean up
    element.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_options_describe_flow() {
    init();

    // Start mock server
    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    // Create a simple pipeline
    let pipeline = gst::Pipeline::new();
    
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .build()
        .expect("Failed to create rtspsrc2");

    let fakesink = gst::ElementFactory::make("fakesink")
        .build()
        .expect("Failed to create fakesink");

    pipeline.add(&src).unwrap();
    pipeline.add(&fakesink).unwrap();

    // rtspsrc2 has dynamic pads, so we need to handle pad-added
    let fakesink_weak = fakesink.downgrade();
    src.connect_pad_added(move |_src, pad| {
        if let Some(fakesink) = fakesink_weak.upgrade() {
            let sink_pad = fakesink.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                pad.link(&sink_pad).unwrap();
            }
        }
    });

    // Set to READY first
    let result = pipeline.set_state(gst::State::Ready);
    assert!(result.is_ok());

    // Wait a bit for state change
    let (result, state, _) = pipeline.state(gst::ClockTime::from_seconds(2));
    assert_eq!(result, Ok(gst::StateChangeSuccess::Success));
    assert_eq!(state, gst::State::Ready);

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_custom_sdp() {
    init();

    // Create server with custom SDP
    let mut server = MockRtspServer::new().await;
    
    // Set custom SDP with two video streams
    let custom_sdp = format!(
        "v=0\r\n\
        o=- 0 0 IN IP4 127.0.0.1\r\n\
        s=Custom Test Stream\r\n\
        c=IN IP4 127.0.0.1\r\n\
        t=0 0\r\n\
        m=video 0 RTP/AVP 96\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=control:stream=0\r\n\
        m=audio 0 RTP/AVP 97\r\n\
        a=rtpmap:97 PCMA/8000\r\n\
        a=control:stream=1\r\n"
    );
    
    server.set_sdp(custom_sdp);
    let url = server.url();
    let _handle = server.start().await;

    // Create element and connect
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .build()
        .expect("Failed to create rtspsrc2");

    // Count pads that get added (should be 2 for video and audio)
    let pad_count = std::sync::Arc::new(std::sync::Mutex::new(0));
    let pad_count_clone = pad_count.clone();
    
    element.connect_pad_added(move |_src, _pad| {
        let mut count = pad_count_clone.lock().unwrap();
        *count += 1;
    });

    // Try to go to PAUSED to trigger DESCRIBE
    element.set_state(gst::State::Paused).ok();
    
    // Give it some time to process
    std::thread::sleep(Duration::from_millis(500));

    // Clean up
    element.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_tcp_transport() {
    init();

    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .build()
        .expect("Failed to create rtspsrc2");

    // Should be able to go to READY with TCP
    let result = element.set_state(gst::State::Ready);
    assert!(result.is_ok());

    element.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_udp_transport() {
    init();

    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "udp")
        .build()
        .expect("Failed to create rtspsrc2");

    // Should be able to go to READY with UDP
    let result = element.set_state(gst::State::Ready);
    assert!(result.is_ok());

    element.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_multicast_transport() {
    init();

    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "udp-mcast")
        .build()
        .expect("Failed to create rtspsrc2");

    // Should be able to go to READY with multicast
    let result = element.set_state(gst::State::Ready);
    assert!(result.is_ok());

    element.set_state(gst::State::Null).unwrap();
}

#[tokio::test]
#[serial]
async fn test_teardown_on_stop() {
    init();

    let server = MockRtspServer::new().await;
    let url = server.url();
    let handle = server.start().await;

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .build()
        .expect("Failed to create rtspsrc2");

    // Go through state changes
    element.set_state(gst::State::Ready).unwrap();
    
    // Back to NULL should send TEARDOWN
    element.set_state(gst::State::Null).unwrap();

    // Server should still be running
    handle.shutdown().await;
}

#[tokio::test]
#[serial]
async fn test_server_disconnect_handling() {
    init();

    let server = MockRtspServer::new().await;
    let url = server.url();
    let handle = server.start().await;

    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("timeout", 1_000_000_000u64) // 1 second timeout
        .build()
        .expect("Failed to create rtspsrc2");

    element.set_state(gst::State::Ready).unwrap();
    
    // Shutdown server while element is connected
    handle.shutdown().await;
    
    // Give it time to detect disconnect
    std::thread::sleep(Duration::from_millis(100));
    
    // Element should handle gracefully
    element.set_state(gst::State::Null).unwrap();
}
*/
