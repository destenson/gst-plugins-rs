#![allow(unused)]
// GStreamer RTSP Source 2
//
// Copyright (C) 2023 Tim-Philipp Müller <tim centricular com>
// Copyright (C) 2023-2024 Nirbheek Chauhan <nirbheek centricular com>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0
//
// https://www.rfc-editor.org/rfc/rfc2326.html

use std::collections::{btree_set::BTreeSet, HashMap, VecDeque};
use std::convert::TryFrom;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::num::NonZeroUsize;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use std::sync::LazyLock;

use futures::{Sink, SinkExt, Stream, StreamExt};
use socket2::Socket;

// Import the new RtspError type from error module
use super::error::RtspError;
use tokio::net::UdpSocket;
use tokio::runtime;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time;

use rtsp_types::headers::{
    CSeq, NptRange, NptTime, Public, Range, RtpInfos, RtpLowerTransport, RtpProfile, RtpTransport,
    RtpTransportParameters, Session, SmpteRange, SmpteTime, SmpteType, Transport, TransportMode,
    Transports, UtcRange, UtcTime, ACCEPT, CONTENT_BASE, CONTENT_LOCATION, USER_AGENT,
};
use rtsp_types::{Message, Method, Request, Response, StatusCode, Version};

use lru::LruCache;
use url::Url;

use gst::buffer::{MappedBuffer, Readable};
use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_net::gio;
use gst_rtsp;
use gst_rtsp::rtsp_message::RTSPMessage;
use gst_sdp;

#[cfg(feature = "tracing")]
use tracing::{event, Level};

use super::auth::{self, AuthState};
use super::body::Body;
use super::buffer_mode::BufferMode;
use super::sdp;
use super::transport::RtspTransportInfo;

const DEFAULT_LOCATION: Option<Url> = None;
const DEFAULT_TIMEOUT: gst::ClockTime = gst::ClockTime::from_seconds(5);
const DEFAULT_PORT_START: u16 = 0;
// Priority list has TCP first as it's most reliable, then UDP, then multicast
const DEFAULT_PROTOCOLS: &str = "tcp,udp,udp-mcast";
// Equal to MTU + 8 by default to avoid incorrectly detecting an MTU sized buffer as having
// possibly overflown our receive buffer, and triggering a doubling of the buffer sizes.
const DEFAULT_RECEIVE_MTU: u32 = 1500 + 8;

// Jitterbuffer control defaults (matching original rtspsrc)
const DEFAULT_LATENCY_MS: u32 = 2000;
const DEFAULT_DROP_ON_LATENCY: bool = false;
const DEFAULT_PROBATION: u32 = 2;

// RTCP control defaults (matching original rtspsrc)
const DEFAULT_DO_RTCP: bool = true;
const DEFAULT_DO_RETRANSMISSION: bool = true;
const DEFAULT_MAX_RTCP_RTP_TIME_DIFF: i32 = -1;

// Keep-alive and timeout defaults (matching original rtspsrc)
const DEFAULT_DO_RTSP_KEEP_ALIVE: bool = true;
const DEFAULT_TCP_TIMEOUT: u64 = 20000000; // 20 seconds in microseconds
const DEFAULT_TEARDOWN_TIMEOUT: u64 = 100000000; // 100ms in nanoseconds
const DEFAULT_UDP_RECONNECT: bool = true;

// Network interface defaults (matching original rtspsrc)
const DEFAULT_MULTICAST_IFACE: Option<String> = None;
const DEFAULT_PORT_RANGE: Option<String> = None;
const DEFAULT_UDP_BUFFER_SIZE: i32 = 524288; // 512KB default

// Source behavior defaults (matching original rtspsrc)
const DEFAULT_IS_LIVE: bool = true;
const DEFAULT_CONNECTION_SPEED: u64 = 0; // 0 = unknown/unspecified

// Timestamp synchronization defaults (matching original rtspsrc)
const DEFAULT_NTP_SYNC: bool = false;
const DEFAULT_RFC7273_SYNC: bool = false;
const DEFAULT_MAX_TS_OFFSET: i64 = 3000000000; // 3 seconds in nanoseconds
const DEFAULT_MAX_TS_OFFSET_ADJUSTMENT: u64 = 0; // 0 = no limit
const DEFAULT_ADD_REFERENCE_TIMESTAMP_META: bool = false;

// RTSP version default (matching original rtspsrc)
const DEFAULT_RTSP_VERSION: RtspVersion = RtspVersion::V1_0;

// RTP-specific defaults (matching original rtspsrc)
const DEFAULT_RTP_BLOCKSIZE: u32 = 0; // 0 = disabled, let server decide
const DEFAULT_TCP_TIMESTAMP: bool = false;
// SDES default is None - will be an Option<gst::Structure>

// TLS/SSL security defaults (matching original rtspsrc)
// Default TLS validation flags: validate all (0x7f = all flags set)
const DEFAULT_TLS_VALIDATION_FLAGS: gio::TlsCertificateFlags =
    gio::TlsCertificateFlags::VALIDATE_ALL;

// Buffer queue management constants
const DEFAULT_MAX_BUFFERED_BUFFERS: usize = 100;
const DEFAULT_MAX_BUFFERED_BYTES: usize = 10 * 1024 * 1024; // 10 MB

// Stream selection defaults
const DEFAULT_REQUIRE_ALL_STREAMS: bool = false;
const BUFFER_QUEUE_CLEANUP_THRESHOLD: f32 = 0.8; // Start cleanup at 80% capacity


const MAX_MESSAGE_SIZE: usize = 1024 * 1024;
const MAX_BIND_PORT_RETRY: u16 = 100;
const UDP_PACKET_MAX_SIZE: u32 = 65535 - 8;
const RTCP_ADDR_CACHE_SIZE: usize = 100;

static RTCP_CAPS: LazyLock<gst::Caps> =
    LazyLock::new(|| gst::Caps::from(gst::Structure::new_empty("application/x-rtcp")));

// Hardcoded for now
const DEFAULT_USER_AGENT: &str = concat!(
    "GStreamer rtspsrc2 ",
    env!("CARGO_PKG_VERSION"),
    "-",
    env!("COMMIT_ID")
);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RtspProtocol {
    UdpMulticast,
    Udp,
    Tcp,
    Http, // HTTP tunneling
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpTunnelMode {
    #[default]
    Auto, // Automatically detect need for tunneling
    Never,  // Never use HTTP tunneling
    Always, // Always use HTTP tunneling
}

impl fmt::Display for RtspProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            RtspProtocol::Udp => write!(f, "udp"),
            RtspProtocol::UdpMulticast => write!(f, "udp-mcast"),
            RtspProtocol::Tcp => write!(f, "tcp"),
            RtspProtocol::Http => write!(f, "http"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeekFormat {
    Npt,   // Normal Play Time (default)
    Smpte, // SMPTE time code
    Clock, // Absolute UTC time
}

impl Default for SeekFormat {
    fn default() -> Self {
        SeekFormat::Npt
    }
}

// NTP time source enum (matching original rtspsrc)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstRtspSrcNtpTimeSource")]
pub enum NtpTimeSource {
    #[default]
    #[enum_value(name = "NTP time based on realtime clock", nick = "ntp")]
    Ntp,
    #[enum_value(name = "UNIX time based on realtime clock", nick = "unix")]
    Unix,
    #[enum_value(name = "Running time based on pipeline clock", nick = "running-time")]
    RunningTime,
    #[enum_value(name = "Pipeline clock time", nick = "clock-time")]
    ClockTime,
}

// RTSP version enum (matching original GStreamer GST_RTSP_VERSION_* values)
#[derive(Debug, Clone, Copy, PartialEq, Eq, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstRtspSrcRtspVersion")]
pub enum RtspVersion {
    #[enum_value(name = "GST_RTSP_VERSION_INVALID", nick = "invalid")]
    Invalid = 0,
    #[enum_value(name = "GST_RTSP_VERSION_1_0", nick = "1-0")]
    V1_0 = 16,
    #[enum_value(name = "GST_RTSP_VERSION_1_1", nick = "1-1")]
    V1_1 = 17,
    #[enum_value(name = "GST_RTSP_VERSION_2_0", nick = "2-0")]
    V2_0 = 32,
}

impl Default for RtspVersion {
    fn default() -> Self {
        RtspVersion::V1_0
    }
}

// NAT traversal method enum (matching original rtspsrc)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstRtspSrcNatMethod")]
pub enum NatMethod {
    #[enum_value(name = "None", nick = "none")]
    None = 0,
    #[default]
    #[enum_value(name = "Send Dummy packets", nick = "dummy")]
    Dummy = 1,
}

// Backchannel type enum (matching original rtspsrc)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, glib::Enum)]
#[repr(u32)]
#[enum_type(name = "GstRtspSrcBackchannelType")]
pub enum BackchannelType {
    #[default]
    #[enum_value(name = "No backchannel", nick = "none")]
    None = 0,
    #[enum_value(name = "ONVIF audio backchannel", nick = "onvif")]
    Onvif = 1,
}

// Stream selection flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StreamSelection: u32 {
        const AUDIO = 0b00000001;
        const VIDEO = 0b00000010;
        const METADATA = 0b00000100;
        const APPLICATION = 0b00001000;
        const ALL = Self::AUDIO.bits() | Self::VIDEO.bits() | Self::METADATA.bits() | Self::APPLICATION.bits();
    }
}

impl Default for StreamSelection {
    fn default() -> Self {
        StreamSelection::ALL
    }
}

impl StreamSelection {
    fn from_string(s: &str) -> Self {
        let mut flags = StreamSelection::empty();
        for part in s.split(',') {
            match part.trim().to_lowercase().as_str() {
                "audio" => flags |= StreamSelection::AUDIO,
                "video" => flags |= StreamSelection::VIDEO,
                "metadata" => flags |= StreamSelection::METADATA,
                "application" => flags |= StreamSelection::APPLICATION,
                "all" => flags = StreamSelection::ALL,
                "none" => flags = StreamSelection::empty(),
                _ => {}
            }
        }
        if flags.is_empty() {
            StreamSelection::ALL
        } else {
            flags
        }
    }

    fn to_string(&self) -> String {
        if *self == StreamSelection::ALL {
            return "all".to_string();
        }
        if self.is_empty() {
            return "none".to_string();
        }
        let mut parts = Vec::new();
        if self.contains(StreamSelection::AUDIO) {
            parts.push("audio");
        }
        if self.contains(StreamSelection::VIDEO) {
            parts.push("video");
        }
        if self.contains(StreamSelection::METADATA) {
            parts.push("metadata");
        }
        if self.contains(StreamSelection::APPLICATION) {
            parts.push("application");
        }
        parts.join(",")
    }

    fn should_select_media(&self, media_type: &str) -> bool {
        match media_type.to_lowercase().as_str() {
            "audio" => self.contains(StreamSelection::AUDIO),
            "video" => self.contains(StreamSelection::VIDEO),
            "metadata" => self.contains(StreamSelection::METADATA),
            "application" => self.contains(StreamSelection::APPLICATION),
            _ => false,
        }
    }
}

/// Buffer queue entry for handling unlinked pads
#[derive(Debug, Clone)]
struct QueuedBuffer {
    buffer: gst::Buffer,
    appsrc: gst_app::AppSrc,
    timestamp: gst::ClockTime,
}

/// Buffer queue manager for handling data when pads are unlinked
#[derive(Debug)]
struct BufferQueue {
    buffers: VecDeque<QueuedBuffer>,
    total_bytes: usize,
    max_buffers: usize,
    max_bytes: usize,
}

impl BufferQueue {
    fn new(max_buffers: usize, max_bytes: usize) -> Self {
        Self {
            buffers: VecDeque::new(),
            total_bytes: 0,
            max_buffers,
            max_bytes,
        }
    }

    fn push(
        &mut self,
        buffer: gst::Buffer,
        appsrc: gst_app::AppSrc,
        timestamp: gst::ClockTime,
    ) -> bool {
        let buffer_size = buffer.size();

        // Check if we're over capacity and need to drop buffers
        while (self.buffers.len() >= self.max_buffers
            || self.total_bytes + buffer_size > self.max_bytes)
            && !self.buffers.is_empty()
        {
            if let Some(dropped_buffer) = self.buffers.pop_front() {
                self.total_bytes = self
                    .total_bytes
                    .saturating_sub(dropped_buffer.buffer.size());
                gst::warning!(
                    CAT,
                    "Dropping buffer due to queue overflow - current: {} buffers, {} bytes",
                    self.buffers.len(),
                    self.total_bytes
                );
            }
        }

        // If we still can't fit the new buffer, reject it
        if buffer_size > self.max_bytes {
            gst::error!(
                CAT,
                "Buffer size {} exceeds max queue capacity {}",
                buffer_size,
                self.max_bytes
            );
            return false;
        }

        self.buffers.push_back(QueuedBuffer {
            buffer: buffer.clone(),
            appsrc: appsrc.clone(),
            timestamp,
        });
        self.total_bytes += buffer_size;

        gst::debug!(
            CAT,
            "Queued buffer - queue size: {} buffers, {} bytes",
            self.buffers.len(),
            self.total_bytes
        );

        true
    }

    fn flush_to_appsrc(&mut self, target_appsrc: &gst_app::AppSrc) -> usize {
        let mut flushed_count = 0;
        let mut remaining_buffers = VecDeque::new();

        while let Some(queued) = self.buffers.pop_front() {
            self.total_bytes = self.total_bytes.saturating_sub(queued.buffer.size());

            if queued.appsrc.name() == target_appsrc.name() {
                // Try to push the buffer
                let buffer_size = queued.buffer.size();
                match target_appsrc.push_buffer(queued.buffer.clone()) {
                    Ok(_) => {
                        gst::debug!(
                            CAT,
                            "Successfully flushed queued buffer to {}",
                            target_appsrc.name()
                        );
                        flushed_count += 1;
                    }
                    Err(err) => {
                        gst::warning!(
                            CAT,
                            "Failed to flush queued buffer to {}: {}",
                            target_appsrc.name(),
                            err
                        );
                        // Put it back if it failed
                        self.total_bytes += buffer_size;
                        remaining_buffers.push_back(queued);
                    }
                }
            } else {
                // Keep buffers for other appsrcs
                let buffer_size = queued.buffer.size();
                self.total_bytes += buffer_size;
                remaining_buffers.push_back(queued);
            }
        }

        self.buffers = remaining_buffers;

        if flushed_count > 0 {
            gst::info!(
                CAT,
                "Flushed {} queued buffers to {} - remaining: {} buffers, {} bytes",
                flushed_count,
                target_appsrc.name(),
                self.buffers.len(),
                self.total_bytes
            );
        }

        flushed_count
    }

    fn clear(&mut self) {
        let cleared_count = self.buffers.len();
        let cleared_bytes = self.total_bytes;

        self.buffers.clear();
        self.total_bytes = 0;

        if cleared_count > 0 {
            gst::info!(
                CAT,
                "Cleared {} queued buffers ({} bytes) from buffer queue",
                cleared_count,
                cleared_bytes
            );
        }
    }

    fn len(&self) -> usize {
        self.buffers.len()
    }

    fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    fn is_over_threshold(&self) -> bool {
        let buffer_threshold = (self.max_buffers as f32 * BUFFER_QUEUE_CLEANUP_THRESHOLD) as usize;
        let bytes_threshold = (self.max_bytes as f32 * BUFFER_QUEUE_CLEANUP_THRESHOLD) as usize;

        self.buffers.len() >= buffer_threshold || self.total_bytes >= bytes_threshold
    }
}

impl Default for BufferQueue {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_BUFFERED_BUFFERS, DEFAULT_MAX_BUFFERED_BYTES)
    }
}

/// Wrapper around AppSrc that integrates with RtspSrc buffer queue management
#[derive(Debug, Clone)]
struct BufferingAppSrc {
    appsrc: gst_app::AppSrc,
    rtsp_src: super::RtspSrc,
}

impl BufferingAppSrc {
    /// Push buffer using the RtspSrc buffer queue system
    fn push_buffer(&self, buffer: gst::Buffer) -> Result<gst::FlowSuccess, gst::FlowError> {
        let rtsp_src_imp = self.rtsp_src.imp();
        rtsp_src_imp.push_buffer_with_queue(&self.appsrc, buffer)
    }

    /// Get the underlying AppSrc name for logging
    fn name(&self) -> glib::GString {
        self.appsrc.name()
    }

    /// Get current running time from the underlying AppSrc
    fn current_running_time(&self) -> Option<gst::ClockTime> {
        self.appsrc.current_running_time()
    }
}

#[derive(Debug, Clone)]
struct Settings {
    location: Option<Url>,
    port_start: u16,
    protocols: Vec<RtspProtocol>,
    timeout: gst::ClockTime,
    receive_mtu: u32,
    retry_strategy: super::retry::RetryStrategy,
    max_reconnection_attempts: i32,
    reconnection_timeout: gst::ClockTime,
    initial_retry_delay: gst::ClockTime,
    linear_retry_step: gst::ClockTime,
    connection_racing: super::connection_racer::ConnectionRacingStrategy,
    max_parallel_connections: u32,
    racing_delay_ms: u32,
    racing_timeout: gst::ClockTime,
    // Auto mode properties
    auto_detection_attempts: u32,
    auto_fallback_enabled: bool,
    seek_format: SeekFormat,
    // Authentication properties
    user_id: Option<String>,
    user_pw: Option<String>,
    // Stream selection properties
    select_streams: StreamSelection,
    stream_filter: Option<String>,
    require_all_streams: bool,
    // Jitterbuffer control properties
    latency_ms: u32,
    drop_on_latency: bool,
    probation: u32,
    buffer_mode: BufferMode,
    // RTCP control properties
    do_rtcp: bool,
    do_retransmission: bool,
    max_rtcp_rtp_time_diff: i32,
    // Keep-alive and timeout properties
    do_rtsp_keep_alive: bool,
    tcp_timeout: u64,
    teardown_timeout: u64,
    udp_reconnect: bool,
    // Network interface properties
    multicast_iface: Option<String>,
    port_range: Option<String>,
    udp_buffer_size: i32,
    // Source behavior properties
    is_live: bool,
    user_agent: String,
    connection_speed: u64,
    // Timestamp synchronization properties
    ntp_sync: bool,
    rfc7273_sync: bool,
    ntp_time_source: NtpTimeSource,
    max_ts_offset: i64,
    max_ts_offset_adjustment: u64,
    add_reference_timestamp_meta: bool,
    // RTSP version negotiation
    default_rtsp_version: RtspVersion,
    // RTP-specific properties
    rtp_blocksize: u32,
    tcp_timestamp: bool,
    sdes: Option<gst::Structure>,
    // TLS/SSL security properties
    // Note: tls_database and tls_interaction are stored separately in the element
    // because they're not Send+Sync and can't be in Settings which gets cloned
    tls_validation_flags: gio::TlsCertificateFlags,
    // Proxy and HTTP tunneling properties
    proxy: Option<String>,
    proxy_id: Option<String>,
    proxy_pw: Option<String>,
    extra_http_request_headers: Option<gst::Structure>,
    http_tunnel_mode: HttpTunnelMode,
    tunnel_port: u16,
    // NAT traversal properties
    nat_method: NatMethod,
    ignore_x_server_reply: bool,
    force_non_compliant_url: bool,
    // ONVIF backchannel properties
    backchannel: BackchannelType,
    onvif_mode: bool,
    onvif_rate_control: bool,
    #[cfg(feature = "adaptive")]
    adaptive_learning: bool,
    #[cfg(feature = "adaptive")]
    adaptive_persistence: bool,
    #[cfg(feature = "adaptive")]
    adaptive_cache_ttl: u64,
    #[cfg(feature = "adaptive")]
    adaptive_discovery_time: gst::ClockTime,
    #[cfg(feature = "adaptive")]
    adaptive_exploration_rate: f32,
    #[cfg(feature = "adaptive")]
    adaptive_confidence_threshold: f32,
    #[cfg(feature = "adaptive")]
    adaptive_change_detection: bool,

    // Compatibility properties (PRP-51)
    short_header: bool,         // Send minimal RTSP headers for broken encoders
    debug: bool,                // Deprecated: use GST_DEBUG instead
    use_pipeline_clock: bool,   // Deprecated: use ntp-time-source instead
    client_managed_mikey: bool, // Enable client-managed MIKEY mode for SRTP
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            location: DEFAULT_LOCATION,
            port_start: DEFAULT_PORT_START,
            timeout: DEFAULT_TIMEOUT,
            protocols: parse_protocols_str(DEFAULT_PROTOCOLS).unwrap(),
            receive_mtu: DEFAULT_RECEIVE_MTU,
            retry_strategy: super::retry::RetryStrategy::default(),
            max_reconnection_attempts: 5,
            reconnection_timeout: gst::ClockTime::from_seconds(30),
            initial_retry_delay: gst::ClockTime::from_seconds(1),
            linear_retry_step: gst::ClockTime::from_seconds(2),
            connection_racing: super::connection_racer::ConnectionRacingStrategy::default(),
            max_parallel_connections: 3,
            racing_delay_ms: 250,
            racing_timeout: gst::ClockTime::from_seconds(5),
            // Auto mode defaults
            auto_detection_attempts: 3,
            auto_fallback_enabled: true,
            #[cfg(feature = "adaptive")]
            adaptive_learning: true,
            #[cfg(feature = "adaptive")]
            adaptive_persistence: true,
            #[cfg(feature = "adaptive")]
            adaptive_cache_ttl: 7 * 24 * 3600, // 7 days in seconds
            #[cfg(feature = "adaptive")]
            adaptive_discovery_time: gst::ClockTime::from_seconds(30),
            seek_format: SeekFormat::default(),
            // Authentication properties
            user_id: None,
            user_pw: None,
            // Stream selection properties
            select_streams: StreamSelection::default(),
            stream_filter: None,
            require_all_streams: DEFAULT_REQUIRE_ALL_STREAMS,
            // Jitterbuffer control properties
            latency_ms: DEFAULT_LATENCY_MS,
            drop_on_latency: DEFAULT_DROP_ON_LATENCY,
            probation: DEFAULT_PROBATION,
            buffer_mode: BufferMode::default(),
            // RTCP control properties
            do_rtcp: DEFAULT_DO_RTCP,
            do_retransmission: DEFAULT_DO_RETRANSMISSION,
            max_rtcp_rtp_time_diff: DEFAULT_MAX_RTCP_RTP_TIME_DIFF,
            // Keep-alive and timeout properties
            do_rtsp_keep_alive: DEFAULT_DO_RTSP_KEEP_ALIVE,
            tcp_timeout: DEFAULT_TCP_TIMEOUT,
            teardown_timeout: DEFAULT_TEARDOWN_TIMEOUT,
            udp_reconnect: DEFAULT_UDP_RECONNECT,
            // Network interface properties
            multicast_iface: DEFAULT_MULTICAST_IFACE,
            port_range: DEFAULT_PORT_RANGE,
            udp_buffer_size: DEFAULT_UDP_BUFFER_SIZE,
            // Source behavior properties
            is_live: DEFAULT_IS_LIVE,
            user_agent: DEFAULT_USER_AGENT.to_string(),
            connection_speed: DEFAULT_CONNECTION_SPEED,
            // Timestamp synchronization properties
            ntp_sync: DEFAULT_NTP_SYNC,
            rfc7273_sync: DEFAULT_RFC7273_SYNC,
            ntp_time_source: NtpTimeSource::default(),
            max_ts_offset: DEFAULT_MAX_TS_OFFSET,
            max_ts_offset_adjustment: DEFAULT_MAX_TS_OFFSET_ADJUSTMENT,
            add_reference_timestamp_meta: DEFAULT_ADD_REFERENCE_TIMESTAMP_META,
            default_rtsp_version: DEFAULT_RTSP_VERSION,
            // RTP-specific properties
            rtp_blocksize: DEFAULT_RTP_BLOCKSIZE,
            tcp_timestamp: DEFAULT_TCP_TIMESTAMP,
            sdes: None,
            // TLS/SSL security properties
            tls_validation_flags: DEFAULT_TLS_VALIDATION_FLAGS,
            // Proxy and HTTP tunneling properties
            proxy: None,
            proxy_id: None,
            proxy_pw: None,
            extra_http_request_headers: None,
            http_tunnel_mode: HttpTunnelMode::default(),
            tunnel_port: 80,
            // NAT traversal properties
            nat_method: NatMethod::default(), // Dummy
            ignore_x_server_reply: false,
            force_non_compliant_url: false,
            // ONVIF backchannel properties
            backchannel: BackchannelType::default(), // None
            onvif_mode: false,
            onvif_rate_control: true,
            #[cfg(feature = "adaptive")]
            adaptive_exploration_rate: 0.1,
            #[cfg(feature = "adaptive")]
            adaptive_confidence_threshold: 0.8,
            #[cfg(feature = "adaptive")]
            adaptive_change_detection: true,
            // Compatibility properties (PRP-51)
            short_header: false,         // Default: false (send full headers)
            debug: false,                // Default: false (deprecated)
            use_pipeline_clock: false,   // Default: false (deprecated)
            client_managed_mikey: false, // Default: false (server-managed mode)
        }
    }
}

#[derive(Debug)]
enum Commands {
    Play,
    Pause,
    Seek {
        position: gst::ClockTime,
        flags: gst::SeekFlags,
    },
    Teardown(Option<oneshot::Sender<()>>),
    Data(rtsp_types::Data<Body>),
    Reconnect,
    GetParameter {
        parameters: Option<Vec<String>>,
        promise: gst::Promise,
    },
    SetParameter {
        parameters: Vec<(String, String)>,
        promise: gst::Promise,
    },
}

#[derive(Debug)]
pub struct RtspSrc {
    settings: Mutex<Settings>,
    task_handle: Mutex<Option<JoinHandle<()>>>,
    command_queue: Mutex<Option<mpsc::Sender<Commands>>>,
    buffer_queue: Arc<Mutex<BufferQueue>>,
    // TODO: tls_database and tls_interaction properties cannot be properly stored
    // because gio::TlsDatabase and gio::TlsInteraction are not Send+Sync.
    // These properties will need to be implemented after removing Tokio.
    #[cfg(feature = "telemetry")]
    metrics: super::telemetry::RtspMetrics,
}

