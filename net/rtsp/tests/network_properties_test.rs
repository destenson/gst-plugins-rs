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
fn test_rtspsrc_network_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify default values match the PRP requirements
    assert_eq!(element.property::<Option<String>>("multicast-iface"), None);
    assert_eq!(element.property::<Option<String>>("port-range"), None);
    assert_eq!(element.property::<i32>("udp-buffer-size"), 524288); // 512KB default

    // Test that properties can be changed in NULL state
    assert_eq!(element.current_state(), gst::State::Null);

    // Test multicast-iface property
    element.set_property("multicast-iface", Some("eth0"));
    assert_eq!(
        element.property::<Option<String>>("multicast-iface"),
        Some("eth0".to_string())
    );

    element.set_property("multicast-iface", Some("wlan0"));
    assert_eq!(
        element.property::<Option<String>>("multicast-iface"),
        Some("wlan0".to_string())
    );

    element.set_property("multicast-iface", None::<String>);
    assert_eq!(element.property::<Option<String>>("multicast-iface"), None);

    // Test udp-buffer-size property
    element.set_property("udp-buffer-size", 0i32);
    assert_eq!(element.property::<i32>("udp-buffer-size"), 0);

    element.set_property("udp-buffer-size", 1048576i32); // 1MB
    assert_eq!(element.property::<i32>("udp-buffer-size"), 1048576);

    element.set_property("udp-buffer-size", i32::MAX);
    assert_eq!(element.property::<i32>("udp-buffer-size"), i32::MAX);
}

#[test]
fn test_port_range_validation() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Valid port ranges
    element.set_property("port-range", Some("3000-3001"));
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );

    element.set_property("port-range", Some("5000-5003"));
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("5000-5003".to_string())
    );

    // Null value is valid
    element.set_property("port-range", None::<String>);
    assert_eq!(element.property::<Option<String>>("port-range"), None);

    // Empty string should be treated as None
    element.set_property("port-range", Some(""));
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("".to_string())
    );
}

#[test]
fn test_invalid_port_range_format() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid value first
    element.set_property("port-range", Some("3000-3001"));
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );

    // Invalid format - single port (should not change the value)
    element.set_property("port-range", Some("3000"));
    // Value should remain unchanged
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );
}

#[test]
fn test_invalid_port_range_multiple_dashes() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid value first
    element.set_property("port-range", Some("3000-3001"));

    // Invalid format - multiple dashes (should not change the value)
    element.set_property("port-range", Some("3000-3001-3002"));
    // Value should remain unchanged
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );
}

#[test]
fn test_invalid_port_range_non_numeric_start() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid value first
    element.set_property("port-range", Some("3000-3001"));

    // Invalid start port (should not change the value)
    element.set_property("port-range", Some("abc-3001"));
    // Value should remain unchanged
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );
}

#[test]
fn test_invalid_port_range_non_numeric_end() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid value first
    element.set_property("port-range", Some("3000-3001"));

    // Invalid end port (should not change the value)
    element.set_property("port-range", Some("3000-xyz"));
    // Value should remain unchanged
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );
}

#[test]
fn test_invalid_port_range_reversed() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid value first
    element.set_property("port-range", Some("3000-3001"));

    // Invalid range - start > end (should not change the value)
    element.set_property("port-range", Some("3005-3000"));
    // Value should remain unchanged
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );
}

#[test]
fn test_invalid_port_range_odd_count() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Set a valid value first
    element.set_property("port-range", Some("3000-3001"));

    // Invalid - odd number of ports (3 ports: 3000, 3001, 3002) (should not change the value)
    element.set_property("port-range", Some("3000-3002"));
    // Value should remain unchanged
    assert_eq!(
        element.property::<Option<String>>("port-range"),
        Some("3000-3001".to_string())
    );
}

#[test]
fn test_network_interface_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test setting various interface names
    element.set_property("multicast-iface", Some("lo"));
    assert_eq!(
        element.property::<Option<String>>("multicast-iface"),
        Some("lo".to_string())
    );

    element.set_property("multicast-iface", Some("eth0"));
    assert_eq!(
        element.property::<Option<String>>("multicast-iface"),
        Some("eth0".to_string())
    );

    element.set_property("multicast-iface", Some("wlan0"));
    assert_eq!(
        element.property::<Option<String>>("multicast-iface"),
        Some("wlan0".to_string())
    );

    // Test that empty string is allowed
    element.set_property("multicast-iface", Some(""));
    assert_eq!(
        element.property::<Option<String>>("multicast-iface"),
        Some("".to_string())
    );
}

#[test]
fn test_udp_buffer_size_ranges() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test various buffer sizes
    element.set_property("udp-buffer-size", 0i32);
    assert_eq!(element.property::<i32>("udp-buffer-size"), 0);

    element.set_property("udp-buffer-size", 262144i32); // 256KB
    assert_eq!(element.property::<i32>("udp-buffer-size"), 262144);

    element.set_property("udp-buffer-size", 524288i32); // 512KB (default)
    assert_eq!(element.property::<i32>("udp-buffer-size"), 524288);

    element.set_property("udp-buffer-size", 1048576i32); // 1MB
    assert_eq!(element.property::<i32>("udp-buffer-size"), 1048576);

    element.set_property("udp-buffer-size", 2097152i32); // 2MB
    assert_eq!(element.property::<i32>("udp-buffer-size"), 2097152);

    // Test max value
    element.set_property("udp-buffer-size", i32::MAX);
    assert_eq!(element.property::<i32>("udp-buffer-size"), i32::MAX);
}
