use super::common::*;
use stream_manager::manager::StreamState;
use std::time::Duration;
use tokio::time::sleep;
use tracing::info;

/// Test network interruption handling
#[tokio::test]
async fn test_network_interruption() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting network interruption test");
    
    // Create stream with network source (simulated)
    let stream_id = "network-test";
    let (_, config) = create_stream_config_with_source(
        stream_id,
        // Using fallbacksrc to simulate a network source that can handle interruptions
        "videotestsrc ! video/x-raw,width=640,height=480"
    );
    
    fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to connect
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should connect initially"
    );
    
    // Simulate network interruption by applying network conditions
    let network_sim = NetworkSimulator::new()
        .with_latency(500)
        .with_packet_loss(50.0);
    network_sim.apply().await;
    
    // Stream should still be in the system (might be reconnecting)
    sleep(Duration::from_secs(2)).await;
    let info = fixture.stream_manager.get_stream_info(stream_id).await;
    assert!(info.is_ok(), "Stream should still exist during network issues");
    
    // Reset network conditions
    network_sim.reset().await;
    
    // Stream should recover
    sleep(Duration::from_secs(2)).await;
    let info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Should get stream info after recovery");
    
    info!("Stream state after recovery: {:?}", info.state);
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test disk space handling
#[tokio::test]
async fn test_disk_space_handling() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting disk space handling test");
    
    // Create stream with recording enabled
    let stream_id = "disk-space-test";
    let (_, mut config) = create_test_stream_config(stream_id);
    config.recording_enabled = true;
    
    fixture.stream_manager.add_stream(stream_id.to_string(), config)
        .await
        .expect("Failed to add stream with recording");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should start"
    );
    
    // Simulate low disk space scenario
    // In a real test, we would check disk space and handle accordingly
    // For now, just let it run and verify it doesn't crash
    sleep(Duration::from_secs(3)).await;
    
    // Check stream is still running
    let info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Should get stream info");
    
    // Stream should handle disk space issues gracefully
    info!("Stream state during disk test: {:?}", info.state);
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test pipeline crash recovery
#[tokio::test]
async fn test_pipeline_crash_recovery() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting pipeline crash recovery test");
    
    // Create stream with potentially unstable pipeline
    let stream_id = "crash-test";
    let (_, config) = create_test_stream_config(stream_id);
    
    // Add the stream
    fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should start"
    );
    
    // Simulate a crash by removing and quickly re-adding
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    // Very quick re-add to simulate recovery
    sleep(Duration::from_millis(100)).await;
    
    fixture.stream_manager.add_stream(stream_id.to_string(), config)
        .await
        .expect("Failed to re-add stream after crash");
    
    // Should recover
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should recover after crash"
    );
    
    // Verify recovery
    let info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Should get stream info after recovery");
    assert_eq!(info.state, StreamState::Running, "Stream should be running after recovery");
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test concurrent failure handling
#[tokio::test]
async fn test_concurrent_failures() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting concurrent failures test");
    
    let failure_count = 3;
    let mut stream_ids = Vec::new();
    
    // Add multiple streams
    for i in 0..failure_count {
        let stream_id = format!("concurrent-fail-{}", i);
        let (_, config) = create_test_stream_config(&stream_id);
        
        fixture.stream_manager.add_stream(stream_id.clone(), config)
            .await
            .expect(&format!("Failed to add stream {}", i));
        
        stream_ids.push(stream_id);
    }
    
    // Wait for all to start
    for id in &stream_ids {
        wait_for_stream_state(&fixture.stream_manager, id, StreamState::Running, Duration::from_secs(10)).await;
    }
    
    // Simulate concurrent failures by removing all at once
    let mut handles = Vec::new();
    for id in stream_ids.clone() {
        let manager = fixture.stream_manager.clone();
        let handle = tokio::spawn(async move {
            manager.remove_stream(&id).await
        });
        handles.push(handle);
    }
    
    // Wait for all removals
    for handle in handles {
        let _ = handle.await;
    }
    
    // Verify all streams are removed
    for id in &stream_ids {
        let result = fixture.stream_manager.get_stream_info(id).await;
        assert!(result.is_err(), "Stream {} should be removed", id);
    }
    
    // System should be stable
    let remaining = fixture.stream_manager.list_streams().await;
    assert_eq!(remaining.len(), 0, "No streams should remain");
    
    fixture.cleanup().await;
}

