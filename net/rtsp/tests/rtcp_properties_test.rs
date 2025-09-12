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