impl Default for RtspSrc {
    fn default() -> Self {
        Self {
            settings: Mutex::new(Settings::default()),
            task_handle: Mutex::new(None),
            command_queue: Mutex::new(None),
            buffer_queue: Arc::new(Mutex::new(BufferQueue::default())),
            #[cfg(feature = "telemetry")]
            metrics: super::telemetry::RtspMetrics::default(),
        }
    }
}

#[derive(thiserror::Error, Debug)]
// TODO: This is the old error enum - should be migrated to use super::error::RtspError
pub enum OldRtspError {
    #[error("Generic I/O error")]
    IOGeneric(#[from] std::io::Error),
    #[error("Read I/O error")]
    Read(#[from] super::tcp_message::ReadError),
    #[error("RTSP header parse error")]
    HeaderParser(#[from] rtsp_types::headers::HeaderParseError),
    #[error("SDP parse error")]
    SDPParser(#[from] sdp_types::ParserError),
    #[error("Unexpected RTSP message: expected, received")]
    UnexpectedMessage(&'static str, rtsp_types::Message<Body>),
    #[error("Invalid RTSP message")]
    InvalidMessage(&'static str),
    #[error("Fatal error")]
    Fatal(String),
}

pub(crate) static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rtspsrc2",
        gst::DebugColorFlags::empty(),
        Some("RTSP source"),
    )
});

static RUNTIME: LazyLock<runtime::Runtime> = LazyLock::new(|| {
    runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(1)
        .build()
        .unwrap()
});

fn parse_protocols_str(s: &str) -> Result<Vec<RtspProtocol>, glib::Error> {
    let mut acc = Vec::new();
    if s.is_empty() {
        return Err(glib::Error::new(
            gst::CoreError::Failed,
            "Protocols list is empty",
        ));
    }
    for each in s.split(',') {
        match each {
            "udp-mcast" => acc.push(RtspProtocol::UdpMulticast),
            "udp" => acc.push(RtspProtocol::Udp),
            "tcp" => acc.push(RtspProtocol::Tcp),
            "http" => acc.push(RtspProtocol::Http),
            _ => {
                return Err(glib::Error::new(
                    gst::CoreError::Failed,
                    &format!("Unsupported RTSP protocol: {each}"),
                ))
            }
        }
    }
    Ok(acc)
}

/// Validates port range format (e.g., "3000-3005")
/// Returns Ok(()) if valid or None, Err if invalid format
fn validate_port_range(range: Option<&str>) -> Result<(), glib::Error> {
    if let Some(range_str) = range {
        if range_str.is_empty() {
            return Ok(()); // Empty string is treated as None
        }

        let parts: Vec<&str> = range_str.split('-').collect();
        if parts.len() != 2 {
            return Err(glib::Error::new(
                gst::CoreError::Failed,
                &format!(
                    "Invalid port range format '{}', expected 'start-end'",
                    range_str
                ),
            ));
        }

        let start = parts[0].parse::<u16>().map_err(|_| {
            glib::Error::new(
                gst::CoreError::Failed,
                &format!("Invalid start port in range '{}'", range_str),
            )
        })?;

        let end = parts[1].parse::<u16>().map_err(|_| {
            glib::Error::new(
                gst::CoreError::Failed,
                &format!("Invalid end port in range '{}'", range_str),
            )
        })?;

        if start > end {
            return Err(glib::Error::new(
                gst::CoreError::Failed,
                &format!("Invalid port range '{}', start must be <= end", range_str),
            ));
        }

        // Check for even number of ports (RTP + RTCP pairs)
        let port_count = (end - start + 1) as usize;
        if port_count % 2 != 0 {
            return Err(glib::Error::new(
                gst::CoreError::Failed,
                &format!(
                    "Port range '{}' must contain an even number of ports for RTP/RTCP pairs",
                    range_str
                ),
            ));
        }
    }
    Ok(())
}

impl RtspSrc {
    fn set_location(&self, uri: Option<&str>) -> Result<(), glib::Error> {
        if self.obj().current_state() > gst::State::Ready {
            return Err(glib::Error::new(
                gst::URIError::BadState,
                "Changing the 'location' property on a started 'rtspsrc2' is not supported",
            ));
        }

        let mut settings = self.settings.lock().unwrap();

        let Some(uri) = uri else {
            settings.location = DEFAULT_LOCATION;
            return Ok(());
        };

        let uri = Url::parse(uri).map_err(|err| {
            glib::Error::new(
                gst::URIError::BadUri,
                &format!("Failed to parse URI '{uri}': {err:?}"),
            )
        })?;

        // Extract credentials from URI if present
        if uri.password().is_some() || !uri.username().is_empty() {
            let username = uri.username();
            let password = uri.password();
            
            // Extract credentials from URL
            if !username.is_empty() {
                gst::debug!(CAT, imp = self, "Setting user-id from URI: {}", username);
                settings.user_id = Some(username.to_string());
            }
            
            if let Some(password) = password {
                gst::debug!(CAT, imp = self, "Setting user-pw from URI");
                settings.user_pw = Some(password.to_string());
            }
        }

        match (uri.host_str(), uri.port()) {
            (Some(_), Some(_)) | (Some(_), None) => Ok(()),
            _ => Err(glib::Error::new(gst::URIError::BadUri, "Invalid host")),
        }?;

        let protocols: &[RtspProtocol] = match uri.scheme() {
            "rtsp" => &settings.protocols,
            "rtspu" => &[RtspProtocol::UdpMulticast, RtspProtocol::Udp],
            "rtspt" => &[RtspProtocol::Tcp],
            "rtsph" => &[RtspProtocol::Tcp], // RTSP over HTTPS (HTTP tunneling)
            "rtsp-sdp" => &settings.protocols, // SDP-only session
            "rtsps" => &settings.protocols,  // RTSP over TLS/SSL
            "rtspsu" => &[RtspProtocol::UdpMulticast, RtspProtocol::Udp], // RTSP over TLS/SSL with UDP
            "rtspst" => &[RtspProtocol::Tcp], // RTSP over TLS/SSL with TCP
            "rtspsh" => &[RtspProtocol::Tcp], // RTSP over TLS/SSL with HTTPS tunneling
            scheme => {
                return Err(glib::Error::new(
                    gst::URIError::UnsupportedProtocol,
                    &format!("Unsupported URI scheme '{scheme}'"),
                ));
            }
        };

        if !settings.protocols.iter().any(|p| protocols.contains(p)) {
            return Err(glib::Error::new(
                gst::URIError::UnsupportedProtocol,
                &format!(
                    "URI scheme '{}' does not match allowed protocols: {:?}",
                    uri.scheme(),
                    settings.protocols,
                ),
            ));
        }

        settings.protocols = protocols.to_vec();
        settings.location = Some(uri);

        Ok(())
    }

    fn set_protocols(&self, protocol_s: Option<&str>) -> Result<(), glib::Error> {
        if self.obj().current_state() > gst::State::Ready {
            return Err(glib::Error::new(
                gst::CoreError::Failed,
                "Changing the 'protocols' property on a started 'rtspsrc2' is not supported",
            ));
        }

        let mut settings = self.settings.lock().unwrap();

        settings.protocols = match protocol_s {
            Some(s) => parse_protocols_str(s)?,
            None => parse_protocols_str(DEFAULT_PROTOCOLS).unwrap(),
        };

        Ok(())
    }

    /// Try to push buffer to AppSrc, queuing it if the pad is not linked
    fn push_buffer_with_queue(
        &self,
        appsrc: &gst_app::AppSrc,
        buffer: gst::Buffer,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let timestamp = appsrc
            .current_running_time()
            .unwrap_or(gst::ClockTime::ZERO);

        match appsrc.push_buffer(buffer.clone()) {
            Ok(success) => {
                gst::trace!(CAT, "Successfully pushed buffer to {}", appsrc.name());
                Ok(success)
            }
            Err(gst::FlowError::NotLinked) => {
                // Queue the buffer for later flushing when pad gets linked
                let mut buffer_queue = self.buffer_queue.lock().unwrap();
                if buffer_queue.push(buffer, appsrc.clone(), timestamp) {
                    gst::debug!(
                        CAT,
                        "Queued buffer for {} (pad not linked yet)",
                        appsrc.name()
                    );

                    // Warn if queue is getting full
                    if buffer_queue.is_over_threshold() {
                        gst::warning!(
                            CAT,
                            "Buffer queue is {}% full - {} buffers, {} bytes",
                            (buffer_queue.len() as f32 / buffer_queue.max_buffers as f32 * 100.0)
                                as u32,
                            buffer_queue.len(),
                            buffer_queue.total_bytes()
                        );
                    }

                    Ok(gst::FlowSuccess::Ok)
                } else {
                    gst::error!(
                        CAT,
                        "Failed to queue buffer for {} - queue full or buffer too large",
                        appsrc.name()
                    );
                    Err(gst::FlowError::Error)
                }
            }
            Err(gst::FlowError::Flushing) => {
                gst::debug!(
                    CAT,
                    "AppSrc {} is flushing - dropping buffer",
                    appsrc.name()
                );
                Ok(gst::FlowSuccess::Ok)
            }
            Err(err) => {
                gst::error!(CAT, "Failed to push buffer to {}: {}", appsrc.name(), err);
                Err(err)
            }
        }
    }

    /// Flush queued buffers for a specific AppSrc when its pad gets linked
    fn flush_queued_buffers(&self, appsrc: &gst_app::AppSrc) {
        let mut buffer_queue = self.buffer_queue.lock().unwrap();
        let flushed_count = buffer_queue.flush_to_appsrc(appsrc);

        if flushed_count > 0 {
            gst::info!(
                CAT,
                "Flushed {} queued buffers to {} after pad link",
                flushed_count,
                appsrc.name()
            );
        }
    }

    /// Clear all queued buffers (called on state changes or errors)
    fn clear_buffer_queue(&self) {
        let mut buffer_queue = self.buffer_queue.lock().unwrap();
        buffer_queue.clear();
    }

    /// Get buffer queue statistics for debugging/telemetry
    fn buffer_queue_stats(&self) -> (usize, usize) {
        let buffer_queue = self.buffer_queue.lock().unwrap();
        (buffer_queue.len(), buffer_queue.total_bytes())
    }

    /// Create an AppSrc wrapper that handles buffering
    fn create_buffering_appsrc(&self, appsrc: gst_app::AppSrc) -> BufferingAppSrc {
        BufferingAppSrc {
            appsrc,
            rtsp_src: self.obj().clone(),
        }
    }

    /// Emit the on-sdp signal (placeholder)
    /// This function will be called when an SDP message is received from the RTSP server.
    /// Applications can connect to this signal to inspect or modify the SDP before stream setup.
    #[allow(dead_code)]
    fn emit_on_sdp(&self, sdp: &gst_sdp::SDPMessage) {
        let obj = self.obj();
        gst::debug!(CAT, obj = obj, "Emitting on-sdp signal");
        obj.emit_by_name::<()>("on-sdp", &[sdp]);
    }

    /// Emit the select-stream signal (placeholder)
    /// This function will be called for each stream found in the SDP.
    /// Applications can connect to this signal to control which streams to activate.
    /// Returns true if the stream should be selected, false otherwise.
    #[allow(dead_code)]
    fn emit_select_stream(&self, stream_id: u32, caps: &gst::Caps) -> bool {
        let obj = self.obj();
        gst::debug!(
            CAT,
            obj = obj,
            "Emitting select-stream signal for stream {}",
            stream_id
        );
        obj.emit_by_name::<bool>("select-stream", &[&stream_id, caps])
    }

    /// Emit the new-manager signal (placeholder)
    /// This function will be called when the RTP manager (rtpbin) is created.
    /// Applications can connect to this signal to configure the RTP manager.
    #[allow(dead_code)]
    fn emit_new_manager(&self, manager: &gst::Element) {
        let obj = self.obj();
        gst::debug!(CAT, obj = obj, "Emitting new-manager signal");
        obj.emit_by_name::<()>("new-manager", &[manager]);
    }

    /// Emit the accept-certificate signal (placeholder)
    /// This function will be called when TLS certificate validation is needed.
    /// Applications can connect to this signal to perform custom certificate validation.
    /// Returns true to accept the certificate, false to reject it.
    #[allow(dead_code)]
    fn emit_accept_certificate(
        &self,
        connection: &gio::TlsConnection,
        certificate: &gio::TlsCertificate,
        flags: gio::TlsCertificateFlags,
    ) -> bool {
        let obj = self.obj();
        gst::debug!(
            CAT,
            obj = obj,
            "Emitting accept-certificate signal with flags {:?}",
            flags
        );
        obj.emit_by_name::<bool>("accept-certificate", &[connection, certificate, &flags])
    }

    /// Emit the before-send signal (placeholder)
    /// This function will be called before sending RTSP messages.
    /// Applications can connect to this signal to modify or cancel RTSP messages.
    /// Returns true to send the message, false to cancel it.
    #[allow(dead_code)]
    fn emit_before_send(&self, message: &RTSPMessage) -> bool {
        let obj = self.obj();
        gst::debug!(CAT, obj = obj, "Emitting before-send signal");
        obj.emit_by_name::<bool>("before-send", &[message])
    }

    /// Emit the request-rtcp-key signal (placeholder)
    /// This function will be called to get RTCP encryption key for a stream.
    /// Applications can connect to this signal to provide SRTCP key parameters.
    /// Returns GstCaps with the encryption key parameters or None.
    #[allow(dead_code)]
    fn emit_request_rtcp_key(&self, stream_id: u32) -> Option<gst::Caps> {
        let obj = self.obj();
        gst::debug!(
            CAT,
            obj = obj,
            "Emitting request-rtcp-key signal for stream {}",
            stream_id
        );
        obj.emit_by_name::<Option<gst::Caps>>("request-rtcp-key", &[&stream_id])
    }

    /// Emit the request-rtp-key signal (placeholder)
    /// This function will be called to get RTP encryption key for a stream.
    /// Applications can connect to this signal to provide SRTP key parameters.
    /// Returns GstCaps with the encryption key parameters or None.
    #[allow(dead_code)]
    fn emit_request_rtp_key(&self, stream_id: u32) -> Option<gst::Caps> {
        let obj = self.obj();
        gst::debug!(
            CAT,
            obj = obj,
            "Emitting request-rtp-key signal for stream {}",
            stream_id
        );
        obj.emit_by_name::<Option<gst::Caps>>("request-rtp-key", &[&stream_id])
    }

    /// Emit soft-limit signal when jitterbuffer reaches soft threshold
    /// This signal is emitted when a jitterbuffer approaches its capacity limit,
    /// allowing applications to implement adaptive streaming strategies.
    /// This is a placeholder for future jitterbuffer monitoring implementation.
    #[allow(dead_code)]
    fn emit_soft_limit(&self, stream_id: u32) {
        let obj = self.obj();
        gst::debug!(
            CAT,
            obj = obj,
            "Emitting soft-limit signal for stream {}",
            stream_id
        );
        obj.emit_by_name::<()>("soft-limit", &[&stream_id]);
    }

    /// Emit hard-limit signal when jitterbuffer reaches hard threshold
    /// This signal is emitted when a jitterbuffer reaches critical capacity,
    /// indicating imminent buffer overflow and possible frame drops.
    /// This is a placeholder for future jitterbuffer monitoring implementation.
    #[allow(dead_code)]
    fn emit_hard_limit(&self, stream_id: u32) {
        let obj = self.obj();
        gst::debug!(
            CAT,
            obj = obj,
            "Emitting hard-limit signal for stream {}",
            stream_id
        );
        obj.emit_by_name::<()>("hard-limit", &[&stream_id]);
    }

    /// Handle get-parameter action signal
    /// Sends a GET_PARAMETER RTSP request for a single parameter
    /// Returns true if request could be sent, false otherwise
    fn handle_get_parameter(
        &self,
        parameter: &str,
        content_type: Option<&str>,
        promise: &gst::Promise,
    ) -> bool {
        let obj = self.obj();

        // Validate parameter name
        if parameter.is_empty() {
            gst::warning!(
                CAT,
                obj = obj,
                "get-parameter: parameter name cannot be empty"
            );
            return false;
        }

        gst::debug!(
            CAT,
            obj = obj,
            "get-parameter action called with parameter: {}, content_type: {:?}",
            parameter,
            content_type
        );

        // Send GET_PARAMETER command to async task
        let cmd_queue = self.cmd_queue();
        let parameters = Some(vec![parameter.to_string()]);
        let promise = promise.clone();

        RUNTIME.spawn(async move {
            let _ = cmd_queue
                .send(Commands::GetParameter {
                    parameters,
                    promise,
                })
                .await;
        });

        true
    }

    /// Handle get-parameters action signal
    /// Sends a GET_PARAMETER RTSP request for multiple parameters
    /// Returns true if request could be sent, false otherwise
    fn handle_get_parameters(
        &self,
        parameters: Vec<String>,
        content_type: Option<&str>,
        promise: &gst::Promise,
    ) -> bool {
        let obj = self.obj();

        // Validate parameters array
        if parameters.is_empty() {
            gst::warning!(
                CAT,
                obj = obj,
                "get-parameters: parameters array cannot be empty"
            );
            return false;
        }

        // Validate each parameter name
        for param in &parameters {
            if param.is_empty() {
                gst::warning!(
                    CAT,
                    obj = obj,
                    "get-parameters: parameter name cannot be empty"
                );
                return false;
            }
        }

        gst::debug!(
            CAT,
            obj = obj,
            "get-parameters action called with parameters: {:?}, content_type: {:?}",
            parameters,
            content_type
        );

        // Send GET_PARAMETER command to async task
        let cmd_queue = self.cmd_queue();
        let parameters = Some(parameters);
        let promise = promise.clone();

        RUNTIME.spawn(async move {
            let _ = cmd_queue
                .send(Commands::GetParameter {
                    parameters,
                    promise,
                })
                .await;
        });

        true
    }

    /// Handle set-parameter action signal
    /// Sends a SET_PARAMETER RTSP request
    /// Returns true if request could be sent, false otherwise
    fn handle_set_parameter(
        &self,
        parameter: &str,
        value: &str,
        content_type: Option<&str>,
        promise: &gst::Promise,
    ) -> bool {
        let obj = self.obj();

        // Validate parameter name
        if parameter.is_empty() {
            gst::warning!(
                CAT,
                obj = obj,
                "set-parameter: parameter name cannot be empty"
            );
            return false;
        }

        gst::debug!(
            CAT,
            obj = obj,
            "set-parameter action called with parameter: {}, value: {}, content_type: {:?}",
            parameter,
            value,
            content_type
        );

        // Send SET_PARAMETER command to async task
        let cmd_queue = self.cmd_queue();
        let parameters = vec![(parameter.to_string(), value.to_string())];
        let promise = promise.clone();

        RUNTIME.spawn(async move {
            let _ = cmd_queue
                .send(Commands::SetParameter {
                    parameters,
                    promise,
                })
                .await;
        });

        true
    }

    /// Handle push-backchannel-buffer action signal
    fn handle_push_backchannel_buffer(
        &self,
        stream_id: u32,
        buffer: &gst::Buffer,
    ) -> gst::FlowReturn {
        let obj = self.obj();

        gst::debug!(
            CAT,
            obj = obj,
            "push-backchannel-buffer action called with stream_id: {}, buffer size: {}",
            stream_id,
            buffer.size()
        );

        // TODO: Implement actual backchannel buffer transmission
        // This is a placeholder implementation
        // Actual implementation would send the buffer through the backchannel

        // Return NOT_SUPPORTED since backchannel isn't implemented yet
        gst::FlowReturn::NotSupported
    }

    /// Handle push-backchannel-sample action signal
    fn handle_push_backchannel_sample(
        &self,
        stream_id: u32,
        sample: &gst::Sample,
    ) -> gst::FlowReturn {
        let obj = self.obj();

        gst::debug!(
            CAT,
            obj = obj,
            "push-backchannel-sample action called with stream_id: {}",
            stream_id
        );

        // TODO: Implement actual backchannel sample transmission
        // This is a placeholder implementation
        // Actual implementation would send the sample through the backchannel

        // Return NOT_SUPPORTED since backchannel isn't implemented yet
        gst::FlowReturn::NotSupported
    }

    /// Handle set-mikey-parameter action signal
    fn handle_set_mikey_parameter(
        &self,
        stream_id: u32,
        caps: &gst::Caps,
        promise: &gst::Promise,
    ) -> bool {
        let obj = self.obj();

        gst::debug!(
            CAT,
            obj = obj,
            "set-mikey-parameter action called with stream_id: {}, caps: {:?}",
            stream_id,
            caps
        );

        // TODO: Implement actual MIKEY parameter setting
        // This is a placeholder implementation
        // Actual implementation would set SRTP keys via MIKEY protocol

        // Return false since MIKEY isn't implemented yet
        false
    }

    /// Handle remove-key action signal
    fn handle_remove_key(&self, stream_id: u32) -> bool {
        let obj = self.obj();

        gst::debug!(
            CAT,
            obj = obj,
            "remove-key action called with stream_id: {}",
            stream_id
        );

        // TODO: Implement actual key removal
        // This is a placeholder implementation
        // Actual implementation would remove encryption keys for the stream

        // Return false since key management isn't implemented yet
        false
    }

    /// Emit the handle-request signal (placeholder)
    /// This function will be called when the server sends an RTSP request to the client.
    /// Applications can connect to this signal to handle server-initiated requests.
    /// The application should fill the response message appropriately.
    fn emit_handle_request(&self, request: &RTSPMessage, response: &RTSPMessage) {
        let obj = self.obj();
        gst::debug!(CAT, obj = obj, "Emitting handle-request signal");
        obj.emit_by_name::<()>("handle-request", &[request, response]);
    }
}

impl ObjectImpl for RtspSrc {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecUInt::builder("receive-mtu")
                    .nick("Receive packet size")
                    .blurb("Initial size of buffers to allocate in the buffer pool, will be increased if too small")
                    .default_value(DEFAULT_RECEIVE_MTU)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("location")
                    .nick("Location")
                    .blurb("RTSP server, credentials and media path, e.g. rtsp://user:p4ssw0rd@camera-5.local:8554/h264_1080p30")
                    .mutable_ready()
                    .build(),
                // We purposely use port-start instead of port-range (like in rtspsrc), because
                // there is no way for the user to know how many ports we actually need. It depends
                // on how many streams the media contains, and whether the server wants RTCP or
                // RTCP-mux, or no RTCP. This property can be used to specify the start of the
                // valid range, and if the user wants to know how many ports were used, we can
                // add API for that later.
                glib::ParamSpecUInt::builder("port-start")
                    .nick("Port start")
                    .blurb("Port number to start allocating client ports for receiving RTP and RTCP data, eg. 3000 (0 = automatic selection)")
                    .default_value(DEFAULT_PORT_START.into())
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("protocols")
                    .nick("Protocols")
                    .blurb("Allowed lower transport protocols, in order of preference (udp-mcast,udp,tcp,http)")
                    .default_value("udp-mcast,udp,tcp")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("http-tunnel-mode")
                    .nick("HTTP Tunnel Mode")
                    .blurb("HTTP tunneling mode: auto, never, always")
                    .default_value("auto")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt::builder("tunnel-port")
                    .nick("Tunnel Port")
                    .blurb("Port to use for HTTP tunneling (default: 80)")
                    .minimum(1)
                    .maximum(65535)
                    .default_value(80)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("timeout")
                    .nick("Timeout")
                    .blurb("Timeout for network activity, in nanoseconds")
                    .maximum(gst::ClockTime::MAX.into())
                    .default_value(DEFAULT_TIMEOUT.into())
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("retry-strategy")
                    .nick("Retry Strategy")
                    .blurb(if cfg!(feature = "adaptive") {
                        "Connection retry strategy: auto, adaptive, none, immediate, linear, exponential, exponential-jitter"
                    } else {
                        "Connection retry strategy: auto, none, immediate, linear, exponential, exponential-jitter"
                    })
                    .default_value("auto")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecInt::builder("max-reconnection-attempts")
                    .nick("Maximum Reconnection Attempts")
                    .blurb("Maximum number of reconnection attempts (-1 for infinite, 0 for no retry)")
                    .minimum(-1)
                    .default_value(5)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("reconnection-timeout")
                    .nick("Reconnection Timeout")
                    .blurb("Maximum backoff delay between retry attempts in nanoseconds")
                    .maximum(gst::ClockTime::MAX.into())
                    .default_value(30_000_000_000) // 30 seconds
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("initial-retry-delay")
                    .nick("Initial Retry Delay")
                    .blurb("Initial delay between retry attempts in nanoseconds")
                    .maximum(gst::ClockTime::MAX.into())
                    .default_value(1_000_000_000) // 1 second
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("linear-retry-step")
                    .nick("Linear Retry Step")
                    .blurb("Step increment for linear retry strategy in nanoseconds")
                    .maximum(gst::ClockTime::MAX.into())
                    .default_value(2_000_000_000) // 2 seconds
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("connection-racing")
                    .nick("Connection Racing Strategy")
                    .blurb("Parallel connection racing strategy: none, first-wins, last-wins, hybrid")
                    .default_value("none")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("auto-mode-status")
                    .nick("Auto Mode Status")
                    .blurb("Current auto mode strategy selection status")
                    .default_value("")
                    .read_only()
                    .build(),
                glib::ParamSpecString::builder("current-racing-strategy")
                    .nick("Current Racing Strategy")
                    .blurb("Currently active connection racing strategy")
                    .default_value("none")
                    .read_only()
                    .build(),
                // Telemetry properties (read-only)
                #[cfg(feature = "telemetry")]
                glib::ParamSpecUInt64::builder("metrics-connection-attempts")
                    .nick("Connection Attempts")
                    .blurb("Total number of connection attempts")
                    .read_only()
                    .build(),
                #[cfg(feature = "telemetry")]
                glib::ParamSpecUInt64::builder("metrics-connection-successes")
                    .nick("Connection Successes")
                    .blurb("Total number of successful connections")
                    .read_only()
                    .build(),
                #[cfg(feature = "telemetry")]
                glib::ParamSpecUInt64::builder("metrics-packets-received")
                    .nick("Packets Received")
                    .blurb("Total number of packets received")
                    .read_only()
                    .build(),
                #[cfg(feature = "telemetry")]
                glib::ParamSpecUInt64::builder("metrics-bytes-received")
                    .nick("Bytes Received")
                    .blurb("Total number of bytes received")
                    .read_only()
                    .build(),
                glib::ParamSpecUInt::builder("max-parallel-connections")
                    .nick("Maximum Parallel Connections")
                    .blurb("Maximum number of parallel connections for racing")
                    .minimum(1)
                    .maximum(10)
                    .default_value(3)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt::builder("racing-delay-ms")
                    .nick("Racing Delay (ms)")
                    .blurb("Delay between parallel connection attempts in milliseconds")
                    .minimum(0)
                    .maximum(5000)
                    .default_value(250)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("racing-timeout")
                    .nick("Racing Timeout")
                    .blurb("Timeout for whole connection race in nanoseconds")
                    .maximum(gst::ClockTime::MAX.into())
                    .default_value(5_000_000_000) // 5 seconds
                    .mutable_ready()
                    .build(),
                // Auto mode specific properties
                glib::ParamSpecUInt::builder("auto-detection-attempts")
                    .nick("Auto Detection Attempts")
                    .blurb("Number of connection attempts before auto mode makes a strategy decision")
                    .minimum(1)
                    .maximum(10)
                    .default_value(3)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("auto-fallback-enabled")
                    .nick("Auto Fallback Enabled")
                    .blurb("Enable automatic fallback to other strategies on failure in auto mode")
                    .default_value(true)
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecBoolean::builder("adaptive-learning")
                    .nick("Adaptive Learning")
                    .blurb("Enable learning-based retry optimization")
                    .default_value(true)
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecBoolean::builder("adaptive-persistence")
                    .nick("Adaptive Persistence")
                    .blurb("Save learned retry patterns to disk")
                    .default_value(true)
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecUInt64::builder("adaptive-cache-ttl")
                    .nick("Adaptive Cache TTL")
                    .blurb("Cache lifetime for learned patterns in seconds")
                    .maximum(30 * 24 * 3600) // 30 days
                    .default_value(7 * 24 * 3600) // 7 days
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecUInt64::builder("adaptive-discovery-time")
                    .nick("Adaptive Discovery Time")
                    .blurb("Initial learning phase duration in nanoseconds")
                    .maximum(gst::ClockTime::MAX.into())
                    .default_value(30_000_000_000) // 30 seconds
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecFloat::builder("adaptive-exploration-rate")
                    .nick("Adaptive Exploration Rate")
                    .blurb("Frequency of exploring alternative strategies (0.0-1.0)")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(0.1)
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecFloat::builder("adaptive-confidence-threshold")
                    .nick("Adaptive Confidence Threshold")
                    .blurb("Minimum confidence before using learned patterns (0.0-1.0)")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(0.8)
                    .mutable_ready()
                    .build(),
                #[cfg(feature = "adaptive")]
                glib::ParamSpecBoolean::builder("adaptive-change-detection")
                    .nick("Adaptive Change Detection")
                    .blurb("Detect and adapt to network changes")
                    .default_value(true)
                    .mutable_ready()
                    .build(),
                // Jitterbuffer control properties (matching original rtspsrc)
                glib::ParamSpecUInt::builder("latency")
                    .nick("Buffer latency in ms")
                    .blurb("Amount of ms to buffer")
                    .default_value(DEFAULT_LATENCY_MS)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("drop-on-latency")
                    .nick("Drop buffers when maximum latency is reached")
                    .blurb("Tells the jitterbuffer to never exceed the given latency in size")
                    .default_value(DEFAULT_DROP_ON_LATENCY)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt::builder("probation")
                    .nick("Number of probations")
                    .blurb("Consecutive packet sequence numbers to accept the source")
                    .default_value(DEFAULT_PROBATION)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("buffer-mode")
                    .nick("Buffer Mode")
                    .blurb("Control the buffering algorithm in use (none, slave, buffer, auto, synced)")
                    .default_value(Some(BufferMode::default().as_str()))
                    .mutable_ready()
                    .build(),
                // RTCP control properties (matching original rtspsrc)
                glib::ParamSpecBoolean::builder("do-rtcp")
                    .nick("Do RTCP")
                    .blurb("Send RTCP packets, disable for old incompatible server.")
                    .default_value(DEFAULT_DO_RTCP)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("do-retransmission")
                    .nick("Retransmission")
                    .blurb("Ask the server to retransmit lost packets")
                    .default_value(DEFAULT_DO_RETRANSMISSION)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecInt::builder("max-rtcp-rtp-time-diff")
                    .nick("Max RTCP RTP Time Diff")
                    .blurb("Maximum amount of time in ms that the RTP time in RTCP SRs is allowed to be ahead (-1 disabled)")
                    .minimum(-1)
                    .maximum(i32::MAX)
                    .default_value(DEFAULT_MAX_RTCP_RTP_TIME_DIFF)
                    .mutable_ready()
                    .build(),
                // Keep-alive and timeout properties (matching original rtspsrc)
                glib::ParamSpecBoolean::builder("do-rtsp-keep-alive")
                    .nick("Send RTSP keep-alive packets")
                    .blurb("Send RTSP keep alive packets, disable for old incompatible server.")
                    .default_value(DEFAULT_DO_RTSP_KEEP_ALIVE)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("tcp-timeout")
                    .nick("TCP Timeout")
                    .blurb("Fail after timeout microseconds on TCP connections (0 = disabled)")
                    .minimum(0)
                    .maximum(u64::MAX)
                    .default_value(DEFAULT_TCP_TIMEOUT)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("teardown-timeout")
                    .nick("Teardown Timeout")
                    .blurb("When transitioning PAUSED-READY, allow up to timeout (in nanoseconds) delay in order to send teardown (0 = disabled)")
                    .minimum(0)
                    .maximum(u64::MAX)
                    .default_value(DEFAULT_TEARDOWN_TIMEOUT)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("udp-reconnect")
                    .nick("Reconnect to server")
                    .blurb("Reconnect to the server if RTSP connection is closed when doing UDP")
                    .default_value(DEFAULT_UDP_RECONNECT)
                    .mutable_ready()
                    .build(),
                // Network interface properties (matching original rtspsrc)
                glib::ParamSpecString::builder("multicast-iface")
                    .nick("Multicast Interface")
                    .blurb("The network interface on which to join the multicast group")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("port-range")
                    .nick("Port range")
                    .blurb("Client port range that can be used to receive RTP and RTCP data, eg. 3000-3005 (NULL = no restrictions)")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecInt::builder("udp-buffer-size")
                    .nick("UDP Buffer Size")
                    .blurb("Size of the kernel UDP receive buffer in bytes, 0=default")
                    .minimum(0)
                    .maximum(i32::MAX)
                    .default_value(DEFAULT_UDP_BUFFER_SIZE)
                    .mutable_ready()
                    .build(),
                // Source behavior properties (matching original rtspsrc)
                glib::ParamSpecBoolean::builder("is-live")
                    .nick("Is Live")
                    .blurb("Whether to act as a live source")
                    .default_value(DEFAULT_IS_LIVE)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("user-agent")
                    .nick("User Agent")
                    .blurb("The User-Agent string to send to the server")
                    .default_value(Some(DEFAULT_USER_AGENT))
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("connection-speed")
                    .nick("Connection Speed")
                    .blurb("Network connection speed in kbps (0 = unknown)")
                    .minimum(0)
                    .maximum(u64::MAX)
                    .default_value(DEFAULT_CONNECTION_SPEED)
                    .mutable_ready()
                    .build(),
                // Timestamp synchronization properties (matching original rtspsrc)
                glib::ParamSpecBoolean::builder("ntp-sync")
                    .nick("NTP Sync")
                    .blurb("Synchronize received streams to the NTP clock")
                    .default_value(DEFAULT_NTP_SYNC)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("rfc7273-sync")
                    .nick("RFC7273 Sync")
                    .blurb("Synchronize received streams to the RFC7273 clock (requires clock and offset to be provided)")
                    .default_value(DEFAULT_RFC7273_SYNC)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecEnum::builder::<NtpTimeSource>("ntp-time-source")
                    .nick("NTP Time Source")
                    .blurb("NTP time source for RTCP packets")
                    .default_value(NtpTimeSource::default())
                    .mutable_ready()
                    .build(),
                glib::ParamSpecInt64::builder("max-ts-offset")
                    .nick("Max Timestamp Offset")
                    .blurb("The maximum absolute value of the time offset in (nanoseconds). Note, if the ntp-sync parameter is set the default value is changed to 0 (no limit)")
                    .minimum(0)
                    .maximum(i64::MAX)
                    .default_value(DEFAULT_MAX_TS_OFFSET)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt64::builder("max-ts-offset-adjustment")
                    .nick("Max Timestamp Offset Adjustment")
                    .blurb("The maximum number of nanoseconds per frame that time stamp offsets may be adjusted (0 = no limit).")
                    .minimum(0)
                    .maximum(u64::MAX)
                    .default_value(DEFAULT_MAX_TS_OFFSET_ADJUSTMENT)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("add-reference-timestamp-meta")
                    .nick("Add Reference Timestamp Meta")
                    .blurb("Add Reference Timestamp Meta to buffers with the original clock timestamp before any adjustments when syncing to an RFC7273 clock.")
                    .default_value(DEFAULT_ADD_REFERENCE_TIMESTAMP_META)
                    .mutable_ready()
                    .build(),
                // RTSP version negotiation property
                glib::ParamSpecEnum::builder::<RtspVersion>("default-rtsp-version")
                    .nick("Default RTSP Version")
                    .blurb("The RTSP version that should be tried first when negotiating version.")
                    .default_value(DEFAULT_RTSP_VERSION)
                    .mutable_ready()
                    .build(),
                // RTP-specific properties
                glib::ParamSpecUInt::builder("rtp-blocksize")
                    .nick("RTP block size")
                    .blurb("RTP package size to suggest to server (0 = disabled)")
                    .maximum(65536)
                    .default_value(DEFAULT_RTP_BLOCKSIZE)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("tcp-timestamp")
                    .nick("TCP Timestamp")
                    .blurb("Timestamp RTP packets with receive times in TCP/HTTP mode")
                    .default_value(DEFAULT_TCP_TIMESTAMP)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoxed::builder::<gst::Structure>("sdes")
                    .nick("SDES")
                    .blurb("The SDES items of this session")
                    .mutable_ready()
                    .build(),
                // TLS/SSL security properties
                glib::ParamSpecObject::builder::<gio::TlsDatabase>("tls-database")
                    .nick("TLS database")
                    .blurb("TLS database with anchor certificate authorities used to validate the server certificate")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecObject::builder::<gio::TlsInteraction>("tls-interaction")
                    .nick("TLS interaction")
                    .blurb("A GTlsInteraction object to prompt the user for password or certificate")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecFlags::builder::<gio::TlsCertificateFlags>("tls-validation-flags")
                    .nick("TLS validation flags")
                    .blurb("TLS certificate validation flags used to validate the server certificate")
                    .default_value(DEFAULT_TLS_VALIDATION_FLAGS)
                    .mutable_ready()
                    .build(),
                // Proxy and HTTP tunneling properties
                glib::ParamSpecString::builder("proxy")
                    .nick("Proxy")
                    .blurb("Proxy settings for HTTP tunneling. Format: [http://][user:passwd@]host[:port]")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("proxy-id")
                    .nick("Proxy ID")
                    .blurb("HTTP proxy URI user id for authentication")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("proxy-pw")
                    .nick("Proxy password")
                    .blurb("HTTP proxy URI user password for authentication")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoxed::builder::<gst::Structure>("extra-http-request-headers")
                    .nick("Extra HTTP request headers")
                    .blurb("Extra headers to append to HTTP requests when in tunneled mode")
                    .mutable_ready()
                    .build(),
                // NAT traversal properties
                glib::ParamSpecEnum::builder::<NatMethod>("nat-method")
                    .nick("NAT method")
                    .blurb("Method to use for traversing firewalls and NAT")
                    .default_value(NatMethod::default())
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("ignore-x-server-reply")
                    .nick("Ignore x-server-reply")
                    .blurb("Whether to ignore the x-server-ip-address server header reply")
                    .default_value(false)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("force-non-compliant-url")
                    .nick("Force non-compliant URL")
                    .blurb("Revert to old non-compliant method of constructing URLs")
                    .default_value(false)
                    .mutable_ready()
                    .build(),
                // ONVIF backchannel properties
                glib::ParamSpecEnum::builder::<BackchannelType>("backchannel")
                    .nick("Backchannel")
                    .blurb("The type of backchannel to setup. Default is 'none'.")
                    .default_value(BackchannelType::default())
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("onvif-mode")
                    .nick("ONVIF mode")
                    .blurb("Act as an ONVIF client")
                    .default_value(false)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("onvif-rate-control")
                    .nick("ONVIF rate control")
                    .blurb("When in onvif-mode, whether to set Rate-Control to yes or no")
                    .default_value(true)
                    .mutable_ready()
                    .build(),
                // Authentication properties
                glib::ParamSpecString::builder("user-id")
                    .nick("RTSP user ID")
                    .blurb("RTSP location URI user id for authentication")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("user-pw")
                    .nick("RTSP user password")
                    .blurb("RTSP location URI user password for authentication")
                    .mutable_ready()
                    .build(),
                // Compatibility properties (PRP-51)
                glib::ParamSpecBoolean::builder("short-header")
                    .nick("Send only basic RTSP headers")
                    .blurb("Only send the basic RTSP headers for broken encoders")
                    .default_value(false)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("debug")
                    .nick("Debug")
                    .blurb("Dump request and response messages to stdout(DEPRECATED: Printed all RTSP message to gstreamer log as 'log' level)")
                    .default_value(false)
                    .mutable_ready()
                    .deprecated()
                    .build(),
                glib::ParamSpecBoolean::builder("use-pipeline-clock")
                    .nick("Use pipeline clock")
                    .blurb("Use the pipeline running-time to set the NTP time in the RTCP SR messages(DEPRECATED: Use ntp-time-source property)")
                    .default_value(false)
                    .mutable_ready()
                    .deprecated()
                    .build(),
                glib::ParamSpecBoolean::builder("client-managed-mikey")
                    .nick("Client Managed MIKEY")
                    .blurb("Enable client-managed MIKEY mode")
                    .default_value(false)
                    .mutable_ready()
                    .build(),
                // Stream selection properties (PRP-RTSP-20)
                glib::ParamSpecString::builder("select-streams")
                    .nick("Select Streams")
                    .blurb("Comma-separated list of stream types to select: audio,video,metadata,application,all,none")
                    .default_value("all")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("stream-filter")
                    .nick("Stream Filter")
                    .blurb("Filter expression for selecting streams by codec or other attributes")
                    .mutable_ready()
                    .build(),
                glib::ParamSpecBoolean::builder("require-all-streams")
                    .nick("Require All Streams")
                    .blurb("Fail if not all selected streams can be linked")
                    .default_value(DEFAULT_REQUIRE_ALL_STREAMS)
                    .mutable_ready()
                    .build(),
                // Debug observability property  
                glib::ParamSpecString::builder("decision-history")
                    .nick("Decision History")
                    .blurb("JSON-formatted history of recent retry and connection decisions for debugging")
                    .default_value("")
                    .read_only()
                    .build(),
            ]
        });

