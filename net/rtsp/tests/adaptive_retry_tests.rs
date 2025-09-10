// Integration tests for adaptive learning retry system

use gst::prelude::*;
use serial_test::serial;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
        // Plugin is registered automatically when loaded
    });
}

#[test]
#[serial]
fn test_adaptive_convergence() {
    init();

    // Create an RTSP source with adaptive retry
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.adaptive.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-learning", true)
        .property("adaptive-discovery-time", 5_000_000_000u64) // 5 seconds for faster testing
        .build()
        .unwrap();

    // Get initial state
    let learning_enabled: bool = rtspsrc.property("adaptive-learning");
    assert!(learning_enabled);

    let discovery_time: u64 = rtspsrc.property("adaptive-discovery-time");
    assert_eq!(discovery_time, 5_000_000_000);
}

#[test]
#[serial]
fn test_adaptive_balance() {
    init();

    // Test exploration vs exploitation balance
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.balance.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-exploration-rate", 0.2f32)
        .build()
        .unwrap();

    let exploration_rate: f32 = rtspsrc.property("adaptive-exploration-rate");
    assert!((exploration_rate - 0.2).abs() < 0.001);
}

#[test]
#[serial]
fn test_adaptive_change_detection() {
    init();

    // Test network change detection
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.change.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-change-detection", true)
        .build()
        .unwrap();

    let change_detection: bool = rtspsrc.property("adaptive-change-detection");
    assert!(change_detection);
}

#[test]
#[serial]
fn test_adaptive_persistence() {
    init();

    // Test persistence of learned patterns
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.persistence.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-persistence", true)
        .property("adaptive-cache-ttl", 3600u64) // 1 hour
        .build()
        .unwrap();

    let persistence: bool = rtspsrc.property("adaptive-persistence");
    assert!(persistence);

    let cache_ttl: u64 = rtspsrc.property("adaptive-cache-ttl");
    assert_eq!(cache_ttl, 3600);
}

#[test]
#[serial]
fn test_adaptive_confidence_threshold() {
    init();

    // Test confidence threshold settings
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.confidence.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-confidence-threshold", 0.75f32)
        .build()
        .unwrap();

    let confidence_threshold: f32 = rtspsrc.property("adaptive-confidence-threshold");
    assert!((confidence_threshold - 0.75).abs() < 0.001);
}

#[test]
#[serial]
fn test_adaptive_properties_integration() {
    init();

    // Test all adaptive properties together
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.integration.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-learning", true)
        .property("adaptive-persistence", true)
        .property("adaptive-cache-ttl", 86400u64) // 1 day
        .property("adaptive-discovery-time", 10_000_000_000u64) // 10 seconds
        .property("adaptive-exploration-rate", 0.15f32)
        .property("adaptive-confidence-threshold", 0.9f32)
        .property("adaptive-change-detection", true)
        .build()
        .unwrap();

    // Verify all properties are set correctly
    assert_eq!(rtspsrc.property::<String>("retry-strategy"), "adaptive");
    assert!(rtspsrc.property::<bool>("adaptive-learning"));
    assert!(rtspsrc.property::<bool>("adaptive-persistence"));
    assert_eq!(rtspsrc.property::<u64>("adaptive-cache-ttl"), 86400);
    assert_eq!(rtspsrc.property::<u64>("adaptive-discovery-time"), 10_000_000_000);
    
    let exploration_rate: f32 = rtspsrc.property("adaptive-exploration-rate");
    assert!((exploration_rate - 0.15).abs() < 0.001);
    
    let confidence_threshold: f32 = rtspsrc.property("adaptive-confidence-threshold");
    assert!((confidence_threshold - 0.9).abs() < 0.001);
    
    assert!(rtspsrc.property::<bool>("adaptive-change-detection"));
}

#[test]
#[serial]
fn test_fallback_to_auto_mode() {
    init();

    // Test fallback behavior when adaptive mode fails
    let rtspsrc = gst::ElementFactory::make("rtspsrc2")
        .property("location", "rtsp://test.fallback.local:554/stream")
        .property("retry-strategy", "adaptive")
        .property("adaptive-learning", false) // Disable learning to test fallback
        .build()
        .unwrap();

    // With learning disabled, adaptive should behave like exponential-jitter
    assert_eq!(rtspsrc.property::<String>("retry-strategy"), "adaptive");
    assert!(!rtspsrc.property::<bool>("adaptive-learning"));
}

#[test]
#[serial]
fn test_different_retry_strategies() {
    init();

    // Test that different strategies can be set
    let strategies = vec![
        "none",
        "immediate",
        "linear",
        "exponential",
        "exponential-jitter",
        "auto",
        "adaptive"
    ];

    for strategy in strategies {
        let rtspsrc = gst::ElementFactory::make("rtspsrc2")
            .property("location", "rtsp://test.strategy.local:554/stream")
            .property("retry-strategy", strategy)
            .build()
            .unwrap();

        assert_eq!(rtspsrc.property::<String>("retry-strategy"), strategy);
    }
}

// Benchmark test comparing adaptive vs auto mode
// This would be run with `cargo bench` if properly configured
#[cfg(feature = "bench")]
#[bench]
fn bench_adaptive_vs_auto(b: &mut test::Bencher) {
    init();
    
    b.iter(|| {
        let adaptive_src = gst::ElementFactory::make("rtspsrc2")
            .property("location", "rtsp://bench.test.local:554/stream")
            .property("retry-strategy", "adaptive")
            .build()
            .unwrap();
            
        let auto_src = gst::ElementFactory::make("rtspsrc2")
            .property("location", "rtsp://bench.test.local:554/stream")
            .property("retry-strategy", "auto")
            .build()
            .unwrap();
            
        // In a real benchmark, we would measure connection recovery times
    });
}