use gst::prelude::*;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("Failed to register rtsp plugin");
    });
}

#[test]
fn test_timeout_property_ranges() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test tcp-timeout property (microseconds, u64 range)
    element.set_property("tcp-timeout", 0u64);
    assert_eq!(element.property::<u64>("tcp-timeout"), 0);

    element.set_property("tcp-timeout", 20000000u64);
    assert_eq!(element.property::<u64>("tcp-timeout"), 20000000);

    element.set_property("tcp-timeout", u64::MAX);
    assert_eq!(element.property::<u64>("tcp-timeout"), u64::MAX);

    // Test teardown-timeout property (nanoseconds, u64 range)
    element.set_property("teardown-timeout", 0u64);
    assert_eq!(element.property::<u64>("teardown-timeout"), 0);

    element.set_property("teardown-timeout", 100000000u64);
    assert_eq!(element.property::<u64>("teardown-timeout"), 100000000);

    element.set_property("teardown-timeout", u64::MAX);
    assert_eq!(element.property::<u64>("teardown-timeout"), u64::MAX);
}

#[test]
fn test_rtspsrc_timeout_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify default values match the PRP requirements
    assert_eq!(element.property::<bool>("do-rtsp-keep-alive"), true);
    assert_eq!(element.property::<u64>("tcp-timeout"), 20000000); // 20 seconds in microseconds
    assert_eq!(element.property::<u64>("teardown-timeout"), 100000000); // 100ms in nanoseconds
    assert_eq!(element.property::<bool>("udp-reconnect"), true);

    // Test that properties can be changed in NULL state
    assert_eq!(element.current_state(), gst::State::Null);

    element.set_property("do-rtsp-keep-alive", false);
    assert_eq!(element.property::<bool>("do-rtsp-keep-alive"), false);

    element.set_property("tcp-timeout", 30000000u64);
    assert_eq!(element.property::<u64>("tcp-timeout"), 30000000);

    element.set_property("teardown-timeout", 200000000u64);
    assert_eq!(element.property::<u64>("teardown-timeout"), 200000000);

    element.set_property("udp-reconnect", false);
    assert_eq!(element.property::<bool>("udp-reconnect"), false);
}

#[test]
fn test_keepalive_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test do-rtsp-keep-alive property (boolean)
    element.set_property("do-rtsp-keep-alive", false);
    assert_eq!(element.property::<bool>("do-rtsp-keep-alive"), false);
    element.set_property("do-rtsp-keep-alive", true);
    assert_eq!(element.property::<bool>("do-rtsp-keep-alive"), true);

    // Test udp-reconnect property (boolean)
    element.set_property("udp-reconnect", false);
    assert_eq!(element.property::<bool>("udp-reconnect"), false);
    element.set_property("udp-reconnect", true);
    assert_eq!(element.property::<bool>("udp-reconnect"), true);
}

#[test]
fn test_timeout_units() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test that tcp-timeout is in microseconds (20 seconds = 20,000,000 microseconds)
    let tcp_timeout_default = element.property::<u64>("tcp-timeout");
    assert_eq!(
        tcp_timeout_default, 20000000,
        "tcp-timeout should be 20 seconds in microseconds"
    );

    // Test that teardown-timeout is in nanoseconds (100ms = 100,000,000 nanoseconds)
    let teardown_timeout_default = element.property::<u64>("teardown-timeout");
    assert_eq!(
        teardown_timeout_default, 100000000,
        "teardown-timeout should be 100ms in nanoseconds"
    );

    // Test setting custom values
    element.set_property("tcp-timeout", 1000000u64); // 1 second in microseconds
    assert_eq!(element.property::<u64>("tcp-timeout"), 1000000);

    element.set_property("teardown-timeout", 1000000000u64); // 1 second in nanoseconds
    assert_eq!(element.property::<u64>("teardown-timeout"), 1000000000);
}
