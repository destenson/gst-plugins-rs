// GStreamer RTSP Debug and Observability Module
//
// This module provides debug categories and decision history tracking
// for comprehensive observability of retry and connection decisions.
//
// Copyright (C) 2025 GStreamer developers
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use gst::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// Debug categories using LazyLock pattern
pub static CAT_RETRY: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-retry",
        gst::DebugColorFlags::empty(),
        Some("RTSP Source 2 Retry Decisions"),
    )
});

pub static CAT_AUTO: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-auto",
        gst::DebugColorFlags::empty(),
        Some("RTSP Source 2 Auto Mode Pattern Detection"),
    )
});

pub static CAT_ADAPTIVE: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-adaptive",
        gst::DebugColorFlags::empty(),
        Some("RTSP Source 2 Adaptive Learning"),
    )
});

pub static CAT_RACING: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2-racing",
        gst::DebugColorFlags::empty(),
        Some("RTSP Source 2 Connection Racing"),
    )
});

// Decision history types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DecisionType {
    RetryDelay {
        attempt: u32,
        strategy: String,
        delay_ms: u64,
        reason: String,
    },
    PatternDetected {
        pattern: String,
        confidence: f32,
        evidence: String,
    },
    StrategyChanged {
        from: String,
        to: String,
        reason: String,
    },
    RacingModeUpdate {
        mode: String,
        reason: String,
    },
    AdaptiveLearning {
        strategy: String,
        confidence: f32,
        phase: String,
    },
    ConnectionResult {
        success: bool,
        duration_ms: Option<u64>,
        retry_count: u32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub timestamp: String,  // ISO 8601 format
    pub decision_type: DecisionType,
    pub context: Option<String>,
}

// Global decision history buffer
pub struct DecisionHistory {
    buffer: Arc<Mutex<VecDeque<Decision>>>,
    max_size: usize,
}

impl Default for DecisionHistory {
    fn default() -> Self {
        Self::new(20)
    }
}

impl DecisionHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
        }
    }

    pub fn record(&self, decision_type: DecisionType, context: Option<String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let decision = Decision {
            timestamp: format!("{}Z", timestamp),  // Simple timestamp format
            decision_type,
            context,
        };

        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push_back(decision);
            while buffer.len() > self.max_size {
                buffer.pop_front();
            }
        }
    }

    pub fn get_history(&self) -> Vec<Decision> {
        if let Ok(buffer) = self.buffer.lock() {
            buffer.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_history_json(&self) -> String {
        let history = self.get_history();
        serde_json::to_string_pretty(&history).unwrap_or_else(|_| "[]".to_string())
    }

    pub fn clear(&self) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.clear();
        }
    }
}

// Helper macros for structured logging
#[macro_export]
macro_rules! debug_decision {
    ($cat:expr, $history:expr, $decision_type:expr, $($arg:tt)*) => {
        {
            let context = format!($($arg)*);
            gst::debug!($cat, "{}", &context);
            if let Some(history) = $history {
                history.record($decision_type, Some(context));
            }
        }
    };
}

#[macro_export]
macro_rules! trace_decision {
    ($cat:expr, $history:expr, $decision_type:expr, $($arg:tt)*) => {
        {
            let context = format!($($arg)*);
            gst::trace!($cat, "{}", &context);
            if let Some(history) = $history {
                history.record($decision_type, Some(context));
            }
        }
    };
}

// Verbose logging controlled by environment variable
pub fn is_verbose_retry_logging() -> bool {
    std::env::var("GST_RTSP_VERBOSE_RETRY")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

// Decision formatting helpers
pub fn format_retry_decision(
    attempt: u32,
    strategy: &str,
    delay: Duration,
    reason: &str,
) -> String {
    format!(
        "Retry decision: attempt={}, strategy={}, delay={}ms, reason={}",
        attempt,
        strategy,
        delay.as_millis(),
        reason
    )
}

pub fn format_pattern_detection(pattern: &str, confidence: f32, evidence: &str) -> String {
    format!(
        "Pattern detected: type={}, confidence={:.2}, evidence={}",
        pattern, confidence, evidence
    )
}

pub fn format_strategy_change(from: &str, to: &str, reason: &str) -> String {
    format!(
        "Strategy changed: from={}, to={}, reason={}",
        from, to, reason
    )
}

pub fn format_racing_update(mode: &str, reason: &str) -> String {
    format!("Racing mode updated: mode={}, reason={}", mode, reason)
}

pub fn format_adaptive_learning(strategy: &str, confidence: f32, phase: &str) -> String {
    format!(
        "Adaptive learning: strategy={}, confidence={:.2}, phase={}",
        strategy, confidence, phase
    )
}

// Tests module
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_history() {
        let history = DecisionHistory::new(3);
        
        // Record some decisions
        history.record(
            DecisionType::RetryDelay {
                attempt: 1,
                strategy: "exponential".to_string(),
                delay_ms: 1000,
                reason: "Initial failure".to_string(),
            },
            None,
        );
        
        history.record(
            DecisionType::PatternDetected {
                pattern: "lossy".to_string(),
                confidence: 0.85,
                evidence: "50% packet loss".to_string(),
            },
            Some("Network analysis".to_string()),
        );
        
        history.record(
            DecisionType::StrategyChanged {
                from: "exponential".to_string(),
                to: "exponential-jitter".to_string(),
                reason: "High packet loss detected".to_string(),
            },
            None,
        );
        
        // Should keep only last 3
        history.record(
            DecisionType::RacingModeUpdate {
                mode: "aggressive".to_string(),
                reason: "Multiple failures".to_string(),
            },
            None,
        );
        
        let decisions = history.get_history();
        assert_eq!(decisions.len(), 3);
        
        // Check JSON serialization
        let json = history.get_history_json();
        assert!(json.contains("\"pattern\": \"lossy\""));
    }

    #[test]
    fn test_debug_categories() {
        // Ensure categories are initialized
        let _ = &*CAT_RETRY;
        let _ = &*CAT_AUTO;
        let _ = &*CAT_ADAPTIVE;
        let _ = &*CAT_RACING;
        
        assert_eq!(CAT_RETRY.name(), "rtspsrc2-retry");
        assert_eq!(CAT_AUTO.name(), "rtspsrc2-auto");
        assert_eq!(CAT_ADAPTIVE.name(), "rtspsrc2-adaptive");
        assert_eq!(CAT_RACING.name(), "rtspsrc2-racing");
    }
}