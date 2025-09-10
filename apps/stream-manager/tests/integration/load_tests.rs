use super::common::*;
use stream_manager::manager::StreamState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::info;

/// Test concurrent stream limits
#[tokio::test]
async fn test_concurrent_stream_limits() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting concurrent stream limits test");
    
    let target_streams = 10;
    let mut success_count = 0;
    let mut failure_count = 0;
    
    // Try to add many streams
    for i in 0..target_streams {
        let (id, config) = create_test_stream_config(&format!("load-{}", i));
        match fixture.stream_manager.add_stream(id.clone(), config).await {
            Ok(_) => {
                success_count += 1;
                info!("Successfully added stream {}", id);
            }
            Err(e) => {
                info!("Failed to add stream {}: {}", i, e);
                failure_count += 1;
            }
        }
    }
    
    info!("Added {} streams successfully, {} failed", success_count, failure_count);
    assert!(success_count > 0, "Should be able to add at least one stream");
    
    // Wait for all added streams to be running
    let streams = fixture.stream_manager.list_streams().await;
    for stream in &streams {
        wait_for_stream_state(&fixture.stream_manager, &stream.id, StreamState::Running, Duration::from_secs(10)).await;
    }
    
    // Verify stream count
    assert_eq!(streams.len(), success_count);
    
    // Clean up
    fixture.cleanup().await;
}

/// Test stream reconnection under load
#[tokio::test]
async fn test_stream_reconnection() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting stream reconnection test");
    
    // Add a stream with simulated connection that might fail
    let stream_id = "reconnect-test";
    let (_, config) = create_stream_config_with_source(
        stream_id,
        "videotestsrc pattern=ball ! video/x-raw,width=320,height=240"
    );
    
    fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
        .await
        .expect("Failed to add stream");
    
    // Wait for initial connection
    assert!(
        wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should connect initially"
    );
    
    // Simulate disconnection by removing and re-adding
    for i in 0..3 {
        info!("Reconnection test iteration {}", i);
        
        // Remove stream (simulate disconnection)
        fixture.stream_manager.remove_stream(stream_id)
            .await
            .expect("Failed to remove stream");
        
        // Wait a bit
        sleep(Duration::from_millis(500)).await;
        
        // Re-add stream (reconnect)
        fixture.stream_manager.add_stream(stream_id.to_string(), config.clone())
            .await
            .expect("Failed to re-add stream");
        
        // Wait for reconnection
        assert!(
            wait_for_stream_state(&fixture.stream_manager, stream_id, StreamState::Running, Duration::from_secs(10)).await,
            "Stream should reconnect"
        );
    }
    
    // Clean up
    fixture.stream_manager.remove_stream(stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}

/// Test load balancing under pressure
#[tokio::test]
async fn test_load_balancing() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting load balancing test");
    
    let stream_count: usize = 5;
    let mut stream_ids = Vec::new();
    
    // Add multiple streams with different configurations
    for i in 0..stream_count {
        let generator = TestStreamGenerator::new()
            .with_resolution(320 + (i as u32) * 160, 240 + (i as u32) * 120)
            .with_fps(15 + (i as u32) * 5);
        
        let stream_id = format!("load-balance-{}", i);
        let (_, config) = create_stream_config_with_source(
            &stream_id,
            &generator.to_pipeline_string()
        );
        
        fixture.stream_manager.add_stream(stream_id.clone(), config)
            .await
            .expect(&format!("Failed to add stream {}", i));
        
        stream_ids.push(stream_id);
    }
    
    // Wait for all streams to start
    for id in &stream_ids {
        wait_for_stream_state(&fixture.stream_manager, id, StreamState::Running, Duration::from_secs(10)).await;
    }
    
    // Monitor for a while
    let monitor_duration = Duration::from_secs(5);
    let start = Instant::now();
    
    while start.elapsed() < monitor_duration {
        let streams = fixture.stream_manager.list_streams().await;
        let running_count = streams.iter()
            .filter(|s| s.state == StreamState::Running)
            .count();
        
        info!("Running streams: {}/{}", running_count, stream_count);
        assert_eq!(running_count, stream_count, "All streams should remain running");
        
        sleep(Duration::from_secs(1)).await;
    }
    
    // Clean up
    for id in &stream_ids {
        fixture.stream_manager.remove_stream(id)
            .await
            .expect(&format!("Failed to remove stream {}", id));
    }
    
    fixture.cleanup().await;
}

/// Test memory management under load
#[tokio::test]
async fn test_memory_management() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting memory management test");
    
    let mut metrics_collector = MetricsCollector::new();
    
    // Phase 1: Add streams gradually
    let max_streams = 5;
    let mut stream_ids = Vec::new();
    
    for i in 0..max_streams {
        let (id, config) = create_test_stream_config(&format!("memory-{}", i));
        
        fixture.stream_manager.add_stream(id.clone(), config)
            .await
            .expect(&format!("Failed to add stream {}", i));
        
        stream_ids.push(id.clone());
        
        // Wait for stream to start
        wait_for_stream_state(&fixture.stream_manager, &id, StreamState::Running, Duration::from_secs(10)).await;
        
        // Collect metrics
        metrics_collector.collect_sample(&fixture.stream_manager).await;
        
        sleep(Duration::from_secs(1)).await;
    }
    
    // Phase 2: Run at full load
    info!("Running at full load with {} streams", max_streams);
    for _ in 0..5 {
        metrics_collector.collect_sample(&fixture.stream_manager).await;
        sleep(Duration::from_secs(1)).await;
    }
    
    // Phase 3: Remove streams gradually
    for id in &stream_ids {
        fixture.stream_manager.remove_stream(id)
            .await
            .expect(&format!("Failed to remove stream {}", id));
        
        metrics_collector.collect_sample(&fixture.stream_manager).await;
        sleep(Duration::from_secs(1)).await;
    }
    
    // Analyze metrics
    let summary = metrics_collector.get_summary();
    info!("Memory test summary: {:?}", summary);
    
    // Basic sanity checks
    assert!(summary.max_concurrent_streams <= max_streams);
    assert!(summary.sample_count > 0);
    
    fixture.cleanup().await;
}

