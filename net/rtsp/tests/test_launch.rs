// Simple RTSP test server launcher
//
// This creates a minimal RTSP server for testing purposes
// It attempts to use gst-rtsp-server if available

use std::env;
use std::process::{Command, Stdio};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    // Parse arguments
    let mut port = 8554;
    let mut mount = "/test";
    let mut pipeline = "( videotestsrc is-live=true ! video/x-raw,width=640,height=480,framerate=30/1 ! x264enc tune=zerolatency ! rtph264pay name=pay0 pt=96 )";

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse()?;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--mount" => {
                if i + 1 < args.len() {
                    mount = &args[i + 1];
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                pipeline = &args[i];
                i += 1;
            }
        }
    }

    println!(
        "Starting RTSP test server on port {} with mount point {}",
        port, mount
    );
    println!("Pipeline: {}", pipeline);

    // Try to use gst-rtsp-server if available
    let result = Command::new("gst-rtsp-server-1.0")
        .arg("--port")
        .arg(port.to_string())
        .arg(pipeline)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn();

    match result {
        Ok(mut child) => {
            println!("Server started successfully");
            let _ = child.wait();
        }
        Err(_) => {
            eprintln!("gst-rtsp-server-1.0 not found");
            eprintln!("Please install gstreamer1.0-rtsp-server or equivalent");
            std::process::exit(1);
        }
    }

    Ok(())
}
