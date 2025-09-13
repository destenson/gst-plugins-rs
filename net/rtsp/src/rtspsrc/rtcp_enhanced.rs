// Enhanced RTCP handling with extended reports and statistics
//
// This module provides RTCP XR (Extended Reports) support, feedback messages,
// and comprehensive statistics collection for stream quality monitoring

use anyhow::{anyhow, Result};
use gst;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// RTCP packet types
const RTCP_SR: u8 = 200; // Sender Report
const RTCP_RR: u8 = 201; // Receiver Report
const RTCP_SDES: u8 = 202; // Source Description
const RTCP_BYE: u8 = 203; // Goodbye
const RTCP_APP: u8 = 204; // Application-defined
const RTCP_XR: u8 = 207; // Extended Report (RFC 3611)

// RTCP Feedback message types (RFC 4585)
const RTCP_RTPFB: u8 = 205; // RTP Feedback
const RTCP_PSFB: u8 = 206; // Payload-specific Feedback

// RTCP XR block types
const XR_LOSS_RLE: u8 = 1; // Loss RLE Report Block
const XR_DUPLICATE_RLE: u8 = 2; // Duplicate RLE Report Block
const XR_PACKET_RECEIPT_TIMES: u8 = 3; // Packet Receipt Times Report Block
const XR_RECEIVER_REFERENCE_TIME: u8 = 4; // Receiver Reference Time Report Block
const XR_DLRR: u8 = 5; // DLRR Report Block
const XR_STATISTICS_SUMMARY: u8 = 6; // Statistics Summary Report Block
const XR_VOIP_METRICS: u8 = 7; // VoIP Metrics Report Block

// Feedback message subtypes
const RTPFB_NACK: u8 = 1; // Generic NACK
const PSFB_PLI: u8 = 1; // Picture Loss Indication
const PSFB_FIR: u8 = 4; // Full Intra Request
const PSFB_REMB: u8 = 15; // Receiver Estimated Max Bitrate

#[derive(Debug, Clone)]
pub struct RtcpStatistics {
    // Basic statistics
    pub packets_received: u64,
    pub packets_lost: u64,
    pub packet_loss_percentage: f32,
    pub jitter: u32,
    pub round_trip_time: Duration,

    // Extended statistics
    pub bandwidth_usage: u64,
    pub interarrival_jitter: f64,
    pub cumulative_lost: u32,
    pub highest_seq_received: u32,

    // XR statistics
    pub burst_loss_rate: f32,
    pub gap_loss_rate: f32,
    pub burst_duration: Duration,
    pub gap_duration: Duration,
    pub min_jitter: u32,
    pub max_jitter: u32,
    pub mean_jitter: u32,

    // Network quality indicators
    pub r_factor: f32,  // Voice quality metric (0-100)
    pub mos_score: f32, // Mean Opinion Score (1-5)
    pub network_delay: Duration,
    pub end_system_delay: Duration,

    // Timestamps
    pub last_sr_timestamp: Option<Instant>,
    pub last_rr_timestamp: Option<Instant>,
    pub stats_updated: Instant,
}

impl Default for RtcpStatistics {
    fn default() -> Self {
        Self {
            packets_received: 0,
            packets_lost: 0,
            packet_loss_percentage: 0.0,
            jitter: 0,
            round_trip_time: Duration::from_secs(0),
            bandwidth_usage: 0,
            interarrival_jitter: 0.0,
            cumulative_lost: 0,
            highest_seq_received: 0,
            burst_loss_rate: 0.0,
            gap_loss_rate: 0.0,
            burst_duration: Duration::from_secs(0),
            gap_duration: Duration::from_secs(0),
            min_jitter: 0,
            max_jitter: 0,
            mean_jitter: 0,
            r_factor: 0.0,
            mos_score: 0.0,
            network_delay: Duration::from_secs(0),
            end_system_delay: Duration::from_secs(0),
            last_sr_timestamp: None,
            last_rr_timestamp: None,
            stats_updated: Instant::now(),
        }
    }
}

