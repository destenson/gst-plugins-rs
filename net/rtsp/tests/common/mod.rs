// Common utilities for E2E tests

use std::process::Command;

pub fn find_gst_plugin_path() -> Option<String> {
    // Try to find the compiled plugin
    let possible_paths = vec![
        "target/debug",
        "target/release",
        "../../../target/debug",
        "../../../target/release",
        "../../target/debug",
        "../../target/release",
    ];

    for path in possible_paths {
        if std::path::Path::new(path).exists() {
            return Some(path.to_string());
        }
    }

    None
}

pub fn check_gstreamer_available() -> bool {
    Command::new("gst-launch-1.0")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
