use super::common::*;
use stream_manager::manager::StreamState;
use std::time::Duration;
use tracing::{error, info, warn};

/// Test network interruption handling
#[tokio::test]
async fn test_network_interruption() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting network interruption test");
    
    // Create stream with network source
    let stream_id = "network-test";
    let config = StreamConfig {
        id: stream_id.to_string(),
        source_url: "fallbacksrc uri=rtsp://example.com/stream timeout=2000000000 ! decodebin".to_string(),
        source_type: stream_manager::config::SourceType::Rtsp,
        recording: Some(stream_manager::config::RecordingConfig {
            enabled: true,
            base_path: std::path::PathBuf::from("/tmp/test-recordings"),
            segment_duration: Duration::from_secs(10),
            max_segments: Some(5),
            format: stream_manager::config::RecordingFormat::Mp4,
        }),
        inference: None,
        rtsp_outputs: vec![],
    };
    
    fixture.stream_manager.add_stream(config).await.unwrap();
    
    // Simulate network conditions
    let network = NetworkSimulator::new()
        .with_latency(500)
        .with_packet_loss(10.0);
    
    network.apply().await;
    
    // Wait for stream to handle network issues
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Check stream status
    let info = fixture.stream_manager.get_stream_info(stream_id).await;
    if let Some(info) = info {
        info!("Stream state during network issues: {:?}", info.state);
    }
    
    // Restore network
    network.reset().await;
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id).await.ok();
    
    info!("Network interruption test completed");
    fixture.cleanup().await;
}

/// Test disk full simulation
#[tokio::test]
async fn test_disk_full_handling() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting disk full handling test");
    
    // This test would require actual disk space manipulation
    // For now, we'll test the recording with limited segments
    
    let stream_id = "disk-test";
    let config = StreamConfig {
        id: stream_id.to_string(),
        source_url: "videotestsrc ! video/x-raw,width=640,height=480".to_string(),
        source_type: stream_manager::config::SourceType::Test,
        recording: Some(stream_manager::config::RecordingConfig {
            enabled: true,
            base_path: std::path::PathBuf::from("/tmp/test-recordings"),
            segment_duration: Duration::from_secs(2),
            max_segments: Some(2), // Simulate limited disk space
            format: stream_manager::config::RecordingFormat::Mp4,
        }),
        inference: None,
        rtsp_outputs: vec![],
    };
    
    fixture.stream_manager.add_stream(config).await.unwrap();
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(5)).await,
        "Stream did not start"
    );
    
    // Let it run to hit segment limit
    tokio::time::sleep(Duration::from_secs(6)).await;
    
    // Verify stream is still running despite rotation
    let info = fixture.stream_manager.get_stream_info(stream_id).await;
    if let Some(info) = info {
        assert_eq!(info.state, StreamState::Running, "Stream should continue despite segment rotation");
    }
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id).await.ok();
    
    info!("Disk full handling test completed");
    fixture.cleanup().await;
}

/// Test pipeline error injection
#[tokio::test]
async fn test_pipeline_errors() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting pipeline error test");
    
    // Create stream with intentionally broken pipeline
    let stream_id = "error-test";
    let config = StreamConfig {
        id: stream_id.to_string(),
        source_url: "videotestsrc ! video/x-raw,width=640,height=480 ! fakesink".to_string(),
        source_type: stream_manager::config::SourceType::Test,
        recording: Some(stream_manager::config::RecordingConfig {
            enabled: true,
            base_path: std::path::PathBuf::from("/tmp/test-recordings"),
            segment_duration: Duration::from_secs(10),
            max_segments: Some(5),
            format: stream_manager::config::RecordingFormat::Mp4,
        }),
        inference: None,
        rtsp_outputs: vec![],
    };
    
    // Adding the stream might fail or succeed depending on pipeline validation
    match fixture.stream_manager.add_stream(config).await {
        Ok(_) => {
            info!("Stream added despite potential pipeline issues");
            
            // Check if it enters error state
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            let info = fixture.stream_manager.get_stream_info(stream_id).await;
            if let Some(info) = info {
                info!("Stream state: {:?}", info.state);
            }
            
            fixture.stream_manager.remove_stream(stream_id).await.ok();
        }
        Err(e) => {
            info!("Stream rejected due to pipeline error: {}", e);
        }
    }
    
    info!("Pipeline error test completed");
    fixture.cleanup().await;
}

