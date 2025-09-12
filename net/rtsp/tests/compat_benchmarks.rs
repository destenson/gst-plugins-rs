#![allow(unused)]
// Performance Benchmark Tests for Camera Compatibility
// Measures latency, throughput, and stability metrics

use gst::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub test_duration: Duration,
    pub measure_latency: bool,
    pub measure_throughput: bool,
    pub measure_jitter: bool,
    pub measure_cpu: bool,
    pub measure_memory: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            test_duration: Duration::from_secs(30),
            measure_latency: true,
            measure_throughput: true,
            measure_jitter: true,
            measure_cpu: false,
            measure_memory: false,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub url: String,
    pub test_duration: Duration,
    pub connection_time: Duration,
    pub first_frame_time: Duration,
    pub average_latency: Duration,
    pub min_latency: Duration,
    pub max_latency: Duration,
    pub jitter: Duration,
    pub frames_received: u64,
    pub frames_dropped: u64,
    pub bytes_received: u64,
    pub average_bitrate: u64, // bits per second
    pub peak_bitrate: u64,
    pub reconnection_count: u64,
    pub error_count: u64,
}

impl Default for BenchmarkResults {
    fn default() -> Self {
        Self {
            url: String::new(),
            test_duration: Duration::from_secs(0),
            connection_time: Duration::from_secs(0),
            first_frame_time: Duration::from_secs(0),
            average_latency: Duration::from_secs(0),
            min_latency: Duration::from_secs(u64::MAX),
            max_latency: Duration::from_secs(0),
            jitter: Duration::from_secs(0),
            frames_received: 0,
            frames_dropped: 0,
            bytes_received: 0,
            average_bitrate: 0,
            peak_bitrate: 0,
            reconnection_count: 0,
            error_count: 0,
        }
    }
}

#[allow(dead_code)]
pub struct CameraBenchmark {
    pub config: BenchmarkConfig,
    results: Arc<std::sync::Mutex<BenchmarkResults>>,
    stop_flag: Arc<AtomicBool>,
}

