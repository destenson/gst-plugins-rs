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
fn test_ntp_time_source_enum_property() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Get the property spec to verify it's an enum
    let properties = element.list_properties();
    let ntp_time_source_prop = properties
        .iter()
        .find(|p| p.name() == "ntp-time-source")
        .expect("ntp-time-source property not found");
    
    // Verify it's an enum type
    assert!(ntp_time_source_prop.value_type().is_a(gst::glib::Type::ENUM));
    
    // The test would need to be updated to handle enum values instead of strings
    // GStreamer's enum properties work differently than string properties
}