impl RtcpStatistics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_packet_loss(&mut self, lost: u64, total: u64) {
        self.packets_lost = lost;
        if total > 0 {
            self.packet_loss_percentage = (lost as f32 / total as f32) * 100.0;
        }
    }

    pub fn update_jitter(&mut self, jitter: u32) {
        if self.min_jitter == 0 || jitter < self.min_jitter {
            self.min_jitter = jitter;
        }
        if jitter > self.max_jitter {
            self.max_jitter = jitter;
        }

        // Simple moving average for mean jitter
        if self.mean_jitter == 0 {
            self.mean_jitter = jitter;
        } else {
            self.mean_jitter = (self.mean_jitter * 7 + jitter) / 8;
        }

        self.jitter = jitter;
    }

    pub fn calculate_mos_score(&mut self) {
        // Simplified E-model to MOS conversion
        // MOS = 1 + 0.035 * R + 7 * 10^-6 * R * (R - 60) * (100 - R)
        let r = self.r_factor.max(0.0).min(100.0);
        self.mos_score = if r < 0.0 {
            1.0
        } else if r > 100.0 {
            4.5
        } else {
            1.0 + 0.035 * r + 7.0e-6 * r * (r - 60.0) * (100.0 - r)
        };
        self.mos_score = self.mos_score.max(1.0).min(5.0);
    }
}

#[derive(Debug)]
pub struct XrReportBlock {
    pub block_type: u8,
    pub type_specific: u8,
    pub block_length: u16,
    pub ssrc: u32,
    pub data: Vec<u8>,
}

impl XrReportBlock {
    pub fn parse_loss_rle(&self) -> Result<LossRleReport> {
        if self.block_type != XR_LOSS_RLE {
            return Err(anyhow!("Not a Loss RLE block"));
        }

        // Parse Loss RLE specific fields
        let begin_seq = u16::from_be_bytes([self.data[0], self.data[1]]);
        let end_seq = u16::from_be_bytes([self.data[2], self.data[3]]);
        let chunks = &self.data[4..];

        Ok(LossRleReport {
            ssrc: self.ssrc,
            begin_seq,
            end_seq,
            chunks: chunks.to_vec(),
        })
    }

    pub fn parse_voip_metrics(&self) -> Result<VoipMetrics> {
        if self.block_type != XR_VOIP_METRICS {
            return Err(anyhow!("Not a VoIP Metrics block"));
        }

        // Parse VoIP metrics
        Ok(VoipMetrics {
            ssrc: self.ssrc,
            loss_rate: self.data[0],
            discard_rate: self.data[1],
            burst_density: self.data[2],
            gap_density: self.data[3],
            burst_duration: u16::from_be_bytes([self.data[4], self.data[5]]),
            gap_duration: u16::from_be_bytes([self.data[6], self.data[7]]),
            round_trip_delay: u16::from_be_bytes([self.data[8], self.data[9]]),
            end_system_delay: u16::from_be_bytes([self.data[10], self.data[11]]),
            signal_level: self.data[12] as i8,
            noise_level: self.data[13] as i8,
            rerl: self.data[14],
            gmin: self.data[15],
            r_factor: self.data[16],
            ext_r_factor: self.data[17],
            mos_lq: self.data[18],
            mos_cq: self.data[19],
        })
    }
}

#[derive(Debug)]
pub struct LossRleReport {
    pub ssrc: u32,
    pub begin_seq: u16,
    pub end_seq: u16,
    pub chunks: Vec<u8>,
}

#[derive(Debug)]
pub struct VoipMetrics {
    pub ssrc: u32,
    pub loss_rate: u8,
    pub discard_rate: u8,
    pub burst_density: u8,
    pub gap_density: u8,
    pub burst_duration: u16,
    pub gap_duration: u16,
    pub round_trip_delay: u16,
    pub end_system_delay: u16,
    pub signal_level: i8,
    pub noise_level: i8,
    pub rerl: u8,
    pub gmin: u8,
    pub r_factor: u8,
    pub ext_r_factor: u8,
    pub mos_lq: u8,
    pub mos_cq: u8,
}

#[derive(Debug)]
pub struct FeedbackMessage {
    pub message_type: u8,
    pub fmt: u8,
    pub sender_ssrc: u32,
    pub media_ssrc: u32,
    pub data: Vec<u8>,
}

impl FeedbackMessage {
    pub fn create_nack(sender_ssrc: u32, media_ssrc: u32, lost_packets: &[u16]) -> Self {
        let mut data = Vec::new();

        // Encode lost packets as PID + BLP (Packet ID + Bitmask of following Lost Packets)
        for &seq in lost_packets {
            data.extend_from_slice(&seq.to_be_bytes());
            data.extend_from_slice(&0u16.to_be_bytes()); // No bitmask for simplicity
        }

        Self {
            message_type: RTCP_RTPFB,
            fmt: RTPFB_NACK,
            sender_ssrc,
            media_ssrc,
            data,
        }
    }

