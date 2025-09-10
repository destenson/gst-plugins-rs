use super::common::*;
use stream_manager::manager::StreamState;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

/// Test basic stream lifecycle
#[tokio::test]
async fn test_basic_stream_lifecycle() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting basic stream lifecycle test");
    
    // Create and add a stream
    let (stream_id, config) = create_test_stream_config("lifecycle-test");
    fixture.stream_manager.add_stream(stream_id.clone(), config)
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, &stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should start and be running"
    );
    
    // Check stream info
    let info = fixture.stream_manager.get_stream_info(&stream_id)
        .await
        .expect("Should get stream info");
    assert_eq!(info.id, stream_id);
    assert_eq!(info.state, StreamState::Running);
    
    // Remove stream
    fixture.stream_manager.remove_stream(&stream_id)
        .await
        .expect("Failed to remove stream");
    
    // Verify stream is removed
    let result = fixture.stream_manager.get_stream_info(&stream_id).await;
    assert!(result.is_err(), "Stream should not exist after removal");
    
    fixture.cleanup().await;
}

/// Test multiple stream management
#[tokio::test]
async fn test_multiple_streams() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting multiple streams test");
    
    let stream_count = 3;
    let mut stream_ids = Vec::new();
    
    // Add multiple streams
    for i in 0..stream_count {
        let (id, config) = create_test_stream_config(&format!("multi-{}", i));
        fixture.stream_manager.add_stream(id.clone(), config)
            .await
            .expect(&format!("Failed to add stream {}", i));
        stream_ids.push(id);
    }
    
    // Wait for all streams to be running
    for id in &stream_ids {
        assert!(
            wait_for_stream_state(&fixture.stream_manager, id, StreamState::Running, Duration::from_secs(10)).await,
            "Stream should be running"
        );
    }
    
    // List streams and verify count
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), stream_count, "Should have {} streams", stream_count);
    
    // Verify all our streams are in the list
    for id in &stream_ids {
        assert!(
            streams.iter().any(|s| s.id == *id),
            "Stream {} should be in the list", id
        );
    }
    
    // Remove all streams
    for id in &stream_ids {
        fixture.stream_manager.remove_stream(id)
            .await
            .expect(&format!("Failed to remove stream {}", id));
    }
    
    // Verify all streams are removed
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), 0, "Should have no streams after removal");
    
    fixture.cleanup().await;
}

/// Test recording functionality
#[tokio::test]
async fn test_recording() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting recording test");
    
    // Create stream with recording enabled
    let (stream_id, mut config) = create_test_stream_config("recording-test");
    config.recording_enabled = true;
    
    fixture.stream_manager.add_stream(stream_id.clone(), config)
        .await
        .expect("Failed to add stream with recording");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, &stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should be running"
    );
    
    // Let it record for a few seconds
    sleep(Duration::from_secs(5)).await;
    
    // Check stream info shows recording
    let info = fixture.stream_manager.get_stream_info(&stream_id)
        .await
        .expect("Should get stream info");
    
    // The recording state should indicate recording is active
    // Note: actual recording validation would require checking files on disk
    info!("Stream recording state: {:?}", info.recording_state);
    
    // Stop the stream
    fixture.stream_manager.remove_stream(&stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test stream health monitoring
#[tokio::test]
async fn test_stream_health_monitoring() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting stream health monitoring test");
    
    // Create and add a test stream
    let (stream_id, config) = create_test_stream_config("health-test");
    fixture.stream_manager.add_stream(stream_id.clone(), config)
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to be running
    assert!(
        wait_for_stream_state(&fixture.stream_manager, &stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should be running"
    );
    
    // Wait for health to be established
    assert!(
        wait_for_stream_health(&fixture.stream_manager, &stream_id, Duration::from_secs(10)).await,
        "Stream should become healthy"
    );
    
    // Get stream info and check health
    let info = fixture.stream_manager.get_stream_info(&stream_id)
        .await
        .expect("Should get stream info");
    
    assert!(info.health.is_healthy, "Stream should be healthy");
    info!("Stream health: frames_received={}, bitrate={:.2} Mbps", 
          info.health.frames_received, 
          info.health.bitrate_mbps);
    
    // Clean up
    fixture.stream_manager.remove_stream(&stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test graceful shutdown
#[tokio::test]
async fn test_graceful_shutdown() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting graceful shutdown test");
    
    // Add multiple streams
    let stream_count = 3;
    let mut stream_ids = Vec::new();
    
    for i in 0..stream_count {
        let (id, config) = create_test_stream_config(&format!("shutdown-{}", i));
        fixture.stream_manager.add_stream(id.clone(), config)
            .await
            .expect(&format!("Failed to add stream {}", i));
        stream_ids.push(id);
    }
    
    // Wait for all streams to be running
    for id in &stream_ids {
        assert!(
            wait_for_stream_state(&fixture.stream_manager, id, StreamState::Running, Duration::from_secs(10)).await,
            "Stream should be running"
        );
    }
    
    // Verify all streams are running
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), stream_count);
    
    // Gracefully clean up all streams
    fixture.cleanup().await;
    
    // Verify all streams are removed
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), 0, "All streams should be removed after cleanup");
    
    info!("Graceful shutdown completed successfully");
}

/// Test stream restart after error
#[tokio::test]
async fn test_stream_restart() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting stream restart test");
    
    // Create stream with a source that might fail
    let stream_id = "restart-test";
    let (_, config) = create_test_stream_config(stream_id);
    
    // Add the stream
    fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should start running"
    );
    
    // Remove the stream (simulating a failure/stop)
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    // Wait a bit
    sleep(Duration::from_secs(1)).await;
    
    // Re-add the stream (restart)
    fixture.stream_manager.add_stream(stream_id.to_string(), config)
        .await
        .expect("Failed to re-add stream");
    
    // Wait for stream to start again
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should restart and be running"
    );
    
    // Verify stream is healthy after restart
    let info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Should get stream info after restart");
    assert_eq!(info.state, StreamState::Running);
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test stream configuration update
#[tokio::test] 
async fn test_stream_config_update() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting stream configuration update test");
    
    // Create initial stream
    let stream_id = "config-update-test";
    let (_, mut config) = create_test_stream_config(stream_id);
    config.recording_enabled = false;
    
    // Add stream without recording
    fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should be running"
    );
    
    // Get initial info
    let info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Should get stream info");
    assert!(!info.config.recording_enabled, "Recording should be disabled initially");
    
    // To update config, we need to remove and re-add the stream
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    // Update config
    config.recording_enabled = true;
    
    // Re-add with new config
    fixture.stream_manager.add_stream(stream_id.to_string(), config)
        .await
        .expect("Failed to re-add stream with updated config");
    
    // Wait for stream to start again
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should be running with new config"
    );
    
    // Verify config is updated
    let updated_info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Should get updated stream info");
    assert!(updated_info.config.recording_enabled, "Recording should be enabled after update");
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}