/// Test recording under high load
#[tokio::test]
async fn test_recording_performance() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting recording performance test");
    
    let stream_count = 3;
    let mut stream_ids = Vec::new();
    
    // Add multiple streams with recording enabled
    for i in 0..stream_count {
        let (id, mut config) = create_test_stream_config(&format!("rec-perf-{}", i));
        config.recording_enabled = true;
        
        fixture.stream_manager.add_stream(id.clone(), config)
            .await
            .expect(&format!("Failed to add stream {}", i));
        
        stream_ids.push(id);
    }
    
    // Wait for all streams to start recording
    for id in &stream_ids {
        assert!(
            wait_for_stream_state(&fixture.stream_manager, id, StreamState::Running, Duration::from_secs(10)).await,
            "Stream should be running"
        );
    }
    
    // Let them record for a while
    info!("Recording {} streams for 10 seconds", stream_count);
    sleep(Duration::from_secs(10)).await;
    
    // Check all streams are still healthy
    for id in &stream_ids {
        let info = fixture.stream_manager.get_stream_info(id)
            .await
            .expect(&format!("Should get info for stream {}", id));
        
        assert_eq!(info.state, StreamState::Running, "Stream {} should still be running", id);
        info!("Stream {} recording state: {:?}", id, info.recording_state);
    }
    
    // Clean up
    for id in &stream_ids {
        fixture.stream_manager.remove_stream(id)
            .await
            .expect(&format!("Failed to remove stream {}", id));
    }
    
    fixture.cleanup().await;
}

/// Test rapid stream creation and deletion
#[tokio::test]
async fn test_rapid_stream_churn() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting rapid stream churn test");
    
    let iterations = 5;
    let streams_per_iteration = 2;
    
    for iter in 0..iterations {
        info!("Churn iteration {}", iter);
        
        let mut stream_ids = Vec::new();
        
        // Rapidly add streams
        for i in 0..streams_per_iteration {
            let (id, config) = create_test_stream_config(&format!("churn-{}-{}", iter, i));
            
            fixture.stream_manager.add_stream(id.clone(), config)
                .await
                .expect(&format!("Failed to add stream in iteration {}", iter));
            
            stream_ids.push(id);
        }
        
        // Wait briefly for streams to start
        sleep(Duration::from_millis(500)).await;
        
        // Rapidly remove streams
        for id in stream_ids {
            fixture.stream_manager.remove_stream(&id)
                .await
                .expect(&format!("Failed to remove stream in iteration {}", iter));
        }
        
        // Brief pause between iterations
        sleep(Duration::from_millis(100)).await;
    }
    
    // Verify no streams are left
    let remaining = fixture.stream_manager.list_streams().await;
    assert_eq!(remaining.len(), 0, "No streams should remain after churn test");
    
    fixture.cleanup().await;
}

/// Test concurrent operations on streams
#[tokio::test]
async fn test_concurrent_operations() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting concurrent operations test");
    
    let manager = fixture.stream_manager.clone();
    let concurrent_count = 5;
    let success_count = Arc::new(AtomicUsize::new(0));
    
    // Spawn multiple concurrent tasks
    let mut handles = Vec::new();
    
    for i in 0..concurrent_count {
        let manager_clone = manager.clone();
        let success_clone = success_count.clone();
        
        let handle = tokio::spawn(async move {
            let (id, config) = create_test_stream_config(&format!("concurrent-{}", i));
            
            // Try to add stream
            if manager_clone.add_stream(id.clone(), config).await.is_ok() {
                success_clone.fetch_add(1, Ordering::SeqCst);
                
                // Wait a bit
                sleep(Duration::from_secs(1)).await;
                
                // Try to get info
                let _ = manager_clone.get_stream_info(&id).await;
                
                // Remove stream
                let _ = manager_clone.remove_stream(&id).await;
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete");
    }
    
    let successful = success_count.load(Ordering::SeqCst);
    info!("Concurrent operations: {}/{} successful", successful, concurrent_count);
    assert!(successful > 0, "At least some concurrent operations should succeed");
    
    fixture.cleanup().await;
}

/// Test stream stability over time
#[tokio::test]
async fn test_long_running_stability() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting long-running stability test");
    
    // Add a stream
    let (stream_id, config) = create_test_stream_config("stability-test");
    fixture.stream_manager.add_stream(stream_id.clone(), config)
        .await
        .expect("Failed to add stream");
    
    // Wait for stream to start
    assert!(
        wait_for_stream_state(&fixture.stream_manager, &stream_id, StreamState::Running, Duration::from_secs(10)).await,
        "Stream should start"
    );
    
    // Monitor stream for extended period
    let test_duration = Duration::from_secs(15);
    let start = Instant::now();
    let mut check_count = 0;
    
    while start.elapsed() < test_duration {
        // Check stream is still running
        let info = fixture.stream_manager.get_stream_info(&stream_id)
            .await
            .expect("Should get stream info");
        
        assert_eq!(info.state, StreamState::Running, "Stream should remain running");
        
        check_count += 1;
        sleep(Duration::from_secs(1)).await;
    }
    
    info!("Stream remained stable for {} checks over {:?}", check_count, test_duration);
    
    // Clean up
    fixture.stream_manager.remove_stream(&stream_id)
        .await
        .expect("Failed to remove stream");
    
    fixture.cleanup().await;
}