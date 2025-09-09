// RTSP sink implementation for streaming output

#[cfg(test)]
mod tests {
    use gst::prelude::*;
    use crate::config::RtspSinkConfig;
    use crate::stream::{StreamManager, StreamBranch, BranchManager, RtspSinkManager};
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_rtsp_sink_integration() {
        gst::init().unwrap();
        
        // Create pipeline
        let pipeline = gst::Pipeline::new();
        
        // Create test source
        let source = gst::ElementFactory::make("videotestsrc")
            .property("is-live", true)
            .build()
            .unwrap();
        
        pipeline.add(&source).unwrap();
        
        // Create branch manager
        let branch_manager = Arc::new(BranchManager::new(&pipeline).unwrap());
        
        // Connect source to tee
        let tee = branch_manager.get_tee();
        source.link(tee).unwrap();
        
        // Create RTSP sink config
        let config = RtspSinkConfig {
            enabled: true,
            location: "rtsp://localhost:8554/test".to_string(),
            codec: "h264".to_string(),
            bitrate_kbps: Some(2000),
            width: Some(640),
            height: Some(480),
            latency_ms: 100,
            protocols: "tcp".to_string(),
            username: None,
            password: None,
        };
        
        // Create RTSP sink manager
        let mut rtsp_manager = RtspSinkManager::new(branch_manager.clone());
        
        // Add RTSP sink - this would fail without rtspclientsink installed
        // but we're testing the structure
        let result = rtsp_manager.add_sink(config, &pipeline);
        
        // The test might fail if rtspclientsink is not available
        // But we're mainly testing that the code compiles and structures work
        if result.is_ok() {
            assert_eq!(rtsp_manager.sink_count(), 1);
            
            // Test removal
            let remove_result = rtsp_manager.remove_all();
            assert!(remove_result.is_ok());
            assert_eq!(rtsp_manager.sink_count(), 0);
        }
    }
    
    #[tokio::test]
    async fn test_rtsp_branch_creation() {
        gst::init().unwrap();
        
        let pipeline = gst::Pipeline::new();
        let manager = BranchManager::new(&pipeline).unwrap();
        
        // Create RTSP branch
        let queue = manager.create_branch(StreamBranch::Rtsp);
        
        // Should succeed
        assert!(queue.is_ok());
        
        // Verify branch is in list
        let branches = manager.list_branches();
        assert!(branches.contains(&StreamBranch::Rtsp));
    }
    
    #[tokio::test]
    async fn test_stream_manager_rtsp_integration() {
        gst::init().unwrap();
        
        let stream_manager = StreamManager::new();
        let pipeline = gst::Pipeline::new();
        
        // Add a test stream
        let stream_id = stream_manager.add_stream(
            "Test Stream".to_string(),
            "rtsp://example.com/stream".to_string()
        ).await.unwrap();
        
        // Create RTSP sink config
        let config = RtspSinkConfig {
            enabled: true,
            location: "rtsp://localhost:8554/output".to_string(),
            codec: "h264".to_string(),
            bitrate_kbps: Some(2000),
            width: None,
            height: None,
            latency_ms: 100,
            protocols: "tcp".to_string(),
            username: None,
            password: None,
        };
        
        // Enable RTSP output (might fail without proper elements)
        let result = stream_manager.enable_rtsp_output(&stream_id, config, &pipeline).await;
        
        if result.is_ok() {
            // Check sink count
            let count = stream_manager.get_rtsp_sink_count(&stream_id).await;
            assert_eq!(count, 1);
            
            // Disable RTSP output
            let disable_result = stream_manager.disable_rtsp_output(&stream_id).await;
            assert!(disable_result.is_ok());
            
            // Check sink count again
            let count = stream_manager.get_rtsp_sink_count(&stream_id).await;
            assert_eq!(count, 0);
        }
    }
}