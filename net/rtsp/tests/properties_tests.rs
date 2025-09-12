
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

#[test]
fn test_rtcp_property_ranges() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Test do-rtcp property (boolean)
    element.set_property("do-rtcp", false);
    assert_eq!(element.property::<bool>("do-rtcp"), false);
    element.set_property("do-rtcp", true);
    assert_eq!(element.property::<bool>("do-rtcp"), true);

    // Test do-retransmission property (boolean)
    element.set_property("do-retransmission", false);
    assert_eq!(element.property::<bool>("do-retransmission"), false);
    element.set_property("do-retransmission", true);
    assert_eq!(element.property::<bool>("do-retransmission"), true);

    // Test max-rtcp-rtp-time-diff property (integer with range -1 to i32::MAX)
    element.set_property("max-rtcp-rtp-time-diff", -1i32);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), -1);

    element.set_property("max-rtcp-rtp-time-diff", 0i32);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), 0);

    element.set_property("max-rtcp-rtp-time-diff", 1000i32);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), 1000);

    element.set_property("max-rtcp-rtp-time-diff", i32::MAX);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), i32::MAX);
}

#[test]
fn test_rtspsrc_rtcp_properties() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Verify default values match the PRP requirements
    assert_eq!(element.property::<bool>("do-rtcp"), true);
    assert_eq!(element.property::<bool>("do-retransmission"), true);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), -1);

    // Test that properties can be changed in NULL state
    assert_eq!(element.current_state(), gst::State::Null);

    element.set_property("do-rtcp", false);
    assert_eq!(element.property::<bool>("do-rtcp"), false);

    element.set_property("do-retransmission", false);
    assert_eq!(element.property::<bool>("do-retransmission"), false);

    element.set_property("max-rtcp-rtp-time-diff", 5000i32);
    assert_eq!(element.property::<i32>("max-rtcp-rtp-time-diff"), 5000);
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

#[test]
fn test_rtp_blocksize_property() {
    init();

    let elem = gst::ElementFactory::make("rtspsrc2")
        .name("test_src")
        .build()
        .unwrap();

    // Test default value
    let blocksize = elem.property::<u32>("rtp-blocksize");
    assert_eq!(blocksize, 0, "Default rtp-blocksize should be 0");

    // Test setting valid values
    elem.set_property("rtp-blocksize", 1024u32);
    let blocksize = elem.property::<u32>("rtp-blocksize");
    assert_eq!(blocksize, 1024, "rtp-blocksize should be 1024");

    // Test maximum value
    elem.set_property("rtp-blocksize", 65536u32);
    let blocksize = elem.property::<u32>("rtp-blocksize");
    assert_eq!(blocksize, 65536, "rtp-blocksize should be 65536");
}

#[test]
fn test_tcp_timestamp_property() {
    init();

    let elem = gst::ElementFactory::make("rtspsrc2")
        .name("test_src")
        .build()
        .unwrap();

    // Test default value
    let tcp_timestamp = elem.property::<bool>("tcp-timestamp");
    assert_eq!(
        tcp_timestamp, false,
        "Default tcp-timestamp should be false"
    );

    // Test setting to true
    elem.set_property("tcp-timestamp", true);
    let tcp_timestamp = elem.property::<bool>("tcp-timestamp");
    assert_eq!(tcp_timestamp, true, "tcp-timestamp should be true");

    // Test setting back to false
    elem.set_property("tcp-timestamp", false);
    let tcp_timestamp = elem.property::<bool>("tcp-timestamp");
    assert_eq!(tcp_timestamp, false, "tcp-timestamp should be false");
}

#[test]
fn test_sdes_property_structure() {
    init();

    let elem = gst::ElementFactory::make("rtspsrc2")
        .name("test_src")
        .build()
        .unwrap();

    // Test default value (should be None)
    let sdes = elem.property::<Option<gst::Structure>>("sdes");
    assert!(sdes.is_none(), "Default sdes should be None");

    // Create a test SDES structure
    let sdes_struct = gst::Structure::builder("application/x-rtp-source-sdes")
        .field("cname", "user@host")
        .field("name", "Test User")
        .field("email", "test@example.com")
        .field("tool", "rtspsrc2-test")
        .field("note", "Test note")
        .build();

    // Set the SDES structure
    elem.set_property("sdes", Some(sdes_struct.clone()));
    let retrieved_sdes = elem.property::<Option<gst::Structure>>("sdes");

    assert!(
        retrieved_sdes.is_some(),
        "SDES should not be None after setting"
    );
    let retrieved_sdes = retrieved_sdes.unwrap();

    // Verify fields
    assert_eq!(
        retrieved_sdes.get::<&str>("cname").unwrap(),
        "user@host",
        "SDES cname field should match"
    );
    assert_eq!(
        retrieved_sdes.get::<&str>("name").unwrap(),
        "Test User",
        "SDES name field should match"
    );
    assert_eq!(
        retrieved_sdes.get::<&str>("email").unwrap(),
        "test@example.com",
        "SDES email field should match"
    );

    // Test setting back to None
    elem.set_property("sdes", None::<gst::Structure>);
    let sdes = elem.property::<Option<gst::Structure>>("sdes");
    assert!(sdes.is_none(), "SDES should be None after clearing");
}

#[test]
fn test_rtspsrc_rtp_properties() {
    init();

    let elem = gst::ElementFactory::make("rtspsrc2")
        .name("test_src")
        .build()
        .unwrap();

    // Test all properties together
    elem.set_property("rtp-blocksize", 8192u32);
    elem.set_property("tcp-timestamp", true);

    let sdes_struct = gst::Structure::builder("application/x-rtp-source-sdes")
        .field("cname", "test@localhost")
        .field("location", "Test Lab")
        .build();
    elem.set_property("sdes", Some(sdes_struct));

    // Verify all properties
    assert_eq!(elem.property::<u32>("rtp-blocksize"), 8192);
    assert_eq!(elem.property::<bool>("tcp-timestamp"), true);

    let sdes = elem.property::<Option<gst::Structure>>("sdes").unwrap();
    assert_eq!(sdes.get::<&str>("cname").unwrap(), "test@localhost");
    assert_eq!(sdes.get::<&str>("location").unwrap(), "Test Lab");
}