        PROPERTIES.as_ref()
    }
    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        let res = match pspec.name() {
            "receive-mtu" => {
                let mut settings = self.settings.lock().unwrap();
                settings.receive_mtu = value.get::<u32>().expect("type checked upstream");
                Ok(())
            }
            "location" => {
                let location = value.get::<Option<&str>>().expect("type checked upstream");
                self.set_location(location)
            }
            "port-start" => {
                let mut settings = self.settings.lock().unwrap();
                let start = value.get::<u32>().expect("type checked upstream");
                match u16::try_from(start) {
                    Ok(start) => {
                        settings.port_start = start;
                        Ok(())
                    }
                    Err(err) => Err(glib::Error::new(
                        gst::CoreError::Failed,
                        &format!("Failed to set port start: {err:?}"),
                    )),
                }
            }
            "protocols" => {
                let protocols = value.get::<Option<&str>>().expect("type checked upstream");
                self.set_protocols(protocols)
            }
            "http-tunnel-mode" => {
                let mut settings = self.settings.lock().unwrap();
                let mode_str = value.get::<&str>().expect("type checked upstream");
                settings.http_tunnel_mode = match mode_str {
                    "never" => HttpTunnelMode::Never,
                    "always" => HttpTunnelMode::Always,
                    _ => HttpTunnelMode::Auto,
                };
                Ok(())
            }
            "tunnel-port" => {
                let mut settings = self.settings.lock().unwrap();
                settings.tunnel_port = value.get::<u32>().expect("type checked upstream") as u16;
                Ok(())
            }
            "timeout" => {
                let mut settings = self.settings.lock().unwrap();
                let timeout = value.get().expect("type checked upstream");
                settings.timeout = timeout;
                Ok(())
            }
            "retry-strategy" => {
                let mut settings = self.settings.lock().unwrap();
                let strategy = value.get::<Option<&str>>().expect("type checked upstream");
                settings.retry_strategy =
                    super::retry::RetryStrategy::from_string(strategy.unwrap_or("auto"));
                Ok(())
            }
            "max-reconnection-attempts" => {
                let mut settings = self.settings.lock().unwrap();
                settings.max_reconnection_attempts =
                    value.get::<i32>().expect("type checked upstream");
                Ok(())
            }
            "reconnection-timeout" => {
                let mut settings = self.settings.lock().unwrap();
                let timeout = value.get().expect("type checked upstream");
                settings.reconnection_timeout = timeout;
                Ok(())
            }
            "initial-retry-delay" => {
                let mut settings = self.settings.lock().unwrap();
                let delay = value.get().expect("type checked upstream");
                settings.initial_retry_delay = delay;
                Ok(())
            }
            "linear-retry-step" => {
                let mut settings = self.settings.lock().unwrap();
                let step = value.get().expect("type checked upstream");
                settings.linear_retry_step = step;
                Ok(())
            }
            "connection-racing" => {
                let mut settings = self.settings.lock().unwrap();
                let strategy = value.get::<Option<&str>>().expect("type checked upstream");
                settings.connection_racing =
                    super::connection_racer::ConnectionRacingStrategy::from_string(
                        strategy.unwrap_or("none"),
                    );
                Ok(())
            }
            "max-parallel-connections" => {
                let mut settings = self.settings.lock().unwrap();
                settings.max_parallel_connections =
                    value.get::<u32>().expect("type checked upstream");
                Ok(())
            }
            "racing-delay-ms" => {
                let mut settings = self.settings.lock().unwrap();
                settings.racing_delay_ms = value.get::<u32>().expect("type checked upstream");
                Ok(())
            }
            "racing-timeout" => {
                let mut settings = self.settings.lock().unwrap();
                settings.racing_timeout = value.get().expect("type checked upstream");
                Ok(())
            }
            "auto-detection-attempts" => {
                let mut settings = self.settings.lock().unwrap();
                settings.auto_detection_attempts = value.get().expect("type checked upstream");
                Ok(())
            }
            "auto-fallback-enabled" => {
                let mut settings = self.settings.lock().unwrap();
                settings.auto_fallback_enabled = value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-learning" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_learning = value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-persistence" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_persistence = value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-cache-ttl" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_cache_ttl = value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-discovery-time" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_discovery_time = value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-exploration-rate" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_exploration_rate = value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-confidence-threshold" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_confidence_threshold =
                    value.get().expect("type checked upstream");
                Ok(())
            }
            #[cfg(feature = "adaptive")]
            "adaptive-change-detection" => {
                let mut settings = self.settings.lock().unwrap();
                settings.adaptive_change_detection = value.get().expect("type checked upstream");
                Ok(())
            }
            // Jitterbuffer control properties
            "latency" => {
                let mut settings = self.settings.lock().unwrap();
                settings.latency_ms = value.get::<u32>().expect("type checked upstream");
                Ok(())
            }
            "drop-on-latency" => {
                let mut settings = self.settings.lock().unwrap();
                settings.drop_on_latency = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "probation" => {
                let mut settings = self.settings.lock().unwrap();
                settings.probation = value.get::<u32>().expect("type checked upstream");
                Ok(())
            }
            "buffer-mode" => {
                let mut settings = self.settings.lock().unwrap();
                let mode_str: Option<String> = value.get().expect("type checked upstream");
                if let Some(mode_str) = mode_str {
                    match BufferMode::from_str(&mode_str) {
                        Ok(mode) => {
                            settings.buffer_mode = mode;
                            Ok(())
                        }
                        Err(e) => Err(gst::glib::Error::new(
                            gst::CoreError::Failed,
                            &format!("Invalid buffer mode '{}': {}", mode_str, e),
                        )),
                    }
                } else {
                    // Use default if None
                    settings.buffer_mode = BufferMode::default();
                    Ok(())
                }
            }
            // RTCP control properties
            "do-rtcp" => {
                let mut settings = self.settings.lock().unwrap();
                settings.do_rtcp = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "do-retransmission" => {
                let mut settings = self.settings.lock().unwrap();
                settings.do_retransmission = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "max-rtcp-rtp-time-diff" => {
                let mut settings = self.settings.lock().unwrap();
                settings.max_rtcp_rtp_time_diff =
                    value.get::<i32>().expect("type checked upstream");
                Ok(())
            }
            // Keep-alive and timeout properties
            "do-rtsp-keep-alive" => {
                let mut settings = self.settings.lock().unwrap();
                settings.do_rtsp_keep_alive = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "tcp-timeout" => {
                let mut settings = self.settings.lock().unwrap();
                settings.tcp_timeout = value.get::<u64>().expect("type checked upstream");
                Ok(())
            }
            "teardown-timeout" => {
                let mut settings = self.settings.lock().unwrap();
                settings.teardown_timeout = value.get::<u64>().expect("type checked upstream");
                Ok(())
            }
            "udp-reconnect" => {
                let mut settings = self.settings.lock().unwrap();
                settings.udp_reconnect = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            // Network interface properties
            "multicast-iface" => {
                let mut settings = self.settings.lock().unwrap();
                settings.multicast_iface = value
                    .get::<Option<String>>()
                    .expect("type checked upstream");
                Ok(())
            }
            "port-range" => {
                let range = value.get::<Option<&str>>().expect("type checked upstream");
                validate_port_range(range).map(|_| {
                    let mut settings = self.settings.lock().unwrap();
                    settings.port_range = range.map(String::from);
                })
            }
            "udp-buffer-size" => {
                let mut settings = self.settings.lock().unwrap();
                settings.udp_buffer_size = value.get::<i32>().expect("type checked upstream");
                Ok(())
            }
            // Source behavior properties
            "is-live" => {
                let mut settings = self.settings.lock().unwrap();
                settings.is_live = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "user-agent" => {
                let mut settings = self.settings.lock().unwrap();
                settings.user_agent = value.get::<String>().expect("type checked upstream");
                Ok(())
            }
            "connection-speed" => {
                let mut settings = self.settings.lock().unwrap();
                settings.connection_speed = value.get::<u64>().expect("type checked upstream");
                Ok(())
            }
            // Timestamp synchronization properties
            "ntp-sync" => {
                let mut settings = self.settings.lock().unwrap();
                settings.ntp_sync = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "rfc7273-sync" => {
                let mut settings = self.settings.lock().unwrap();
                settings.rfc7273_sync = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "ntp-time-source" => {
                let mut settings = self.settings.lock().unwrap();
                settings.ntp_time_source =
                    value.get::<NtpTimeSource>().expect("type checked upstream");
                Ok(())
            }
            "max-ts-offset" => {
                let mut settings = self.settings.lock().unwrap();
                settings.max_ts_offset = value.get::<i64>().expect("type checked upstream");
                Ok(())
            }
            "max-ts-offset-adjustment" => {
                let mut settings = self.settings.lock().unwrap();
                settings.max_ts_offset_adjustment =
                    value.get::<u64>().expect("type checked upstream");
                Ok(())
            }
            "add-reference-timestamp-meta" => {
                let mut settings = self.settings.lock().unwrap();
                settings.add_reference_timestamp_meta =
                    value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "default-rtsp-version" => {
                let mut settings = self.settings.lock().unwrap();
                settings.default_rtsp_version =
                    value.get::<RtspVersion>().expect("type checked upstream");
                Ok(())
            }
            "rtp-blocksize" => {
                let mut settings = self.settings.lock().unwrap();
                settings.rtp_blocksize = value.get::<u32>().expect("type checked upstream");
                Ok(())
            }
            "tcp-timestamp" => {
                let mut settings = self.settings.lock().unwrap();
                settings.tcp_timestamp = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "sdes" => {
                let mut settings = self.settings.lock().unwrap();
                settings.sdes = value
                    .get::<Option<gst::Structure>>()
                    .expect("type checked upstream");
                Ok(())
            }
            "tls-database" => {
                // TODO: Cannot store TlsDatabase due to Send+Sync requirements
                // Will be implemented after removing Tokio
                gst::warning!(CAT, imp = self, "tls-database property not yet implemented");
                Ok(())
            }
            "tls-interaction" => {
                // TODO: Cannot store TlsInteraction due to Send+Sync requirements
                // Will be implemented after removing Tokio
                gst::warning!(
                    CAT,
                    imp = self,
                    "tls-interaction property not yet implemented"
                );
                Ok(())
            }
            "tls-validation-flags" => {
                let mut settings = self.settings.lock().unwrap();
                settings.tls_validation_flags = value
                    .get::<gio::TlsCertificateFlags>()
                    .expect("type checked upstream");
                Ok(())
            }
            // Proxy and HTTP tunneling properties
            "proxy" => {
                let mut settings = self.settings.lock().unwrap();
                let proxy = value
                    .get::<Option<String>>()
                    .expect("type checked upstream");
                // TODO: Add proxy URL validation once implemented
                settings.proxy = proxy;
                Ok(())
            }
            "proxy-id" => {
                let mut settings = self.settings.lock().unwrap();
                settings.proxy_id = value
                    .get::<Option<String>>()
                    .expect("type checked upstream");
                Ok(())
            }
            "proxy-pw" => {
                let mut settings = self.settings.lock().unwrap();
                settings.proxy_pw = value
                    .get::<Option<String>>()
                    .expect("type checked upstream");
                Ok(())
            }
            "extra-http-request-headers" => {
                let mut settings = self.settings.lock().unwrap();
                settings.extra_http_request_headers = value
                    .get::<Option<gst::Structure>>()
                    .expect("type checked upstream");
                Ok(())
            }
            // NAT traversal properties
            "nat-method" => {
                let mut settings = self.settings.lock().unwrap();
                settings.nat_method = value.get::<NatMethod>().expect("type checked upstream");
                Ok(())
            }
            "ignore-x-server-reply" => {
                let mut settings = self.settings.lock().unwrap();
                settings.ignore_x_server_reply =
                    value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "force-non-compliant-url" => {
                let mut settings = self.settings.lock().unwrap();
                settings.force_non_compliant_url =
                    value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            // ONVIF backchannel properties
            "backchannel" => {
                let mut settings = self.settings.lock().unwrap();
                settings.backchannel = value
                    .get::<BackchannelType>()
                    .expect("type checked upstream");
                Ok(())
            }
            "onvif-mode" => {
                let mut settings = self.settings.lock().unwrap();
                settings.onvif_mode = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "onvif-rate-control" => {
                let mut settings = self.settings.lock().unwrap();
                settings.onvif_rate_control = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            // Authentication properties
            "user-id" => {
                let mut settings = self.settings.lock().unwrap();
                settings.user_id = value
                    .get::<Option<String>>()
                    .expect("type checked upstream");
                Ok(())
            }
            "user-pw" => {
                let mut settings = self.settings.lock().unwrap();
                settings.user_pw = value
                    .get::<Option<String>>()
                    .expect("type checked upstream");
                Ok(())
            }
            // Compatibility properties (PRP-51)
            "short-header" => {
                let mut settings = self.settings.lock().unwrap();
                settings.short_header = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "debug" => {
                let mut settings = self.settings.lock().unwrap();
                let debug_enabled = value.get::<bool>().expect("type checked upstream");
                if debug_enabled {
                    gst::warning!(
                        CAT,
                        imp = self,
                        "debug property is deprecated. Use GST_DEBUG=rtspsrc2:LOG instead"
                    );
                }
                settings.debug = debug_enabled;
                Ok(())
            }
            "use-pipeline-clock" => {
                let mut settings = self.settings.lock().unwrap();
                let use_pipeline_clock = value.get::<bool>().expect("type checked upstream");
                if use_pipeline_clock {
                    gst::warning!(
                        CAT,
                        imp = self,
                        "use-pipeline-clock is deprecated. Use ntp-time-source property instead"
                    );
                }
                settings.use_pipeline_clock = use_pipeline_clock;
                Ok(())
            }
            "client-managed-mikey" => {
                let mut settings = self.settings.lock().unwrap();
                settings.client_managed_mikey = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            "select-streams" => {
                let mut settings = self.settings.lock().unwrap();
                let stream_str = value
                    .get::<Option<String>>()
                    .expect("type checked upstream")
                    .unwrap_or_else(|| "all".to_string());
                settings.select_streams = StreamSelection::from_string(&stream_str);
                Ok(())
            }
            "stream-filter" => {
                let mut settings = self.settings.lock().unwrap();
                settings.stream_filter = value.get().expect("type checked upstream");
                Ok(())
            }
            "require-all-streams" => {
                let mut settings = self.settings.lock().unwrap();
                settings.require_all_streams = value.get::<bool>().expect("type checked upstream");
                Ok(())
            }
            name => unimplemented!("Property '{name}'"),
        };

        if let Err(err) = res {
            gst::error!(
                CAT,
                imp = self,
                "Failed to set property `{}`: {:?}",
                pspec.name(),
                err
            );
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "receive-mtu" => {
                let settings = self.settings.lock().unwrap();
                settings.receive_mtu.to_value()
            }
            "location" => {
                let settings = self.settings.lock().unwrap();
                let location = settings.location.as_ref().map(Url::to_string);

                location.to_value()
            }
            "port-start" => {
                let settings = self.settings.lock().unwrap();
                (settings.port_start as u32).to_value()
            }
            "protocols" => {
                let settings = self.settings.lock().unwrap();
                (settings
                    .protocols
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(","))
                .to_value()
            }
            "http-tunnel-mode" => {
                let settings = self.settings.lock().unwrap();
                let mode_str = match settings.http_tunnel_mode {
                    HttpTunnelMode::Auto => "auto",
                    HttpTunnelMode::Never => "never",
                    HttpTunnelMode::Always => "always",
                };
                mode_str.to_value()
            }
            "tunnel-port" => {
                let settings = self.settings.lock().unwrap();
                (settings.tunnel_port as u32).to_value()
            }
            "timeout" => {
                let settings = self.settings.lock().unwrap();
                settings.timeout.to_value()
            }
            "retry-strategy" => {
                let settings = self.settings.lock().unwrap();
                settings.retry_strategy.as_str().to_value()
            }
            "max-reconnection-attempts" => {
                let settings = self.settings.lock().unwrap();
                settings.max_reconnection_attempts.to_value()
            }
            "reconnection-timeout" => {
                let settings = self.settings.lock().unwrap();
                settings.reconnection_timeout.to_value()
            }
            "initial-retry-delay" => {
                let settings = self.settings.lock().unwrap();
                settings.initial_retry_delay.to_value()
            }
            "linear-retry-step" => {
                let settings = self.settings.lock().unwrap();
                settings.linear_retry_step.to_value()
            }
            "connection-racing" => {
                let settings = self.settings.lock().unwrap();
                settings.connection_racing.as_str().to_value()
            }
            "auto-mode-status" => {
                // TODO: Store and retrieve auto mode status from task
                // For now, return empty string
                String::new().to_value()
            }
            "current-racing-strategy" => {
                // TODO: Store and retrieve current racing strategy from task
                // For now, return the configured strategy
                let settings = self.settings.lock().unwrap();
                settings.connection_racing.as_str().to_value()
            }
            // Telemetry metrics properties
            #[cfg(feature = "telemetry")]
            "metrics-connection-attempts" => {
                #[cfg(feature = "telemetry")]
                {
                    let summary = self.metrics.get_metrics_summary();
                    summary.connection_attempts.to_value()
                }
                #[cfg(not(feature = "telemetry"))]
                0u64.to_value()
            }
            #[cfg(feature = "telemetry")]
            "metrics-connection-successes" => {
                #[cfg(feature = "telemetry")]
                {
                    let summary = self.metrics.get_metrics_summary();
                    summary.connection_successes.to_value()
                }
                #[cfg(not(feature = "telemetry"))]
                0u64.to_value()
            }
            #[cfg(feature = "telemetry")]
            "metrics-packets-received" => {
                #[cfg(feature = "telemetry")]
                {
                    let summary = self.metrics.get_metrics_summary();
                    summary.total_packets_received.to_value()
                }
                #[cfg(not(feature = "telemetry"))]
                0u64.to_value()
            }
            #[cfg(feature = "telemetry")]
            "metrics-bytes-received" => {
                #[cfg(feature = "telemetry")]
                {
                    let summary = self.metrics.get_metrics_summary();
                    summary.total_bytes_received.to_value()
                }
                #[cfg(not(feature = "telemetry"))]
                0u64.to_value()
            }
            "max-parallel-connections" => {
                let settings = self.settings.lock().unwrap();
                settings.max_parallel_connections.to_value()
            }
            "racing-delay-ms" => {
                let settings = self.settings.lock().unwrap();
                settings.racing_delay_ms.to_value()
            }
            "racing-timeout" => {
                let settings = self.settings.lock().unwrap();
                settings.racing_timeout.to_value()
            }
            "auto-detection-attempts" => {
                let settings = self.settings.lock().unwrap();
                settings.auto_detection_attempts.to_value()
            }
            "auto-fallback-enabled" => {
                let settings = self.settings.lock().unwrap();
                settings.auto_fallback_enabled.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-learning" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_learning.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-persistence" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_persistence.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-cache-ttl" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_cache_ttl.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-discovery-time" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_discovery_time.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-exploration-rate" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_exploration_rate.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-confidence-threshold" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_confidence_threshold.to_value()
            }
            #[cfg(feature = "adaptive")]
            "adaptive-change-detection" => {
                let settings = self.settings.lock().unwrap();
                settings.adaptive_change_detection.to_value()
            }
            // Jitterbuffer control properties
            "latency" => {
                let settings = self.settings.lock().unwrap();
                settings.latency_ms.to_value()
            }
            "drop-on-latency" => {
                let settings = self.settings.lock().unwrap();
                settings.drop_on_latency.to_value()
            }
            "probation" => {
                let settings = self.settings.lock().unwrap();
                settings.probation.to_value()
            }
            "buffer-mode" => {
                let settings = self.settings.lock().unwrap();
                settings.buffer_mode.as_str().to_value()
            }
            // RTCP control properties
            "do-rtcp" => {
                let settings = self.settings.lock().unwrap();
                settings.do_rtcp.to_value()
            }
            "do-retransmission" => {
                let settings = self.settings.lock().unwrap();
                settings.do_retransmission.to_value()
            }
            "max-rtcp-rtp-time-diff" => {
                let settings = self.settings.lock().unwrap();
                settings.max_rtcp_rtp_time_diff.to_value()
            }
            // Keep-alive and timeout properties
            "do-rtsp-keep-alive" => {
                let settings = self.settings.lock().unwrap();
                settings.do_rtsp_keep_alive.to_value()
            }
            "tcp-timeout" => {
                let settings = self.settings.lock().unwrap();
                settings.tcp_timeout.to_value()
            }
            "teardown-timeout" => {
                let settings = self.settings.lock().unwrap();
                settings.teardown_timeout.to_value()
            }
            "udp-reconnect" => {
                let settings = self.settings.lock().unwrap();
                settings.udp_reconnect.to_value()
            }
            // Network interface properties
            "multicast-iface" => {
                let settings = self.settings.lock().unwrap();
                settings.multicast_iface.to_value()
            }
            "port-range" => {
                let settings = self.settings.lock().unwrap();
                settings.port_range.to_value()
            }
            "udp-buffer-size" => {
                let settings = self.settings.lock().unwrap();
                settings.udp_buffer_size.to_value()
            }
            // Source behavior properties
            "is-live" => {
                let settings = self.settings.lock().unwrap();
                settings.is_live.to_value()
            }
            "user-agent" => {
                let settings = self.settings.lock().unwrap();
                settings.user_agent.to_value()
            }
            "connection-speed" => {
                let settings = self.settings.lock().unwrap();
                settings.connection_speed.to_value()
            }
            // Timestamp synchronization properties
            "ntp-sync" => {
                let settings = self.settings.lock().unwrap();
                settings.ntp_sync.to_value()
            }
            "rfc7273-sync" => {
                let settings = self.settings.lock().unwrap();
                settings.rfc7273_sync.to_value()
            }
            "ntp-time-source" => {
                let settings = self.settings.lock().unwrap();
                settings.ntp_time_source.to_value()
            }
            "max-ts-offset" => {
                let settings = self.settings.lock().unwrap();
                settings.max_ts_offset.to_value()
            }
            "max-ts-offset-adjustment" => {
                let settings = self.settings.lock().unwrap();
                settings.max_ts_offset_adjustment.to_value()
            }
            "add-reference-timestamp-meta" => {
                let settings = self.settings.lock().unwrap();
                settings.add_reference_timestamp_meta.to_value()
            }
            "default-rtsp-version" => {
                let settings = self.settings.lock().unwrap();
                settings.default_rtsp_version.to_value()
            }
            "rtp-blocksize" => {
                let settings = self.settings.lock().unwrap();
                settings.rtp_blocksize.to_value()
            }
            "tcp-timestamp" => {
                let settings = self.settings.lock().unwrap();
                settings.tcp_timestamp.to_value()
            }
            "sdes" => {
                let settings = self.settings.lock().unwrap();
                settings.sdes.to_value()
            }
            "tls-database" => {
                // TODO: Cannot store TlsDatabase due to Send+Sync requirements
                // Always returns None for now
                None::<gio::TlsDatabase>.to_value()
            }
            "tls-interaction" => {
                // TODO: Cannot store TlsInteraction due to Send+Sync requirements
                // Always returns None for now
                None::<gio::TlsInteraction>.to_value()
            }
            "tls-validation-flags" => {
                let settings = self.settings.lock().unwrap();
                settings.tls_validation_flags.to_value()
            }
            // Proxy and HTTP tunneling properties
            "proxy" => {
                let settings = self.settings.lock().unwrap();
                settings.proxy.to_value()
            }
            "proxy-id" => {
                let settings = self.settings.lock().unwrap();
                settings.proxy_id.to_value()
            }
            "proxy-pw" => {
                let settings = self.settings.lock().unwrap();
                settings.proxy_pw.to_value()
            }
            "extra-http-request-headers" => {
                let settings = self.settings.lock().unwrap();
                settings.extra_http_request_headers.to_value()
            }
            // NAT traversal properties
            "nat-method" => {
                let settings = self.settings.lock().unwrap();
                settings.nat_method.to_value()
            }
            "ignore-x-server-reply" => {
                let settings = self.settings.lock().unwrap();
                settings.ignore_x_server_reply.to_value()
            }
            "force-non-compliant-url" => {
                let settings = self.settings.lock().unwrap();
                settings.force_non_compliant_url.to_value()
            }
            // ONVIF backchannel properties
            "backchannel" => {
                let settings = self.settings.lock().unwrap();
                settings.backchannel.to_value()
            }
            "onvif-mode" => {
                let settings = self.settings.lock().unwrap();
                settings.onvif_mode.to_value()
            }
            "onvif-rate-control" => {
                let settings = self.settings.lock().unwrap();
                settings.onvif_rate_control.to_value()
            }
            // Authentication properties
            "user-id" => {
                let settings = self.settings.lock().unwrap();
                settings.user_id.to_value()
            }
            "user-pw" => {
                let settings = self.settings.lock().unwrap();
                settings.user_pw.to_value()
            }
            // Compatibility properties (PRP-51)
            "short-header" => {
                let settings = self.settings.lock().unwrap();
                settings.short_header.to_value()
            }
            "debug" => {
                let settings = self.settings.lock().unwrap();
                settings.debug.to_value()
            }
            "use-pipeline-clock" => {
                let settings = self.settings.lock().unwrap();
                settings.use_pipeline_clock.to_value()
            }
            "client-managed-mikey" => {
                let settings = self.settings.lock().unwrap();
                settings.client_managed_mikey.to_value()
            }
            "select-streams" => {
                let settings = self.settings.lock().unwrap();
                settings.select_streams.to_string().to_value()
            }
            "stream-filter" => {
                let settings = self.settings.lock().unwrap();
                settings.stream_filter.to_value()
            }
            "require-all-streams" => {
                let settings = self.settings.lock().unwrap();
                settings.require_all_streams.to_value()
            }
            "decision-history" => {
                // Get decision history from task if available
                // For now, return empty JSON array as we need to access the retry calculator from the task
                // TODO: Store reference to retry calculator or decision history in RtspSrc
                "[]".to_value()
            }
            name => unimplemented!("Property '{name}'"),
        }
    }

    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        obj.set_suppressed_flags(gst::ElementFlags::SINK | gst::ElementFlags::SOURCE);
        obj.set_element_flags(gst::ElementFlags::SOURCE);
    }

    fn signals() -> &'static [glib::subclass::Signal] {
        static SIGNALS: LazyLock<Vec<glib::subclass::Signal>> = LazyLock::new(|| {
            vec![
                // on-sdp signal: emitted when SDP is received
                glib::subclass::Signal::builder("on-sdp")
                    .param_types([gst_sdp::SDPMessage::static_type()])
                    .build(),
                // select-stream signal: emitted for stream selection
                glib::subclass::Signal::builder("select-stream")
                    .param_types([u32::static_type(), gst::Caps::static_type()])
                    .return_type::<bool>()
                    .build(),
                // new-manager signal: emitted when RTP manager is created
                glib::subclass::Signal::builder("new-manager")
                    .param_types([gst::Element::static_type()])
                    .build(),
                // accept-certificate signal: emitted for TLS certificate validation
                // Returns true to accept certificate, false to reject
                // Since: 1.14
                glib::subclass::Signal::builder("accept-certificate")
                    .param_types([
                        gio::TlsConnection::static_type(),
                        gio::TlsCertificate::static_type(),
                        gio::TlsCertificateFlags::static_type(),
                    ])
                    .return_type::<bool>()
                    .accumulator(|_hint, acc, value| {
                        use std::ops::ControlFlow;
                        // Stop emission if a handler returns true
                        if acc.get::<bool>().unwrap_or(false) {
                            ControlFlow::Break(acc.clone())
                        } else {
                            ControlFlow::Continue(value.clone())
                        }
                    })
                    .build(),
                // before-send signal: emitted before sending RTSP messages
                // Returns true to send message, false to cancel
                // Since: 1.14
                glib::subclass::Signal::builder("before-send")
                    .param_types([RTSPMessage::static_type()])
                    .return_type::<bool>()
                    .accumulator(|_hint, _acc, value| {
                        use std::ops::ControlFlow;
                        // Stop emission if a handler returns false
                        let val = value.get::<bool>().unwrap_or(true);
                        if !val {
                            ControlFlow::Break(value.clone())
                        } else {
                            ControlFlow::Continue(value.clone())
                        }
                    })
                    .build(),
                // request-rtcp-key signal: emitted to get RTCP encryption key
                // Returns GstCaps with SRTCP key parameters
                // Since: 1.4
                glib::subclass::Signal::builder("request-rtcp-key")
                    .param_types([u32::static_type()])
                    .return_type::<Option<gst::Caps>>()
                    .build(),
                // request-rtp-key signal: emitted to get RTP encryption key
                // Returns GstCaps with SRTP key parameters
                // Since: 1.26
                glib::subclass::Signal::builder("request-rtp-key")
                    .param_types([u32::static_type()])
                    .return_type::<Option<gst::Caps>>()
                    .build(),
                // get-parameter action: send GET_PARAMETER RTSP request
                // Returns true if request sent, false otherwise
                glib::subclass::Signal::builder("get-parameter")
                    .action()
                    .param_types([
                        String::static_type(),           // parameter name
                        Option::<String>::static_type(), // content type
                        gst::Promise::static_type(),     // promise for async result
                    ])
                    .return_type::<bool>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let parameter = args[1].get::<String>().expect("parameter arg");
                        let content_type = args[2].get::<Option<String>>().ok().flatten();
                        let promise = args[3].get::<gst::Promise>().expect("promise arg");

                        Some(
                            imp.handle_get_parameter(&parameter, content_type.as_deref(), &promise)
                                .to_value(),
                        )
                    })
                    .build(),
                // get-parameters action: send GET_PARAMETER RTSP request for multiple parameters
                // Returns true if request sent, false otherwise
                glib::subclass::Signal::builder("get-parameters")
                    .action()
                    .param_types([
                        Vec::<String>::static_type(),    // parameter names array
                        Option::<String>::static_type(), // content type
                        gst::Promise::static_type(),     // promise for async result
                    ])
                    .return_type::<bool>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let parameters = args[1].get::<Vec<String>>().expect("parameters arg");
                        let content_type = args[2].get::<Option<String>>().ok().flatten();
                        let promise = args[3].get::<gst::Promise>().expect("promise arg");

                        Some(
                            imp.handle_get_parameters(
                                parameters,
                                content_type.as_deref(),
                                &promise,
                            )
                            .to_value(),
                        )
                    })
                    .build(),
                // set-parameter action: send SET_PARAMETER RTSP request
                // Returns true if request sent, false otherwise
                glib::subclass::Signal::builder("set-parameter")
                    .action()
                    .param_types([
                        String::static_type(),           // parameter name
                        String::static_type(),           // parameter value
                        Option::<String>::static_type(), // content type
                        gst::Promise::static_type(),     // promise for async result
                    ])
                    .return_type::<bool>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let parameter = args[1].get::<String>().expect("parameter arg");
                        let value = args[2].get::<String>().expect("value arg");
                        let content_type = args[3].get::<Option<String>>().ok().flatten();
                        let promise = args[4].get::<gst::Promise>().expect("promise arg");

                        Some(
                            imp.handle_set_parameter(
                                &parameter,
                                &value,
                                content_type.as_deref(),
                                &promise,
                            )
                            .to_value(),
                        )
                    })
                    .build(),
                // push-backchannel-buffer action: send audio buffer through backchannel
                // Returns GstFlowReturn indicating success/failure
                glib::subclass::Signal::builder("push-backchannel-buffer")
                    .action()
                    .param_types([
                        u32::static_type(),         // stream index
                        gst::Buffer::static_type(), // buffer with media data
                    ])
                    .return_type::<gst::FlowReturn>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let stream_id = args[1].get::<u32>().expect("stream_id arg");
                        let buffer = args[2].get::<gst::Buffer>().expect("buffer arg");

                        Some(
                            imp.handle_push_backchannel_buffer(stream_id, &buffer)
                                .to_value(),
                        )
                    })
                    .build(),
                // push-backchannel-sample action: send audio sample through backchannel
                // Returns GstFlowReturn indicating success/failure
                glib::subclass::Signal::builder("push-backchannel-sample")
                    .action()
                    .param_types([
                        u32::static_type(),         // stream index
                        gst::Sample::static_type(), // sample with media data and caps
                    ])
                    .return_type::<gst::FlowReturn>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let stream_id = args[1].get::<u32>().expect("stream_id arg");
                        let sample = args[2].get::<gst::Sample>().expect("sample arg");

                        Some(
                            imp.handle_push_backchannel_sample(stream_id, &sample)
                                .to_value(),
                        )
                    })
                    .build(),
                // set-mikey-parameter action: set SRTP key via MIKEY protocol
                // Returns true if request accepted, false otherwise
                glib::subclass::Signal::builder("set-mikey-parameter")
                    .action()
                    .param_types([
                        u32::static_type(),          // stream index
                        gst::Caps::static_type(),    // MIKEY capabilities
                        gst::Promise::static_type(), // promise for async result
                    ])
                    .return_type::<bool>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let stream_id = args[1].get::<u32>().expect("stream_id arg");
                        let caps = args[2].get::<gst::Caps>().expect("caps arg");
                        let promise = args[3].get::<gst::Promise>().expect("promise arg");

                        Some(
                            imp.handle_set_mikey_parameter(stream_id, &caps, &promise)
                                .to_value(),
                        )
                    })
                    .build(),
                // remove-key action: remove encryption key for stream
                // Returns true if key removed, false otherwise
                glib::subclass::Signal::builder("remove-key")
                    .action()
                    .param_types([
                        u32::static_type(), // stream index
                    ])
                    .return_type::<bool>()
                    .class_handler(|args| {
                        let obj = args[0].get::<super::RtspSrc>().expect("signal arg");
                        let imp = obj.imp();
                        let stream_id = args[1].get::<u32>().expect("stream_id arg");

                        Some(imp.handle_remove_key(stream_id).to_value())
                    })
                    .build(),
                // soft-limit signal: emitted when jitterbuffer reaches soft threshold (warning level)
                // Notifies application of buffer fill approaching limit for adaptive streaming
                // Since: 1.0
                glib::subclass::Signal::builder("soft-limit")
                    .param_types([u32::static_type()]) // stream index experiencing soft limit
                    .build(),
                // hard-limit signal: emitted when jitterbuffer reaches hard threshold (critical level)
                // Alerts application of critical buffer overflow condition requiring immediate action
                // Since: 1.0
                glib::subclass::Signal::builder("hard-limit")
                    .param_types([u32::static_type()]) // stream index experiencing hard limit
                    .build(),
                // backchannel-detected signal: emitted when ONVIF backchannel stream is detected
                // Notifies application that a backchannel stream is available for two-way audio
                // Since: 1.0
                glib::subclass::Signal::builder("backchannel-detected")
                    .param_types([
                        u32::static_type(),       // stream index of backchannel
                        gst::Caps::static_type(), // capabilities of backchannel stream
                    ])
                    .build(),
                // handle-request signal: emitted when server sends an RTSP request to the client (PRP-52)
                // Allows application to handle server-initiated requests like ANNOUNCE, REDIRECT, etc.
                // Since: 1.0
                glib::subclass::Signal::builder("handle-request")
                    .param_types([
                        RTSPMessage::static_type(), // request message from server
                        RTSPMessage::static_type(), // response message to be filled by application
                    ])
                    .build(),
            ]
        });
        SIGNALS.as_ref()
    }
}

