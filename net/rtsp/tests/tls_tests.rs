// TLS transport tests for RTSP source

#[cfg(test)]
mod tests {
    use gst::prelude::*;
    use gst_plugin_rtsp::rtspsrc::tls::{
        get_default_port, is_tls_url, TlsConfig, DEFAULT_RTSP_PORT, DEFAULT_RTSPS_PORT,
    };
    use url::Url;

    fn init() {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            gst::init().unwrap();
            gstrsrtsp::plugin_register_static().expect("rtsp plugin register failed");
        });
    }

    #[test]
    fn test_rtsps_url_detection() {
        let rtsp_url = Url::parse("rtsp://example.com/stream").unwrap();
        let rtsps_url = Url::parse("rtsps://example.com/stream").unwrap();

        assert!(!is_tls_url(&rtsp_url));
        assert!(is_tls_url(&rtsps_url));
    }

    #[test]
    fn test_default_port_for_scheme() {
        let rtsp_url = Url::parse("rtsp://example.com/stream").unwrap();
        let rtsps_url = Url::parse("rtsps://example.com/stream").unwrap();

        assert_eq!(get_default_port(&rtsp_url), DEFAULT_RTSP_PORT);
        assert_eq!(get_default_port(&rtsps_url), DEFAULT_RTSPS_PORT);

        // Verify actual port values
        assert_eq!(DEFAULT_RTSP_PORT, 554);
        assert_eq!(DEFAULT_RTSPS_PORT, 322);
    }

    #[test]
    fn test_rtsps_url_with_explicit_port() {
        let url = Url::parse("rtsps://example.com:8554/stream").unwrap();
        
        assert!(is_tls_url(&url));
        assert_eq!(url.port(), Some(8554));
    }

    #[test]
    fn test_element_with_rtsps_location() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2");

        // Set RTSPS location
        let location = "rtsps://secure.example.com/stream";
        element.set_property("location", location);

        let retrieved_location: Option<String> = element.property("location");
        assert_eq!(retrieved_location, Some(location.to_string()));
    }

    #[test]
    fn test_tls_validation_flags() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2");

        // Get default TLS validation flags
        let default_flags: gst_net::gio::TlsCertificateFlags = 
            element.property("tls-validation-flags");
        
        // Set to allow untrusted certificates
        element.set_property(
            "tls-validation-flags",
            gst_net::gio::TlsCertificateFlags::empty(),
        );

        let new_flags: gst_net::gio::TlsCertificateFlags = 
            element.property("tls-validation-flags");
        assert_eq!(new_flags, gst_net::gio::TlsCertificateFlags::empty());
    }

    #[test]
    fn test_tls_config_defaults() {
        let config = TlsConfig::default();
        
        assert!(!config.enabled);
        assert!(!config.accept_invalid_certs);
        assert!(!config.accept_invalid_hostnames);
        assert_eq!(
            config.min_version,
            Some(tokio_native_tls::native_tls::Protocol::Tlsv12)
        );
        assert_eq!(config.max_version, None);
    }

    #[test]
    fn test_rtsps_with_credentials() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2");

        // Set RTSPS location with credentials
        let location = "rtsps://user:pass@secure.example.com:322/stream";
        element.set_property("location", location);

        // Should parse both TLS requirement and credentials
        let user_id: Option<String> = element.property("user-id");
        let user_pw: Option<String> = element.property("user-pw");

        assert_eq!(user_id, Some("user".to_string()));
        assert_eq!(user_pw, Some("pass".to_string()));
    }

    #[test]
    fn test_tls_config_with_custom_settings() {
        let mut config = TlsConfig::default();
        
        config.enabled = true;
        config.accept_invalid_certs = true;
        config.accept_invalid_hostnames = true;
        config.min_version = Some(tokio_native_tls::native_tls::Protocol::Tlsv10);
        config.max_version = Some(tokio_native_tls::native_tls::Protocol::Tlsv13);
        
        assert!(config.enabled);
        assert!(config.accept_invalid_certs);
        assert!(config.accept_invalid_hostnames);
        assert_eq!(
            config.min_version,
            Some(tokio_native_tls::native_tls::Protocol::Tlsv10)
        );
        assert_eq!(
            config.max_version,
            Some(tokio_native_tls::native_tls::Protocol::Tlsv13)
        );
    }

    #[test]
    fn test_mixed_plain_and_tls_urls() {
        let plain_urls = vec![
            "rtsp://example.com/stream",
            "rtsp://192.168.1.1:554/live",
            "rtsp://user:pass@camera.local/ch1",
        ];
        
        let tls_urls = vec![
            "rtsps://example.com/stream",
            "rtsps://192.168.1.1:322/live",
            "rtsps://user:pass@camera.local/ch1",
        ];
        
        for url_str in plain_urls {
            let url = Url::parse(url_str).unwrap();
            assert!(!is_tls_url(&url), "{} should not be TLS", url_str);
        }
        
        for url_str in tls_urls {
            let url = Url::parse(url_str).unwrap();
            assert!(is_tls_url(&url), "{} should be TLS", url_str);
        }
    }
}