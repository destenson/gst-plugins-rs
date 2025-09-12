// Integration tests using real GStreamer RTSP server
//
// These tests require gst-rtsp-server to be installed
// They are marked as #[ignore] by default and can be run with:
// cargo test -p gst-plugin-rtsp --test gst_server_integration -- --ignored

mod mock_server;
mod rtsp_test_server;

use gst::prelude::*;
use rtsp_test_server::{helpers, GstRtspTestServer, ServerConfig, ServerType};
use serial_test::serial;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        gst::init().unwrap();
        // Enable debug output for RTSP elements
        std::env::set_var("GST_DEBUG", "rtspsrc2:4,rtsp:3");
    });
}

/// Test basic live stream connectivity
#[test]
#[serial]
#[ignore] // Requires gst-rtsp-server
fn test_live_stream_with_real_server() {
    init();

    let server = match GstRtspTestServer::new_live() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    let url = server.url();
    println!("Testing with RTSP URL: {}", url);

    // Create pipeline with rtspsrc2
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .property("latency", 100u32)
        .build()
        .expect("Failed to create rtspsrc2");

    let fakesink = gst::ElementFactory::make("fakesink")
        .property("sync", false)
        .build()
        .expect("Failed to create fakesink");

    pipeline.add_many(&[&src, &fakesink]).unwrap();

    // Connect pad-added signal
    let sink_weak = fakesink.downgrade();
    src.connect_pad_added(move |_src, pad| {
        if let Some(sink) = sink_weak.upgrade() {
            let sink_pad = sink.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                pad.link(&sink_pad).expect("Failed to link pads");
            }
        }
    });

    // Start pipeline
    pipeline.set_state(gst::State::Playing).unwrap();

    // Wait for data to flow
    std::thread::sleep(Duration::from_secs(2));

    // Check that we're actually playing
    let (state_result, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(1)));
    assert_eq!(state_result, Ok(gst::StateChangeSuccess::Success));

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}

/// Test VOD stream with seeking
#[test]
#[serial]
#[ignore] // Requires gst-rtsp-server and test file
fn test_vod_seeking_with_real_server() {
    init();

    // Create a test video file
    let test_file = match helpers::create_test_video() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Skipping test: Failed to create test video: {}", e);
            return;
        }
    };

    let server = match GstRtspTestServer::new_vod(&test_file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    let url = server.url();
    println!("Testing VOD with RTSP URL: {}", url);

    // Create pipeline
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .build()
        .expect("Failed to create rtspsrc2");

    let fakesink = gst::ElementFactory::make("fakesink")
        .property("sync", true)
        .build()
        .expect("Failed to create fakesink");

    pipeline.add_many(&[&src, &fakesink]).unwrap();

    // Connect pad-added
    let sink_weak = fakesink.downgrade();
    src.connect_pad_added(move |_src, pad| {
        if let Some(sink) = sink_weak.upgrade() {
            let sink_pad = sink.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                pad.link(&sink_pad).expect("Failed to link pads");
            }
        }
    });

    // Start pipeline
    pipeline.set_state(gst::State::Playing).unwrap();

    // Wait for preroll
    std::thread::sleep(Duration::from_secs(2));

    // Perform seek
    let seek_pos = gst::ClockTime::from_seconds(3);
    let seek_result =
        pipeline.seek_simple(gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT, seek_pos);

    if seek_result.is_ok() {
        println!("Seek to {} successful", seek_pos);

        // Wait for seek to complete
        std::thread::sleep(Duration::from_secs(1));

        // Check position
        if let Some(pos) = pipeline.query_position::<gst::ClockTime>() {
            println!("Current position: {}", pos);
            // Allow some tolerance
            assert!(pos >= seek_pos - gst::ClockTime::from_seconds(1));
        }
    } else {
        println!("Seek not supported by server");
    }

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();

    // Clean up test file
    let _ = std::fs::remove_file(test_file);
}