    pub fn create_pli(sender_ssrc: u32, media_ssrc: u32) -> Self {
        Self {
            message_type: RTCP_PSFB,
            fmt: PSFB_PLI,
            sender_ssrc,
            media_ssrc,
            data: Vec::new(),
        }
    }

    pub fn create_fir(sender_ssrc: u32, media_ssrc: u32, seq_num: u8) -> Self {
        Self {
            message_type: RTCP_PSFB,
            fmt: PSFB_FIR,
            sender_ssrc,
            media_ssrc,
            data: vec![0, 0, 0, seq_num],
        }
    }

    pub fn create_remb(sender_ssrc: u32, bitrate: u64, ssrcs: &[u32]) -> Self {
        let mut data = Vec::new();

        // Unique identifier 'REMB'
        data.extend_from_slice(b"REMB");

        // Number of SSRCs
        data.push(ssrcs.len() as u8);

        // Bitrate in mantissa/exponent form
        let (mantissa, exponent) = Self::encode_remb_bitrate(bitrate);
        data.push(exponent << 2 | (mantissa >> 16) as u8);
        data.push((mantissa >> 8) as u8);
        data.push(mantissa as u8);

        // SSRCs
        for &ssrc in ssrcs {
            data.extend_from_slice(&ssrc.to_be_bytes());
        }

        Self {
            message_type: RTCP_PSFB,
            fmt: PSFB_REMB,
            sender_ssrc,
            media_ssrc: 0, // Not used for REMB
            data,
        }
    }

    fn encode_remb_bitrate(bitrate: u64) -> (u32, u8) {
        // Find the exponent
        let mut exponent = 0u8;
        let mut mantissa = bitrate;

        while mantissa > 0x3FFFF && exponent < 63 {
            mantissa >>= 1;
            exponent += 1;
        }

        (mantissa as u32, exponent)
    }
}

pub struct RtcpEnhancedHandler {
    statistics: Arc<Mutex<HashMap<u32, RtcpStatistics>>>, // Per SSRC statistics
    xr_enabled: bool,
    feedback_enabled: bool,
}

impl RtcpEnhancedHandler {
    pub fn new(xr_enabled: bool, feedback_enabled: bool) -> Self {
        Self {
            statistics: Arc::new(Mutex::new(HashMap::new())),
            xr_enabled,
            feedback_enabled,
        }
    }

    pub fn process_rtcp_packet(&self, data: &[u8]) -> Result<()> {
        let packet_type = data[1];

        match packet_type {
            RTCP_SR => self.process_sender_report(data)?,
            RTCP_RR => self.process_receiver_report(data)?,
            RTCP_XR if self.xr_enabled => self.process_extended_report(data)?,
            RTCP_RTPFB | RTCP_PSFB if self.feedback_enabled => self.process_feedback(data)?,
            _ => {}
        }

        Ok(())
    }

    fn process_sender_report(&self, data: &[u8]) -> Result<()> {
        if data.len() < 28 {
            return Err(anyhow!("SR packet too short"));
        }

        let ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        let mut stats = self.statistics.lock().unwrap();
        let stat = stats.entry(ssrc).or_insert_with(RtcpStatistics::new);
        stat.last_sr_timestamp = Some(Instant::now());

        Ok(())
    }

    fn process_receiver_report(&self, data: &[u8]) -> Result<()> {
        if data.len() < 32 {
            return Err(anyhow!("RR packet too short"));
        }

        let _ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        // Parse report blocks
        let report_count = data[0] & 0x1F;
        let mut offset = 8;

        for _ in 0..report_count {
            if offset + 24 > data.len() {
                break;
            }

            let source_ssrc = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            let _fraction_lost = data[offset + 4];
            let cumulative_lost =
                u32::from_be_bytes([0, data[offset + 5], data[offset + 6], data[offset + 7]]);
            let highest_seq = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);
            let jitter = u32::from_be_bytes([
                data[offset + 12],
                data[offset + 13],
                data[offset + 14],
                data[offset + 15],
            ]);

            let mut stats = self.statistics.lock().unwrap();
            let stat = stats.entry(source_ssrc).or_insert_with(RtcpStatistics::new);

            stat.cumulative_lost = cumulative_lost;
            stat.highest_seq_received = highest_seq;
            stat.update_jitter(jitter);
            stat.last_rr_timestamp = Some(Instant::now());

            offset += 24;
        }

