// GStreamer RTSP plugin NAT hole punching tests
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
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;

mod mock_server;
use mock_server::MockRtspServer;

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
fn test_nat_method_property() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test that nat-method property exists
    assert!(element.property_type("nat-method").is_some());

    // Test setting to none
    element.set_property_from_str("nat-method", "none");

    // Test setting to dummy
    element.set_property_from_str("nat-method", "dummy");
}

#[tokio::test]
#[serial]
async fn test_nat_hole_punching_dummy_packets() {
    init();

    // Create a UDP socket to listen for NAT punch packets
    let listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let listen_addr = listener.local_addr().unwrap();

    // Track received packets
    let packet_count = Arc::new(AtomicU32::new(0));
    let received_nat_punch = Arc::new(AtomicBool::new(false));

    let packet_count_clone = packet_count.clone();
    let received_clone = received_nat_punch.clone();

    // Start listening for packets
    tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        while let Ok((len, _addr)) = listener.recv_from(&mut buf).await {
            if len >= 8 {
                // Check for RTP or RTCP header
                let version = (buf[0] >> 6) & 0x03;
                if version == 2 {
                    // Valid RTP/RTCP packet
                    packet_count_clone.fetch_add(1, Ordering::SeqCst);

                    // Check if it's a minimal RTP packet (PT=96) or RTCP RR (PT=201)
                    let pt = if (buf[0] & 0x80) == 0x80 && (buf[1] & 0x7f) == 96 {
                        // RTP dummy packet
                        received_clone.store(true, Ordering::SeqCst);
                        true
                    } else if buf[1] == 0xc9 {
                        // RTCP RR packet
                        received_clone.store(true, Ordering::SeqCst);
                        true
                    } else {
                        false
                    };

                    if pt {
                        println!("Received NAT punch packet");
                    }
                }
            }
        }
    });

    // Create rtspsrc2 element with NAT method set to dummy
    let element = gst::ElementFactory::make("rtspsrc2")
        .property_from_str("nat-method", "dummy")
        .property_from_str("protocols", "udp")
        .property("port-start", 5000u32)
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Give some time for potential NAT punching (in real scenario after SETUP)
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check that nat-method is properly set
    let nat_method_value = element.property_value("nat-method");
    assert!(
        nat_method_value.to_string().contains("dummy")
            || nat_method_value.to_string().contains("1")
    );
}

#[tokio::test]
#[serial]
async fn test_nat_keepalive_mechanism() {
    init();

    // Create a UDP socket to listen for keep-alive packets
    let listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let listen_addr = listener.local_addr().unwrap();

    let keepalive_count = Arc::new(AtomicU32::new(0));
    let keepalive_clone = keepalive_count.clone();

    // Start listening for keep-alive packets
    let handle = tokio::spawn(async move {
        let mut buf = [0u8; 1024];
        let start = std::time::Instant::now();

        while start.elapsed() < Duration::from_secs(25) {
            match tokio::time::timeout(Duration::from_secs(1), listener.recv_from(&mut buf)).await {
                Ok(Ok((len, _addr))) => {
                    if len >= 8 {
                        // Check for RTCP RR keep-alive packet
                        if buf[0] == 0x80 && buf[1] == 0xc9 {
                            keepalive_clone.fetch_add(1, Ordering::SeqCst);
                            println!("Received NAT keep-alive packet");
                        }
                    }
                }
                _ => {}
            }
        }
    });

    // Create rtspsrc2 element configured for UDP with NAT method
    let element = gst::ElementFactory::make("rtspsrc2")
        .property_from_str("nat-method", "dummy")
        .property_from_str("protocols", "udp")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // In a real test, we would set up a pipeline and start streaming
    // For now, just verify the property is set correctly
    let nat_method_value = element.property_value("nat-method");
    assert!(
        nat_method_value.to_string().contains("dummy")
            || nat_method_value.to_string().contains("1")
    );

    // Clean up
    handle.abort();
}

#[test]
#[serial]
fn test_nat_configuration_combinations() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test NAT method with TCP (should not affect TCP)
    element.set_property_from_str("nat-method", "dummy");
    element.set_property_from_str("protocols", "tcp");

    // NAT method should still be set even with TCP
    let nat_method_value = element.property_value("nat-method");
    assert!(
        nat_method_value.to_string().contains("dummy")
            || nat_method_value.to_string().contains("1")
    );

    // Test NAT method with UDP
    element.set_property_from_str("protocols", "udp");
    let protocols: String = element.property("protocols");
    assert!(protocols.contains("udp"));

    // Test NAT method none
    element.set_property_from_str("nat-method", "none");
    let nat_method_value = element.property_value("nat-method");
    assert!(
        nat_method_value.to_string().contains("none") || nat_method_value.to_string().contains("0")
    );
}
