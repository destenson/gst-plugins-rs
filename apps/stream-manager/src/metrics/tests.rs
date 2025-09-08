#[cfg(test)]
mod integration_tests {
    use crate::{Config, manager::StreamManager, api::AppState};
    use crate::metrics::{MetricsCollector, MetricsRegistry, StreamMetricsCollector};
    use actix_web::{test, App, web};
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};
    
    fn init_gst() {
        gst::init().ok();
    }
    
    #[tokio::test]
    async fn test_complete_metrics_pipeline() {
        init_gst();
        
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let collector = MetricsCollector::new(
            stream_manager.clone(), 
            Some(&config.monitoring.metrics)
        ).unwrap();
        
        // Start metrics collection
        collector.start_collection().await;
        
        // Wait a bit for metrics to be collected
        sleep(Duration::from_millis(100)).await;
        
        // Export metrics and verify basic structure
        let prometheus_output = collector.export_prometheus().await.unwrap();
        
        // Verify essential metrics are present
        assert!(prometheus_output.contains("stream_manager_uptime_seconds"));
        assert!(prometheus_output.contains("stream_manager_streams_total"));
        assert!(prometheus_output.contains("stream_manager_streams_active"));
        
        // Verify Prometheus format
        assert!(prometheus_output.contains("# HELP"));
        assert!(prometheus_output.contains("# TYPE"));
    }
    
    #[actix_web::test]
    async fn test_metrics_api_endpoint() {
        init_gst();
        
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let app_state = AppState::new(stream_manager, config);
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .service(
                    web::scope("/api/v1")
                        .route("/metrics", web::get().to(crate::api::routes::get_metrics))
                )
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/api/v1/metrics")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        
        assert!(json.get("total_streams").is_some());
        assert!(json.get("active_streams").is_some());
        assert!(json.get("timestamp").is_some());
    }
    
    #[actix_web::test]
    async fn test_prometheus_endpoint() {
        init_gst();
        
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let app_state = AppState::new(stream_manager, config);
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .service(
                    web::scope("/api/v1")
                        .route("/metrics/prometheus", web::get().to(crate::api::routes::get_prometheus_metrics))
                )
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/api/v1/metrics/prometheus")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        let body = test::read_body(resp).await;
        let prometheus_text = String::from_utf8(body.to_vec()).unwrap();
        
        // Verify it's valid Prometheus format
        assert!(prometheus_text.contains("# HELP"));
        assert!(prometheus_text.contains("# TYPE"));
        assert!(prometheus_text.contains("stream_manager_"));
    }
    
    #[tokio::test]
    async fn test_stream_metrics_collection() {
        init_gst();
        
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let collector = MetricsCollector::new(
            stream_manager.clone(),
            Some(&config.monitoring.metrics)
        ).unwrap();
        
        // Record some stream events
        collector.record_api_request("GET", "/api/v1/streams", 200, Duration::from_millis(25));
        collector.record_api_request("POST", "/api/v1/streams", 201, Duration::from_millis(150));
        collector.record_api_request("GET", "/api/v1/health", 200, Duration::from_millis(5));
        
        // Record pipeline events
        collector.record_pipeline_state_change("stream1", "NULL", "READY");
        collector.record_pipeline_state_change("stream1", "READY", "PLAYING");
        collector.record_pipeline_message("stream1", "info");
        collector.update_pipeline_element_count("stream1", 5);
        
        // Record recording events
        collector.record_recording_segment("stream1", "mp4");
        collector.record_recording_bytes("stream1", "/recordings", 1024 * 1024);
        collector.update_recording_duration("stream1", 30.5);
        
        // Export and verify metrics
        let metrics = collector.export_prometheus().await.unwrap();
        
        // Verify API metrics
        assert!(metrics.contains("stream_manager_api_requests_total"));
        assert!(metrics.contains("stream_manager_api_request_duration_seconds"));
        
        // Verify pipeline metrics
        assert!(metrics.contains("stream_manager_pipeline_state_changes_total"));
        assert!(metrics.contains("stream_manager_pipeline_bus_messages_total"));
        assert!(metrics.contains("stream_manager_pipeline_elements"));
        
        // Verify recording metrics
        assert!(metrics.contains("stream_manager_recording_segments_total"));
        assert!(metrics.contains("stream_manager_recording_bytes_written_total"));
        assert!(metrics.contains("stream_manager_recording_duration_seconds"));
    }
    
    
    #[tokio::test]
    async fn test_stream_metrics_aggregation() {
        let registry = Arc::new(MetricsRegistry::new().unwrap());
        let collector = crate::metrics::StreamMetricsCollector::new(registry);
        
        // Add multiple streams with metrics
        let mut metrics1 = crate::metrics::stream_metrics::StreamMetrics::default();
        metrics1.stream_id = "stream1".to_string();
        metrics1.source_type = "rtsp".to_string();
        metrics1.frames_processed = 1000;
        metrics1.bytes_processed = 1024 * 1024;
        metrics1.current_fps = 30.0;
        metrics1.current_bitrate = 2_000_000.0;
        
        let mut metrics2 = crate::metrics::stream_metrics::StreamMetrics::default();
        metrics2.stream_id = "stream2".to_string();
        metrics2.source_type = "rtsp".to_string();
        metrics2.frames_processed = 2000;
        metrics2.bytes_processed = 2 * 1024 * 1024;
        metrics2.current_fps = 25.0;
        metrics2.current_bitrate = 1_500_000.0;
        
        collector.update_stream_metrics("stream1", metrics1);
        collector.update_stream_metrics("stream2", metrics2);
        
        // Test aggregation
        let aggregate = collector.calculate_aggregate_metrics();
        assert_eq!(aggregate.total_streams, 2);
        assert_eq!(aggregate.active_streams, 2);
        assert_eq!(aggregate.total_frames_processed, 3000);
        assert_eq!(aggregate.total_bytes_processed, 3 * 1024 * 1024);
        assert_eq!(aggregate.average_bitrate, 1_750_000.0);
    }
    
    #[tokio::test]
    async fn test_metrics_configuration() {
        init_gst();
        
        let mut config = Config::default();
        config.monitoring.metrics.collection_interval_seconds = 2;
        config.monitoring.metrics.system_metrics_interval_seconds = 5;
        config.monitoring.metrics.prometheus_enabled = true;
        config.monitoring.metrics.include_stream_metrics = true;
        config.monitoring.metrics.include_system_metrics = true;
        
        let config = Arc::new(config);
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let collector = MetricsCollector::new(
            stream_manager,
            Some(&config.monitoring.metrics)
        ).unwrap();
        
        // Verify configuration was applied (intervals should be from config)
        assert!(collector.export_prometheus().await.is_ok());
    }
    
    #[tokio::test] 
    async fn test_prometheus_format_validation() {
        let registry = MetricsRegistry::new().unwrap();
        
        // Update some basic metrics
        registry.streams_total.set(5.0);
        registry.streams_active.set(3.0);
        
        let output = registry.export_prometheus().unwrap();
        
        // Basic Prometheus format validation
        let lines: Vec<&str> = output.lines().collect();
        let mut help_lines = 0;
        let mut type_lines = 0;
        let mut metric_lines = 0;
        
        for line in lines {
            if line.starts_with("# HELP") {
                help_lines += 1;
            } else if line.starts_with("# TYPE") {
                type_lines += 1;
            } else if line.contains("stream_manager_") && !line.starts_with("#") {
                metric_lines += 1;
            }
        }
        
        assert!(help_lines > 0, "Should have HELP comments");
        assert!(type_lines > 0, "Should have TYPE comments");
        assert!(metric_lines > 0, "Should have metric values");
        
        // Verify specific metrics are formatted correctly
        assert!(output.contains("stream_manager_streams_total 5"));
        assert!(output.contains("stream_manager_streams_active 3"));
    }
}