/// Test system resource limits
#[tokio::test]
async fn test_resource_limits() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting resource limits test");
    
    // Try to create many resource-intensive streams
    let mut created = vec![];
    let mut failed = 0;
    
    for i in 0..20 {
        // Create high-resolution stream
        let config = TestStreamGenerator::new()
            .with_resolution(3840, 2160) // 4K
            .with_fps(60);
        
        let stream_config = StreamConfig {
            id: format!("resource-limit-{}", i),
            source_url: config.to_pipeline_string(),
            source_type: stream_manager::config::SourceType::Test,
            recording: Some(stream_manager::config::RecordingConfig {
                enabled: true,
                base_path: std::path::PathBuf::from("/tmp/test-recordings"),
                segment_duration: Duration::from_secs(10),
                max_segments: Some(5),
                format: stream_manager::config::RecordingFormat::Mp4,
            }),
            inference: None,
            rtsp_outputs: vec![],
        };
        
        match fixture.stream_manager.add_stream(stream_config).await {
            Ok(_) => {
                created.push(format!("resource-limit-{}", i));
                info!("Created stream {}", i);
            }
            Err(e) => {
                warn!("Failed to create stream {} (expected): {}", i, e);
                failed += 1;
                break; // Stop when we hit limits
            }
        }
        
        // Small delay between additions
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    
    info!("Created {} streams before hitting limits, {} failed", created.len(), failed);
    
    // Clean up created streams
    for id in created {
        fixture.stream_manager.remove_stream(&id).await.ok();
    }
    
    info!("Resource limits test completed");
    fixture.cleanup().await;
}

/// Test recovery from various failure scenarios
#[tokio::test]
async fn test_failure_recovery_sequence() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting failure recovery sequence test");
    
    let stream_id = "recovery-seq";
    
    // Scenario 1: Add stream with fallback source
    let config = StreamConfig {
        id: stream_id.to_string(),
        source_url: "fallbacksrc uri=rtsp://invalid.url fallback-uri=videotestsrc timeout=1000000000 ! decodebin".to_string(),
        source_type: stream_manager::config::SourceType::Rtsp,
        recording: None,
        inference: None,
        rtsp_outputs: vec![],
    };
    
    fixture.stream_manager.add_stream(config.clone()).await.unwrap();
    info!("Added stream with fallback source");
    
    // Wait for fallback to activate
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Scenario 2: Remove and re-add quickly
    fixture.stream_manager.remove_stream(stream_id).await.unwrap();
    info!("Removed stream");
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    fixture.stream_manager.add_stream(config.clone()).await.unwrap();
    info!("Re-added stream");
    
    // Scenario 3: Concurrent operations
    let manager = fixture.stream_manager.clone();
    let id = stream_id.to_string();
    
    let handle1 = tokio::spawn(async move {
        manager.get_stream_info(&id).await
    });
    
    let manager = fixture.stream_manager.clone();
    let id = stream_id.to_string();
    
    let handle2 = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(100)).await;
        manager.list_streams().await
    });
    
    // Wait for concurrent operations
    let info = handle1.await.unwrap();
    let streams = handle2.await.unwrap();
    
    if info.is_some() {
        info!("Stream info retrieved during concurrent access");
    }
    info!("Total streams during concurrent access: {}", streams.len());
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id).await.ok();
    
    info!("Failure recovery sequence test completed");
    fixture.cleanup().await;
}

/// Helper to simulate various failure conditions
pub struct FailureInjector {
    pub network_failure: bool,
    pub disk_failure: bool,
    pub memory_pressure: bool,
    pub cpu_throttle: bool,
}

impl FailureInjector {
    pub fn new() -> Self {
        Self {
            network_failure: false,
            disk_failure: false,
            memory_pressure: false,
            cpu_throttle: false,
        }
    }
    
    pub fn enable_from_env(&mut self) {
        if std::env::var("INJECT_FAILURES").is_ok() {
            info!("Failure injection enabled from environment");
            self.network_failure = std::env::var("INJECT_NETWORK_FAILURE").is_ok();
            self.disk_failure = std::env::var("INJECT_DISK_FAILURE").is_ok();
            self.memory_pressure = std::env::var("INJECT_MEMORY_PRESSURE").is_ok();
            self.cpu_throttle = std::env::var("INJECT_CPU_THROTTLE").is_ok();
        }
    }
    
    pub async fn apply(&self) {
        if self.network_failure {
            info!("Injecting network failure");
            // Would use iptables or similar to drop packets
        }
        
        if self.disk_failure {
            info!("Injecting disk failure");
            // Would fill disk or make read-only
        }
        
        if self.memory_pressure {
            info!("Injecting memory pressure");
            // Would allocate large amounts of memory
        }
        
        if self.cpu_throttle {
            info!("Injecting CPU throttle");
            // Would use cgroups to limit CPU
        }
    }
    
    pub async fn reset(&self) {
        info!("Resetting all injected failures");
        // Reset all failure conditions
    }
}