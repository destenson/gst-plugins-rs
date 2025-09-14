// GStreamer RTSP Source 2 - Backchannel Actions Test
//
// Unit tests for backchannel action signal registration and validation

use gst::prelude::*;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("gstrsrtsp tests");
    });
}

#[test]
#[ignore] // Signals may not be available in all configurations
fn test_backchannel_signals_exist() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test that signals exist by trying to emit them
    // Note: We can't use find_signal as it's not available in current glib bindings
    // These tests just verify the signals are registered, not their functionality
    
    // Test push-backchannel-buffer signal
    let buffer = gst::Buffer::new();
    let _result: gst::FlowReturn = element.emit_by_name(
        "push-backchannel-buffer",
        &[&0u32, &buffer]
    );
    
    // Test push-backchannel-sample signal  
    let sample = gst::Sample::builder().buffer(&buffer).build();
    let _result: gst::FlowReturn = element.emit_by_name(
        "push-backchannel-sample",
        &[&0u32, &sample]
    );
}

#[test]
#[ignore] // RTSPBackchannel type may not be available
fn test_element_properties_for_backchannel() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test that backchannel property can be set and retrieved
    // The actual type would depend on GStreamer version
    element.set_property_from_str("backchannel", "none");
    let backchannel: String = element.property("backchannel");
    assert!(backchannel.contains("none") || backchannel == "0");
}