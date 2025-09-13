// GStreamer RTSP plugin GET_PARAMETER and SET_PARAMETER tests
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use gst::prelude::*;
use serial_test::serial;

mod mock_server;
use mock_server::MockRtspServer;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        gstrsrtsp::plugin_register_static().expect("rtsp plugin registration failed");
    });
}

#[tokio::test]
#[serial]
async fn test_get_parameter_action() {
    init();

    // Start mock RTSP server
    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    // Create rtspsrc2 element
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a promise for the get-parameter action
    let promise = gst::Promise::new();

    // Call get-parameter action signal
    let result = element.emit_by_name::<bool>(
        "get-parameter",
        &[
            &"test_param".to_value(),
            &None::<String>.to_value(),
            &promise.to_value(),
        ],
    );

    // The action should return true (request sent)
    assert!(result, "get-parameter action should return true");
}

#[tokio::test]
#[serial]
async fn test_get_parameters_action() {
    init();

    // Start mock RTSP server
    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    // Create rtspsrc2 element
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a promise for the get-parameters action
    let promise = gst::Promise::new();

    // Create parameter list
    let params = vec!["param1".to_string(), "param2".to_string()];
    let params_variant = gst::glib::Variant::from(params);

    // Call get-parameters action signal
    let result = element.emit_by_name::<bool>(
        "get-parameters",
        &[
            &params_variant.to_value(),
            &None::<String>.to_value(),
            &promise.to_value(),
        ],
    );

    // The action should return true (request sent)
    assert!(result, "get-parameters action should return true");
}

#[tokio::test]
#[serial]
async fn test_set_parameter_action() {
    init();

    // Start mock RTSP server
    let server = MockRtspServer::new().await;
    let url = server.url();
    let _handle = server.start().await;

    // Create rtspsrc2 element
    let element = gst::ElementFactory::make("rtspsrc2")
        .property("location", &url)
        .build()
        .expect("Failed to create rtspsrc2 element");

    // Create a promise for the set-parameter action
    let promise = gst::Promise::new();

    // Call set-parameter action signal
    let result = element.emit_by_name::<bool>(
        "set-parameter",
        &[
            &"test_param".to_value(),
            &"test_value".to_value(),
            &None::<String>.to_value(),
            &promise.to_value(),
        ],
    );

    // The action should return true (request sent)
    assert!(result, "set-parameter action should return true");
}

#[test]
#[serial]
fn test_parameter_signal_validation() {
    init();

    let element = gst::ElementFactory::make("rtspsrc2")
        .build()
        .expect("Failed to create rtspsrc2 element");

    let promise = gst::Promise::new();

    // Test empty parameter name (should return false)
    let result = element.emit_by_name::<bool>(
        "get-parameter",
        &[
            &"".to_value(),
            &None::<String>.to_value(),
            &promise.to_value(),
        ],
    );
    assert!(!result, "get-parameter with empty name should return false");

    // Test empty parameters array (should return false)
    let empty_params = Vec::<String>::new();
    let empty_variant = gst::glib::Variant::from(empty_params);
    let result = element.emit_by_name::<bool>(
        "get-parameters",
        &[
            &empty_variant.to_value(),
            &None::<String>.to_value(),
            &promise.to_value(),
        ],
    );
    assert!(
        !result,
        "get-parameters with empty array should return false"
    );

    // Test empty parameter name in set-parameter (should return false)
    let result = element.emit_by_name::<bool>(
        "set-parameter",
        &[
            &"".to_value(),
            &"value".to_value(),
            &None::<String>.to_value(),
            &promise.to_value(),
        ],
    );
    assert!(!result, "set-parameter with empty name should return false");
}