impl CameraBenchmark {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: Arc::new(std::sync::Mutex::new(BenchmarkResults::default())),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn benchmark_camera(
        &self,
        url: &str,
    ) -> Result<BenchmarkResults, Box<dyn std::error::Error>> {
        gst::init()?;

        let start_time = Instant::now();
        let connection_start = Instant::now();

        // Create pipeline for benchmarking
        let pipeline_str = format!(
            "rtspsrc2 location={} name=src latency=0 ! rtph264depay ! h264parse ! fakesink name=sink sync=false",
            url
        );

        let pipeline = gst::parse::launch(&pipeline_str)?;
        let pipeline = pipeline.downcast::<gst::Pipeline>().unwrap();

        // Set up statistics collection
        let frames_received = Arc::new(AtomicU64::new(0));
        let bytes_received = Arc::new(AtomicU64::new(0));
        let frames_dropped = Arc::new(AtomicU64::new(0));
        let first_frame_time = Arc::new(std::sync::Mutex::new(None::<Instant>));

        // Install probe on sink pad
        if let Some(sink) = pipeline.by_name("sink") {
            if let Some(pad) = sink.static_pad("sink") {
                let frames = frames_received.clone();
                let bytes = bytes_received.clone();
                let first_frame = first_frame_time.clone();
                let start = start_time.clone();

                pad.add_probe(gst::PadProbeType::BUFFER, move |_, probe_info| {
                    if let Some(gst::PadProbeData::Buffer(ref buffer)) = probe_info.data {
                        frames.fetch_add(1, Ordering::Relaxed);
                        bytes.fetch_add(buffer.size() as u64, Ordering::Relaxed);

                        // Record first frame time
                        let mut first = first_frame.lock().unwrap();
                        if first.is_none() {
                            *first = Some(Instant::now());
                        }
                    }
                    gst::PadProbeReturn::Ok
                });
            }
        }

        // Set up bus message handling
        let bus = pipeline.bus().unwrap();
        let results = self.results.clone();
        let stop_flag = self.stop_flag.clone();

        // Start pipeline
        pipeline.set_state(gst::State::Playing)?;

        // Record connection time
        let (state_res, _, _) = pipeline.state(Some(gst::ClockTime::from_seconds(10)));
        let connection_time = connection_start.elapsed();

        if state_res.is_err() {
            return Err("Failed to connect to camera".into());
        }

        // Monitor pipeline for test duration
        let test_start = Instant::now();
        let mut latency_samples = Vec::new();
        let mut bitrate_samples = Vec::new();
        let mut last_bytes = 0u64;
        let mut last_sample_time = Instant::now();

        while test_start.elapsed() < self.config.test_duration && !stop_flag.load(Ordering::Relaxed)
        {
            // Check for bus messages
            if let Some(msg) = bus.timed_pop(gst::ClockTime::from_mseconds(100)) {
                match msg.view() {
                    gst::MessageView::Error(err) => {
                        let mut res = results.lock().unwrap();
                        res.error_count += 1;
                        eprintln!("Pipeline error: {:?}", err);
                    }
                    gst::MessageView::Qos(_qos) => {
                        // Track dropped frames
                        // Note: dropped() method not available in current GStreamer bindings
                        // Would need to parse QoS stats differently
                    }
                    gst::MessageView::Latency(_) => {
                        // Measure latency if requested
                        if self.config.measure_latency {
                            if let Some(latency) = pipeline.latency() {
                                let latency_duration = Duration::from_nanos(latency.nseconds());
                                latency_samples.push(latency_duration);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Calculate bitrate
            if self.config.measure_throughput {
                let current_bytes = bytes_received.load(Ordering::Relaxed);
                let elapsed = last_sample_time.elapsed();

                if elapsed >= Duration::from_secs(1) {
                    let bytes_diff = current_bytes - last_bytes;
                    let bitrate = (bytes_diff * 8) / elapsed.as_secs().max(1);
                    bitrate_samples.push(bitrate);

                    last_bytes = current_bytes;
                    last_sample_time = Instant::now();
                }
            }

            // Small delay to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // Stop pipeline
        pipeline.set_state(gst::State::Null)?;

        // Calculate final results
        let mut final_results = self.results.lock().unwrap();
        final_results.url = url.to_string();
        final_results.test_duration = test_start.elapsed();
        final_results.connection_time = connection_time;

        // First frame time
        if let Some(first_time) = *first_frame_time.lock().unwrap() {
            final_results.first_frame_time = first_time.duration_since(start_time);
        }

        // Frame statistics
        final_results.frames_received = frames_received.load(Ordering::Relaxed);
        final_results.frames_dropped = frames_dropped.load(Ordering::Relaxed);
        final_results.bytes_received = bytes_received.load(Ordering::Relaxed);

        // Latency statistics
        if !latency_samples.is_empty() {
            let sum: Duration = latency_samples.iter().sum();
            final_results.average_latency = sum / latency_samples.len() as u32;
            final_results.min_latency = *latency_samples.iter().min().unwrap();
            final_results.max_latency = *latency_samples.iter().max().unwrap();

            // Calculate jitter (standard deviation)
            if latency_samples.len() > 1 {
                let mean = final_results.average_latency.as_nanos() as f64;
                let variance = latency_samples
                    .iter()
                    .map(|&d| {
                        let diff = d.as_nanos() as f64 - mean;
                        diff * diff
                    })
                    .sum::<f64>()
                    / latency_samples.len() as f64;

                final_results.jitter = Duration::from_nanos(variance.sqrt() as u64);
            }
        }

        // Bitrate statistics
        if !bitrate_samples.is_empty() {
            let sum: u64 = bitrate_samples.iter().sum();
            final_results.average_bitrate = sum / bitrate_samples.len() as u64;
            final_results.peak_bitrate = *bitrate_samples.iter().max().unwrap_or(&0);
        }

        Ok(final_results.clone())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn format_results(results: &BenchmarkResults) -> String {
        let mut output = String::new();

        output.push_str(&format!("=== Benchmark Results for {} ===\n", results.url));
        output.push_str(&format!("Test Duration: {:?}\n", results.test_duration));
        output.push_str("\nConnection Metrics:\n");
        output.push_str(&format!(
            "  Connection Time: {:?}\n",
            results.connection_time
        ));
        output.push_str(&format!(
            "  First Frame Time: {:?}\n",
            results.first_frame_time
        ));

        output.push_str("\nLatency Metrics:\n");
        output.push_str(&format!(
            "  Average Latency: {:?}\n",
            results.average_latency
        ));
        output.push_str(&format!("  Min Latency: {:?}\n", results.min_latency));
        output.push_str(&format!("  Max Latency: {:?}\n", results.max_latency));
        output.push_str(&format!("  Jitter: {:?}\n", results.jitter));

        output.push_str("\nThroughput Metrics:\n");
        output.push_str(&format!("  Frames Received: {}\n", results.frames_received));
        output.push_str(&format!(
            "  Frames Dropped: {} ({:.2}%)\n",
            results.frames_dropped,
            if results.frames_received > 0 {
                (results.frames_dropped as f64 / results.frames_received as f64) * 100.0
            } else {
                0.0
            }
        ));
        output.push_str(&format!(
            "  Bytes Received: {} MB\n",
            results.bytes_received / (1024 * 1024)
        ));
        output.push_str(&format!(
            "  Average Bitrate: {:.2} Mbps\n",
            results.average_bitrate as f64 / 1_000_000.0
        ));
        output.push_str(&format!(
            "  Peak Bitrate: {:.2} Mbps\n",
            results.peak_bitrate as f64 / 1_000_000.0
        ));

        output.push_str("\nReliability Metrics:\n");
        output.push_str(&format!(
            "  Reconnection Count: {}\n",
            results.reconnection_count
        ));
        output.push_str(&format!("  Error Count: {}\n", results.error_count));

        output
    }
}

#[allow(dead_code)]
pub async fn run_benchmark_suite(
    urls: Vec<String>,
    config: BenchmarkConfig,
) -> Vec<BenchmarkResults> {
    let mut all_results = Vec::new();

    for url in urls {
        println!("Benchmarking: {}", url);

        let benchmark = CameraBenchmark::new(config.clone());
        match benchmark.benchmark_camera(&url).await {
            Ok(results) => {
                println!("{}", CameraBenchmark::format_results(&results));
                all_results.push(results);
            }
            Err(e) => {
                eprintln!("Benchmark failed for {}: {}", url, e);
            }
        }
    }

    all_results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_benchmark_config() {
        let config = BenchmarkConfig::default();
        assert_eq!(config.test_duration, Duration::from_secs(30));
        assert!(config.measure_latency);
        assert!(config.measure_throughput);
    }

    #[tokio::test]
    #[ignore] // Only run with real camera/stream
    async fn test_benchmark_public_stream() {
        gst::init().unwrap();

        let config = BenchmarkConfig {
            test_duration: Duration::from_secs(10),
            ..Default::default()
        };

        let benchmark = CameraBenchmark::new(config);
        let url =
            "rtsp://807e9439d5ca.entrypoint.cloud.wowza.com:1935/app-rC94792j/068b9c9a_stream2";

        match benchmark.benchmark_camera(url).await {
            Ok(results) => {
                println!("{}", CameraBenchmark::format_results(&results));

                assert!(results.connection_time > Duration::from_secs(0));
                assert!(results.frames_received > 0);
            }
            Err(e) => {
                eprintln!("Benchmark failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_format_results() {
        let results = BenchmarkResults {
            url: "rtsp://test.example.com/stream".to_string(),
            test_duration: Duration::from_secs(30),
            connection_time: Duration::from_millis(500),
            first_frame_time: Duration::from_millis(750),
            average_latency: Duration::from_millis(100),
            min_latency: Duration::from_millis(50),
            max_latency: Duration::from_millis(200),
            jitter: Duration::from_millis(25),
            frames_received: 900,
            frames_dropped: 5,
            bytes_received: 50_000_000,
            average_bitrate: 10_000_000,
            peak_bitrate: 15_000_000,
            reconnection_count: 0,
            error_count: 0,
        };

        let formatted = CameraBenchmark::format_results(&results);

        assert!(formatted.contains("Benchmark Results"));
        assert!(formatted.contains("10.00 Mbps"));
        assert!(formatted.contains("0.56%")); // Drop rate
    }
}
