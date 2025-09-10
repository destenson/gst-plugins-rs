#![allow(unused)]
// GStreamer RTSP plugin session timeout management
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use std::time::{Duration, Instant};
use rtsp_types::headers::Session;
use tokio::sync::mpsc;
use tokio::time::{interval, Interval};

const DEFAULT_SESSION_TIMEOUT: Duration = Duration::from_secs(60);
const KEEPALIVE_FACTOR: f64 = 0.8; // Send keep-alive at 80% of timeout

#[derive(Debug, Clone)]
pub enum KeepAliveMethod {
    GetParameter,  // Empty GET_PARAMETER (preferred)
    Options,       // OPTIONS request
    RtcpRr,       // RTCP Receiver Reports
}

impl Default for KeepAliveMethod {
    fn default() -> Self {
        KeepAliveMethod::GetParameter
    }
}

#[derive(Debug)]
pub struct SessionManager {
    session: Option<Session>,
    timeout: Duration,
    last_activity: Instant,
    keepalive_interval: Option<Interval>,
    keepalive_method: KeepAliveMethod,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            session: None,
            timeout: DEFAULT_SESSION_TIMEOUT,
            last_activity: Instant::now(),
            keepalive_interval: None,
            keepalive_method: KeepAliveMethod::default(),
        }
    }

    /// Parse and store session information from RTSP headers
    pub fn set_session(&mut self, session: Session) {
        // Parse timeout from session header if present
        // Session header format: "session-id[;timeout=seconds]"
        // Note: The Session type from rtsp_types strips the timeout field
        // See: https://github.com/sdroege/rtsp-types/issues/24
        
        self.session = Some(session.clone());
        
        // For now, we'll use the default timeout since rtsp_types strips it
        // In the future, we should parse the raw header to extract timeout
        self.timeout = DEFAULT_SESSION_TIMEOUT;
        
        self.reset_activity();
        self.start_keepalive_timer();
    }

    /// Parse timeout from raw Session header value
    pub fn parse_session_with_timeout(&mut self, header_value: &str) {
        // Parse format: "session-id[;timeout=seconds]"
        let parts: Vec<&str> = header_value.split(';').collect();
        
        if let Some(session_id) = parts.first() {
            self.session = Some(Session(session_id.trim().to_string(), None));
        }
        
        // Look for timeout parameter
        for part in parts.iter().skip(1) {
            let trimmed = part.trim();
            if trimmed.starts_with("timeout=") {
                if let Some(timeout_str) = trimmed.strip_prefix("timeout=") {
                    if let Ok(timeout_secs) = timeout_str.parse::<u64>() {
                        self.timeout = Duration::from_secs(timeout_secs);
                        gst::debug!(
                            gst::CAT_RUST,
                            "Session timeout set to {} seconds",
                            timeout_secs
                        );
                    }
                }
            }
        }
        
        self.reset_activity();
        self.start_keepalive_timer();
    }

    /// Get the current session
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// Clear the session (on teardown or error)
    pub fn clear_session(&mut self) {
        self.session = None;
        self.stop_keepalive_timer();
    }

    /// Reset the activity timestamp (called on any server response)
    pub fn reset_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if we need to send a keep-alive
    pub fn needs_keepalive(&self) -> bool {
        if self.session.is_none() {
            return false;
        }
        
        let elapsed = self.last_activity.elapsed();
        let keepalive_threshold = Duration::from_secs_f64(self.timeout.as_secs_f64() * KEEPALIVE_FACTOR);
        
        elapsed >= keepalive_threshold
    }

    /// Check if the session has timed out
    pub fn is_timed_out(&self) -> bool {
        if self.session.is_none() {
            return false;
        }
        
        self.last_activity.elapsed() > self.timeout
    }

    /// Get the time until next keep-alive is needed
    pub fn time_until_keepalive(&self) -> Option<Duration> {
        if self.session.is_none() {
            return None;
        }
        
        let elapsed = self.last_activity.elapsed();
        let keepalive_threshold = Duration::from_secs_f64(self.timeout.as_secs_f64() * KEEPALIVE_FACTOR);
        
        if elapsed >= keepalive_threshold {
            Some(Duration::ZERO)
        } else {
            Some(keepalive_threshold - elapsed)
        }
    }

    /// Set the keep-alive method
    pub fn set_keepalive_method(&mut self, method: KeepAliveMethod) {
        self.keepalive_method = method;
    }

    /// Get the current keep-alive method
    pub fn keepalive_method(&self) -> &KeepAliveMethod {
        &self.keepalive_method
    }

    /// Start the keep-alive timer
    fn start_keepalive_timer(&mut self) {
        let keepalive_interval_duration = Duration::from_secs_f64(self.timeout.as_secs_f64() * KEEPALIVE_FACTOR);
        self.keepalive_interval = Some(interval(keepalive_interval_duration));
    }

    /// Stop the keep-alive timer
    fn stop_keepalive_timer(&mut self) {
        self.keepalive_interval = None;
    }

    /// Get mutable reference to the keep-alive interval for polling
    pub fn keepalive_interval_mut(&mut self) -> Option<&mut Interval> {
        self.keepalive_interval.as_mut()
    }
}

