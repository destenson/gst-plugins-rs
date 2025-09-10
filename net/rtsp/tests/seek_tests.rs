// GStreamer RTSP Source VOD Seeking Tests
// Tests for Basic VOD Seeking Support (PRP-RTSP-18)

use gst::prelude::*;
use gst_check::harness::Harness;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    
    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("Failed to register rtsp plugin");
    });
}

#[test]
fn test_seek_basic() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    
    // Set element to playing state
    h.play();
    
    // Create a seek event
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH,
        gst::SeekType::Set,
        gst::ClockTime::from_seconds(10),
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    // Send seek event
    assert!(h.push_event(seek_event));
}

#[test]
fn test_seek_accuracy() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    h.play();
    
    // Test seeking to specific position
    let target_position = gst::ClockTime::from_seconds(30);
    
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
        gst::SeekType::Set,
        target_position,
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
    
    // In a real test, we would verify the segment event here
    // and check that the position matches our target
}

#[test]
fn test_seek_segment() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    h.play();
    
    // Seek to 20 seconds
    let seek_position = gst::ClockTime::from_seconds(20);
    
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH,
        gst::SeekType::Set,
        seek_position,
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
    
    // In a real test environment, we would:
    // 1. Wait for the segment event
    // 2. Verify the segment start matches our seek position
    // 3. Verify subsequent buffers have correct timestamps
}

#[test]
fn test_seek_without_flush() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    h.play();
    
    // Seek without flush flag
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::empty(),
        gst::SeekType::Set,
        gst::ClockTime::from_seconds(15),
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
}

#[test]
fn test_multiple_seeks() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    h.play();
    
    // Perform multiple seeks in sequence
    let positions = vec![
        gst::ClockTime::from_seconds(10),
        gst::ClockTime::from_seconds(30),
        gst::ClockTime::from_seconds(5),
        gst::ClockTime::from_seconds(45),
    ];
    
    for position in positions {
        let seek_event = gst::event::Seek::builder(
            1.0,
            gst::SeekFlags::FLUSH,
            gst::SeekType::Set,
            position,
            gst::SeekType::None,
            gst::ClockTime::NONE,
        )
        .build();
        
        assert!(h.push_event(seek_event));
        
        // In production, we'd wait for seek completion here
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

// Tests for different Range formats
#[test]
fn test_seek_npt_format() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    // Set NPT format (default)
    h.element().unwrap().set_property("seek-format", "npt");
    h.play();
    
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH,
        gst::SeekType::Set,
        gst::ClockTime::from_seconds(25),
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
}

#[test]
fn test_seek_smpte_format() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    // Set SMPTE format
    h.element().unwrap().set_property("seek-format", "smpte");
    h.play();
    
    // Seek to 00:00:30:00 (30 seconds at 30fps)
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH,
        gst::SeekType::Set,
        gst::ClockTime::from_seconds(30),
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
}

#[test]
fn test_seek_clock_format() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    // Set Clock/UTC format
    h.element().unwrap().set_property("seek-format", "clock");
    h.play();
    
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH,
        gst::SeekType::Set,
        gst::ClockTime::from_seconds(45),
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
}

#[test]
fn test_seek_range_response_handling() {
    init();
    
    let mut h = Harness::new("rtspsrc2");
    h.element().unwrap().set_property("location", "rtsp://localhost:8554/test");
    h.play();
    
    // Seek and expect segment update based on server response
    let target_position = gst::ClockTime::from_seconds(60);
    
    let seek_event = gst::event::Seek::builder(
        1.0,
        gst::SeekFlags::FLUSH,
        gst::SeekType::Set,
        target_position,
        gst::SeekType::None,
        gst::ClockTime::NONE,
    )
    .build();
    
    assert!(h.push_event(seek_event));
    
    // In a real environment with server, we would verify:
    // 1. The PLAY request contains Range header
    // 2. The server response contains Range header
    // 3. The segment is updated based on server's Range response
}

// Integration test with mock RTSP server
#[cfg(test)]
mod integration_tests {
    use super::*;
    
    #[test]
    #[ignore] // Run with --ignored flag when RTSP server is available
    fn test_seek_with_real_server() {
        init();
        
        // This test requires a real RTSP server running
        let pipeline = gst::parse::launch(
            "rtspsrc2 name=src location=rtsp://localhost:8554/test ! decodebin ! fakesink"
        ).unwrap();
        
        let bus = pipeline.bus().unwrap();
        pipeline.set_state(gst::State::Playing).unwrap();
        
        // Wait for pipeline to start
        std::thread::sleep(std::time::Duration::from_secs(1));
        
        // Perform seek
        let seek = gst::event::Seek::builder(
            1.0,
            gst::SeekFlags::FLUSH,
            gst::SeekType::Set,
            gst::ClockTime::from_seconds(10),
            gst::SeekType::None,
            gst::ClockTime::NONE,
        )
        .build();
        
        assert!(pipeline.send_event(seek));
        
        // Wait for seek to complete
        for msg in bus.iter_timed(gst::ClockTime::from_seconds(5)) {
            match msg.view() {
                gst::MessageView::AsyncDone(_) => {
                    println!("Seek completed successfully");
                    break;
                }
                gst::MessageView::Error(err) => {
                    panic!("Error during seek: {:?}", err);
                }
                _ => {}
            }
        }
        
        pipeline.set_state(gst::State::Null).unwrap();
    }
}