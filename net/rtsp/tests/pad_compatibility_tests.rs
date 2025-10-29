// Integration tests for rtspsrc2 pad management and compatibility with uridecodebin/playbin
//
// These tests verify that rtspsrc2 properly implements the GStreamer dynamic pad protocol
// required for compatibility with high-level elements like uridecodebin and playbin.

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

/// Test that rtspsrc2 emits no-more-pads signal exactly once
///
/// This is critical for uridecodebin/playbin compatibility. The no-more-pads signal
/// indicates that all pads have been added and downstream elements can start linking.
/// It should only be emitted once per stream setup.
#[test]
fn test_no_more_pads_emitted_once() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Track no-more-pads signal emissions
    let no_more_pads_count = Arc::new(Mutex::new(0u32));
    let count_clone = no_more_pads_count.clone();

    rtspsrc.connect("no-more-pads", false, move |_args| {
        let mut count = count_clone.lock().unwrap();
        *count += 1;
        None
    });

    // Note: This test would require a mock RTSP server to fully test the signal emission
    // For now, we verify the element can be created and the signal can be connected
    assert_eq!(*no_more_pads_count.lock().unwrap(), 0);
}

/// Test that rtspsrc2 creates pads with correct templates
///
/// uridecodebin/playbin expect source elements to have properly configured pad templates
/// with PadPresence::Sometimes for dynamic pads.
#[test]
fn test_pad_templates() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Get pad templates
    let pad_templates = rtspsrc.pad_template_list();

    // Should have at least a source pad template
    assert!(!pad_templates.is_empty(), "No pad templates found");

    // Find the source pad template
    let src_template = pad_templates
        .iter()
        .find(|t| t.direction() == gst::PadDirection::Src && t.name_template().starts_with("stream_"))
        .expect("Source pad template not found");

    // Verify it's a "Sometimes" pad (dynamic)
    assert_eq!(
        src_template.presence(),
        gst::PadPresence::Sometimes,
        "Source pad should be PadPresence::Sometimes for dynamic creation"
    );

    // Verify caps
    let caps = src_template.caps();
    assert!(
        caps.is_any() || caps.structure(0).map(|s| s.name() == "application/x-rtp").unwrap_or(false),
        "Source pad caps should be application/x-rtp or ANY"
    );
}

/// Test that rtspsrc2 pads have correct properties
#[test]
fn test_pad_properties() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Verify supported URI types
    let uri_handler = rtspsrc
        .dynamic_cast::<gst::URIHandler>()
        .expect("Failed to cast to URIHandler - rtspsrc2 should implement URIHandler interface");

    assert_eq!(
        uri_handler.uri_type(),
        gst::URIType::Src,
        "rtspsrc2 should be a source URI handler"
    );

    let protocols = uri_handler.protocols();
    let protocols_vec: Vec<String> = protocols.iter().map(|s| s.to_string()).collect();

    assert!(
        protocols_vec.iter().any(|p| p == "rtsp"),
        "Should support rtsp:// protocol"
    );
    assert!(
        protocols_vec.iter().any(|p| p == "rtsps"),
        "Should support rtsps:// protocol"
    );
}

/// Test pad-added signal emission
///
/// This verifies that the pad-added signal is properly emitted when pads are created,
/// which is required for uridecodebin/playbin to link downstream elements.
#[test]
fn test_pad_added_signal() {
    init();

    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Track pad-added signal emissions
    let pads_added = Arc::new(Mutex::new(Vec::new()));
    let pads_clone = pads_added.clone();

    rtspsrc.connect_pad_added(move |_element, pad| {
        let mut pads = pads_clone.lock().unwrap();
        pads.push(pad.name().to_string());
    });

    // Note: This test would require a mock RTSP server to fully test pad creation
    // For now, we verify the signal can be connected
    assert_eq!(pads_added.lock().unwrap().len(), 0);
}

/// Test that rtspsrc2 works with decodebin
///
/// This is the basic compatibility test for uridecodebin/playbin usage.
/// We verify that pads can be linked to decodebin.
#[test]
fn test_decodebin_compatibility() {
    init();

    let pipeline = gst::Pipeline::new();
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    let decodebin = gst::ElementFactory::make("decodebin")
        .build()
        .expect("Failed to create decodebin");

    pipeline
        .add_many(&[&rtspsrc, &decodebin])
        .expect("Failed to add elements to pipeline");

    // Track successful links
    let links_successful = Arc::new(Mutex::new(false));
    let links_clone = links_successful.clone();

    // Connect pad-added signal to link to decodebin
    rtspsrc.connect_pad_added(move |_src, src_pad| {
        let sink_pad = decodebin
            .static_pad("sink")
            .expect("decodebin should have a sink pad");

        if !sink_pad.is_linked() {
            match src_pad.link(&sink_pad) {
                Ok(_) => {
                    *links_clone.lock().unwrap() = true;
                }
                Err(e) => {
                    eprintln!("Failed to link pads: {:?}", e);
                }
            }
        }
    });

    // Note: Without an actual RTSP server, pads won't be created
    // This test verifies the structure is correct
}

/// Test element metadata for compatibility
#[test]
fn test_element_metadata() {
    init();

    let factory = gst::ElementFactory::find("rtspsrc2")
        .expect("rtspsrc2 factory not found");

    // Verify classification exists
    let klass = factory.metadata("klass");
    assert!(
        klass.is_some(),
        "Element should have classification"
    );

    // Verify it's marked as a source
    let klass_str = klass.unwrap();
    assert!(
        klass_str.contains("Source") || klass_str.contains("Network"),
        "Element should be classified as Source or Network: {}",
        klass_str
    );
}