/// Commands for session management
#[derive(Debug)]
pub enum SessionCommand {
    SendKeepAlive,
    CheckTimeout,
}

/// Create a session monitor task
pub async fn session_monitor_task(
    mut session_manager: SessionManager,
    mut command_rx: mpsc::Receiver<SessionCommand>,
    keepalive_tx: mpsc::Sender<()>,
) {
    loop {
        tokio::select! {
            Some(cmd) = command_rx.recv() => {
                match cmd {
                    SessionCommand::SendKeepAlive => {
                        if session_manager.needs_keepalive() {
                            let _ = keepalive_tx.send(()).await;
                        }
                    }
                    SessionCommand::CheckTimeout => {
                        if session_manager.is_timed_out() {
                            gst::warning!(gst::CAT_RUST, "Session timed out");
                            break;
                        }
                    }
                }
            }
            _ = async {
                if let Some(interval) = session_manager.keepalive_interval_mut() {
                    interval.tick().await;
                } else {
                    // No session, wait indefinitely
                    std::future::pending::<()>().await;
                }
            } => {
                // Time to send keep-alive
                if session_manager.needs_keepalive() {
                    let _ = keepalive_tx.send(()).await;
                }
            }
            else => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_timeout_parsing() {
        let mut manager = SessionManager::new();
        
        // Test with timeout
        manager.parse_session_with_timeout("12345;timeout=90");
        assert_eq!(manager.session().unwrap().0, "12345");
        assert_eq!(manager.timeout, Duration::from_secs(90));
        
        // Test without timeout
        manager.parse_session_with_timeout("67890");
        assert_eq!(manager.session().unwrap().0, "67890");
        
        // Test with spaces
        manager.parse_session_with_timeout("abcdef ; timeout=30");
        assert_eq!(manager.session().unwrap().0, "abcdef");
        assert_eq!(manager.timeout, Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_keepalive_timing() {
        let mut manager = SessionManager::new();
        manager.set_session(Session("test".to_string(), None));
        
        // Initially, should not need keep-alive
        assert!(!manager.needs_keepalive());
        
        // Simulate time passing (we can't actually wait in tests)
        manager.last_activity = Instant::now() - Duration::from_secs(50);
        
        // At 50 seconds with 60 second timeout, should need keep-alive (>80%)
        assert!(manager.needs_keepalive());
        assert!(!manager.is_timed_out());
        
        // Simulate timeout
        manager.last_activity = Instant::now() - Duration::from_secs(61);
        assert!(manager.is_timed_out());
    }

    #[tokio::test]
    async fn test_activity_reset() {
        let mut manager = SessionManager::new();
        manager.set_session(Session("test".to_string(), None));
        
        // Simulate old activity
        manager.last_activity = Instant::now() - Duration::from_secs(50);
        assert!(manager.needs_keepalive());
        
        // Reset activity
        manager.reset_activity();
        assert!(!manager.needs_keepalive());
    }
}