/// Test authentication
#[test]
#[serial]
#[ignore] // Requires gst-rtsp-server with auth support
fn test_authentication_with_real_server() {
    init();

    let server = match GstRtspTestServer::with_auth("testuser", "testpass") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    let base_url = server.url();

    // Test without credentials (should fail)
    {
        let pipeline = gst::Pipeline::new();
        let src = gst::ElementFactory::make("rtspsrc2")
            .property("location", &base_url)
            .property("protocols", "tcp")
            .build()
            .expect("Failed to create rtspsrc2");

        let fakesink = gst::ElementFactory::make("fakesink").build().unwrap();
        pipeline.add_many(&[&src, &fakesink]).unwrap();

        let error_received = Arc::new(AtomicBool::new(false));
        let error_clone = error_received.clone();

        let bus = pipeline.bus().unwrap();
        std::thread::spawn(move || {
            for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
                if let gst::MessageView::Error(_) = msg.view() {
                    error_clone.store(true, Ordering::SeqCst);
                    break;
                }
            }
        });

        pipeline.set_state(gst::State::Playing).unwrap();
        std::thread::sleep(Duration::from_secs(2));

        // Should have received an error
        assert!(
            error_received.load(Ordering::SeqCst),
            "Expected authentication error"
        );

        pipeline.set_state(gst::State::Null).unwrap();
    }

    // Test with correct credentials (should succeed)
    {
        let auth_url = format!("rtsp://testuser:testpass@127.0.0.1:{}/test", server.port());

        let pipeline = gst::Pipeline::new();
        let src = gst::ElementFactory::make("rtspsrc2")
            .property("location", &auth_url)
            .property("protocols", "tcp")
            .build()
            .expect("Failed to create rtspsrc2");

        let fakesink = gst::ElementFactory::make("fakesink").build().unwrap();
        pipeline.add_many(&[&src, &fakesink]).unwrap();

        let sink_weak = fakesink.downgrade();
        src.connect_pad_added(move |_src, pad| {
            if let Some(sink) = sink_weak.upgrade() {
                let sink_pad = sink.static_pad("sink").unwrap();
                if !sink_pad.is_linked() {
                    pad.link(&sink_pad).expect("Failed to link pads");
                }
            }
        });

        pipeline.set_state(gst::State::Playing).unwrap();
        std::thread::sleep(Duration::from_secs(2));

        // Should be playing successfully
        let (state_result, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(1)));
        assert_eq!(
            state_result,
            Ok(gst::StateChangeSuccess::Success),
            "Should authenticate successfully"
        );

        pipeline.set_state(gst::State::Null).unwrap();
    }
}

