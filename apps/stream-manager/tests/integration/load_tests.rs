use super::common::*;
use stream_manager::manager::StreamState;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info};

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
        let config = create_test_stream_config(&format!("load-{}", i));
        match fixture.stream_manager.add_stream(config).await {
            Ok(_) => success_count += 1,
            Err(e) => {
                info!("Failed to add stream {}: {}", i, e);
                failure_count += 1;
            }
        }
    }
    
    info!("Added {} streams successfully, {} failed", success_count, failure_count);
    
    // Verify active streams
    let streams = fixture.stream_manager.list_streams().await;
    assert_eq!(streams.len(), success_count, "Stream count mismatch");
    
    // Clean up
    for stream in streams {
        fixture.stream_manager.remove_stream(&stream.id).await.ok();
    }
    
    info!("Concurrent stream limits test completed");
    fixture.cleanup().await;
}

/// Test sustained load over time
#[tokio::test]
#[ignore] // This test takes a long time, run with --ignored flag
async fn test_sustained_load() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting sustained load test");
    
    let test_duration = Duration::from_secs(60); // 1 minute
    let stream_count = 5;
    let mut metrics = MetricsCollector::new();
    
    // Add initial streams
    for i in 0..stream_count {
        let config = create_test_stream_config(&format!("sustained-{}", i));
        fixture.stream_manager.add_stream(config).await.unwrap();
    }
    
    // Run under load
    let start = Instant::now();
    let mut sample_count = 0;
    
    while start.elapsed() < test_duration {
        metrics.collect_sample(&fixture.stream_manager).await;
        sample_count += 1;
        
        // Periodically verify streams are still running
        if sample_count % 10 == 0 {
            let streams = fixture.stream_manager.list_streams().await;
            let running = streams.iter()
                .filter(|s| matches!(s.state, StreamState::Running))
                .count();
            
            if running < stream_count {
                error!("Some streams stopped: {}/{}", running, stream_count);
            }
        }
        
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    
    // Get final metrics
    let summary = metrics.get_summary();
    info!("Sustained load test summary: {:?}", summary);
    
    // Verify stability
    assert_eq!(summary.max_concurrent_streams, stream_count, "Stream count changed during test");
    
    // Clean up
    for i in 0..stream_count {
        fixture.stream_manager.remove_stream(&format!("sustained-{}", i)).await.ok();
    }
    
    info!("Sustained load test completed");
    fixture.cleanup().await;
}

/// Test burst traffic handling
#[tokio::test]
async fn test_burst_traffic() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting burst traffic test");
    
    let burst_size = 5;
    let burst_count = 3;
    let delay_between_bursts = Duration::from_secs(2);
    
    for burst in 0..burst_count {
        info!("Starting burst {}", burst);
        
        // Add streams in burst
        let mut handles = vec![];
        for i in 0..burst_size {
            let manager = fixture.stream_manager.clone();
            let stream_id = format!("burst-{}-{}", burst, i);
            
            let handle = tokio::spawn(async move {
                let config = create_test_stream_config(&stream_id);
                manager.add_stream(config).await
            });
            handles.push((stream_id, handle));
        }
        
        // Wait for burst to complete
        for (id, handle) in handles {
            match handle.await {
                Ok(Ok(_)) => info!("Stream {} added", id),
                Ok(Err(e)) => error!("Failed to add stream {}: {}", id, e),
                Err(e) => error!("Task panic for stream {}: {}", id, e),
            }
        }
        
        // Verify streams
        let streams = fixture.stream_manager.list_streams().await;
        info!("Active streams after burst {}: {}", burst, streams.len());
        
        // Wait before next burst
        if burst < burst_count - 1 {
            tokio::time::sleep(delay_between_bursts).await;
        }
    }
    
    // Clean up all streams
    let streams = fixture.stream_manager.list_streams().await;
    for stream in streams {
        fixture.stream_manager.remove_stream(&stream.id).await.ok();
    }
    
    info!("Burst traffic test completed");
    fixture.cleanup().await;
}

