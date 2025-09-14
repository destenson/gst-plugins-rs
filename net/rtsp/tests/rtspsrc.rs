// Integration tests for RTSP source element

use gst::prelude::*;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("rtsp plugin register failed");
    });
}

#[test]
fn test_element_with_user_credentials() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Set username and password
    element.set_property("user-id", "testuser");
    element.set_property("user-pw", "testpass");

    // Get properties back
    let user_id: Option<String> = element.property("user-id");
    let user_pw: Option<String> = element.property("user-pw");

    assert_eq!(user_id, Some("testuser".to_string()));
    assert_eq!(user_pw, Some("testpass".to_string()));
}

#[test]
fn test_location_with_credentials() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // Set location with credentials
    let location = "rtsp://user:pass@example.com:554/stream";
    element.set_property("location", location);

    // BUG: URI credentials are currently ignored (see imp.rs:910)
    // This test SHOULD pass once the bug is fixed
    // Credentials should be parsed from URL and set as properties
    let user_id: Option<String> = element.property("user-id");
    let user_pw: Option<String> = element.property("user-pw");

    assert_eq!(user_id, Some("user".to_string()));
    assert_eq!(user_pw, Some("pass".to_string()));
}

#[test]
fn test_priority_of_property_over_url_credentials() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2");

    // First set via URL
    element.set_property("location", "rtsp://urluser:urlpass@example.com/stream");

    // Then override via properties
    element.set_property("user-id", "propuser");
    element.set_property("user-pw", "proppass");

    // Properties should take precedence
    let user_id: Option<String> = element.property("user-id");
    let user_pw: Option<String> = element.property("user-pw");

    assert_eq!(user_id, Some("propuser".to_string()));
    assert_eq!(user_pw, Some("proppass".to_string()));
}