        Ok(())
    }

    fn process_extended_report(&self, data: &[u8]) -> Result<()> {
        if data.len() < 8 {
            return Err(anyhow!("XR packet too short"));
        }

        let ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        // Parse XR blocks
        let mut offset = 8;
        while offset + 4 <= data.len() {
            let block_type = data[offset];
            let type_specific = data[offset + 1];
            let block_length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            let block_bytes = (block_length as usize + 1) * 4;

            if offset + block_bytes > data.len() {
                break;
            }

            let block_data = &data[offset + 4..offset + block_bytes];

            match block_type {
                XR_VOIP_METRICS => {
                    let block = XrReportBlock {
                        block_type,
                        type_specific,
                        block_length,
                        ssrc,
                        data: block_data.to_vec(),
                    };

                    if let Ok(metrics) = block.parse_voip_metrics() {
                        let mut stats = self.statistics.lock().unwrap();
                        let stat = stats.entry(ssrc).or_insert_with(RtcpStatistics::new);

                        stat.r_factor = metrics.r_factor as f32;
                        stat.calculate_mos_score();
                        stat.burst_loss_rate = metrics.burst_density as f32 / 255.0 * 100.0;
                        stat.gap_loss_rate = metrics.gap_density as f32 / 255.0 * 100.0;
                        stat.round_trip_time =
                            Duration::from_millis(metrics.round_trip_delay as u64);
                        stat.end_system_delay =
                            Duration::from_millis(metrics.end_system_delay as u64);
                    }
                }
                _ => {}
            }

            offset += block_bytes;
        }

        Ok(())
    }

    fn process_feedback(&self, data: &[u8]) -> Result<()> {
        if data.len() < 12 {
            return Err(anyhow!("Feedback packet too short"));
        }

        let message_type = data[1];
        let fmt = data[0] & 0x1F;
        let sender_ssrc = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let media_ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        gst::debug!(
            gst::CAT_DEFAULT,
            "Received feedback message: type={}, fmt={}, sender={}, media={}",
            message_type,
            fmt,
            sender_ssrc,
            media_ssrc
        );

        Ok(())
    }

    pub fn get_statistics(&self, ssrc: u32) -> Option<RtcpStatistics> {
        let stats = self.statistics.lock().unwrap();
        stats.get(&ssrc).cloned()
    }

    pub fn get_all_statistics(&self) -> HashMap<u32, RtcpStatistics> {
        let stats = self.statistics.lock().unwrap();
        stats.clone()
    }

    pub fn reset_statistics(&self, ssrc: Option<u32>) {
        let mut stats = self.statistics.lock().unwrap();
        if let Some(ssrc) = ssrc {
            stats.remove(&ssrc);
        } else {
            stats.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feedback_message_creation() {
        let nack = FeedbackMessage::create_nack(12345, 67890, &[100, 101, 102]);
        assert_eq!(nack.message_type, RTCP_RTPFB);
        assert_eq!(nack.fmt, RTPFB_NACK);

        let pli = FeedbackMessage::create_pli(12345, 67890);
        assert_eq!(pli.message_type, RTCP_PSFB);
        assert_eq!(pli.fmt, PSFB_PLI);

        let fir = FeedbackMessage::create_fir(12345, 67890, 1);
        assert_eq!(fir.message_type, RTCP_PSFB);
        assert_eq!(fir.fmt, PSFB_FIR);

        let remb = FeedbackMessage::create_remb(12345, 1_000_000, &[67890]);
        assert_eq!(remb.message_type, RTCP_PSFB);
        assert_eq!(remb.fmt, PSFB_REMB);
    }

    #[test]
    fn test_statistics_update() {
        let mut stats = RtcpStatistics::new();

        stats.update_packet_loss(10, 100);
        assert_eq!(stats.packet_loss_percentage, 10.0);

        stats.update_jitter(100);
        assert_eq!(stats.jitter, 100);
        assert_eq!(stats.min_jitter, 100);
        assert_eq!(stats.max_jitter, 100);

        stats.update_jitter(50);
        assert_eq!(stats.min_jitter, 50);

        stats.update_jitter(200);
        assert_eq!(stats.max_jitter, 200);
    }

    #[test]
    fn test_mos_calculation() {
        let mut stats = RtcpStatistics::new();

        stats.r_factor = 93.0;
        stats.calculate_mos_score();
        assert!(stats.mos_score > 4.0 && stats.mos_score < 4.5);

        stats.r_factor = 50.0;
        stats.calculate_mos_score();
        assert!(stats.mos_score > 2.5 && stats.mos_score < 3.5);
    }
}
