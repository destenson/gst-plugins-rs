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
fn test_rtspsrc_behavior_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify default values match the PRP requirements
    assert_eq!(element.property::<bool>("is-live"), true);
    // The actual default user-agent includes version info, just check it's not empty
    let user_agent = element.property::<String>("user-agent");
    assert!(!user_agent.is_empty());
    assert!(user_agent.starts_with("GStreamer"));
    assert_eq!(element.property::<u64>("connection-speed"), 0); // 0 = unknown

    // Test that properties can be changed in NULL state
    assert_eq!(element.current_state(), gst::State::Null);

    // Test is-live property
    element.set_property("is-live", false);
    assert_eq!(element.property::<bool>("is-live"), false);

    element.set_property("is-live", true);
    assert_eq!(element.property::<bool>("is-live"), true);

    // Test user-agent property
    element.set_property("user-agent", "CustomAgent/1.0");
    assert_eq!(
        element.property::<String>("user-agent"),
        "CustomAgent/1.0".to_string()
    );

    element.set_property("user-agent", "MyApp/2.0 (Linux)");
    assert_eq!(
        element.property::<String>("user-agent"),
        "MyApp/2.0 (Linux)".to_string()
    );

    // Test connection-speed property
    element.set_property("connection-speed", 1000u64); // 1000 kbps
    assert_eq!(element.property::<u64>("connection-speed"), 1000);

    element.set_property("connection-speed", 100000u64); // 100 Mbps
    assert_eq!(element.property::<u64>("connection-speed"), 100000);
}

#[test]
fn test_user_agent_property() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test default user agent
    let default_agent = element.property::<String>("user-agent");
    assert!(!default_agent.is_empty());
    assert!(default_agent.starts_with("GStreamer"));

    // Test setting various user agent strings
    element.set_property("user-agent", "VLC/3.0.0");
    assert_eq!(element.property::<String>("user-agent"), "VLC/3.0.0");

    element.set_property("user-agent", "FFmpeg/4.4");
    assert_eq!(element.property::<String>("user-agent"), "FFmpeg/4.4");

    // Test empty user agent (should be allowed)
    element.set_property("user-agent", "");
    assert_eq!(element.property::<String>("user-agent"), "");

    // Test long user agent string
    let long_agent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36";
    element.set_property("user-agent", long_agent);
    assert_eq!(element.property::<String>("user-agent"), long_agent);
}

#[test]
fn test_connection_speed_ranges() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test minimum value (0 = unknown)
    element.set_property("connection-speed", 0u64);
    assert_eq!(element.property::<u64>("connection-speed"), 0);

    // Test various speeds in kbps
    element.set_property("connection-speed", 56u64); // 56k modem
    assert_eq!(element.property::<u64>("connection-speed"), 56);

    element.set_property("connection-speed", 256u64); // 256 kbps
    assert_eq!(element.property::<u64>("connection-speed"), 256);

    element.set_property("connection-speed", 1024u64); // 1 Mbps
    assert_eq!(element.property::<u64>("connection-speed"), 1024);

    element.set_property("connection-speed", 10000u64); // 10 Mbps
    assert_eq!(element.property::<u64>("connection-speed"), 10000);

    element.set_property("connection-speed", 100000u64); // 100 Mbps
    assert_eq!(element.property::<u64>("connection-speed"), 100000);

    element.set_property("connection-speed", 1000000u64); // 1 Gbps
    assert_eq!(element.property::<u64>("connection-speed"), 1000000);

    // Test maximum value
    element.set_property("connection-speed", u64::MAX);
    assert_eq!(element.property::<u64>("connection-speed"), u64::MAX);
}

#[test]
fn test_is_live_property() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test default value (should be true)
    assert_eq!(element.property::<bool>("is-live"), true);

    // Test toggling the property
    element.set_property("is-live", false);
    assert_eq!(element.property::<bool>("is-live"), false);

    element.set_property("is-live", true);
    assert_eq!(element.property::<bool>("is-live"), true);

    // Test multiple toggles
    for _ in 0..5 {
        element.set_property("is-live", false);
        assert_eq!(element.property::<bool>("is-live"), false);
        element.set_property("is-live", true);
        assert_eq!(element.property::<bool>("is-live"), true);
    }
}

#[test]
fn test_source_behavior_defaults() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify all defaults match PRP-38 requirements
    assert_eq!(
        element.property::<bool>("is-live"),
        true,
        "is-live should default to true"
    );
    let user_agent = element.property::<String>("user-agent");
    assert!(
        user_agent.starts_with("GStreamer"),
        "user-agent should start with 'GStreamer', got: {}",
        user_agent
    );
    assert_eq!(
        element.property::<u64>("connection-speed"),
        0,
        "connection-speed should default to 0 (unknown)"
    );
}
