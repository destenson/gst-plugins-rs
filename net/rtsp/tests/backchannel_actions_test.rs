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
fn test_backchannel_action_registration() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test push-backchannel-buffer action exists
    let signal_id = element
        .object_class()
        .find_signal("push-backchannel-buffer")
        .expect("push-backchannel-buffer action should be registered");

    let query = signal_id.query();
    assert_eq!(
        query.n_params(),
        2,
        "push-backchannel-buffer should have 2 parameters"
    );
    assert_eq!(
        query.param_types()[0],
        u32::static_type(),
        "First param should be u32 (stream_id)"
    );
    assert_eq!(
        query.param_types()[1],
        gst::Buffer::static_type(),
        "Second param should be GstBuffer"
    );
    assert_eq!(
        query.return_type(),
        gst::FlowReturn::static_type(),
        "Return type should be GstFlowReturn"
    );

    // Test push-backchannel-sample action exists
    let signal_id = element
        .object_class()
        .find_signal("push-backchannel-sample")
        .expect("push-backchannel-sample action should be registered");

    let query = signal_id.query();
    assert_eq!(
        query.n_params(),
        2,
        "push-backchannel-sample should have 2 parameters"
    );
    assert_eq!(
        query.param_types()[0],
        u32::static_type(),
        "First param should be u32 (stream_id)"
    );
    assert_eq!(
        query.param_types()[1],
        gst::Sample::static_type(),
        "Second param should be GstSample"
    );
    assert_eq!(
        query.return_type(),
        gst::FlowReturn::static_type(),
        "Return type should be GstFlowReturn"
    );

    // Test set-mikey-parameter action exists
    let signal_id = element
        .object_class()
        .find_signal("set-mikey-parameter")
        .expect("set-mikey-parameter action should be registered");

    let query = signal_id.query();
    assert_eq!(
        query.n_params(),
        3,
        "set-mikey-parameter should have 3 parameters"
    );
    assert_eq!(
        query.param_types()[0],
        u32::static_type(),
        "First param should be u32 (stream_id)"
    );
    assert_eq!(
        query.param_types()[1],
        gst::Caps::static_type(),
        "Second param should be GstCaps"
    );
    assert_eq!(
        query.param_types()[2],
        gst::Promise::static_type(),
        "Third param should be GstPromise"
    );
    assert_eq!(
        query.return_type(),
        bool::static_type(),
        "Return type should be bool"
    );

    // Test remove-key action exists
    let signal_id = element
        .object_class()
        .find_signal("remove-key")
        .expect("remove-key action should be registered");

    let query = signal_id.query();
    assert_eq!(query.n_params(), 1, "remove-key should have 1 parameter");
    assert_eq!(
        query.param_types()[0],
        u32::static_type(),
        "First param should be u32 (stream_id)"
    );
    assert_eq!(
        query.return_type(),
        bool::static_type(),
        "Return type should be bool"
    );

    println!("All backchannel actions are properly registered");
}

#[test]
fn test_push_backchannel_buffer_action() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a test buffer
    let buffer = gst::Buffer::new();

    // Test push-backchannel-buffer action invocation
    let result = element.emit_by_name::<gst::FlowReturn>(
        "push-backchannel-buffer",
        &[&0u32.to_value(), &buffer.to_value()],
    );

    // Should return NOT_SUPPORTED since backchannel isn't implemented
    assert_eq!(
        result,
        gst::FlowReturn::NotSupported,
        "push-backchannel-buffer should return NOT_SUPPORTED"
    );

    println!("push-backchannel-buffer action works correctly");
}

#[test]
fn test_push_backchannel_sample_action() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a test sample
    let buffer = gst::Buffer::new();
    let caps = gst::Caps::builder("audio/x-raw")
        .field("format", "S16LE")
        .field("rate", 48000i32)
        .field("channels", 2i32)
        .build();
    let sample = gst::Sample::builder().buffer(&buffer).caps(&caps).build();

    // Test push-backchannel-sample action invocation
    let result = element.emit_by_name::<gst::FlowReturn>(
        "push-backchannel-sample",
        &[&0u32.to_value(), &sample.to_value()],
    );

    // Should return NOT_SUPPORTED since backchannel isn't implemented
    assert_eq!(
        result,
        gst::FlowReturn::NotSupported,
        "push-backchannel-sample should return NOT_SUPPORTED"
    );

    println!("push-backchannel-sample action works correctly");
}

#[test]
fn test_set_mikey_parameter_action() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create MIKEY caps
    let caps = gst::Caps::builder("application/x-mikey")
        .field("key-mgmt", "prot=MIKEY;uri=\"sip:alice@atlanta.com\"")
        .build();

    // Create a promise for async result
    let promise = gst::Promise::new();

    // Test set-mikey-parameter action invocation
    let result = element.emit_by_name::<bool>(
        "set-mikey-parameter",
        &[&0u32.to_value(), &caps.to_value(), &promise.to_value()],
    );

    // Should return false since MIKEY isn't implemented
    assert_eq!(result, false, "set-mikey-parameter should return false");

    println!("set-mikey-parameter action works correctly");
}

#[test]
fn test_remove_key_action() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test remove-key action invocation
    let result = element.emit_by_name::<bool>("remove-key", &[&0u32.to_value()]);

    // Should return false since key management isn't implemented
    assert_eq!(result, false, "remove-key should return false");

    println!("remove-key action works correctly");
}

#[test]
fn test_rtspsrc_backchannel_actions() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify all backchannel actions are present
    assert!(
        element
            .object_class()
            .find_signal("push-backchannel-buffer")
            .is_some(),
        "push-backchannel-buffer action must be registered"
    );
    assert!(
        element
            .object_class()
            .find_signal("push-backchannel-sample")
            .is_some(),
        "push-backchannel-sample action must be registered"
    );
    assert!(
        element
            .object_class()
            .find_signal("set-mikey-parameter")
            .is_some(),
        "set-mikey-parameter action must be registered"
    );
    assert!(
        element.object_class().find_signal("remove-key").is_some(),
        "remove-key action must be registered"
    );

    println!("All backchannel action methods are available");
}