/// Test resource usage under load
#[tokio::test]
async fn test_resource_monitoring() {
    super::init_test_environment();
    let fixture = TestFixture::new().await;
    
    info!("Starting resource monitoring test");
    
    let mut metrics = MetricsCollector::new();
    
    // Baseline measurement
    metrics.collect_sample(&fixture.stream_manager).await;
    let baseline = metrics.get_summary();
    
    // Add streams progressively and monitor resources
    let max_streams = 5;
    for i in 0..max_streams {
        let config = TestStreamGenerator::new()
            .with_resolution(1920, 1080)
            .with_fps(30)
            .with_pattern("smpte");
        
        let stream_config = StreamConfig {
            id: format!("resource-{}", i),
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
        
        fixture.stream_manager.add_stream(stream_config).await.unwrap();
        
        // Let stream stabilize
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Collect metrics
        for _ in 0..3 {
            metrics.collect_sample(&fixture.stream_manager).await;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
    
    // Get final metrics
    let summary = metrics.get_summary();
    
    info!("Resource monitoring results:");
    info!("  Baseline CPU: {:.1}%", baseline.avg_cpu_percent);
    info!("  Under load CPU: {:.1}% (avg), {:.1}% (max)", summary.avg_cpu_percent, summary.max_cpu_percent);
    info!("  Baseline Memory: {:.1}MB", baseline.avg_memory_mb);
    info!("  Under load Memory: {:.1}MB (avg), {:.1}MB (max)", summary.avg_memory_mb, summary.max_memory_mb);
    
    // Verify resource usage is reasonable
    assert!(summary.max_cpu_percent < 100.0, "CPU usage too high");
    assert!(summary.max_memory_mb < 2000.0, "Memory usage too high");
    
    // Clean up
    for i in 0..max_streams {
        fixture.stream_manager.remove_stream(&format!("resource-{}", i)).await.ok();
    }
    
    info!("Resource monitoring test completed");
    fixture.cleanup().await;
}

/// Load test helper for parallel operations
async fn parallel_load_test<F, Fut>(
    name: &str,
    parallelism: usize,
    iterations: usize,
    operation: F,
) -> LoadTestResult
where
    F: Fn(usize) -> Fut + Send + Sync + 'static + Clone,
    Fut: std::future::Future<Output = Result<(), String>> + Send,
{
    info!("Starting parallel load test: {}", name);
    
    let start = Instant::now();
    let success_count = Arc::new(AtomicUsize::new(0));
    let failure_count = Arc::new(AtomicUsize::new(0));
    
    let mut handles = vec![];
    
    for i in 0..parallelism {
        let op = operation.clone();
        let success = success_count.clone();
        let failure = failure_count.clone();
        
        let handle = tokio::spawn(async move {
            for j in 0..iterations {
                let id = i * iterations + j;
                match op(id).await {
                    Ok(_) => success.fetch_add(1, Ordering::Relaxed),
                    Err(e) => {
                        error!("Operation {} failed: {}", id, e);
                        failure.fetch_add(1, Ordering::Relaxed)
                    }
                };
            }
        });
        
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.ok();
    }
    
    let duration = start.elapsed();
    let total = parallelism * iterations;
    let success = success_count.load(Ordering::Relaxed);
    let failure = failure_count.load(Ordering::Relaxed);
    
    LoadTestResult {
        name: name.to_string(),
        duration,
        total_operations: total,
        successful: success,
        failed: failure,
        ops_per_second: success as f64 / duration.as_secs_f64(),
    }
}

#[derive(Debug)]
struct LoadTestResult {
    name: String,
    duration: Duration,
    total_operations: usize,
    successful: usize,
    failed: usize,
    ops_per_second: f64,
}

impl LoadTestResult {
    fn print_summary(&self) {
        info!("Load test '{}' completed:", self.name);
        info!("  Duration: {:?}", self.duration);
        info!("  Total operations: {}", self.total_operations);
        info!("  Successful: {}", self.successful);
        info!("  Failed: {}", self.failed);
        info!("  Success rate: {:.1}%", (self.successful as f64 / self.total_operations as f64) * 100.0);
        info!("  Throughput: {:.2} ops/sec", self.ops_per_second);
    }
}