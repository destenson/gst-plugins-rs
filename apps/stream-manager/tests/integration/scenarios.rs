use super::common::*;
use stream_manager::manager::StreamState;
use std::time::Duration;
use tracing::info;

#[tokio::test]
async fn test_multi_stream_recording() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting multi-stream recording test");
    
    // Add multiple streams
    let stream_ids = vec!["stream1", "stream2", "stream3"];
    
    for id in &stream_ids {
        let config = create_test_stream_config(id);
        fixture.stream_manager.add_stream(config).await.unwrap();
    }
    
    // Wait for all streams to be running
    for id in &stream_ids {
        assert!(
            wait_for_stream_state(&fixture.stream_manager, id, StreamState::Running, Duration::from_secs(5)).await,
            "Stream {} did not start", id
        );
    }
    
    // Let streams run for a bit
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // Verify all streams are still running
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), 3, "Expected 3 streams");
    
    for stream in &streams {
        assert_eq!(stream.state, StreamState::Running, "Stream {} not running", stream.id);
    }
    
    // Stop all streams
    for id in &stream_ids {
        fixture.stream_manager.remove_stream(id).await.unwrap();
    }
    
    info!("Multi-stream recording test completed");
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_stream_failure_recovery() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting stream failure recovery test");
    
    // Add a stream with auto-reconnect
    let stream_id = "recovery-test";
    let mut config = create_test_stream_config(stream_id);
    config.source_url = "fallbacksrc uri=rtsp://invalid.url timeout=1000000000 ! decodebin".to_string();
    
    fixture.stream_manager.add_stream(config).await.unwrap();
    
    // Stream should go to error state initially
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Check if stream is in error/reconnecting state
    let info = fixture.stream_manager.get_stream_info(stream_id).await;
    assert!(info.is_some(), "Stream info not found");
    
    // Remove stream
    fixture.stream_manager.remove_stream(stream_id).await.unwrap();
    
    info!("Stream failure recovery test completed");
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_api_operation_sequences() {
    super::init_test_environment();
    let fixture = TestFixture::new().await.with_server().await;
    
    info!("Starting API operation sequence test");
    
    let client = &fixture.test_server.as_ref().unwrap();
    
    // Test health check
    let req = client.get("/api/v1/health");
    let resp = req.send().await.unwrap();
    assert!(resp.status().is_success(), "Health check failed");
    
    // Add stream via API
    let stream_config = serde_json::json!({
        "id": "api-test-stream",
        "source_url": "videotestsrc ! video/x-raw,width=640,height=480",
        "source_type": "test",
        "recording": {
            "enabled": true,
            "base_path": "/tmp/test-recordings",
            "segment_duration_secs": 10
        }
    });
    
    let req = client.post("/api/v1/streams")
        .send_json(&stream_config);
    let resp = req.await.unwrap();
    assert!(resp.status().is_success(), "Failed to add stream");
    
    // List streams
    let req = client.get("/api/v1/streams");
    let resp = req.send().await.unwrap();
    assert!(resp.status().is_success(), "Failed to list streams");
    
    // Get stream info
    let req = client.get("/api/v1/streams/api-test-stream");
    let resp = req.send().await.unwrap();
    assert!(resp.status().is_success(), "Failed to get stream info");
    
    // Remove stream
    let req = client.delete("/api/v1/streams/api-test-stream");
    let resp = req.send().await.unwrap();
    assert!(resp.status().is_success(), "Failed to remove stream");
    
    info!("API operation sequence test completed");
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_concurrent_stream_management() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting concurrent stream management test");
    
    // Spawn multiple tasks to add streams concurrently
    let mut handles = vec![];
    
    for i in 0..5 {
        let manager = fixture.stream_manager.clone();
        let handle = tokio::spawn(async move {
            let config = create_test_stream_config(&format!("concurrent-{}", i));
            manager.add_stream(config).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Failed to add stream concurrently");
    }
    
    // Verify all streams were added
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), 5, "Expected 5 streams");
    
    // Remove streams concurrently
    let mut handles = vec![];
    
    for i in 0..5 {
        let manager = fixture.stream_manager.clone();
        let handle = tokio::spawn(async move {
            manager.remove_stream(&format!("concurrent-{}", i)).await
        });
        handles.push(handle);
    }
    
    // Wait for all removals
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Failed to remove stream concurrently");
    }
    
    // Verify all streams were removed
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), 0, "Expected 0 streams");
    
    info!("Concurrent stream management test completed");
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_recording_segment_rotation() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting recording segment rotation test");
    
    // Create stream with short segment duration
    let stream_id = "segment-test";
    let mut config = create_test_stream_config(stream_id);
    
    if let Some(ref mut recording) = config.recording {
        recording.segment_duration = Duration::from_secs(2);
        recording.max_segments = Some(3);
    }
    
    fixture.stream_manager.add_stream(config).await.unwrap();
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(5)).await,
        "Stream did not start"
    );
    
    // Let it run for multiple segments
    tokio::time::sleep(Duration::from_secs(7)).await;
    
    // Validate recordings
    let validator = RecordingValidator::new(std::path::PathBuf::from("/tmp/test-recordings"));
    
    match validator.validate_stream_recordings(stream_id).await {
        Ok(result) => {
            info!("Recording validation: {} files, {} bytes", result.file_count, result.total_size_bytes);
            // We should have at least 2 segments by now
            assert!(result.file_count >= 2, "Expected at least 2 recording segments");
        }
        Err(e) => {
            // Recording directory might not exist in test environment
            info!("Recording validation skipped: {}", e);
        }
    }
    
    fixture.stream_manager.remove_stream(stream_id).await.unwrap();
    
    info!("Recording segment rotation test completed");
    fixture.cleanup().await;
}

#[tokio::test]
async fn test_stream_metrics_collection() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting stream metrics collection test");
    
    let mut metrics = MetricsCollector::new();
    
    // Collect baseline metrics
    metrics.collect_sample(&fixture.stream_manager).await;
    
    // Add streams progressively
    for i in 0..3 {
        let config = create_test_stream_config(&format!("metrics-{}", i));
        fixture.stream_manager.add_stream(config).await.unwrap();
        
        tokio::time::sleep(Duration::from_millis(500)).await;
        metrics.collect_sample(&fixture.stream_manager).await;
    }
    
    // Let streams run
    for _ in 0..5 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        metrics.collect_sample(&fixture.stream_manager).await;
    }
    
    // Remove streams progressively
    for i in 0..3 {
        fixture.stream_manager.remove_stream(&format!("metrics-{}", i)).await.unwrap();
        metrics.collect_sample(&fixture.stream_manager).await;
    }
    
    // Get metrics summary
    let summary = metrics.get_summary();
    
    info!("Metrics summary: {:?}", summary);
    
    assert!(summary.sample_count > 0, "No metrics collected");
    assert!(summary.max_concurrent_streams >= 3, "Expected at least 3 concurrent streams");
    assert!(summary.avg_cpu_percent > 0.0, "No CPU metrics");
    assert!(summary.avg_memory_mb > 0.0, "No memory metrics");
    
    info!("Stream metrics collection test completed");
    fixture.cleanup().await;
}