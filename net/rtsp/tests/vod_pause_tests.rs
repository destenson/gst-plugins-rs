// VOD PAUSE Support Tests

use gst::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
    });
}

#[test]
fn test_vod_pause_support() {
    init();

    // Create a simple pipeline with rtspsrc2
    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("protocols", "tcp")
        .build()
        .unwrap();

    let fakesink = gst::ElementFactory::make("fakesink").build().unwrap();

    pipeline.add_many([&src, &fakesink]).unwrap();

    // Connect pad-added signal to link dynamically
    src.connect_pad_added(move |_src, pad| {
        let sink_pad = fakesink.static_pad("sink").unwrap();
        if !sink_pad.is_linked() {
            pad.link(&sink_pad).unwrap();
        }
    });

    // Test state transitions for pause
    pipeline.set_state(gst::State::Playing).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Transition to paused state - should send PAUSE command
    pipeline.set_state(gst::State::Paused).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Resume playback - should send PLAY command
    pipeline.set_state(gst::State::Playing).unwrap();
    std::thread::sleep(Duration::from_millis(100));

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}

#[test]
fn test_pause_resume_cycle() {
    init();

    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("protocols", "tcp")
        .build()
        .unwrap();

    let fakesink = gst::ElementFactory::make("fakesink").build().unwrap();

    pipeline.add_many([&src, &fakesink]).unwrap();

    // Connect pad-added signal
    let linked = Arc::new(AtomicBool::new(false));
    let linked_clone = linked.clone();
    src.connect_pad_added(move |_src, pad| {
        let sink_pad = fakesink.static_pad("sink").unwrap();
        if !linked_clone.load(Ordering::SeqCst) {
            pad.link(&sink_pad).unwrap();
            linked_clone.store(true, Ordering::SeqCst);
        }
    });

    // Start playing
    pipeline.set_state(gst::State::Playing).unwrap();
    std::thread::sleep(Duration::from_millis(200));

    // Multiple pause/resume cycles
    for _ in 0..3 {
        pipeline.set_state(gst::State::Paused).unwrap();
        std::thread::sleep(Duration::from_millis(100));

        pipeline.set_state(gst::State::Playing).unwrap();
        std::thread::sleep(Duration::from_millis(100));
    }

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}

#[test]
fn test_long_pause() {
    init();

    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("protocols", "tcp")
        .build()
        .unwrap();

    let fakesink = gst::ElementFactory::make("fakesink").build().unwrap();

    pipeline.add_many([&src, &fakesink]).unwrap();

    // Connect pad-added signal
    src.connect_pad_added(move |_src, pad| {
        let sink_pad = fakesink.static_pad("sink").unwrap();
        if !sink_pad.is_linked() {
            pad.link(&sink_pad).unwrap();
        }
    });

    // Start playing
    pipeline.set_state(gst::State::Playing).unwrap();
    std::thread::sleep(Duration::from_millis(200));

    // Long pause - test session keep-alive during pause
    pipeline.set_state(gst::State::Paused).unwrap();
    std::thread::sleep(Duration::from_secs(2)); // Long pause to test keep-alive

    // Resume should still work after long pause
    pipeline.set_state(gst::State::Playing).unwrap();
    std::thread::sleep(Duration::from_millis(200));

    // Clean up
    pipeline.set_state(gst::State::Null).unwrap();
}
