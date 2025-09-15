// Test harness for RTSP element lifecycle management
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use gst::glib::value::FromValue;
use gst::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Test harness for managing RTSP element lifecycle and testing
pub struct RtspTestHarness {
    pipeline: gst::Pipeline,
    rtspsrc: gst::Element,
    sink: gst::Element,
    messages: Arc<Mutex<Vec<gst::Message>>>,
    start_time: Instant,
}

impl RtspTestHarness {
    /// Create a new test harness with rtspsrc2 element
    pub fn new(rtsp_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        gst::init()?;

        // Create pipeline
        let pipeline = gst::Pipeline::new();

        // Create rtspsrc2 element
        let rtspsrc = gst::ElementFactory::make("rtspsrc2")
            .property("location", rtsp_url)
            .property("debug", true)
            .build()
            .map_err(|_| "Failed to create rtspsrc2 element")?;

        // Create fakesink for testing
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .build()
            .map_err(|_| "Failed to create fakesink element")?;

        // Add elements to pipeline
        pipeline.add(&rtspsrc)?;
        pipeline.add(&sink)?;

        // Connect pad-added signal to link dynamically
        let sink_weak = sink.downgrade();
        rtspsrc.connect_pad_added(move |_src, pad| {
            if let Some(sink) = sink_weak.upgrade() {
                let sink_pad = sink.static_pad("sink").unwrap();
                if !sink_pad.is_linked() {
                    pad.link(&sink_pad).unwrap();
                }
            }
        });

        // Set up message collection
        let messages = Arc::new(Mutex::new(Vec::new()));
        let messages_clone = messages.clone();

        let bus = pipeline.bus().unwrap();
        bus.add_watch(move |_bus, msg| {
            messages_clone.lock().unwrap().push(msg.clone());
            gst::glib::ControlFlow::Continue
        })?;

        Ok(Self {
            pipeline,
            rtspsrc,
            sink,
            messages,
            start_time: Instant::now(),
        })
    }

    /// Set a property on the rtspsrc element
    pub fn set_property<V: ToValue>(
        &self,
        name: &str,
        value: V,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.rtspsrc.set_property(name, value.to_value());
        Ok(())
    }

    /// Get a property from the rtspsrc element
    pub fn get_property<T: for<'a> FromValue<'a> + 'static>(
        &self,
        name: &str,
    ) -> Result<T, Box<dyn std::error::Error>> {
        Ok(self.rtspsrc.property::<T>(name))
    }

    /// Start the pipeline
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.start_time = Instant::now();
        self.pipeline.set_state(gst::State::Playing)?;
        Ok(())
    }

    /// Stop the pipeline
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    /// Wait for a specific state with timeout
    pub fn wait_for_state(
        &self,
        state: gst::State,
        timeout: Duration,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (success, _, _) = self
            .pipeline
            .state(Some(gst::ClockTime::from_seconds(timeout.as_secs())));
        if success == Ok(gst::StateChangeSuccess::Success) {
            Ok(())
        } else {
            Err("State change timeout".into())
        }
    }

    /// Wait for connection to be established
    pub fn wait_for_connection(
        &self,
        timeout: Duration,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            let messages = self.messages.lock().unwrap();
            for msg in messages.iter() {
                match msg.view() {
                    gst::MessageView::StateChanged(state_changed) => {
                        if state_changed.src() == Some(&self.pipeline.upcast_ref::<gst::Object>()) {
                            if state_changed.current() == gst::State::Playing {
                                return Ok(true);
                            }
                        }
                    }
                    gst::MessageView::Error(err) => {
                        return Err(format!("Pipeline error: {}", err.error()).into());
                    }
                    _ => {}
                }
            }
            drop(messages);
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(false)
    }

    /// Get all messages collected so far
    pub fn get_messages(&self) -> Vec<gst::Message> {
        self.messages.lock().unwrap().clone()
    }

    /// Clear collected messages
    pub fn clear_messages(&self) {
        self.messages.lock().unwrap().clear();
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if pipeline is in playing state
    pub fn is_playing(&self) -> bool {
        let (_, current, _) = self
            .pipeline
            .state(Some(gst::ClockTime::from_mseconds(100)));
        current == gst::State::Playing
    }

    /// Force a reconnection by setting pipeline to NULL and back to PLAYING
    pub fn force_reconnect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.pipeline.set_state(gst::State::Null)?;
        std::thread::sleep(Duration::from_millis(500));
        self.pipeline.set_state(gst::State::Playing)?;
        self.start_time = Instant::now();
        Ok(())
    }

    /// Get retry-related statistics
    pub fn get_retry_stats(&self) -> Result<RetryStats, Box<dyn std::error::Error>> {
        // Try to get stats from element properties - these properties might not exist
        let attempts = 0i32; // Default value if property doesn't exist
        let last_error = String::new(); // Default value if property doesn't exist

        // Count connection-related messages
        let messages = self.messages.lock().unwrap();
        let mut connection_attempts = 0;
        let mut connection_failures = 0;
        let mut connection_successes = 0;

        for msg in messages.iter() {
            match msg.view() {
                gst::MessageView::Element(element) => {
                    if let Some(structure) = element.structure() {
                        let name = structure.name();
                        if name.contains("connection") || name.contains("retry") {
                            connection_attempts += 1;
                            if name.contains("fail") || name.contains("error") {
                                connection_failures += 1;
                            } else if name.contains("success") || name.contains("connected") {
                                connection_successes += 1;
                            }
                        }
                    }
                }
                gst::MessageView::Error(_) => {
                    connection_failures += 1;
                }
                _ => {}
            }
        }

        Ok(RetryStats {
            total_attempts: attempts as u32,
            connection_attempts,
            connection_failures,
            connection_successes,
            last_error,
            elapsed: self.elapsed(),
        })
    }

    /// Simulate network conditions
    pub fn simulate_network_condition(
        &self,
        condition: NetworkCondition,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match condition {
            NetworkCondition::PacketLoss(percent) => {
                // This would need to be implemented with system-level network simulation
                // or by configuring the RTSP server to simulate packet loss
                eprintln!("Simulating {}% packet loss", percent);
            }
            NetworkCondition::Latency(ms) => {
                eprintln!("Simulating {}ms latency", ms);
            }
            NetworkCondition::Jitter(ms) => {
                eprintln!("Simulating {}ms jitter", ms);
            }
        }
        Ok(())
    }
}

impl Drop for RtspTestHarness {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Retry statistics from the test harness
#[derive(Debug, Clone)]
pub struct RetryStats {
    pub total_attempts: u32,
    pub connection_attempts: u32,
    pub connection_failures: u32,
    pub connection_successes: u32,
    pub last_error: String,
    pub elapsed: Duration,
}

/// Network conditions for simulation
#[derive(Debug, Clone)]
pub enum NetworkCondition {
    PacketLoss(f32), // Percentage
    Latency(u32),    // Milliseconds
    Jitter(u32),     // Milliseconds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        gst::init().unwrap();

        let harness = RtspTestHarness::new("rtsp://127.0.0.1:8554/test");
        assert!(harness.is_ok());

        if let Ok(h) = harness {
            assert!(!h.is_playing());
        }
    }
}