impl GstObjectImpl for RtspSrc {}

impl ElementImpl for RtspSrc {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "RTSP Source",
                "Source/Network",
                "Receive audio or video from a network device via the Real Time Streaming Protocol (RTSP) (RFC 2326, 7826)",
                "Nirbheek Chauhan <nirbheek centricular com>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let src_pad_template = gst::PadTemplate::new(
                "stream_%u",
                gst::PadDirection::Src,
                gst::PadPresence::Sometimes,
                &gst::Caps::new_empty_simple("application/x-rtp"),
            )
            .unwrap();

            // Sink pad template for ONVIF backchannel
            let sink_pad_template = gst::PadTemplate::new(
                "backchannel_%u",
                gst::PadDirection::Sink,
                gst::PadPresence::Request,
                &gst::Caps::new_empty_simple("application/x-rtp"),
            )
            .unwrap();

            vec![src_pad_template, sink_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }

    fn send_event(&self, event: gst::Event) -> bool {
        match event.view() {
            gst::EventView::Seek(seek) => {
                let (_rate, flags, _start_type, start, _stop_type, _stop) = seek.get();

                gst::debug!(CAT, "Received seek event to position: {:?}", start);

                let cmd_queue = self.cmd_queue();
                let position = if let gst::GenericFormattedValue::Time(Some(time)) = start {
                    time
                } else {
                    gst::ClockTime::ZERO
                };

                RUNTIME.spawn(async move {
                    let _ = cmd_queue.send(Commands::Seek { position, flags }).await;
                });

                true
            }
            _ => self.parent_send_event(event),
        }
    }

    fn change_state(
        &self,
        transition: gst::StateChange,
    ) -> Result<gst::StateChangeSuccess, gst::StateChangeError> {
        match transition {
            gst::StateChange::NullToReady => {
                self.start().map_err(|err_msg| {
                    self.post_error_message(err_msg);
                    gst::StateChangeError
                })?;
            }
            gst::StateChange::PausedToPlaying => {
                let cmd_queue = self.cmd_queue();
                //self.async_start().map_err(|_| gst::StateChangeError)?;
                RUNTIME.spawn(async move { cmd_queue.send(Commands::Play).await });
            }
            gst::StateChange::PlayingToPaused => {
                let cmd_queue = self.cmd_queue();
                RUNTIME.spawn(async move { cmd_queue.send(Commands::Pause).await });
            }
            _ => {}
        }

        let mut ret = self.parent_change_state(transition)?;

        match transition {
            gst::StateChange::ReadyToPaused | gst::StateChange::PlayingToPaused => {
                ret = gst::StateChangeSuccess::NoPreroll;
            }
            gst::StateChange::PausedToReady => {
                // Clear buffer queue when going to READY state
                self.clear_buffer_queue();
                match tokio::runtime::Handle::try_current() {
                    Ok(_) => {
                        // If the app does set_state(NULL) from a block_on() inside its own tokio
                        // runtime, calling block_on() on our own runtime will cause a panic
                        // because of nested blocking calls. So, shutdown the task from another
                        // thread.
                        // The app's usage is also incorrect since they are blocking the runtime
                        // on I/O, so emit a warning.
                        gst::warning!(
                            CAT,
                            "Blocking I/O: state change to NULL called from an async \
                            tokio context, redirecting to another thread to prevent \
                            the tokio panic, but you should refactor your code to \
                            make use of gst::Element::call_async and set the state \
                            to NULL from there, without blocking the runtime"
                        );
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.obj().call_async(move |element| {
                            tx.send(element.imp().stop()).unwrap();
                        });
                        rx.recv().unwrap()
                    }
                    Err(_) => self.stop(),
                }
                .map_err(|err_msg| {
                    self.post_error_message(err_msg);
                    gst::StateChangeError
                })?;
            }
            _ => (),
        }

        Ok(ret)
    }
}

