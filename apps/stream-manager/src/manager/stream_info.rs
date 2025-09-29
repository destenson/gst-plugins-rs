use std::time::Duration;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use super::{HealthStatus, StreamStatistics};
use crate::config::StreamConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    pub id: String,
    pub config: StreamConfig,
    pub state: StreamState,
    pub health: StreamHealth,
    pub recording_state: RecordingState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StreamState {
    Idle,
    Starting,
    Running,
    Stopping,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamHealth {
    pub is_healthy: bool,
    pub last_frame_time: Option<DateTime<Utc>>,
    pub frames_received: u64,
    pub frames_dropped: u64,
    pub bitrate_mbps: f64,
}

impl Default for StreamHealth {
    fn default() -> Self {
        Self {
            is_healthy: false,
            last_frame_time: None,
            frames_received: 0,
            frames_dropped: 0,
            bitrate_mbps: 0.0,
        }
    }
}

impl From<HealthStatus> for bool {
    fn from(status: HealthStatus) -> Self {
        matches!(status, HealthStatus::Healthy)
    }
}

impl From<&StreamStatistics> for StreamHealth {
    fn from(stats: &StreamStatistics) -> Self {
        // Calculate bitrate, avoiding division by zero
        let elapsed_secs = stats.last_update.elapsed().as_secs_f64();
        let bitrate_mbps = if elapsed_secs > 0.0 {
            (stats.bytes_received as f64 * 8.0) / 1_000_000.0 / elapsed_secs
        } else {
            0.0
        };

        Self {
            is_healthy: true, // Will be overridden by actual health status
            last_frame_time: Some(Utc::now()),
            frames_received: stats.packets_received,
            frames_dropped: stats.dropped_frames,
            bitrate_mbps,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingState {
    pub is_recording: bool,
    pub current_file: Option<String>,
    pub duration: Option<Duration>,
    pub bytes_written: Option<u64>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            is_recording: false,
            current_file: None,
            duration: None,
            bytes_written: None,
        }
    }
}