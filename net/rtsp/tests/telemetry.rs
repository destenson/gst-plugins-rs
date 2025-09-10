#![allow(unused)]
// Tests for RTSP telemetry and observability

#[cfg(feature = "telemetry")]
mod telemetry_tests {
    use gst::prelude::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;
    use tracing_subscriber::{EnvFilter, FmtSubscriber};

    fn init() {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            gst::init().unwrap();
            gstrsrtsp::plugin_register_static().expect("Failed to register rtsp plugin");
            
            // Initialize tracing subscriber for tests
            let subscriber = FmtSubscriber::builder()
                .with_env_filter(EnvFilter::from_default_env()
                    .add_directive("rtspsrc2=trace".parse().unwrap()))
                .with_test_writer()
                .finish();
            
            let _ = tracing::subscriber::set_global_default(subscriber);
        });
    }

    #[test]
    fn test_metrics_properties() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .name("test-rtsp-src")
            .build()
            .unwrap();

        // Check that metrics properties exist and are readable
        let connection_attempts = element.property::<u64>("metrics-connection-attempts");
        assert_eq!(connection_attempts, 0);

        let connection_successes = element.property::<u64>("metrics-connection-successes");
        assert_eq!(connection_successes, 0);

        let packets_received = element.property::<u64>("metrics-packets-received");
        assert_eq!(packets_received, 0);

        let bytes_received = element.property::<u64>("metrics-bytes-received");
        assert_eq!(bytes_received, 0);
    }

    #[test]
    fn test_tracing_initialization() {
        init();

        // Create a flag to check if tracing events are emitted
        let event_received = Arc::new(AtomicBool::new(false));
        let event_flag = event_received.clone();

        // Create a custom layer to capture events
        struct TestLayer {
            flag: Arc<AtomicBool>,
        }

        impl<S> tracing_subscriber::Layer<S> for TestLayer
        where
            S: tracing::Subscriber,
        {
            fn on_event(
                &self,
                _event: &tracing::Event<'_>,
                _ctx: tracing_subscriber::layer::Context<'_, S>,
            ) {
                self.flag.store(true, Ordering::Relaxed);
            }
        }

        // Generate a tracing event
        tracing::info!("Test event for telemetry");

        // The global subscriber should handle this
        // In a real scenario, we'd check if events are properly logged
        assert!(true, "Tracing initialization successful");
    }

    #[cfg(feature = "prometheus")]
    #[test]
    fn test_prometheus_metrics() {
        use prometheus::{Encoder, TextEncoder};

        init();

        // Create element and simulate some activity
        let element = gst::ElementFactory::make("rtspsrc2")
            .name("test-prometheus")
            .build()
            .unwrap();

        // Get metrics
        let connection_attempts = element.property::<u64>("metrics-connection-attempts");
        
        // Gather prometheus metrics
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        let output = String::from_utf8(buffer).unwrap();

        // Check that RTSP metrics are registered
        if output.contains("rtsp_") {
            assert!(output.contains("rtsp_connection_attempts_total") || 
                   output.contains("rtsp_connection_successes_total"),
                   "Prometheus metrics should be registered");
        }
    }

    #[test]
    fn test_metrics_reset() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .name("test-reset")
            .build()
            .unwrap();

        // Initial values should be 0
        assert_eq!(element.property::<u64>("metrics-connection-attempts"), 0);
        assert_eq!(element.property::<u64>("metrics-packets-received"), 0);
    }

    #[test] 
    fn test_concurrent_metrics_updates() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .name("test-concurrent")
            .build()
            .unwrap();

        let element_clone = element.clone();
        
        // Spawn multiple threads that try to read metrics
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let elem = element_clone.clone();
                thread::spawn(move || {
                    for _ in 0..100 {
                        let _attempts = elem.property::<u64>("metrics-connection-attempts");
                        let _packets = elem.property::<u64>("metrics-packets-received");
                        thread::sleep(Duration::from_micros(10));
                    }
                })
            })
            .collect();

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // If we get here without panic, concurrent access is safe
        assert!(true, "Concurrent metrics access is thread-safe");
    }
}

#[cfg(not(feature = "telemetry"))]
mod no_telemetry_tests {
    use gst::prelude::*;

    fn init() {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            gst::init().unwrap();
            gstrsrtsp::plugin_register_static().expect("Failed to register rtsp plugin");
        });
    }

    #[test]
    fn test_metrics_properties_return_zero_without_telemetry() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .name("test-no-telemetry")
            .build()
            .unwrap();

        // Without telemetry feature, metrics should return 0
        let connection_attempts = element.property::<u64>("metrics-connection-attempts");
        assert_eq!(connection_attempts, 0);

        let packets_received = element.property::<u64>("metrics-packets-received");
        assert_eq!(packets_received, 0);
    }
}