impl BinImpl for RtspSrc {}

impl URIHandlerImpl for RtspSrc {
    const URI_TYPE: gst::URIType = gst::URIType::Src;

    fn protocols() -> &'static [&'static str] {
        &[
            "rtsp", "rtspu", "rtspt", "rtsph", "rtsp-sdp", "rtsps", "rtspsu", "rtspst", "rtspsh",
        ]
    }

    fn uri(&self) -> Option<String> {
        let settings = self.settings.lock().unwrap();

        settings.location.as_ref().map(Url::to_string)
    }

    fn set_uri(&self, uri: &str) -> Result<(), glib::Error> {
        self.set_location(Some(uri))
    }
}

type RtspStream =
    Pin<Box<dyn Stream<Item = Result<Message<Body>, super::tcp_message::ReadError>> + Send>>;
type RtspSink = Pin<Box<dyn Sink<Message<Body>, Error = std::io::Error> + Send>>;

impl RtspSrc {
    #[track_caller]
    fn cmd_queue(&self) -> mpsc::Sender<Commands> {
        self.command_queue.lock().unwrap().as_ref().unwrap().clone()
    }

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let Some(url) = self.settings.lock().unwrap().location.clone() else {
            return Err(gst::error_msg!(
                gst::ResourceError::Settings,
                ["No location set"]
            ));
        };

        gst::info!(CAT, imp = self, "Location: {url}",);

        gst::info!(CAT, imp = self, "Starting RTSP connection thread.. ");

        let task_src = self.ref_counted();

        let mut task_handle = self.task_handle.lock().unwrap();

        let (tx, rx) = mpsc::channel(1);
        {
            let mut cmd_queue_opt = self.command_queue.lock().unwrap();
            debug_assert!(cmd_queue_opt.is_none());
            cmd_queue_opt.replace(tx);
        }

        let join_handle = RUNTIME.spawn(async move {
            gst::info!(CAT, "Connecting to {url} ..");
            #[cfg(feature = "telemetry")]
            let _connection_span = super::telemetry::SpanHelper::connection_span(url.as_str());
            #[cfg(feature = "telemetry")]
            task_src.metrics.record_connection_attempt();

            // Get retry configuration from settings
            let settings = task_src.settings.lock().unwrap().clone();
            let retry_config = super::retry::RetryConfig {
                strategy: settings.retry_strategy,
                max_attempts: settings.max_reconnection_attempts,
                initial_delay: std::time::Duration::from_nanos(settings.initial_retry_delay.nseconds()),
                max_delay: std::time::Duration::from_nanos(settings.reconnection_timeout.nseconds()),
                linear_step: std::time::Duration::from_nanos(settings.linear_retry_step.nseconds()),
            };

            let mut retry_calc = super::retry::RetryCalculator::new(retry_config)
                .with_server_url(&url.to_string());
            
            #[cfg(feature = "telemetry")]
            {
                retry_calc = retry_calc.with_telemetry(task_src.metrics.clone());
            }

            // Create proxy config if configured
            let proxy_config = if let Some(ref proxy_url) = settings.proxy {
                match super::proxy::ProxyConfig::from_url(
                    proxy_url,
                    settings.proxy_id.clone(),
                    settings.proxy_pw.clone(),
                ) {
                    Ok(config) => Some(config),
                    Err(e) => {
                        gst::warning!(CAT, "Failed to parse proxy URL: {}", e);
                        None
                    }
                }
            } else {
                // Try to get from environment if not explicitly configured
                super::proxy::ProxyConfig::from_env()
            };

            // Check if HTTP tunneling should be used
            let use_tunneling = super::http_tunnel::should_use_tunneling(&url, settings.http_tunnel_mode);
            
            // Apply timeout to entire connection process
            let connection_timeout = std::time::Duration::from_nanos(settings.timeout.nseconds());
            let connection_result = if use_tunneling {
                gst::info!(CAT, "Using HTTP tunneling for connection");
                
                // HTTP tunneling connection logic
                time::timeout(connection_timeout, async {
                    loop {
                        // Mark connection attempt start
                        retry_calc.mark_connection_start();
                        
                        // Create HTTP tunnel
                        let mut tunnel = match super::http_tunnel::HttpTunnel::new(
                            &url,
                            settings.proxy.clone(),
                            settings.proxy_id.clone(),
                            settings.proxy_pw.clone(),
                        ) {
                            Ok(t) => t,
                            Err(e) => {
                                return Err(format!("Failed to create HTTP tunnel: {}", e));
                            }
                        };
                        
                        match tunnel.connect().await {
                            Ok(_) => {
                                // Record successful connection
                                retry_calc.record_connection_result(true, false);
                                // Return a marker to indicate tunneled connection
                                return Ok(Err(tunnel)); // Using Err to differentiate from regular TcpStream
                            }
                            Err(err) => {
                                // Record failed connection
                                retry_calc.record_connection_result(false, false);
                                
                                if let Some(delay) = retry_calc.next_delay() {
                                    let attempt = retry_calc.current_attempt();
                                    let auto_summary = retry_calc.get_auto_summary().unwrap_or_default();
                                    gst::warning!(
                                        CAT,
                                        "HTTP tunnel connection to '{}' failed (attempt {attempt}): {err:#?}. Retrying in {} ms... Auto mode: {}",
                                        url, delay.as_millis(), auto_summary
                                    );
                                    
                                    // Post a message about the retry attempt
                                    let msg = gst::message::Element::builder(
                                        gst::Structure::builder("rtsp-connection-retry")
                                            .field("attempt", attempt)
                                            .field("error", format!("{err:#?}"))
                                            .field("next-delay-ms", delay.as_millis() as u64)
                                            .build(),
                                    )
                                    .src(&*task_src.obj())
                                    .build();
                                    let _ = task_src.obj().post_message(msg);
                                    
                                    tokio::time::sleep(delay).await;
                                } else {
                                    return Err(format!("Failed to establish HTTP tunnel to '{}' after {} attempts: {err:#?}", url, retry_calc.current_attempt()));
                                }
                            }
                        }
                    }
                }).await
            } else {
                // Normal connection using connection racer
                let racing_config = super::connection_racer::ConnectionRacingConfig {
                    strategy: settings.connection_racing,
                    max_parallel_connections: settings.max_parallel_connections,
                    racing_delay_ms: settings.racing_delay_ms,
                    racing_timeout: std::time::Duration::from_nanos(settings.racing_timeout.nseconds()),
                    proxy_config,
                };
                let mut racer = super::connection_racer::ConnectionRacer::new(racing_config);
                
                time::timeout(connection_timeout, async {
                    loop {
                        // Update racing strategy based on auto mode recommendations
                        if let Some(recommended_strategy) = retry_calc.get_racing_strategy() {
                            racer.update_strategy(recommended_strategy);
                        }
                        
                        // Mark connection attempt start
                        retry_calc.mark_connection_start();
                        
                        match racer.connect(&url).await {
                            Ok(s) => {
                                // Record successful connection
                                retry_calc.record_connection_result(true, false);
                                return Ok(Ok(s)); // Ok(Ok()) for regular connection
                            }
                            Err(err) => {
                                // Record failed connection
                                retry_calc.record_connection_result(false, false);

                                if let Some(delay) = retry_calc.next_delay() {
                                    let attempt = retry_calc.current_attempt();
                                    let auto_summary = retry_calc.get_auto_summary().unwrap_or_default();
                                    gst::warning!(
                                        CAT,
                                        "Connection to '{}' failed (attempt {attempt}): {err:#?}. Retrying in {} ms... Auto mode: {}",
                                        url, delay.as_millis(), auto_summary
                                    );

                                    // Post a message about the retry attempt
                                    let msg = gst::message::Element::builder(
                                        gst::Structure::builder("rtsp-connection-retry")
                                            .field("attempt", attempt)
                                            .field("error", format!("{err:#?}"))
                                            .field("next-delay-ms", delay.as_millis() as u64)
                                            .build(),
                                    )
                                    .src(&*task_src.obj())
                                    .build();
                                    let _ = task_src.obj().post_message(msg);

                                    tokio::time::sleep(delay).await;
                                } else {
                                    return Err(format!("Failed to connect to RTSP server '{}' after {} attempts: {err:#?}", url, retry_calc.current_attempt()));
                                }
                            }
                        }
                    }
                }).await
            };

            // Handle both tunneled and normal connections
            let is_tunneled = matches!(connection_result, Ok(Ok(Err(_))));
            
            let (stream, sink) = if is_tunneled {
                // HTTP tunneled connection
                match connection_result {
                    Ok(Ok(Err(tunnel))) => {
                        gst::info!(CAT, "Connected via HTTP tunnel!");
                        
                        // For now, we'll need to properly implement the tunnel stream/sink conversion
                        // This is a placeholder implementation
                        gst::element_imp_error!(
                            task_src,
                            gst::CoreError::NotImplemented,
                            ["HTTP tunnel stream conversion not yet fully implemented"]
                        );
                        return;
                    }
                    _ => unreachable!(),
                }
            } else {
                // Normal TCP/TLS connection
                match connection_result {
                    Ok(Ok(Ok(mut rtsp_stream))) => {
                        // Set TCP nodelay if it's a plain TCP stream
                        if let super::tls::RtspStream::Plain(ref tcp_stream) = rtsp_stream {
                            let _ = tcp_stream.set_nodelay(true);
                            gst::info!(CAT, "Connected via plain TCP!");
                        } else {
                            gst::info!(CAT, "Connected via TLS!");
                        }
                        
                        // Use the RtspStream directly which implements AsyncRead + AsyncWrite
                        // We can use tokio::io::split for both Plain and TLS variants
                        let (read, write) = tokio::io::split(rtsp_stream);
                        let stream = Box::pin(super::tcp_message::async_read(read, MAX_MESSAGE_SIZE).fuse());
                        let sink = Box::pin(super::tcp_message::async_write(write));
                        (stream, sink)
                    }
                    Ok(Err(err)) => {
                        gst::element_imp_error!(
                            task_src,
                            gst::ResourceError::OpenRead,
                            ["{}", err]
                        );
                        return;
                    }
                    Err(_elapsed) => {
                        gst::element_imp_error!(
                            task_src,
                            gst::ResourceError::OpenRead,
                            ["Connection timeout after {} seconds", connection_timeout.as_secs()]
                        );
                        #[cfg(feature = "telemetry")]
                        task_src.metrics.record_timeout_error();
                        return;
                    }
                    _ => unreachable!(),
                }
            };

            gst::info!(CAT, "Connection established (tunneled: {})", is_tunneled);

            #[cfg(feature = "telemetry")]
            {
                let connection_time = std::time::Instant::now().duration_since(std::time::Instant::now()).as_millis() as u64;
                task_src.metrics.record_connection_success(connection_time);
                event!(Level::INFO, "RTSP connection established (tunneled: {})", is_tunneled);
            }

            // Get authentication credentials from settings
            let (user_id, user_pw) = {
                let settings = task_src.settings.lock().unwrap();
                (settings.user_id.clone(), settings.user_pw.clone())
            };

            let mut state = RtspTaskState::new(url, stream, sink, user_id, user_pw);

            let task_ret = task_src.rtsp_task(&mut state, rx).await;
            gst::info!(CAT, "Exited rtsp_task");

            // Cleanup after stopping
            for h in &state.handles {
                h.abort();
            }
            for h in state.handles {
                let _ = h.await;
            }
            let obj = task_src.obj();
            for e in obj.iterate_sorted() {
                let Ok(e) = e else {
                    continue;
                };
                if let Err(err) = e.set_state(gst::State::Null) {
                    gst::warning!(CAT, "{} failed to go to Null state: {err:?}", e.name());
                }
            }
            for pad in obj.src_pads() {
                if let Err(err) = obj.remove_pad(&pad) {
                    gst::warning!(CAT, "Failed to remove pad {}: {err:?}", pad.name());
                }
            }
            for e in obj.iterate_sorted() {
                let Ok(e) = e else {
                    continue;
                };
                if let Err(err) = obj.remove(&e) {
                    gst::warning!(CAT, "Failed to remove element {}: {err:?}", e.name());
                }
            }

            // Post the element error after cleanup
            if let Err(err) = task_ret {
                gst::element_imp_error!(
                    task_src,
                    gst::CoreError::Failed,
                    ["RTSP task exited: {err:#?}"]
                );
            }
            gst::info!(CAT, "Cleanup complete");
        });

        debug_assert!(task_handle.is_none());
        task_handle.replace(join_handle);

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        gst::info!(CAT, "Stopping...");
        let cmd_queue = self.cmd_queue();
        let task_handle = { self.task_handle.lock().unwrap().take() };

        RUNTIME.block_on(async {
            let (tx, rx) = oneshot::channel();
            if let Ok(()) = cmd_queue.send(Commands::Teardown(Some(tx))).await {
                if let Err(_elapsed) = time::timeout(Duration::from_millis(500), rx).await {
                    gst::warning!(
                        CAT,
                        "Timeout waiting for Teardown, going to NULL asynchronously"
                    );
                }
            }
        });

        if let Some(join_handle) = task_handle {
            gst::debug!(CAT, "Waiting for RTSP connection thread to shut down..");
            let _ = RUNTIME.block_on(join_handle);
        }

        self.command_queue.lock().unwrap().take();

        gst::info!(CAT, imp = self, "Stopped");

