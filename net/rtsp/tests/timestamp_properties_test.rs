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
fn test_rtspsrc_timestamp_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify default values match the PRP requirements
    assert_eq!(element.property::<bool>("ntp-sync"), false);
    assert_eq!(element.property::<bool>("rfc7273-sync"), false);
    assert_eq!(element.property::<String>("ntp-time-source"), "ntp");
    assert_eq!(element.property::<i64>("max-ts-offset"), 3000000000); // 3 seconds in nanoseconds
    assert_eq!(element.property::<u64>("max-ts-offset-adjustment"), 0);
    assert_eq!(
        element.property::<bool>("add-reference-timestamp-meta"),
        false
    );

    // Test that properties can be changed in NULL state
    assert_eq!(element.current_state(), gst::State::Null);

    // Test boolean properties
    element.set_property("ntp-sync", true);
    assert_eq!(element.property::<bool>("ntp-sync"), true);

    element.set_property("rfc7273-sync", true);
    assert_eq!(element.property::<bool>("rfc7273-sync"), true);

    element.set_property("add-reference-timestamp-meta", true);
    assert_eq!(
        element.property::<bool>("add-reference-timestamp-meta"),
        true
    );

    // Test timestamp offset properties
    element.set_property("max-ts-offset", 1000000000i64); // 1 second
    assert_eq!(element.property::<i64>("max-ts-offset"), 1000000000);

    element.set_property("max-ts-offset-adjustment", 100000u64);
    assert_eq!(element.property::<u64>("max-ts-offset-adjustment"), 100000);
}

#[test]
fn test_ntp_time_source_enum() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test default value
    assert_eq!(element.property::<String>("ntp-time-source"), "ntp");

    // Test all enum values
    element.set_property("ntp-time-source", "ntp");
    assert_eq!(element.property::<String>("ntp-time-source"), "ntp");

    element.set_property("ntp-time-source", "unix");
    assert_eq!(element.property::<String>("ntp-time-source"), "unix");

    element.set_property("ntp-time-source", "running-time");
    assert_eq!(
        element.property::<String>("ntp-time-source"),
        "running-time"
    );

    element.set_property("ntp-time-source", "clock-time");
    assert_eq!(element.property::<String>("ntp-time-source"), "clock-time");
}

#[test]
fn test_max_ts_offset_ranges() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test minimum value
    element.set_property("max-ts-offset", 0i64);
    assert_eq!(element.property::<i64>("max-ts-offset"), 0);

    // Test various values
    element.set_property("max-ts-offset", 1000000i64); // 1 millisecond
    assert_eq!(element.property::<i64>("max-ts-offset"), 1000000);

    element.set_property("max-ts-offset", 1000000000i64); // 1 second
    assert_eq!(element.property::<i64>("max-ts-offset"), 1000000000);

    element.set_property("max-ts-offset", 60000000000i64); // 1 minute
    assert_eq!(element.property::<i64>("max-ts-offset"), 60000000000);

    // Test maximum value
    element.set_property("max-ts-offset", i64::MAX);
    assert_eq!(element.property::<i64>("max-ts-offset"), i64::MAX);
}

#[test]
fn test_max_ts_offset_adjustment_ranges() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test minimum value (0 = no limit)
    element.set_property("max-ts-offset-adjustment", 0u64);
    assert_eq!(element.property::<u64>("max-ts-offset-adjustment"), 0);

    // Test various values
    element.set_property("max-ts-offset-adjustment", 1000u64); // 1 microsecond
    assert_eq!(element.property::<u64>("max-ts-offset-adjustment"), 1000);

    element.set_property("max-ts-offset-adjustment", 1000000u64); // 1 millisecond
    assert_eq!(element.property::<u64>("max-ts-offset-adjustment"), 1000000);

    element.set_property("max-ts-offset-adjustment", 33333333u64); // ~30fps frame time
    assert_eq!(
        element.property::<u64>("max-ts-offset-adjustment"),
        33333333
    );

    // Test maximum value
    element.set_property("max-ts-offset-adjustment", u64::MAX);
    assert_eq!(
        element.property::<u64>("max-ts-offset-adjustment"),
        u64::MAX
    );
}

#[test]
fn test_sync_properties_interaction() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test that both sync properties can be set independently
    element.set_property("ntp-sync", true);
    element.set_property("rfc7273-sync", false);
    assert_eq!(element.property::<bool>("ntp-sync"), true);
    assert_eq!(element.property::<bool>("rfc7273-sync"), false);

    element.set_property("ntp-sync", false);
    element.set_property("rfc7273-sync", true);
    assert_eq!(element.property::<bool>("ntp-sync"), false);
    assert_eq!(element.property::<bool>("rfc7273-sync"), true);

    // Both can be enabled
    element.set_property("ntp-sync", true);
    element.set_property("rfc7273-sync", true);
    assert_eq!(element.property::<bool>("ntp-sync"), true);
    assert_eq!(element.property::<bool>("rfc7273-sync"), true);
}

#[test]
fn test_timestamp_sync_defaults() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify all defaults match PRP-39 requirements
    assert_eq!(
        element.property::<bool>("ntp-sync"),
        false,
        "ntp-sync should default to false"
    );
    assert_eq!(
        element.property::<bool>("rfc7273-sync"),
        false,
        "rfc7273-sync should default to false"
    );
    assert_eq!(
        element.property::<String>("ntp-time-source"),
        "ntp",
        "ntp-time-source should default to 'ntp'"
    );
    assert_eq!(
        element.property::<i64>("max-ts-offset"),
        3000000000,
        "max-ts-offset should default to 3000000000 ns (3 seconds)"
    );
    assert_eq!(
        element.property::<u64>("max-ts-offset-adjustment"),
        0,
        "max-ts-offset-adjustment should default to 0 (no limit)"
    );
    assert_eq!(
        element.property::<bool>("add-reference-timestamp-meta"),
        false,
        "add-reference-timestamp-meta should default to false"
    );
}