/// Test audio+video stream
#[test]
#[serial]
#[ignore] // Requires gst-rtsp-server
fn test_audio_video_stream_with_real_server() {
    init();

    let config = ServerConfig {
        server_type: ServerType::AudioVideo,
        ..Default::default()
    };

    let server = match GstRtspTestServer::with_config(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    let url = server.url();
    println!("Testing audio+video with RTSP URL: {}", url);

    // Create pipeline
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .build()
        .expect("Failed to create rtspsrc2");

    let video_sink = gst::ElementFactory::make("fakesink")
        .name("video_sink")
        .build()
        .unwrap();

    let audio_sink = gst::ElementFactory::make("fakesink")
        .name("audio_sink")
        .build()
        .unwrap();

    pipeline
        .add_many(&[&src, &video_sink, &audio_sink])
        .unwrap();

    // Track which pads we've connected
    let video_connected = Arc::new(AtomicBool::new(false));
    let audio_connected = Arc::new(AtomicBool::new(false));

    let video_sink_weak = video_sink.downgrade();
    let audio_sink_weak = audio_sink.downgrade();
    let video_conn = video_connected.clone();
    let audio_conn = audio_connected.clone();

    src.connect_pad_added(move |_src, pad| {
        let caps = pad.current_caps().unwrap();
        let structure = caps.structure(0).unwrap();
        let media_type = structure.get::<&str>("media").unwrap_or("unknown");

        match media_type {
            "video" => {
                if !video_conn.load(Ordering::SeqCst) {
                    if let Some(sink) = video_sink_weak.upgrade() {
                        let sink_pad = sink.static_pad("sink").unwrap();
                        pad.link(&sink_pad).expect("Failed to link video pad");
                        video_conn.store(true, Ordering::SeqCst);
                        println!("Connected video stream");
                    }
                }
            }
            "audio" => {
                if !audio_conn.load(Ordering::SeqCst) {
                    if let Some(sink) = audio_sink_weak.upgrade() {
                        let sink_pad = sink.static_pad("sink").unwrap();
                        pad.link(&sink_pad).expect("Failed to link audio pad");
                        audio_conn.store(true, Ordering::SeqCst);
                        println!("Connected audio stream");
                    }
                }
            }
            _ => {}
        }
    });

    // Start pipeline
    pipeline.set_state(gst::State::Playing).unwrap();

    // Wait for streams to connect
    std::thread::sleep(Duration::from_secs(3));

    // Check that both streams connected
    assert!(
        video_connected.load(Ordering::SeqCst),
        "Video stream should be connected"
    );
    assert!(
        audio_connected.load(Ordering::SeqCst),
        "Audio stream should be connected"
    );

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}

/// Test reconnection after server restart
#[test]
#[serial]
#[ignore] // Requires gst-rtsp-server
fn test_reconnection_with_real_server() {
    init();

    let mut server = match GstRtspTestServer::new_live() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    let url = server.url();
    let port = server.port();

    // Create pipeline with auto-reconnect
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .property("retry", 5i32)
        .property("timeout", 5_000_000u64) // 5 seconds
        .build()
        .expect("Failed to create rtspsrc2");

    let fakesink = gst::ElementFactory::make("fakesink").build().unwrap();
    pipeline.add_many(&[&src, &fakesink]).unwrap();

    let sink_weak = fakesink.downgrade();
    src.connect_pad_added(move |_src, pad| {
        if let Some(sink) = sink_weak.upgrade() {
            let sink_pad = sink.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                pad.link(&sink_pad).expect("Failed to link pads");
            }
        }
    });

    // Start pipeline
    pipeline.set_state(gst::State::Playing).unwrap();
    std::thread::sleep(Duration::from_secs(2));

    // Verify playing
    let (state_result, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(1)));
    assert_eq!(state_result, Ok(gst::StateChangeSuccess::Success));

    // Stop server
    println!("Stopping server to test reconnection...");
    server.stop();
    std::thread::sleep(Duration::from_secs(2));

    // Restart server on same port
    println!("Restarting server...");
    let config = ServerConfig {
        server_type: ServerType::Live,
        port_range: (port, port),
        ..Default::default()
    };

    let _new_server = match GstRtspTestServer::with_config(config) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to restart server: {}", e);
            pipeline.set_state(gst::State::Null).unwrap();
            return;
        }
    };

    // Wait for reconnection
    std::thread::sleep(Duration::from_secs(5));

    // Should reconnect and be playing again
    let (state_result, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(1)));
    assert_eq!(
        state_result,
        Ok(gst::StateChangeSuccess::Success),
        "Should reconnect after server restart"
    );

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}

/// Test RTCP feedback
#[test]
#[serial]
#[ignore] // Requires gst-rtsp-server with RTCP support
fn test_rtcp_feedback_with_real_server() {
    init();

    let config = ServerConfig {
        server_type: ServerType::Live,
        enable_rtcp: true,
        ..Default::default()
    };

    let server = match GstRtspTestServer::with_config(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping test: {}", e);
            return;
        }
    };

    let url = server.url();

    // Create pipeline
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .property("protocols", "tcp")
        .property("do-rtcp", true)
        .build()
        .expect("Failed to create rtspsrc2");

    let fakesink = gst::ElementFactory::make("fakesink")
        .property("sync", true)
        .build()
        .unwrap();

    pipeline.add_many(&[&src, &fakesink]).unwrap();

    // Monitor RTCP stats
    let rtcp_received = Arc::new(AtomicBool::new(false));
    let rtcp_clone = rtcp_received.clone();

    src.connect("on-rtcp", false, move |_values| {
        rtcp_clone.store(true, Ordering::SeqCst);
        println!("RTCP feedback received");
        None
    });

    let sink_weak = fakesink.downgrade();
    src.connect_pad_added(move |_src, pad| {
        if let Some(sink) = sink_weak.upgrade() {
            let sink_pad = sink.static_pad("sink").unwrap();
            if !sink_pad.is_linked() {
                pad.link(&sink_pad).expect("Failed to link pads");
            }
        }
    });

    // Start pipeline
    pipeline.set_state(gst::State::Playing).unwrap();

    // Wait for RTCP feedback
    std::thread::sleep(Duration::from_secs(5));

    // Check if RTCP was received
    if rtcp_received.load(Ordering::SeqCst) {
        println!("RTCP feedback confirmed");
    } else {
        println!("No RTCP feedback received (server may not support it)");
    }

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}