        Ok(())
    }

    fn make_rtp_appsrc(
        &self,
        rtpsession_n: usize,
        caps: &gst::Caps,
        manager: &RtspManager,
    ) -> std::result::Result<gst_app::AppSrc, glib::Error> {
        let callbacks = gst_app::AppSrcCallbacks::builder()
            .enough_data(|appsrc| {
                gst::warning!(CAT, "appsrc {} is overrunning: enough data!", appsrc.name());
            })
            .build();
        let builder = gst_app::AppSrc::builder()
            .name(format!("rtp_appsrc_{rtpsession_n}"))
            .format(gst::Format::Time);

        #[cfg(feature = "v1_18")]
        let builder = builder.handle_segment_change(true);

        let appsrc = builder
            .caps(caps)
            .stream_type(gst_app::AppStreamType::Stream)
            .max_bytes(0)
            .callbacks(callbacks)
            .is_live(true)
            .build();

        // Set properties for v1_16 compatibility
        #[cfg(feature = "v1_20")]
        appsrc.set_property_from_str("leaky-type", "downstream"); // 2 = downstream
        #[cfg(feature = "v1_20")]
        appsrc.set_property("max-time", 2_000_000_000u64); // 2 seconds in nanoseconds
        let obj = self.obj();
        obj.add(&appsrc).map_err(|e| glib::Error::new(gst::ResourceError::Failed, &e.to_string()))?;
        appsrc
            .static_pad("src")
            .unwrap()
            .link(&manager.rtp_recv_sinkpad(rtpsession_n).unwrap())
            .map_err(|_| glib::Error::new(gst::ResourceError::Failed, "Failed to link pads"))?;
        let templ = obj.pad_template("stream_%u").unwrap();
        let ghostpad = gst::GhostPad::builder_from_template(&templ)
            .name(format!("stream_{rtpsession_n}"))
            .build();
        gst::info!(CAT, "Adding ghost srcpad {}", ghostpad.name());
        obj.add_pad(&ghostpad)
            .expect("Adding a ghostpad should never fail");
        appsrc.sync_state_with_parent().map_err(|e| glib::Error::new(gst::ResourceError::Failed, &e.to_string()))?;
        Ok(appsrc)
    }

    fn make_rtcp_appsrc(
        &self,
        rtpsession_n: usize,
        manager: &RtspManager,
    ) -> std::result::Result<gst_app::AppSrc, glib::Error> {
        let builder = gst_app::AppSrc::builder()
            .name(format!("rtcp_appsrc_{rtpsession_n}"))
            .format(gst::Format::Time);

        #[cfg(feature = "v1_18")]
        let builder = builder.handle_segment_change(true);

        let appsrc = builder
            .caps(&RTCP_CAPS)
            .stream_type(gst_app::AppStreamType::Stream)
            .is_live(true)
            .build();
        self.obj().add(&appsrc).map_err(|e| glib::Error::new(gst::ResourceError::Failed, &e.to_string()))?;
        appsrc
            .static_pad("src")
            .unwrap()
            .link(&manager.rtcp_recv_sinkpad(rtpsession_n).unwrap())
            .map_err(|_| glib::Error::new(gst::ResourceError::Failed, "Failed to link pads"))?;
        appsrc.sync_state_with_parent().map_err(|e| glib::Error::new(gst::ResourceError::Failed, &e.to_string()))?;
        Ok(appsrc)
    }

    fn make_rtcp_appsink<
        F: FnMut(&gst_app::AppSink) -> Result<gst::FlowSuccess, gst::FlowError> + Send + 'static,
    >(
        &self,
        rtpsession_n: usize,
        manager: &RtspManager,
        on_rtcp: F,
    ) -> std::result::Result<(), glib::Error> {
        let cmd_tx_eos = self.cmd_queue();
        let cbs = gst_app::app_sink::AppSinkCallbacks::builder()
            .eos(move |_appsink| {
                let cmd_tx = cmd_tx_eos.clone();
                RUNTIME.spawn(async move {
                    let _ = cmd_tx.send(Commands::Teardown(None)).await;
                });
            })
            .new_sample(on_rtcp)
            .build();

        let rtcp_appsink = gst_app::AppSink::builder()
            .name(format!("rtcp_appsink_{rtpsession_n}"))
            .sync(false)
            .async_(false)
            .callbacks(cbs)
            .build();
        self.obj().add(&rtcp_appsink).map_err(|e| glib::Error::new(gst::ResourceError::Failed, &e.to_string()))?;
        manager
            .rtcp_send_srcpad(rtpsession_n)
            .unwrap()
            .link(&rtcp_appsink.static_pad("sink").unwrap())
            .map_err(|_| glib::Error::new(gst::ResourceError::Failed, "Failed to link pads"))?;
        Ok(())
    }

    fn post_start(&self, code: &str, text: &str) {
        let obj = self.obj();
        let msg = gst::message::Progress::builder(gst::ProgressType::Start, code, text)
            .src(&*obj)
            .build();
        let _ = obj.post_message(msg);
    }

    fn post_complete(&self, code: &str, text: &str) {
        let obj = self.obj();
        let msg = gst::message::Progress::builder(gst::ProgressType::Complete, code, text)
            .src(&*obj)
            .build();
        let _ = obj.post_message(msg);
    }

    fn post_cancelled(&self, code: &str, text: &str) {
        let obj = self.obj();
        let msg = gst::message::Progress::builder(gst::ProgressType::Canceled, code, text)
            .src(&*obj)
            .build();
        let _ = obj.post_message(msg);
    }

    async fn rtsp_task(
        &self,
        state: &mut RtspTaskState,
        mut cmd_rx: mpsc::Receiver<Commands>,
    ) -> std::result::Result<(), super::error::RtspError> {
        let cmd_tx = self.cmd_queue();

        let settings = { self.settings.lock().unwrap().clone() };

        // OPTIONS
        state.options().await?;

        // DESCRIBE
        state.describe().await?;

        let mut session: Option<Session> = None;
        // SETUP streams (TCP interleaved)
        state.setup_params = {
            state
                .setup(
                    &mut session,
                    settings.port_start,
                    &settings.protocols,
                    TransportMode::Play,
                    &settings.select_streams,
                    &settings.stream_filter,
                )
                .await?
        };

        // Punch NAT holes for UDP transport after SETUP
        RtspTaskState::punch_nat_holes(&state.setup_params, settings.nat_method).await;
        let manager = RtspManager::new_with_settings(
            std::env::var("USE_RTP2").is_ok_and(|s| s == "1"),
            Some(&settings),
        );

        let obj = self.obj();
        manager
            .add_to(obj.upcast_ref::<gst::Bin>())
            .expect("Adding the manager cannot fail");

        let mut tcp_interleave_appsrcs = HashMap::new();
        for (rtpsession_n, p) in state.setup_params.iter_mut().enumerate() {
            let (tx, rx) = mpsc::channel(1);
            let on_rtcp = move |appsink: &_| on_rtcp_udp(appsink, tx.clone());
            match &mut p.transport {
                RtspTransportInfo::UdpMulticast {
                    dest,
                    port: (rtp_port, rtcp_port),
                    ttl,
                } => {
                    let rtp_socket = bind_port(*rtp_port, dest.is_ipv4())?;
                    let rtcp_socket = rtcp_port.and_then(|p| {
                        bind_port(p, dest.is_ipv4())
                            .map_err(|err| {
                                gst::warning!(CAT, "Could not bind to RTCP port: {err:?}");
                                err
                            })
                            .ok()
                    });

                    match &dest {
                        IpAddr::V4(addr) => {
                            rtp_socket.join_multicast_v4(*addr, Ipv4Addr::UNSPECIFIED)?;
                            if let Some(ttl) = ttl {
                                let _ = rtp_socket.set_multicast_ttl_v4(*ttl as u32);
                            }
                            let _ = rtp_socket.set_multicast_loop_v4(false);
                            if let Some(rtcp_socket) = &rtcp_socket {
                                if let Err(err) =
                                    rtcp_socket.join_multicast_v4(*addr, Ipv4Addr::UNSPECIFIED)
                                {
                                    gst::warning!(
                                        CAT,
                                        "Failed to join RTCP multicast address {addr}: {err:?}"
                                    );
                                } else {
                                    if let Some(ttl) = ttl {
                                        let _ = rtcp_socket.set_multicast_ttl_v4(*ttl as u32);
                                    }
                                    let _ = rtcp_socket.set_multicast_loop_v4(false);
                                }
                            }
                        }
                        IpAddr::V6(addr) => {
                            rtp_socket.join_multicast_v6(addr, 0)?;
                            let _ = rtp_socket.set_multicast_loop_v6(false);
                            if let Some(rtcp_socket) = &rtcp_socket {
                                if let Err(err) = rtcp_socket.join_multicast_v6(addr, 0) {
                                    gst::warning!(
                                        CAT,
                                        "Failed to join RTCP multicast address {addr}: {err:?}"
                                    );
                                } else {
                                    let _ = rtcp_socket.set_multicast_loop_v6(false);
                                }
                            }
                        }
                    };

                    let rtp_appsrc = self.make_rtp_appsrc(rtpsession_n, &p.caps, &manager)?;
                    p.rtp_appsrc = Some(rtp_appsrc.clone());

                    // Configure probation on the RTP session
                    manager.configure_session(rtpsession_n as u32, settings.probation)?;
                    // Spawn RTP udp receive task
                    let buffer_queue = self.buffer_queue.clone();
                    state.handles.push(RUNTIME.spawn(async move {
                        udp_rtp_task(
                            &rtp_socket,
                            rtp_appsrc,
                            settings.timeout,
                            settings.receive_mtu,
                            None,
                            Some(buffer_queue),
                        )
                        .await
                    }));

                    // Spawn RTCP udp send/recv task
                    if let Some(rtcp_socket) = rtcp_socket {
                        let rtcp_dest = rtcp_port.and_then(|p| Some(SocketAddr::new(*dest, p)));
                        let rtcp_appsrc = self.make_rtcp_appsrc(rtpsession_n, &manager)?;
                        self.make_rtcp_appsink(rtpsession_n, &manager, on_rtcp)?;
                        let buffer_queue = self.buffer_queue.clone();
                        state.handles.push(RUNTIME.spawn(async move {
                            udp_rtcp_task(
                                &rtcp_socket,
                                rtcp_appsrc,
                                rtcp_dest,
                                true,
                                rx,
                                Some(buffer_queue),
                            )
                            .await
                        }));
                    }
                }
                RtspTransportInfo::Udp {
                    source,
                    server_port,
                    client_port: _,
                    sockets,
                } => {
                    let Some((rtp_socket, rtcp_socket)) = sockets.take() else {
                        gst::warning!(
                            CAT,
                            "Skipping: no UDP sockets for {rtpsession_n}: {:#?}",
                            p.transport
                        );
                        continue;
                    };
                    let (rtp_sender_addr, rtcp_sender_addr) = match (source, server_port) {
                        (Some(ip), Some((rtp_port, Some(rtcp_port)))) => {
                            let ip = ip.parse().unwrap();
                            (
                                Some(SocketAddr::new(ip, *rtp_port)),
                                Some(SocketAddr::new(ip, *rtcp_port)),
                            )
                        }
                        (Some(ip), Some((rtp_port, None))) => {
                            (Some(SocketAddr::new(ip.parse().unwrap(), *rtp_port)), None)
                        }
                        _ => (None, None),
                    };

                    // Spawn RTP udp receive task
                    let rtp_appsrc = self.make_rtp_appsrc(rtpsession_n, &p.caps, &manager)?;
                    p.rtp_appsrc = Some(rtp_appsrc.clone());

                    // Configure probation on the RTP session
                    manager.configure_session(rtpsession_n as u32, settings.probation)?;
                    let buffer_queue = self.buffer_queue.clone();
                    state.handles.push(RUNTIME.spawn(async move {
                        udp_rtp_task(
                            &rtp_socket,
                            rtp_appsrc,
                            settings.timeout,
                            settings.receive_mtu,
                            rtp_sender_addr,
                            Some(buffer_queue),
                        )
                        .await
                    }));

                    // Spawn RTCP udp send/recv task
                    if let Some(rtcp_socket) = rtcp_socket {
                        let rtcp_appsrc = self.make_rtcp_appsrc(rtpsession_n, &manager)?;
                        self.make_rtcp_appsink(rtpsession_n, &manager, on_rtcp)?;
                        let buffer_queue = self.buffer_queue.clone();
                        state.handles.push(RUNTIME.spawn(async move {
                            udp_rtcp_task(
                                &rtcp_socket,
                                rtcp_appsrc,
                                rtcp_sender_addr,
                                false,
                                rx,
                                Some(buffer_queue),
                            )
                            .await
                        }));
                    }
                }
                RtspTransportInfo::Tcp {
                    channels: (rtp_channel, rtcp_channel),
                } => {
                    let rtp_appsrc = self.make_rtp_appsrc(rtpsession_n, &p.caps, &manager)?;
                    p.rtp_appsrc = Some(rtp_appsrc.clone());

                    // Configure probation on the RTP session
                    manager.configure_session(rtpsession_n as u32, settings.probation)?;
                    tcp_interleave_appsrcs.insert(*rtp_channel, rtp_appsrc);

                    if let Some(rtcp_channel) = rtcp_channel {
                        // RTCP SR
                        let rtcp_appsrc = self.make_rtcp_appsrc(rtpsession_n, &manager)?;
                        tcp_interleave_appsrcs.insert(*rtcp_channel, rtcp_appsrc.clone());
                        // RTCP RR
                        let rtcp_channel = *rtcp_channel;
                        let cmd_tx = cmd_tx.clone();
                        self.make_rtcp_appsink(rtpsession_n, &manager, move |appsink| {
                            on_rtcp_tcp(appsink, cmd_tx.clone(), rtcp_channel)
                        })?;
                    }
                }
            }
        }

        // Setup RTP srcpad handling first, before signaling no_more_pads
        // This ensures ghost pad targets are set before element claims to be ready
        manager.recv.connect_pad_added(|manager, pad| {
            if pad.direction() != gst::PadDirection::Src {
                return;
            }
            let Some(obj) = manager
                .parent()
                .and_then(|o| o.downcast::<gst::Element>().ok())
            else {
                return;
            };
            let name = pad.name();
            match *name.split('_').collect::<Vec<_>>() {
                // rtpbin and rtp2
                ["recv", "rtp", "src", stream_id, ssrc, pt]
                | ["rtp", "src", stream_id, ssrc, pt] => {
                    if stream_id.parse::<u32>().is_err() {
                        gst::info!(CAT, "Ignoring srcpad with invalid stream id: {name}");
                        return;
                    };
                    gst::info!(CAT, "Setting rtpbin pad {} as ghostpad target", name);
                    let srcpad = obj
                        .static_pad(&format!("stream_{stream_id}"))
                        .expect("ghostpad should've been available already");
                    let ghostpad = srcpad
                        .downcast::<gst::GhostPad>()
                        .expect("rtspsrc src pads are ghost pads");
                    if let Err(err) = ghostpad.set_target(Some(pad)) {
                        gst::element_error!(
                            obj,
                            gst::ResourceError::Failed,
                            (
                                "Failed to set ghostpad {} target {}: {err:?}",
                                ghostpad.name(),
                                name
                            ),
                            ["pt: {pt}, ssrc: {ssrc}"]
                        );
                    } else {
                        gst::info!(
                            CAT,
                            "Successfully set ghostpad {} target - signaling no_more_pads",
                            ghostpad.name()
                        );

                        // Flush any queued buffers for this stream now that the pad is linked
                        if let Ok(rtsp_src) = obj.clone().downcast::<super::RtspSrc>() {
                            // Try to find the corresponding AppSrc for this stream
                            // AppSrcs are named like "rtp_appsrc_{stream_id}"
                            if let Ok(stream_id_num) = stream_id.parse::<u32>() {
                                let appsrc_name = format!("rtp_appsrc_{}", stream_id_num);
                                if let Ok(bin) = obj.clone().downcast::<gst::Bin>() {
                                    if let Some(appsrc_elem) = bin.by_name(&appsrc_name) {
                                        if let Ok(appsrc) =
                                            appsrc_elem.downcast::<gst_app::AppSrc>()
                                        {
                                            gst::debug!(
                                                CAT,
                                                "Flushing queued buffers for {} after pad link",
                                                appsrc_name
                                            );
                                            rtsp_src.imp().flush_queued_buffers(&appsrc);
                                        }
                                    }
                                }
                            }
                        }

                        // Signal that pads are ready now that ghost pad has a target
                        obj.no_more_pads();
                    }
                }
                _ => {
                    gst::info!(CAT, "Ignoring unknown srcpad: {name}");
                }
            }
        });

        let mut expected_response: Option<(Method, u32)> = None;
        let mut keepalive_interval = tokio::time::interval(Duration::from_secs(30)); // Will be updated after session
        keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                msg = state.stream.next() => match msg {
                    Some(Ok(rtsp_types::Message::Data(data))) => {
                        let Some(appsrc) = tcp_interleave_appsrcs.get(&data.channel_id()) else {
                            gst::warning!(CAT,
                                "ignored data of size {}: unknown channel {}",
                                data.len(),
                                data.channel_id()
                            );
                            continue;
                        };
                        let t = appsrc.current_running_time();
                        let channel_id = data.channel_id();
                        gst::trace!(CAT, "Received data on channel {channel_id}");
                        // TODO: this should be from_mut_slice() after making the necessary
                        // modifications to Body
                        let mut buffer = gst::Buffer::from_slice(data.into_body());
                        let bufref = buffer.make_mut();
                        bufref.set_dts(t);
                        // Handle flow errors gracefully - unlinked pads are expected during setup
                        match self.push_buffer_with_queue(&appsrc, buffer) {
                            Ok(_) => {
                                gst::trace!(CAT, "Successfully handled buffer on pad {} for channel {}", appsrc.name(), channel_id);
                            }
                            Err(err) => {
                                gst::error!(CAT, "Failed to handle buffer on pad {} for channel {}: {}", appsrc.name(), channel_id, err);
                                return Err(err.into());
                            }
                        }
                    }
                    Some(Ok(rtsp_types::Message::Request(req))) => {
                        // TODO: implement incoming GET_PARAMETER requests
                        gst::debug!(CAT, "<-- {req:#?}");
                    }
                    Some(Ok(rtsp_types::Message::Response(rsp))) => {
                        gst::debug!(CAT, "<-- {rsp:#?}");

                        // Reset session activity on any response
                        state.session_manager.reset_activity();

                        // Check if this is a GET_PARAMETER or SET_PARAMETER response
                        if let Some((expected, cseq)) = &expected_response {
                            if *expected == Method::GetParameter {
                                // GET_PARAMETER response received
                                let parameters = state.get_parameter_response(&rsp, *cseq, session.as_ref()).await?;

                                // Check if this was a user request with a promise
                                if let Some(promise) = state.pending_get_parameter_promises.remove(cseq) {
                                    // Fulfill promise with parameters
                                    let mut s = gst::Structure::builder("rtsp-parameters");
                                    for (key, value) in parameters {
                                        s = s.field(&key, value);
                                    }
                                    promise.reply(Some(s.build()));
                                    gst::debug!(CAT, "GET_PARAMETER promise fulfilled");
                                } else {
                                    // This was a keep-alive request
                                    gst::debug!(CAT, "Keep-alive response received");
                                }
                                expected_response = None;
                                continue;
                            } else if *expected == Method::SetParameter {
                                // SET_PARAMETER response received
                                state.set_parameter_response(&rsp, *cseq, session.as_ref()).await?;

                                // Check if this was a user request with a promise
                                if let Some(promise) = state.pending_set_parameter_promises.remove(cseq) {
                                    // Fulfill promise with success
                                    let s = gst::Structure::builder("rtsp-success")
                                        .field("success", true)
                                        .build();
                                    promise.reply(Some(s));
                                    gst::debug!(CAT, "SET_PARAMETER promise fulfilled");
                                }
                                expected_response = None;
                                continue;
                            }
                        }

                        let Some((expected, cseq)) = &expected_response else {
                            continue;
                        };
                        let Some(s) = &session else {
                            return Err(RtspError::internal(format!("Can't handle {expected:?} response, no SETUP")));
                        };
                        match expected {
                            Method::Play => {
                                state.play_response(&rsp, *cseq, s).await?;
                                self.post_complete("request", "PLAY response received");
                            }
                            Method::Pause => {
                                state.pause_response(&rsp, *cseq, s).await?;
                                self.post_complete("request", "PAUSE response received");
                            }
                            Method::Teardown => state.teardown_response(&rsp, *cseq, s).await?,
                            Method::GetParameter | Method::SetParameter => {
                                // Already handled above
                                unreachable!("GET_PARAMETER and SET_PARAMETER should have been handled above");
                            }
                            m => unreachable!("BUG: unexpected response method: {m:?}"),
                        };
                    }
                    Some(Err(e)) => {
                        // TODO: reconnect or ignore if UDP sockets are still receiving data
                        gst::error!(CAT, "I/O error: {e:?}, quitting");
                        return Err(gst::FlowError::Error.into());
                    }
                    None => {
                        // TODO: reconnect or ignore if UDP sockets are still receiving data
                        gst::error!(CAT, "TCP connection EOF, quitting");
                        return Err(gst::FlowError::Eos.into());
                    }
                },
                Some(cmd) = cmd_rx.recv() => match cmd {
                    Commands::Play => {
                        let Some(s) = &session else {
                            return Err(RtspError::internal("Can't PLAY, no SETUP"));
                        };
                        self.post_start("request", "PLAY request sent");
                        let cseq = state.play(s).await.inspect_err(|_err| {
                            self.post_cancelled("request", "PLAY request cancelled");
                        })?;
                        expected_response = Some((Method::Play, cseq));
                    },
                    Commands::Pause => {
                        let Some(s) = &session else {
                            return Err(RtspError::internal("Can't PAUSE, no SETUP"));
                        };
                        self.post_start("request", "PAUSE request sent");
                        let cseq = state.pause(s).await.inspect_err(|_err| {
                            self.post_cancelled("request", "PAUSE request cancelled");
                        })?;
                        expected_response = Some((Method::Pause, cseq));
                    },
                    Commands::Seek { position, flags } => {
                        let Some(s) = &session else {
                            return Err(RtspError::internal("Can't SEEK, no SETUP"));
                        };
                        gst::info!(CAT, "Processing seek to position {:?} with flags {:?}", position, flags);

                        // Get seek format from settings
                        let seek_format = self.settings.lock().unwrap().seek_format;

                        // Send PLAY request with Range header for seek
                        let cseq = state.play_with_range(s, Some(position), seek_format).await.inspect_err(|_err| {
                            self.post_cancelled("request", "SEEK request cancelled");
                        })?;
                        expected_response = Some((Method::Play, cseq));

                        // Handle flush if needed
                        if flags.contains(gst::SeekFlags::FLUSH) {
                            // Flush all appsrcs
                            for params in &state.setup_params {
                                if let Some(ref appsrc) = params.rtp_appsrc {
                                    let _ = appsrc.send_event(gst::event::FlushStart::new());
                                    let _ = appsrc.send_event(gst::event::FlushStop::builder(true).build());
                                }
                            }
                        }

                        // Send new segment event
                        let segment = gst::FormattedSegment::<gst::ClockTime>::new();
                        let mut segment = segment.clone();
                        segment.set_start(position);
                        segment.set_position(position);

                        for params in &state.setup_params {
                            if let Some(ref appsrc) = params.rtp_appsrc {
                                let _ = appsrc.send_event(gst::event::Segment::new(&segment));
                            }
                        }
                    },
                    Commands::Teardown(tx) => {
                        gst::info!(CAT, "Received Teardown command");
                        let Some(s) = &session else {
                            return Err(RtspError::internal("Can't TEARDOWN, no SETUP"));
                        };
                        let _ = state.teardown(s).await;
                        if let Some(tx) = tx {
                            let _ = tx.send(());
                        }
                        break;
                    }
                    Commands::Data(data) => {
                        // We currently only send RTCP RR as data messages, this will change when
                        // we support TCP ONVIF backchannels
                        state.sink.send(Message::Data(data)).await?;
                        gst::debug!(CAT, "Sent RTCP RR over TCP");
                    }
                    Commands::Reconnect => {
                        gst::info!(CAT, "Received Reconnect command - not yet implemented");
                        // TODO: Implement reconnection logic for lost connections
                        // This would involve re-establishing the TCP connection and
                        // re-sending DESCRIBE/SETUP/PLAY commands
                    }
                    Commands::GetParameter { parameters, promise } => {
                        gst::debug!(CAT, "Received GetParameter command: {:?}", parameters);

                        // Send GET_PARAMETER request
                        match state.get_parameter(session.as_ref(), parameters).await {
                            Ok(cseq) => {
                                // Store promise to be fulfilled when response arrives
                                state.pending_get_parameter_promises.insert(cseq, promise);
                                expected_response = Some((Method::GetParameter, cseq));
                            }
                            Err(e) => {
                                gst::error!(CAT, "Failed to send GET_PARAMETER: {e:?}");
                                // Reject the promise with error
                                let s = gst::Structure::builder("rtsp-error")
                                    .field("error", format!("Failed to send GET_PARAMETER: {e}"))
                                    .build();
                                promise.reply(Some(s));
                            }
                        }
                    }
                    Commands::SetParameter { parameters, promise } => {
                        gst::debug!(CAT, "Received SetParameter command: {:?}", parameters);

                        // Send SET_PARAMETER request
                        match state.set_parameter(session.as_ref(), parameters).await {
                            Ok(cseq) => {
                                // Store promise to be fulfilled when response arrives
                                state.pending_set_parameter_promises.insert(cseq, promise);
                                expected_response = Some((Method::SetParameter, cseq));
                            }
                            Err(e) => {
                                gst::error!(CAT, "Failed to send SET_PARAMETER: {e:?}");
                                // Reject the promise with error
                                let s = gst::Structure::builder("rtsp-error")
                                    .field("error", format!("Failed to send SET_PARAMETER: {e}"))
                                    .build();
                                promise.reply(Some(s));
                            }
                        }
                    }
                },
                _ = keepalive_interval.tick() => {
                    // Check if we need to send keep-alive
                    if let Some(s) = &session {
                        if state.session_manager.needs_keepalive() {
                            gst::debug!(CAT, "Sending keep-alive GET_PARAMETER");
                            // Send empty GET_PARAMETER for keep-alive
                            match state.get_parameter(Some(s), None).await {
                                Ok(cseq) => {
                                    expected_response = Some((Method::GetParameter, cseq));
                                }
                                Err(e) => {
                                    gst::warning!(CAT, "Failed to send keep-alive: {e:?}");
                                }
                            }
                        }

                        // Check for session timeout
                        if state.session_manager.is_timed_out() {
                            gst::error!(CAT, "Session timed out, terminating");
                            return Err(RtspError::internal("Session timeout"));
                        }
                    }
                },
                else => {
                    gst::error!(CAT, "No select statement matched, breaking loop");
                    break;
                }
            }
        }
        Ok(())
    }
}

struct RtspManager {
    recv: gst::Element,
    send: gst::Element,
    using_rtp2: bool,
}

impl RtspManager {
    fn new(rtp2: bool) -> Self {
        Self::new_with_settings(rtp2, None)
    }

    fn new_with_settings(rtp2: bool, jitter_settings: Option<&Settings>) -> Self {
        let (recv, send) = if rtp2 {
            let recv = gst::ElementFactory::make_with_name("rtprecv", None)
                .unwrap_or_else(|_| panic!("rtprecv not found"));
            let send = gst::ElementFactory::make("rtpsend")
                .property("rtp-id", recv.property::<String>("rtp-id"))
                .build()
                .unwrap_or_else(|_| panic!("rtpsend not found"));
            (recv, send)
        } else {
            let e = gst::ElementFactory::make_with_name("rtpbin", None)
                .unwrap_or_else(|_| panic!("rtpbin not found"));

            // Apply jitterbuffer settings to rtpbin (similar to original rtspsrc)
            if let Some(settings) = jitter_settings {
                e.set_property("latency", settings.latency_ms);
                e.set_property("drop-on-latency", settings.drop_on_latency);

                // Apply buffer mode (similar to original rtspsrc set_manager_buffer_mode)
                Self::apply_buffer_mode(&e, settings.buffer_mode);

                // Apply RTCP settings (similar to original rtspsrc rtpbin configuration)
                Self::apply_rtcp_settings(&e, settings);
            }

            (e.clone(), e)
        };
        if !rtp2 {
            let on_bye = |args: &[glib::Value]| {
                let m = args[0].get::<gst::Element>().unwrap();
                let obj = m.parent()?;
                let bin = obj.downcast::<gst::Bin>().unwrap();
                bin.send_event(gst::event::Eos::new());
                None
            };
            recv.connect("on-bye-ssrc", true, move |args| {
                gst::info!(CAT, "Received BYE packet");
                on_bye(args)
            });
            recv.connect("on-bye-timeout", true, move |args| {
                gst::info!(CAT, "BYE due to timeout");
                on_bye(args)
            });
        }
        RtspManager {
            recv,
            send,
            using_rtp2: rtp2,
        }
    }

    fn rtp_recv_sinkpad(&self, rtpsession: usize) -> Option<gst::Pad> {
        let name = if self.using_rtp2 {
            format!("rtp_sink_{rtpsession}")
        } else {
            format!("recv_rtp_sink_{rtpsession}")
        };
        gst::info!(CAT, "requesting {name} for receiving RTP");
        self.recv.request_pad_simple(&name)
    }

    fn rtcp_recv_sinkpad(&self, rtpsession: usize) -> Option<gst::Pad> {
        let name = if self.using_rtp2 {
            format!("rtcp_sink_{rtpsession}")
        } else {
            format!("recv_rtcp_sink_{rtpsession}")
        };
        gst::info!(CAT, "requesting {name} for receiving RTCP");
        self.recv.request_pad_simple(&name)
    }

    fn rtcp_send_srcpad(&self, rtpsession: usize) -> Option<gst::Pad> {
        let name = if self.using_rtp2 {
            format!("rtcp_src_{rtpsession}")
        } else {
            format!("send_rtcp_src_{rtpsession}")
        };
        gst::info!(CAT, "requesting {name} for sending RTCP");
        self.send.request_pad_simple(&name)
    }

    fn add_to<T: IsA<gst::Bin>>(&self, bin: &T) -> Result<(), glib::BoolError> {
        if self.using_rtp2 {
            bin.add_many([&self.recv, &self.send])?;
            self.recv.sync_state_with_parent()?;
            self.send.sync_state_with_parent()?;
        } else {
            bin.add_many([&self.recv])?;
            self.recv.sync_state_with_parent()?;
        }
        Ok(())
    }

    fn configure_session(&self, session_id: u32, probation: u32) -> Result<(), super::error::RtspError> {
        // Configure probation on the RTP session (similar to original rtspsrc)
        if !self.using_rtp2 {
            // Use get-internal-session signal to get the rtpsession object
            if let Some(session) = self
                .recv
                .emit_by_name::<Option<glib::Object>>("get-internal-session", &[&session_id])
            {
                session.set_property("probation", probation);
                gst::debug!(CAT, "Set probation={} on session {}", probation, session_id);
            } else {
                gst::warning!(CAT, "Could not get internal session {}", session_id);
            }
        }
        Ok(())
    }

    fn apply_buffer_mode(rtpbin: &gst::Element, buffer_mode: BufferMode) {
        // Apply buffer mode to rtpbin (similar to original rtspsrc)
        // Check if rtpbin supports buffer-mode property
        if let Some(_property) = rtpbin.find_property("buffer-mode") {
            let mode_int = buffer_mode.as_int();
            gst::debug!(
                CAT,
                "Setting buffer-mode={} ({}) on rtpbin",
                mode_int,
                buffer_mode.as_str()
            );

            if buffer_mode != BufferMode::Auto {
                // Direct mode setting (non-auto modes)
                rtpbin.set_property_from_str("buffer-mode", buffer_mode.as_str());
            } else {
                // Auto mode - let rtpbin decide based on conditions
                // For now, default to Buffer mode for auto
                gst::debug!(CAT, "Auto buffer mode - using buffer(2) as default");
                rtpbin.set_property_from_str("buffer-mode", "buffer"); // Buffer mode
            }
        } else {
            gst::warning!(CAT, "rtpbin does not support buffer-mode property");
        }
    }

    fn apply_rtcp_settings(rtpbin: &gst::Element, settings: &Settings) {
        // Apply RTCP settings to rtpbin (similar to original rtspsrc)

        // Apply max-rtcp-rtp-time-diff (similar to original rtspsrc line 4623-4626)
        if let Some(_property) = rtpbin.find_property("max-rtcp-rtp-time-diff") {
            gst::debug!(
                CAT,
                "Setting max-rtcp-rtp-time-diff={} on rtpbin",
                settings.max_rtcp_rtp_time_diff
            );
            rtpbin.set_property("max-rtcp-rtp-time-diff", settings.max_rtcp_rtp_time_diff);
        } else {
            gst::warning!(
                CAT,
                "rtpbin does not support max-rtcp-rtp-time-diff property"
            );
        }

        // Apply do-retransmission (similar to original rtspsrc line 4531)
        if let Some(_property) = rtpbin.find_property("do-retransmission") {
            gst::debug!(
                CAT,
                "Setting do-retransmission={} on rtpbin",
                settings.do_retransmission
            );
            rtpbin.set_property("do-retransmission", settings.do_retransmission);
        } else {
            gst::warning!(CAT, "rtpbin does not support do-retransmission property");
        }

        gst::debug!(
            CAT,
            "Applied RTCP settings: do-rtcp={}, do-retransmission={}, max-rtcp-rtp-time-diff={}",
            settings.do_rtcp,
            settings.do_retransmission,
            settings.max_rtcp_rtp_time_diff
        );
    }
}

struct RtspTaskState {
    cseq: u32,
    url: Url,
    version: Version,
    content_base_or_location: Option<String>,
    aggregate_control: Option<Url>,
    sdp: Option<sdp_types::Session>,

    stream:
        Pin<Box<dyn Stream<Item = Result<Message<Body>, super::tcp_message::ReadError>> + Send>>,
    sink: Pin<Box<dyn Sink<Message<Body>, Error = std::io::Error> + Send>>,

    setup_params: Vec<RtspSetupParams>,
    handles: Vec<JoinHandle<()>>,
    session_manager: super::session_manager::SessionManager,
    auth_state: AuthState,
    user_id: Option<String>,
    user_pw: Option<String>,
    pending_get_parameter_promises: std::collections::HashMap<u32, gst::Promise>,
    pending_set_parameter_promises: std::collections::HashMap<u32, gst::Promise>,
}

struct RtspSetupParams {
    control_url: Url,
    transport: RtspTransportInfo,
    rtp_appsrc: Option<gst_app::AppSrc>,
    caps: gst::Caps,
}

