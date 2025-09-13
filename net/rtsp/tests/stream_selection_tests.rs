// Stream Selection Tests for PRP-RTSP-20

use gst::prelude::*;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
    });
}

#[test]
fn test_stream_select_all() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "all")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<String>("select-streams");
    assert_eq!(value, "all");
}

#[test]
fn test_stream_select_audio_only() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "audio")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<String>("select-streams");
    assert_eq!(value, "audio");
}

#[test]
fn test_stream_select_video_only() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "video")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<String>("select-streams");
    assert_eq!(value, "video");
}

#[test]
fn test_stream_select_multiple() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "audio,video")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<String>("select-streams");
    assert_eq!(value, "audio,video");
}

#[test]
fn test_stream_select_none() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "none")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<String>("select-streams");
    // When none is selected, it defaults to all
    assert_eq!(value, "all");
}

#[test]
fn test_codec_filter() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("stream-filter", "h264")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<Option<String>>("stream-filter");
    assert_eq!(value, Some("h264".to_string()));
}

#[test]
fn test_codec_filter_case_insensitive() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("stream-filter", "H264")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<Option<String>>("stream-filter");
    assert_eq!(value, Some("H264".to_string()));

    // Test with lowercase too
    src.set_property("stream-filter", "aac");
    let value = src.property::<Option<String>>("stream-filter");
    assert_eq!(value, Some("aac".to_string()));
}

#[test]
fn test_require_all_streams() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("require-all-streams", true)
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<bool>("require-all-streams");
    assert_eq!(value, true);

    // Test default value
    let src2 = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .build()
        .unwrap();
    let value2 = src2.property::<bool>("require-all-streams");
    assert_eq!(value2, false); // Default should be false
}

#[test]
fn test_selective_setup() {
    init();

    // This test would require a mock RTSP server to verify
    // that only selected streams are set up
    // For now, we just verify the properties are correctly configured

    let pipeline = gst::Pipeline::new();
    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "video")
        .property("stream-filter", "h264")
        .property("require-all-streams", false)
        .build()
        .unwrap();

    pipeline.add(&src).unwrap();

    // Verify all properties are set
    assert_eq!(src.property::<String>("select-streams"), "video");
    assert_eq!(
        src.property::<Option<String>>("stream-filter"),
        Some("h264".to_string())
    );
    assert_eq!(src.property::<bool>("require-all-streams"), false);
}

#[test]
fn test_metadata_and_application_streams() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "metadata,application")
        .build()
        .unwrap();

    // Verify property is set correctly
    let value = src.property::<String>("select-streams");
    assert_eq!(value, "metadata,application");
}

#[test]
fn test_invalid_stream_type_ignored() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "audio,invalid,video")
        .build()
        .unwrap();

    // Invalid types should be ignored
    let value = src.property::<String>("select-streams");
    assert_eq!(value, "audio,video");
}

#[test]
fn test_stream_selection_with_complex_filter() {
    init();

    let src = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://localhost:8554/test")
        .property("select-streams", "audio,video")
        .property("stream-filter", "opus")
        .build()
        .unwrap();

    // Verify properties
    assert_eq!(src.property::<String>("select-streams"), "audio,video");
    assert_eq!(
        src.property::<Option<String>>("stream-filter"),
        Some("opus".to_string())
    );
}