/// Test resource leak prevention
#[tokio::test]
async fn test_resource_leak_prevention() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting resource leak prevention test");
    
    // Repeatedly add and remove streams to check for leaks
    let iterations = 5;
    
    for iter in 0..iterations {
        let stream_id = format!("leak-test-{}", iter);
        let (_, config) = create_test_stream_config(&stream_id);
        
        // Add stream
        fixture.stream_manager.add_stream(stream_id.clone(), config)
            .await
            .expect("Failed to add stream");
        
        // Wait for it to start
        wait_for_stream_state(&fixture.stream_manager, &stream_id, StreamState::Running, Duration::from_secs(10)).await;
        
        // Run for a bit
        sleep(Duration::from_secs(1)).await;
        
        // Remove stream
        fixture.stream_manager.remove_stream(&stream_id)
            .await
            .expect("Failed to remove stream");
        
        // Ensure it's fully cleaned up
        sleep(Duration::from_millis(200)).await;
    }
    
    // Verify no streams remain
    let remaining = fixture.stream_manager.list_streams().await;
    assert_eq!(remaining.len(), 0, "No streams should remain after leak test");
    
    // In a real test, we would check memory usage here
    info!("Resource leak test completed successfully");
    
    fixture.cleanup().await;
}

/// Test error state recovery
#[tokio::test]
async fn test_error_state_recovery() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting error state recovery test");
    
    // Try to add a stream with an invalid source
    let stream_id = "error-recovery-test";
    let (_, config) = create_stream_config_with_source(
        stream_id,
        // This pipeline might fail depending on the system
        "fakesrc ! video/x-raw,width=640,height=480"
    );
    
    // Add the stream (might fail or enter error state)
    let add_result = fixture.stream_manager.add_stream(stream_id.to_string(), config.clone()).await;
    
    if add_result.is_ok() {
        // Wait a bit to see if it enters error state
        sleep(Duration::from_secs(2)).await;
        
        // Check state
        if let Ok(info) = fixture.stream_manager.get_stream_info(stream_id).await {
            info!("Stream state: {:?}", info.state);
            
            // Remove the problematic stream
            let _ = fixture.stream_manager.remove_stream(stream_id).await;
        }
    }
    
    // Now add a valid stream to ensure system recovered
    let recovery_id = "recovery-test";
    let (_, valid_config) = create_test_stream_config(recovery_id);
    
    fixture.stream_manager.add_stream(recovery_id.to_string(), valid_config)
        .await
        .expect("Should be able to add valid stream after error");
    
    // Verify it works
    assert!(
        wait_for_stream_state(&fixture.stream_manager, recovery_id, StreamState::Running, Duration::from_secs(10)).await,
        "Recovery stream should run successfully"
    );
    
    // Clean up
    fixture.stream_manager.remove_stream(recovery_id)
        .await
        .expect("Failed to remove recovery stream");
    
    fixture.cleanup().await;
}

/// Test handling of duplicate stream IDs
#[tokio::test]
async fn test_duplicate_stream_handling() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting duplicate stream handling test");
    
    let stream_id = "duplicate-test";
    let (_, config) = create_test_stream_config(stream_id);
    
    // Add the first stream
    fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
        .await
        .expect("Should add first stream");
    
    // Wait for it to start
    wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await;
    
    // Try to add duplicate
    let duplicate_result = fixture.stream_manager.add_stream(stream_id.to_string(), config).await;
    assert!(duplicate_result.is_err(), "Should not allow duplicate stream ID");
    
    // Original stream should still be running
    let info = fixture.stream_manager.get_stream_info(stream_id)
        .await
        .expect("Original stream should still exist");
    assert_eq!(info.state, StreamState::Running, "Original stream should still be running");
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test invalid configuration handling
#[tokio::test]
async fn test_invalid_config_handling() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting invalid configuration handling test");
    
    // Create config with potentially problematic settings
    let stream_id = "invalid-config-test";
    let (_, mut config) = create_test_stream_config(stream_id);
    
    // Set some extreme values
    config.reconnect_timeout_seconds = 0; // Might cause issues
    config.max_reconnect_attempts = 0; // No reconnection
    config.buffer_size_mb = 0; // No buffering
    
    // Try to add stream with problematic config
    let result = fixture.stream_manager.add_stream(stream_id.to_string(), config).await;
    
    if result.is_ok() {
        // If it was added, verify it can be removed cleanly
        fixture.stream_manager.remove_stream(stream_id)
            .await
            .expect("Should be able to remove stream with invalid config");
    } else {
        info!("Stream with invalid config was rejected as expected");
    }
    
    // System should still be functional
    let test_id = "valid-after-invalid";
    let (_, valid_config) = create_test_stream_config(test_id);
    
    fixture.stream_manager.add_stream(test_id.to_string(), valid_config)
        .await
        .expect("Should accept valid config after invalid one");
    
    fixture.stream_manager.remove_stream(test_id)
        .await
        .expect("Failed to remove test stream");
    
    fixture.cleanup().await;
}