impl RtspTaskState {
    /// Send dummy packets to punch holes in NAT for UDP transport
    async fn punch_nat_holes(setup_params: &[RtspSetupParams], nat_method: NatMethod) {
        if nat_method != NatMethod::Dummy {
            return;
        }

        for params in setup_params {
            if let RtspTransportInfo::Udp {
                source,
                server_port,
                sockets,
                ..
            } = &params.transport
            {
                if let (Some(sockets), Some(source), Some(server_port)) =
                    (sockets, source, server_port)
                {
                    // Parse server address
                    let server_addr = match source.parse::<IpAddr>() {
                        Ok(addr) => addr,
                        Err(_) => {
                            gst::warning!(
                                CAT,
                                "Failed to parse server address for NAT punching: {}",
                                source
                            );
                            continue;
                        }
                    };

                    // Send dummy RTP packet
                    if let Some(rtp_port) = server_port.0.checked_add(0) {
                        let rtp_dest = SocketAddr::new(server_addr, rtp_port);
                        // Send 3 dummy packets with small delay between them
                        for i in 0..3 {
                            // Minimal RTP packet (12 bytes header)
                            let dummy_rtp = vec![
                                0x80, 0x60, // V=2, P=0, X=0, CC=0, M=0, PT=96
                                0x00, 0x00, // Sequence number
                                0x00, 0x00, 0x00, 0x00, // Timestamp
                                0x00, 0x00, 0x00, 0x00, // SSRC
                            ];
                            match sockets.0.send_to(&dummy_rtp, rtp_dest).await {
                                Ok(_) => gst::debug!(
                                    CAT,
                                    "Sent NAT punch packet {} to RTP port {}",
                                    i + 1,
                                    rtp_port
                                ),
                                Err(e) => {
                                    gst::warning!(CAT, "Failed to send NAT punch packet: {}", e)
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    }

                    // Send dummy RTCP packet if we have RTCP socket
                    if let (Some(rtcp_socket), Some(rtcp_port)) = (&sockets.1, server_port.1) {
                        let rtcp_dest = SocketAddr::new(server_addr, rtcp_port);
                        // Send 3 dummy packets
                        for i in 0..3 {
                            // Minimal RTCP RR packet (8 bytes)
                            let dummy_rtcp = vec![
                                0x80, 0xc9, // V=2, P=0, RC=0, PT=201 (RR)
                                0x00, 0x01, // Length
                                0x00, 0x00, 0x00, 0x00, // SSRC
                            ];
                            match rtcp_socket.send_to(&dummy_rtcp, rtcp_dest).await {
                                Ok(_) => gst::debug!(
                                    CAT,
                                    "Sent NAT punch packet {} to RTCP port {}",
                                    i + 1,
                                    rtcp_port
                                ),
                                Err(e) => {
                                    gst::warning!(CAT, "Failed to send NAT punch packet: {}", e)
                                }
                            }
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                    }
                }
            }
        }
    }

    fn new(
        url: Url,
        stream: RtspStream,
        sink: RtspSink,
        user_id: Option<String>,
        user_pw: Option<String>,
    ) -> Self {
        RtspTaskState {
            cseq: 0u32,
            url,
            version: Version::V1_0,
            content_base_or_location: None,
            aggregate_control: None,
            sdp: None,
            stream,
            sink,
            setup_params: Vec::new(),
            handles: Vec::new(),
            session_manager: super::session_manager::SessionManager::new(),
            auth_state: AuthState::default(),
            user_id,
            user_pw,
            pending_get_parameter_promises: std::collections::HashMap::new(),
            pending_set_parameter_promises: std::collections::HashMap::new(),
        }
    }

    /// Send request with authentication retry on 401
    async fn send_request_with_auth(
        &mut self,
        method: Method,
        uri: Url,
        mut req_builder: rtsp_types::RequestBuilder,
    ) -> Result<Response<Body>, super::error::RtspError> {
        // Add auth header if we already have auth state
        if let Some(auth_header) = auth::generate_auth_header(
            &mut self.auth_state,
            self.user_id.as_deref(),
            self.user_pw.as_deref(),
            &method,
            &uri.to_string(),
        ) {
            gst::debug!(CAT, "Adding authentication header to {method:?} request");
            req_builder = req_builder.header(auth::AUTHORIZATION.clone(), auth_header);
        }
        let req = req_builder.build(Body::default());

        gst::debug!(CAT, "-->> {req:#?}");
        self.sink.send(req.into()).await?;

        let rsp = match self.stream.next().await {
            Some(Ok(rtsp_types::Message::Response(rsp))) => Ok(rsp),
            Some(Ok(m)) => Err(RtspError::Protocol(super::error::ProtocolError::InvalidResponse {
                details: format!("Expected authentication response, got: {:?}", m),
            })),
            Some(Err(e)) => Err(e.into()),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "authentication response",
            )
            .into()),
        }?;
        gst::debug!(CAT, "<<-- {rsp:#?}");

        // Check if we got a 401 Unauthorized response
        if auth::requires_auth(&rsp) {
            gst::debug!(
                CAT,
                "Got 401 Unauthorized for {method:?}, attempting authentication"
            );

            // Parse the authentication challenge
            if let Err(e) = self.auth_state.parse_challenge(&rsp) {
                gst::warning!(CAT, "Failed to parse authentication challenge: {e}");
                return Ok(rsp);
            }

            // If we have credentials, retry with authentication
            if self.user_id.is_some() && self.user_pw.is_some() {
                gst::debug!(CAT, "Retrying {method:?} with authentication");

                // Increment cseq for retry
                self.cseq += 1;

                // Rebuild request with new cseq and auth header
                let mut retry_builder = Request::builder(method.clone(), self.version)
                    .typed_header::<CSeq>(&self.cseq.into())
                    .request_uri(uri.clone())
                    .header(USER_AGENT, DEFAULT_USER_AGENT);

                // Add auth header
                if let Some(auth_header) = auth::generate_auth_header(
                    &mut self.auth_state,
                    self.user_id.as_deref(),
                    self.user_pw.as_deref(),
                    &method,
                    &uri.to_string(),
                ) {
                    retry_builder = retry_builder.header(auth::AUTHORIZATION.clone(), auth_header);
                }
                let retry_req = retry_builder.build(Body::default());

                gst::debug!(CAT, "-->> (retry) {retry_req:#?}");
                self.sink.send(retry_req.into()).await?;

                let retry_rsp = match self.stream.next().await {
                    Some(Ok(rtsp_types::Message::Response(rsp))) => Ok(rsp),
                    Some(Ok(m)) => Err(RtspError::Protocol(super::error::ProtocolError::InvalidResponse {
                        details: format!("Expected authentication retry response, got: {:?}", m),
                    })),
                    Some(Err(e)) => Err(e.into()),
                    None => Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        "authentication retry response",
                    )
                    .into()),
                }?;
                gst::debug!(CAT, "<<-- (retry) {retry_rsp:#?}");

                return Ok(retry_rsp);
            } else {
                gst::warning!(CAT, "Authentication required but no credentials provided");
            }
        }

        Ok(rsp)
    }

    fn check_response(
        rsp: &Response<Body>,
        cseq: u32,
        req_name: Method,
        session: Option<&Session>,
    ) -> Result<(), super::error::RtspError> {
        if rsp.status() != StatusCode::Ok {
            return Err(RtspError::internal(format!(
                "{req_name:?} request failed: {}",
                rsp.reason_phrase()
            )));
        }
        match rsp.typed_header::<CSeq>() {
            Ok(Some(v)) => {
                if *v != cseq {
                    return Err(RtspError::internal("cseq does not match"));
                }
            }
            Ok(None) => {
                gst::warning!(
                    CAT,
                    "No cseq in response, continuing... {:#?}",
                    rsp.headers().collect::<Vec<_>>()
                );
            }
            Err(_) => {
                gst::warning!(
                    CAT,
                    "Invalid cseq in response, continuing... {:#?}",
                    rsp.headers().collect::<Vec<_>>()
                );
            }
        };
        if let Some(s) = session {
            if let Some(have_s) = rsp.typed_header::<Session>()? {
                if s.0 != have_s.0 {
                    return Err(RtspError::internal(format!(
                        "Session in header {} does not match our session {}",
                        s.0, have_s.0
                    )));
                }
            } else {
                gst::warning!(
                    CAT,
                    "No Session header in response, continuing... {:#?}",
                    rsp.headers().collect::<Vec<_>>()
                );
            }
        }
        Ok(())
    }

    async fn options(&mut self) -> Result<(), RtspError> {
        self.cseq += 1;
        let req_builder = Request::builder(Method::Options, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .request_uri(self.url.clone())
            .header(USER_AGENT, DEFAULT_USER_AGENT);

        let rsp = self
            .send_request_with_auth(Method::Options, self.url.clone(), req_builder)
            .await?;
        Self::check_response(&rsp, self.cseq, Method::Options, None)?;

        let Ok(Some(methods)) = rsp.typed_header::<Public>() else {
            return Err(RtspError::internal(
                "OPTIONS response does not contain a valid Public header",
            ));
        };

        let needed = [
            Method::Describe,
            Method::Setup,
            Method::Play,
            Method::Teardown,
        ];
        let mut unsupported = Vec::new();
        for method in &needed {
            if !methods.contains(method) {
                unsupported.push(format!("{method:?}"));
            }
        }
        if !unsupported.is_empty() {
            Err(RtspError::internal(format!(
                "Server doesn't support the required method{} {}",
                if unsupported.len() == 1 { "" } else { "s:" },
                unsupported.join(",")
            )))
        } else {
            Ok(())
        }
    }

    async fn describe(&mut self) -> Result<(), RtspError> {
        self.cseq += 1;
        let req_builder = Request::builder(Method::Describe, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header(ACCEPT, "application/sdp")
            .request_uri(self.url.clone());

        let rsp = self
            .send_request_with_auth(Method::Describe, self.url.clone(), req_builder)
            .await?;
        gst::debug!(
            CAT,
            "<<-- Response {:#?}",
            rsp.headers().collect::<Vec<_>>()
        );
        Self::check_response(&rsp, self.cseq, Method::Describe, None)?;

        self.content_base_or_location = rsp
            .header(&CONTENT_BASE)
            .or(rsp.header(&CONTENT_LOCATION))
            .map(|v| v.to_string());

        gst::info!(CAT, "{}", std::str::from_utf8(rsp.body()).unwrap());
        // TODO: read range attribute from SDP for VOD use-cases
        let sdp = sdp_types::Session::parse(rsp.body())?;
        gst::debug!(CAT, "{sdp:#?}");

        self.sdp.replace(sdp);
        Ok(())
    }

    #[allow(clippy::result_large_err)]
    fn parse_setup_transports(
        transports: &Transports,
        s: &mut gst::Structure,
        protocols: &[RtspProtocol],
        mode: &TransportMode,
    ) -> Result<RtspTransportInfo, RtspError> {
        let mut last_error =
            RtspError::internal("No matching transport found matching selected protocols");
        let mut parsed_transports = Vec::new();
        for transport in transports.iter() {
            let Transport::Rtp(t) = transport else {
                last_error =
                    RtspError::internal(format!("Expected RTP transport, got {transports:#?}"));
                continue;
            };
            // RTSP 2 specifies that we can have multiple SSRCs in the response
            // Transport header, but it's not clear why, so we don't support it
            if let Some(ssrc) = t.params.ssrc.first() {
                s.set("ssrc", ssrc)
            }
            if !t.params.mode.is_empty() && !t.params.mode.contains(mode) {
                last_error = RtspError::internal(format!(
                    "Requested mode {:?} doesn't match server modes: {:?}",
                    mode, t.params.mode
                ));
                continue;
            }
            let parsed = match RtspTransportInfo::try_from(t) {
                Ok(p) => p,
                Err(err) => {
                    last_error = err.into();
                    continue;
                }
            };
            parsed_transports.push(parsed);
        }
        for protocol in protocols {
            for n in 0..parsed_transports.len() {
                if parsed_transports[n].to_protocol() == *protocol {
                    let t = parsed_transports.swap_remove(n);
                    return Ok(t);
                }
            }
        }
        Err(last_error)
    }

    async fn setup(
        &mut self,
        session: &mut Option<Session>,
        port_start: u16,
        protocols: &[RtspProtocol],
        mode: TransportMode,
        select_streams: &StreamSelection,
        stream_filter: &Option<String>,
    ) -> Result<Vec<RtspSetupParams>, RtspError> {
        // Clone what we need from sdp to avoid borrow conflicts
        let sdp_clone = self.sdp.clone().expect("Must have SDP by now");
        let base = self
            .content_base_or_location
            .as_ref()
            .and_then(|s| Url::parse(s).ok())
            .unwrap_or_else(|| self.url.clone());
        self.aggregate_control = sdp_clone
            .get_first_attribute_value("control")
            // No attribute and no value have the same meaning for us
            .ok()
            .flatten()
            .and_then(|v| sdp::parse_control_path(v, &base));
        let mut b = gst::Structure::builder("application/x-rtp");

        // TODO: parse range for VOD
        let skip_attrs = ["control", "range"];
        for sdp_types::Attribute { attribute, value } in &sdp_clone.attributes {
            if skip_attrs.contains(&attribute.as_str()) {
                continue;
            }
            b = b.field(format!("a-{attribute}"), value);
        }
        // TODO: parse global extmap

        let message_structure = b.build();

        let conn_source = sdp_clone
            .connection
            .as_ref()
            .map(|c| c.connection_address.as_str())
            .filter(|c| !c.is_empty())
            .unwrap_or_else(|| base.host_str().unwrap());
        let mut port_next = port_start;
        let mut stream_num = 0;
        let mut setup_params: Vec<RtspSetupParams> = Vec::new();
        for m in &sdp_clone.medias {
            // Check if media type is supported
            if !["audio", "video", "metadata", "application"].contains(&m.media.as_str()) {
                gst::info!(CAT, "Ignoring unsupported media type: {}", m.media);
                continue;
            }

            // Apply stream selection filter
            if !select_streams.should_select_media(&m.media) {
                gst::info!(
                    CAT,
                    "Skipping {} stream based on select-streams property",
                    m.media
                );
                continue;
            }

            // Apply codec filter if specified
            if let Some(filter) = stream_filter {
                let mut skip = true;

                // Check if codec matches filter
                // fmt is a string with space-separated format identifiers
                for format in m.fmt.split_whitespace() {
                    if let Ok(_pt) = format.parse::<u8>() {
                        for attr in &m.attributes {
                            if attr.attribute == "rtpmap" {
                                if let Some(val) = &attr.value {
                                    // Parse rtpmap: "96 H264/90000"
                                    let parts: Vec<&str> = val.split_whitespace().collect();
                                    if parts.len() >= 2 {
                                        let pt_str = parts[0];
                                        if pt_str == format {
                                            let codec_info = parts[1];
                                            let codec_name =
                                                codec_info.split('/').next().unwrap_or("");

                                            // Check if codec matches filter (case-insensitive)
                                            if codec_name
                                                .to_lowercase()
                                                .contains(&filter.to_lowercase())
                                            {
                                                skip = false;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if skip {
                    gst::info!(
                        CAT,
                        "Skipping {} stream - codec doesn't match filter '{}'",
                        m.media,
                        filter
                    );
                    continue;
                }
            }
            let media_control = m
                .get_first_attribute_value("control")
                // No attribute and no value have the same meaning for us
                .ok()
                .flatten()
                .and_then(|v| sdp::parse_control_path(v, &base));
            let Some(control_url) = media_control
                .clone()
                .or_else(|| self.aggregate_control.clone())
            else {
                gst::warning!(
                    CAT,
                    "No session control or media control for {} fmt {}, ignoring",
                    m.media,
                    m.fmt
                );
                continue;
            };

            // RTP caps
            let Ok(pt) = m.fmt.parse::<u8>() else {
                gst::error!(CAT, "Could not parse pt: {}, ignoring media", m.fmt);
                continue;
            };

            let mut s = message_structure.clone();
            let media = m.media.to_ascii_lowercase();
            s.set("media", &media);
            s.set("payload", pt as i32);

            // Check if SRTP is indicated by the media protocol
            if super::srtp::is_srtp_protocol(&m.proto) {
                s.set("uses-srtp", true);
                s.set("srtp-profile", &m.proto);
            }

            if let Err(err) = sdp::parse_media_attributes(&m.attributes, pt, &media, &mut s) {
                gst::warning!(
                    CAT,
                    "Skipping media {} {}, no rtpmap: {err:?}",
                    m.media,
                    m.fmt
                );
                continue;
            }

            // SETUP
            let mut rtp_socket: Option<UdpSocket> = None;
            let mut rtcp_socket: Option<UdpSocket> = None;
            let mut transports = Vec::new();
            let (conn_protocols, is_ipv4) = sdp::parse_connections(&m.connections);

            let protocols = if !conn_protocols.is_empty() {
                let p = protocols.iter().cloned().collect::<BTreeSet<_>>();
                p.intersection(&conn_protocols).cloned().collect::<Vec<_>>()
            } else {
                protocols.to_owned()
            };

            if protocols.is_empty() {
                gst::error!(CAT, "No available protocols left, skipping media");
                continue;
            }

            if protocols.contains(&RtspProtocol::UdpMulticast) {
                let params = RtpTransportParameters {
                    mode: vec![mode.clone()],
                    multicast: true,
                    ..Default::default()
                };
                transports.push(Transport::Rtp(RtpTransport {
                    profile: RtpProfile::Avp,
                    lower_transport: Some(RtpLowerTransport::Udp),
                    params,
                }));
            }
            if protocols.contains(&RtspProtocol::Udp) {
                let (sock1, rtp_port) = bind_start_port(port_next, is_ipv4).await;
                // Get the actual port that was successfully bound
                port_next = rtp_port;
                let (sock2, rtcp_port) = bind_start_port(rtp_port + 1, is_ipv4).await;
                rtp_socket = Some(sock1);
                rtcp_socket = Some(sock2);
                let params = RtpTransportParameters {
                    mode: vec![mode.clone()],
                    unicast: true,
                    client_port: Some((rtp_port, Some(rtcp_port))),
                    ..Default::default()
                };
                transports.push(Transport::Rtp(RtpTransport {
                    profile: RtpProfile::Avp,
                    lower_transport: Some(RtpLowerTransport::Udp),
                    params,
                }));
            }
            if protocols.contains(&RtspProtocol::Tcp) {
                let params = RtpTransportParameters {
                    mode: vec![mode.clone()],
                    interleaved: Some((stream_num, Some(stream_num + 1))),
                    ..Default::default()
                };
                transports.push(Transport::Rtp(RtpTransport {
                    // RTSP 2.0 adds AVPF and more
                    profile: RtpProfile::Avp,
                    lower_transport: Some(RtpLowerTransport::Tcp),
                    params,
                }));
            }

            self.cseq += 1;
            let transports: Transports = transports.as_slice().into();
            let mut req_builder = Request::builder(Method::Setup, self.version)
                .typed_header::<CSeq>(&self.cseq.into())
                .header(USER_AGENT, DEFAULT_USER_AGENT)
                .typed_header::<Transports>(&transports)
                .request_uri(control_url.clone());
            if let Some(s) = session {
                req_builder = req_builder.typed_header::<Session>(s);
            }

            let rsp = self
                .send_request_with_auth(Method::Setup, control_url.clone(), req_builder)
                .await?;
            gst::debug!(CAT, "<<-- {rsp:#?}");
            Self::check_response(&rsp, self.cseq, Method::Setup, session.as_ref())?;
            let new_session = rsp
                .typed_header::<Session>()?
                .ok_or_else(|| RtspError::internal("No session in SETUP response"))?;
            // Manually strip timeout field: https://github.com/sdroege/rtsp-types/issues/24
            session.replace(Session(new_session.0.clone(), None));

            // Also parse the raw Session header to get timeout value
            if let Some(session_header) = rsp
                .headers()
                .find(|(name, _)| name == &rtsp_types::headers::SESSION)
                .map(|(_, value)| value.as_str())
            {
                self.session_manager
                    .parse_session_with_timeout(session_header);
            } else {
                self.session_manager.set_session(new_session);
            }
            let mut parsed_transport = if let Some(transports) = rsp.typed_header::<Transports>()? {
                Self::parse_setup_transports(&transports, &mut s, &protocols, &mode)
            } else {
                // Transport header in response is optional if only one transport was offered
                // https://datatracker.ietf.org/doc/html/rfc2326#section-12.39
                if transports.len() == 1 {
                    Self::parse_setup_transports(&transports, &mut s, &protocols, &mode)
                } else {
                    Err(RtspError::internal(
                        "No transport header in SETUP response",
                    ))
                }
            }?;
            match &mut parsed_transport {
                RtspTransportInfo::UdpMulticast { .. } => {}
                RtspTransportInfo::Udp {
                    source,
                    server_port: _,
                    client_port,
                    sockets,
                } => {
                    if source.is_none() {
                        *source = Some(conn_source.to_string());
                    }
                    if let Some((rtp_port, rtcp_port)) = client_port {
                        // There is no reason for the server to reject the client ports WE
                        // selected, so if it does, just ignore it.
                        if *rtp_port != port_next {
                            gst::warning!(
                                CAT,
                                "RTP port changed: {port_next} -> {rtp_port}, ignoring"
                            );
                            *rtp_port = port_next;
                        }
                        port_next += 1;
                        *sockets = if let Some(rtcp_port) = rtcp_port {
                            if *rtcp_port != port_next {
                                gst::warning!(
                                    CAT,
                                    "RTCP port changed: {port_next} -> {rtcp_port}, ignoring"
                                );
                                *rtcp_port = port_next;
                            }
                            port_next += 1;
                            Some((rtp_socket.unwrap(), rtcp_socket))
                        } else {
                            Some((rtp_socket.unwrap(), None))
                        }
                    };
                }
                RtspTransportInfo::Tcp {
                    channels: (rtp_ch, rtcp_ch),
                } => {
                    if rtp_ch != &stream_num {
                        gst::info!(CAT, "RTP channel changed: {stream_num} -> {rtp_ch}");
                    }
                    stream_num += 1;
                    if let Some(rtcp_ch) = rtcp_ch {
                        if rtcp_ch != &stream_num {
                            gst::info!(CAT, "RTCP channel changed: {stream_num} -> {rtcp_ch}");
                        }
                        stream_num += 1;
                    }
                }
            };
            let caps = gst::Caps::from(s);
            setup_params.push(RtspSetupParams {
                control_url: control_url.clone(),
                transport: parsed_transport,
                rtp_appsrc: None,
                caps,
            });
        }
        Ok(setup_params)
    }

    async fn play(&mut self, session: &Session) -> Result<u32, RtspError> {
        self.play_with_range(session, None, SeekFormat::Npt).await
    }

    fn create_range_header(position: Option<gst::ClockTime>, format: SeekFormat) -> Range {
        match format {
            SeekFormat::Npt => {
                if let Some(position) = position {
                    // Convert GStreamer ClockTime to seconds for NPT
                    let seconds = position.nseconds() / 1_000_000_000;
                    let nanos = (position.nseconds() % 1_000_000_000) as u32;
                    let fraction = if nanos > 0 {
                        Some(nanos / 1_000_000)
                    } else {
                        None
                    };
                    Range::Npt(NptRange::From(NptTime::Seconds(seconds, fraction)))
                } else {
                    Range::Npt(NptRange::From(NptTime::Now))
                }
            }
            SeekFormat::Smpte => {
                if let Some(position) = position {
                    // Convert to SMPTE time code (assuming 30fps drop frame)
                    let total_frames = (position.nseconds() * 30 / 1_000_000_000) as u32;
                    let seconds = total_frames / 30;
                    let frames = total_frames % 30;
                    let minutes = seconds / 60;
                    let hours = minutes / 60;

                    Range::Smpte(SmpteRange::From(
                        SmpteType::Smpte30Drop,
                        SmpteTime {
                            hours: (hours % 24) as u8,
                            minutes: (minutes % 60) as u8,
                            seconds: (seconds % 60) as u8,
                            frames: Some((frames as u8, None)),
                        },
                    ))
                } else {
                    // Start from beginning for SMPTE
                    Range::Smpte(SmpteRange::From(
                        SmpteType::Smpte30Drop,
                        SmpteTime {
                            hours: 0,
                            minutes: 0,
                            seconds: 0,
                            frames: Some((0, None)),
                        },
                    ))
                }
            }
            SeekFormat::Clock => {
                if let Some(position) = position {
                    // Convert to UTC time (offset from now)
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap();
                    let target_time = now + std::time::Duration::from_nanos(position.nseconds());
                    let total_secs = target_time.as_secs();
                    let nanos = target_time.subsec_nanos();

                    // Convert to date and time format
                    let days = total_secs / 86400;
                    let remaining_secs = total_secs % 86400;
                    let hours = remaining_secs / 3600;
                    let minutes = (remaining_secs % 3600) / 60;
                    let seconds = remaining_secs % 60;

                    // Convert days to YYYYMMDD (simplified - assumes starting from 1970-01-01)
                    let date = 19700101 + (days as u32);
                    let time = (hours as u32) * 10000 + (minutes as u32) * 100 + (seconds as u32);

                    Range::Utc(UtcRange::From(UtcTime {
                        date,
                        time,
                        nanoseconds: if nanos > 0 { Some(nanos) } else { None },
                    }))
                } else {
                    // Use current time for Clock format
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap();
                    let total_secs = now.as_secs();
                    let nanos = now.subsec_nanos();

                    // Convert to date and time format
                    let days = total_secs / 86400;
                    let remaining_secs = total_secs % 86400;
                    let hours = remaining_secs / 3600;
                    let minutes = (remaining_secs % 3600) / 60;
                    let seconds = remaining_secs % 60;

                    // Convert days to YYYYMMDD (simplified - assumes starting from 1970-01-01)
                    let date = 19700101 + (days as u32);
                    let time = (hours as u32) * 10000 + (minutes as u32) * 100 + (seconds as u32);

                    Range::Utc(UtcRange::From(UtcTime {
                        date,
                        time,
                        nanoseconds: Some(nanos),
                    }))
                }
            }
        }
    }

    async fn play_with_range(
        &mut self,
        session: &Session,
        seek_position: Option<gst::ClockTime>,
        seek_format: SeekFormat,
    ) -> Result<u32, RtspError> {
        self.cseq += 1;
        let request_uri = self.aggregate_control.as_ref().unwrap_or(&self.url).clone();

        // Create Range header based on seek position and format
        let range_header = Self::create_range_header(seek_position, seek_format);

        let mut req_builder = Request::builder(Method::Play, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .typed_header::<Range>(&range_header)
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .request_uri(request_uri.clone())
            .typed_header::<Session>(session);

        // Add auth header if we have auth state
        if let Some(auth_header) = auth::generate_auth_header(
            &mut self.auth_state,
            self.user_id.as_deref(),
            self.user_pw.as_deref(),
            &Method::Play,
            &request_uri.to_string(),
        ) {
            req_builder = req_builder.header(auth::AUTHORIZATION.clone(), auth_header);
        }

        let req = req_builder.build(Body::default());
        gst::debug!(CAT, "-->> {req:#?}");
        self.sink.send(req.into()).await?;
        Ok(self.cseq)
    }

    async fn play_response(
        &mut self,
        rsp: &Response<Body>,
        cseq: u32,
        session: &Session,
    ) -> Result<(), RtspError> {
        Self::check_response(rsp, cseq, Method::Play, Some(session))?;

        // Handle Range header in response for seek operations
        if let Some(range) = rsp.typed_header::<Range>()? {
            let (start_time, _end_time) = match range {
                Range::Npt(npt_range) => {
                    match npt_range {
                        NptRange::From(start) => {
                            let start_ns = match start {
                                NptTime::Now => 0,
                                NptTime::Seconds(s, frac) => {
                                    s * 1_000_000_000 + frac.unwrap_or(0) as u64 * 1_000_000
                                }
                                NptTime::Hms(h, m, s, frac) => {
                                    (h * 3600 + m as u64 * 60 + s as u64) * 1_000_000_000
                                        + frac.unwrap_or(0) as u64 * 1_000_000
                                }
                            };
                            (Some(gst::ClockTime::from_nseconds(start_ns)), None)
                        }
                        NptRange::FromTo(start, end) => {
                            let start_ns = match start {
                                NptTime::Now => 0,
                                NptTime::Seconds(s, frac) => {
                                    s * 1_000_000_000 + frac.unwrap_or(0) as u64 * 1_000_000
                                }
                                NptTime::Hms(h, m, s, frac) => {
                                    (h * 3600 + m as u64 * 60 + s as u64) * 1_000_000_000
                                        + frac.unwrap_or(0) as u64 * 1_000_000
                                }
                            };
                            let end_ns = match end {
                                NptTime::Now => u64::MAX,
                                NptTime::Seconds(s, frac) => {
                                    s * 1_000_000_000 + frac.unwrap_or(0) as u64 * 1_000_000
                                }
                                NptTime::Hms(h, m, s, frac) => {
                                    (h * 3600 + m as u64 * 60 + s as u64) * 1_000_000_000
                                        + frac.unwrap_or(0) as u64 * 1_000_000
                                }
                            };
                            (
                                Some(gst::ClockTime::from_nseconds(start_ns)),
                                Some(gst::ClockTime::from_nseconds(end_ns)),
                            )
                        }
                        NptRange::Empty => (None, None),
                        NptRange::To(_) => (None, None), // Not commonly used for seeking
                    }
                }
                Range::Smpte(smpte_range) => {
                    // Convert SMPTE to nanoseconds (assuming 30fps)
                    match smpte_range {
                        SmpteRange::From(_smpte_type, time) => {
                            let frames_val = time.frames.map(|(f, _)| f as u32).unwrap_or(0);
                            let total_frames = time.hours as u32 * 108000
                                + time.minutes as u32 * 1800
                                + time.seconds as u32 * 30
                                + frames_val;
                            let ns = (total_frames as u64 * 1_000_000_000) / 30;
                            (Some(gst::ClockTime::from_nseconds(ns)), None)
                        }
                        SmpteRange::FromTo(_smpte_type, start, end) => {
                            let start_frames_val = start.frames.map(|(f, _)| f as u32).unwrap_or(0);
                            let start_frames = start.hours as u32 * 108000
                                + start.minutes as u32 * 1800
                                + start.seconds as u32 * 30
                                + start_frames_val;
                            let end_frames_val = end.frames.map(|(f, _)| f as u32).unwrap_or(0);
                            let end_frames = end.hours as u32 * 108000
                                + end.minutes as u32 * 1800
                                + end.seconds as u32 * 30
                                + end_frames_val;
                            let start_ns = (start_frames as u64 * 1_000_000_000) / 30;
                            let end_ns = (end_frames as u64 * 1_000_000_000) / 30;
                            (
                                Some(gst::ClockTime::from_nseconds(start_ns)),
                                Some(gst::ClockTime::from_nseconds(end_ns)),
                            )
                        }
                        SmpteRange::Empty(_) => (None, None),
                        SmpteRange::To(_, _) => (None, None), // Not commonly used for seeking
                    }
                }
                Range::Utc(utc_range) => {
                    // Convert UTC to relative time
                    match utc_range {
                        UtcRange::From(time) => {
                            // Convert date/time format back to nanoseconds
                            // Extract hours, minutes, seconds from time field (HHMMSS)
                            let hours = (time.time / 10000) as u64;
                            let minutes = ((time.time % 10000) / 100) as u64;
                            let seconds = (time.time % 100) as u64;
                            let total_seconds = hours * 3600 + minutes * 60 + seconds;
                            let ns = total_seconds * 1_000_000_000
                                + time.nanoseconds.unwrap_or(0) as u64;
                            (Some(gst::ClockTime::from_nseconds(ns)), None)
                        }
                        UtcRange::FromTo(start, end) => {
                            // Convert start time
                            let start_hours = (start.time / 10000) as u64;
                            let start_minutes = ((start.time % 10000) / 100) as u64;
                            let start_seconds = (start.time % 100) as u64;
                            let start_total_seconds =
                                start_hours * 3600 + start_minutes * 60 + start_seconds;
                            let start_ns = start_total_seconds * 1_000_000_000
                                + start.nanoseconds.unwrap_or(0) as u64;

                            // Convert end time
                            let end_hours = (end.time / 10000) as u64;
                            let end_minutes = ((end.time % 10000) / 100) as u64;
                            let end_seconds = (end.time % 100) as u64;
                            let end_total_seconds =
                                end_hours * 3600 + end_minutes * 60 + end_seconds;
                            let end_ns = end_total_seconds * 1_000_000_000
                                + end.nanoseconds.unwrap_or(0) as u64;

                            (
                                Some(gst::ClockTime::from_nseconds(start_ns)),
                                Some(gst::ClockTime::from_nseconds(end_ns)),
                            )
                        }
                        UtcRange::Empty => (None, None),
                        UtcRange::To(_) => (None, None), // Not commonly used for seeking
                    }
                }
                Range::Other(_) => {
                    // Unsupported range format
                    gst::warning!(CAT, "Unsupported Range format in response");
                    (None, None)
                }
            };

            // Log the actual range from server
            gst::info!(CAT, "Server responded with Range: start={:?}", start_time);

            // Update segment if we got a valid start time
            if let Some(start) = start_time {
                let segment = gst::FormattedSegment::<gst::ClockTime>::new();
                let mut segment = segment.clone();
                segment.set_start(start);
                segment.set_position(start);

                // Send updated segment to all appsrcs
                for params in &self.setup_params {
                    if let Some(ref appsrc) = params.rtp_appsrc {
                        let _ = appsrc.send_event(gst::event::Segment::new(&segment));
                    }
                }
            }
        }

        if let Some(RtpInfos::V1(rtpinfos)) = rsp.typed_header::<RtpInfos>()? {
            for rtpinfo in rtpinfos {
                for params in self.setup_params.iter_mut() {
                    if params.control_url == rtpinfo.uri {
                        let mut changed = false;
                        let mut caps = params.rtp_appsrc.as_ref().unwrap().caps().unwrap();
                        let capsref = caps.make_mut();
                        if let Some(v) = rtpinfo.seq {
                            capsref.set("seqnum-base", v as u32);
                            changed = true;
                        }
                        if let Some(v) = rtpinfo.rtptime {
                            capsref.set("clock-base", v);
                            changed = true;
                        }
                        if changed {
                            params.rtp_appsrc.as_ref().unwrap().set_caps(Some(&caps));
                        }
                    }
                }
            }
        } else {
            gst::warning!(CAT, "No RTPInfos V1 header in PLAY response");
        };
        Ok(())
    }

    async fn pause(&mut self, session: &Session) -> Result<u32, RtspError> {
        self.cseq += 1;
        let request_uri = self.aggregate_control.as_ref().unwrap_or(&self.url).clone();
        let mut req_builder = Request::builder(Method::Pause, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .request_uri(request_uri.clone())
            .typed_header::<Session>(session);

        // Add auth header if we have auth state
        if let Some(auth_header) = auth::generate_auth_header(
            &mut self.auth_state,
            self.user_id.as_deref(),
            self.user_pw.as_deref(),
            &Method::Pause,
            &request_uri.to_string(),
        ) {
            req_builder = req_builder.header(auth::AUTHORIZATION.clone(), auth_header);
        }

        let req = req_builder.build(Body::default());
        gst::debug!(CAT, "-->> {req:#?}");
        self.sink.send(req.into()).await?;
        Ok(self.cseq)
    }

    async fn pause_response(
        &mut self,
        rsp: &Response<Body>,
        cseq: u32,
        session: &Session,
    ) -> Result<(), RtspError> {
        Self::check_response(rsp, cseq, Method::Pause, Some(session))?;
        Ok(())
    }

    async fn teardown(&mut self, session: &Session) -> Result<u32, RtspError> {
        self.cseq += 1;
        let request_uri = self.aggregate_control.as_ref().unwrap_or(&self.url).clone();
        let mut req_builder = Request::builder(Method::Teardown, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .request_uri(request_uri.clone())
            .typed_header::<Session>(session);

        // Add auth header if we have auth state
        if let Some(auth_header) = auth::generate_auth_header(
            &mut self.auth_state,
            self.user_id.as_deref(),
            self.user_pw.as_deref(),
            &Method::Teardown,
            &request_uri.to_string(),
        ) {
            req_builder = req_builder.header(auth::AUTHORIZATION.clone(), auth_header);
        }

        let req = req_builder.build(Body::default());
        gst::debug!(CAT, "-->> {req:#?}");
        self.sink.send(req.into()).await?;
        Ok(self.cseq)
    }

    async fn teardown_response(
        &mut self,
        rsp: &Response<Body>,
        cseq: u32,
        session: &Session,
    ) -> Result<(), RtspError> {
        Self::check_response(rsp, cseq, Method::Teardown, Some(session))?;
        Ok(())
    }

    async fn get_parameter(
        &mut self,
        session: Option<&Session>,
        parameters: Option<Vec<String>>,
    ) -> Result<u32, RtspError> {
        self.cseq += 1;
        let request_uri = self.aggregate_control.as_ref().unwrap_or(&self.url).clone();
        let mut req = Request::builder(Method::GetParameter, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .request_uri(request_uri.clone());

        if let Some(s) = session {
            req = req.typed_header::<Session>(s);
        }

        let body = if let Some(params) = parameters {
            // Content-Type: text/parameters
            req = req.header(rtsp_types::headers::CONTENT_TYPE, "text/parameters");
            Body::from(params.join("\r\n").into_bytes())
        } else {
            // Empty GET_PARAMETER for keep-alive
            Body::default()
        };

        // Add auth header if we have auth state
        if let Some(auth_header) = auth::generate_auth_header(
            &mut self.auth_state,
            self.user_id.as_deref(),
            self.user_pw.as_deref(),
            &Method::GetParameter,
            &request_uri.to_string(),
        ) {
            req = req.header(auth::AUTHORIZATION.clone(), auth_header);
        }

        let req = req.build(body);
        gst::debug!(CAT, "-->> {req:#?}");
        self.sink.send(req.into()).await?;
        Ok(self.cseq)
    }

    async fn get_parameter_response(
        &mut self,
        rsp: &Response<Body>,
        cseq: u32,
        session: Option<&Session>,
    ) -> Result<Vec<(String, String)>, RtspError> {
        Self::check_response(rsp, cseq, Method::GetParameter, session)?;

        let mut parameters = Vec::new();
        if !rsp.body().is_empty() {
            let body_str = std::str::from_utf8(rsp.body())
                .map_err(|e| RtspError::internal(format!("Invalid UTF-8 in response: {}", e)))?;

            for line in body_str.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    parameters.push((key.trim().to_string(), value.trim().to_string()));
                }
            }
        }

        Ok(parameters)
    }

    async fn set_parameter(
        &mut self,
        session: Option<&Session>,
        parameters: Vec<(String, String)>,
    ) -> Result<u32, RtspError> {
        self.cseq += 1;
        let request_uri = self.aggregate_control.as_ref().unwrap_or(&self.url).clone();
        let mut req = Request::builder(Method::SetParameter, self.version)
            .typed_header::<CSeq>(&self.cseq.into())
            .header(USER_AGENT, DEFAULT_USER_AGENT)
            .header(rtsp_types::headers::CONTENT_TYPE, "text/parameters")
            .request_uri(request_uri.clone());

        if let Some(s) = session {
            req = req.typed_header::<Session>(s);
        }

        let body_str = parameters
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<_>>()
            .join("\r\n");

        // Add auth header if we have auth state
        if let Some(auth_header) = auth::generate_auth_header(
            &mut self.auth_state,
            self.user_id.as_deref(),
            self.user_pw.as_deref(),
            &Method::SetParameter,
            &request_uri.to_string(),
        ) {
            req = req.header(auth::AUTHORIZATION.clone(), auth_header);
        }

        let req = req.build(Body::from(body_str.into_bytes()));
        gst::debug!(CAT, "-->> {req:#?}");
        self.sink.send(req.into()).await?;
        Ok(self.cseq)
    }

    async fn set_parameter_response(
        &mut self,
        rsp: &Response<Body>,
        cseq: u32,
        session: Option<&Session>,
    ) -> Result<(), RtspError> {
        Self::check_response(rsp, cseq, Method::SetParameter, session)?;
        Ok(())
    }
}

fn bind_port(port: u16, is_ipv4: bool) -> Result<UdpSocket, std::io::Error> {
    let domain = if is_ipv4 {
        socket2::Domain::IPV4
    } else {
        socket2::Domain::IPV6
    };
    let sock = Socket::new(domain, socket2::Type::DGRAM, Some(socket2::Protocol::UDP))?;
    let _ = sock.set_reuse_address(true);
    #[cfg(unix)]
    let _ = sock.set_reuse_port(true);
    sock.set_nonblocking(true)?;
    let addr: SocketAddr = if is_ipv4 {
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))
    } else {
        SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::UNSPECIFIED, port, 0, 0))
    };
    sock.bind(&addr.into())?;
    let bound_port = if is_ipv4 {
        sock.local_addr()?.as_socket_ipv4().unwrap().port()
    } else {
        sock.local_addr()?.as_socket_ipv6().unwrap().port()
    };
    gst::debug!(CAT, "Bound to UDP port {bound_port}");

    UdpSocket::from_std(sock.into())
}

async fn bind_start_port(port: u16, is_ipv4: bool) -> (UdpSocket, u16) {
    let mut next_port = port;
    loop {
        match bind_port(next_port, is_ipv4) {
            Ok(socket) => {
                if next_port != 0 {
                    return (socket, next_port);
                }
                let addr = socket
                    .local_addr()
                    .expect("Newly-bound port should not fail");
                return (socket, addr.port());
            }
            Err(err) => {
                gst::debug!(CAT, "Failed to bind to {next_port}: {err:?}, trying next");
                next_port += 1;
                // If we fail too much, panic instead of forever doing a hot-loop
                if (next_port - MAX_BIND_PORT_RETRY) > port {
                    panic!("Failed to allocate any ports from {port} to {next_port}");
                }
            }
        };
    }
}

fn on_rtcp_udp(
    appsink: &gst_app::AppSink,
    tx: mpsc::Sender<MappedBuffer<Readable>>,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    let Ok(sample) = appsink.pull_sample() else {
        return Err(gst::FlowError::Error);
    };
    let Some(buffer) = sample.buffer_owned() else {
        return Ok(gst::FlowSuccess::Ok);
    };
    let map = buffer.into_mapped_buffer_readable();
    match map {
        Ok(map) => match tx.try_send(map) {
            Ok(_) => Ok(gst::FlowSuccess::Ok),
            Err(mpsc::error::TrySendError::Full(_)) => {
                gst::error!(CAT, "Could not send RTCP, channel is full");
                Err(gst::FlowError::Error)
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(gst::FlowError::Eos),
        },
        Err(err) => {
            gst::error!(CAT, "Failed to map buffer: {err:?}");
            Err(gst::FlowError::Error)
        }
    }
}

fn on_rtcp_tcp(
    appsink: &gst_app::AppSink,
    cmd_tx: mpsc::Sender<Commands>,
    rtcp_channel: u8,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    let Ok(sample) = appsink.pull_sample() else {
        return Err(gst::FlowError::Error);
    };
    let Some(buffer) = sample.buffer_owned() else {
        return Ok(gst::FlowSuccess::Ok);
    };
    let map = buffer.into_mapped_buffer_readable();
    match map {
        Ok(map) => {
            let data: rtsp_types::Data<Body> =
                rtsp_types::Data::new(rtcp_channel, Body::mapped(map));
            let cmd_tx = cmd_tx.clone();
            RUNTIME.spawn(async move { cmd_tx.send(Commands::Data(data)).await });
            Ok(gst::FlowSuccess::Ok)
        }
        Err(err) => {
            gst::error!(CAT, "Failed to map buffer: {err:?}");
            Err(gst::FlowError::Error)
        }
    }
}

async fn udp_rtp_task(
    socket: &UdpSocket,
    appsrc: gst_app::AppSrc,
    timeout: gst::ClockTime,
    receive_mtu: u32,
    sender_addr: Option<SocketAddr>,
    buffer_queue: Option<Arc<Mutex<BufferQueue>>>,
) {
    let t = Duration::from_secs(timeout.into());
    let sender_addr = match sender_addr {
        Some(addr) => addr,
        // Server didn't give us a Transport header or its Transport header didn't specify the
        // server port, so we don't know the sender port from which we will get data till we get
        // the first packet here.
        None => {
            let ret = match time::timeout(t, socket.peek_sender()).await {
                Ok(Ok(addr)) => Ok(addr),
                Ok(Err(_elapsed)) => Err(format!(
                    "No data after {} seconds, exiting",
                    timeout.seconds()
                )),
                Err(err) => Err(format!("UDP socket was closed: {err:?}")),
            };
            match ret {
                Ok(addr) => addr,
                Err(err) => {
                    gst::element_error!(
                        appsrc,
                        gst::ResourceError::Failed,
                        ("{}", err),
                        ["{:#?}", socket]
                    );
                    return;
                }
            }
        }
    };
    gst::info!(CAT, "Receiving from address {sender_addr:?}");
    let gio_addr = {
        let inet_addr: gio::InetAddress = sender_addr.ip().into();
        gio::InetSocketAddress::new(&inet_addr, sender_addr.port())
    };
    let mut size = receive_mtu;
    let caps = appsrc.caps();
    let mut pool = gst::BufferPool::new();
    let mut config = pool.config();
    config.set_params(caps.as_ref(), size, 2, 0);
    pool.set_config(config).unwrap();
    pool.set_active(true).unwrap();
    let error = loop {
        let Ok(buffer) = pool.acquire_buffer(None) else {
            break "Failed to acquire buffer".to_string();
        };
        let Ok(mut map) = buffer.into_mapped_buffer_writable() else {
            break "Failed to map buffer writable".to_string();
        };
        match time::timeout(t, socket.recv_from(map.as_mut_slice())).await {
            Ok(Ok((len, addr))) => {
                // Ignore packets from the wrong sender
                if addr != sender_addr {
                    continue;
                }
                if size < UDP_PACKET_MAX_SIZE && len == size as usize {
                    gst::warning!(
                        CAT,
                        "Data maybe lost: UDP buffer size {size} filled, doubling"
                    );
                    size = (size * 2).min(UDP_PACKET_MAX_SIZE);
                    if let Err(err) = pool.set_active(false) {
                        break format!("Failed to deactivate buffer pool: {err:?}");
                    }
                    pool = gst::BufferPool::new();
                    let mut config = pool.config();
                    config.set_params(caps.as_ref(), size, 2, 0);
                    pool.set_config(config).unwrap();
                    if let Err(err) = pool.set_active(true) {
                        break format!("Failed to reallocate buffer pool: {err:?}");
                    }
                }
                let t = appsrc.current_running_time();
                let mut buffer = map.into_buffer();
                let bufref = buffer.make_mut();
                bufref.set_size(len);
                bufref.set_dts(t);
                gst_net::NetAddressMeta::add(bufref, &gio_addr);
                gst::trace!(CAT, "received RTP packet from {addr:?}");

                // Use buffer queue system if available, otherwise drop buffers
                let push_result = if let Some(ref buffer_queue_mutex) = buffer_queue {
                    // Implement buffer queue logic directly since we can't call push_buffer_with_queue
                    let timestamp = appsrc
                        .current_running_time()
                        .unwrap_or(gst::ClockTime::ZERO);
                    match appsrc.push_buffer(buffer.clone()) {
                        Ok(success) => Ok(success),
                        Err(gst::FlowError::NotLinked) => {
                            // Queue the buffer for later flushing
                            let mut queue = buffer_queue_mutex.lock().unwrap();
                            if queue.push(buffer, appsrc.clone(), timestamp) {
                                gst::debug!(CAT, "Queued UDP RTP buffer (pad not linked yet)");
                                Ok(gst::FlowSuccess::Ok)
                            } else {
                                gst::error!(CAT, "Failed to queue UDP RTP buffer - queue full");
                                Err(gst::FlowError::Error)
                            }
                        }
                        Err(gst::FlowError::Flushing) => {
                            gst::debug!(CAT, "UDP RTP pad is flushing - dropping buffer");
                            Ok(gst::FlowSuccess::Ok)
                        }
                        Err(err) => Err(err),
                    }
                } else {
                    appsrc.push_buffer(buffer)
                };

                match push_result {
                    Ok(_) => {
                        gst::trace!(CAT, "Successfully handled UDP RTP buffer");
                    }
                    Err(gst::FlowError::Eos) => {
                        gst::debug!(CAT, "UDP RTP stream ended");
                        break "UDP RTP stream ended".to_string();
                    }
                    Err(err) => {
                        gst::error!(CAT, "Failed to handle UDP RTP buffer: {}", err);
                        break format!("UDP RTP buffer handling failed: {err:?}");
                    }
                }
            }
            Ok(Err(_elapsed)) => {
                break format!("No data after {} seconds, exiting", timeout.seconds())
            }
            Err(err) => break format!("UDP socket was closed: {err:?}"),
        };
    };
    gst::element_error!(
        appsrc,
        gst::ResourceError::Failed,
        ("{}", error),
        ["{:#?}", socket]
    );
}

async fn udp_rtcp_task(
    socket: &UdpSocket,
    appsrc: gst_app::AppSrc,
    mut sender_addr: Option<SocketAddr>,
    is_multicast: bool,
    mut rx: mpsc::Receiver<MappedBuffer<Readable>>,
    buffer_queue: Option<Arc<Mutex<BufferQueue>>>,
) {
    let mut buf = vec![0; UDP_PACKET_MAX_SIZE as usize];
    let mut cache: LruCache<_, _> = LruCache::new(NonZeroUsize::new(RTCP_ADDR_CACHE_SIZE).unwrap());

    // NAT keep-alive timer - send dummy packet every 20 seconds to keep NAT mapping alive
    let mut nat_keepalive_interval = tokio::time::interval(Duration::from_secs(20));
    nat_keepalive_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    let error = loop {
        tokio::select! {
            send_rtcp = rx.recv() => match send_rtcp {
                // The server either didn't specify a server_port for RTCP, or if the server didn't
                // send a Transport header in the SETUP response at all.
                Some(data) => if let Some(addr) = sender_addr.as_ref() {
                    match socket.send_to(data.as_ref(), addr).await {
                        Ok(_) => gst::debug!(CAT, "Sent RTCP RR packet"),
                        Err(err) => {
                            rx.close();
                            break format!("RTCP send error: {err:?}, stopping task");
                        }
                    }
                } else {
                    gst::warning!(CAT, "Can't send RTCP yet: don't have dest addr");
                },
                None => {
                    rx.close();
                    break format!("UDP socket {socket:?} closed, no more RTCP will be sent");
                }
            },
            recv_rtcp = socket.recv_from(&mut buf) => match recv_rtcp {
                Ok((len, addr)) => {
                    gst::debug!(CAT, "Received RTCP packet");
                    if let Some(sender_addr) = sender_addr {
                        // Ignore RTCP from the wrong sender
                        if !is_multicast && addr != sender_addr {
                            continue;
                        }
                    } else {
                        sender_addr.replace(addr);
                        gst::info!(CAT, "Delayed RTCP UDP send address: {addr:?}");
                    };
                    let t = appsrc.current_running_time();
                    let mut buffer = gst::Buffer::from_slice(buf[..len].to_owned());
                    let bufref = buffer.make_mut();
                    bufref.set_dts(t);
                    let gio_addr = cache.get_or_insert(addr, || {
                        let inet_addr: gio::InetAddress = addr.ip().into();
                        gio::InetSocketAddress::new(&inet_addr, addr.port())
                    });
                    gst_net::NetAddressMeta::add(bufref, gio_addr);
                    // Use buffer queue system if available, otherwise drop buffers
                    let push_result = if let Some(ref buffer_queue_mutex) = buffer_queue {
                        // Implement buffer queue logic directly since we can't call push_buffer_with_queue
                        let timestamp = appsrc.current_running_time().unwrap_or(gst::ClockTime::ZERO);
                        match appsrc.push_buffer(buffer.clone()) {
                            Ok(success) => Ok(success),
                            Err(gst::FlowError::NotLinked) => {
                                // Queue the buffer for later flushing
                                let mut queue = buffer_queue_mutex.lock().unwrap();
                                if queue.push(buffer, appsrc.clone(), timestamp) {
                                    gst::debug!(CAT, "Queued UDP RTCP buffer (pad not linked yet)");
                                    Ok(gst::FlowSuccess::Ok)
                                } else {
                                    gst::error!(CAT, "Failed to queue UDP RTCP buffer - queue full");
                                    Err(gst::FlowError::Error)
                                }
                            }
                            Err(gst::FlowError::Flushing) => {
                                gst::debug!(CAT, "UDP RTCP pad is flushing - dropping buffer");
                                Ok(gst::FlowSuccess::Ok)
                            }
                            Err(err) => Err(err),
                        }
                    } else {
                        appsrc.push_buffer(buffer)
                    };

                    match push_result {
                        Ok(_) => {
                            gst::trace!(CAT, "Successfully handled UDP RTCP buffer");
                        }
                        Err(gst::FlowError::Eos) => {
                            gst::debug!(CAT, "UDP RTCP stream ended");
                            break "UDP RTCP stream ended".to_string();
                        }
                        Err(err) => {
                            gst::error!(CAT, "Failed to handle UDP RTCP buffer: {}", err);
                            break format!("UDP RTCP buffer handling failed: {err:?}");
                        }
                    }
                }
                Err(err) => break format!("UDP socket was closed: {err:?}"),
            },
            _ = nat_keepalive_interval.tick() => {
                // Send NAT keep-alive packet if we have a destination address
                if let Some(addr) = sender_addr.as_ref() {
                    // Minimal RTCP RR packet for keep-alive (8 bytes)
                    let keepalive_rtcp = vec![
                        0x80, 0xc9, // V=2, P=0, RC=0, PT=201 (RR)
                        0x00, 0x01, // Length
                        0x00, 0x00, 0x00, 0x00, // SSRC
                    ];
                    match socket.send_to(&keepalive_rtcp, addr).await {
                        Ok(_) => gst::trace!(CAT, "Sent NAT keep-alive RTCP packet to {}", addr),
                        Err(e) => gst::warning!(CAT, "Failed to send NAT keep-alive: {}", e),
                    }
                }
            },
        }
    };
    gst::element_error!(
        appsrc,
        gst::ResourceError::Failed,
        ("{}", error),
        ["{:#?}", socket]
    );
}

#[glib::object_subclass]
impl ObjectSubclass for RtspSrc {
    const NAME: &'static str = "GstRtspSrc2";
    type Type = super::RtspSrc;
    type ParentType = gst::Bin;
    type Interfaces = (gst::URIHandler,);
}
