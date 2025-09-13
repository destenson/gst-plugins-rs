// Authentication tests for RTSP source

#[cfg(test)]
mod tests {
    use gst::prelude::*;
    use gst_plugin_rtsp::rtspsrc::auth::{generate_auth_header, AuthMethod, AuthState};
    use rtsp_types::Method;

    fn init() {
        use std::sync::Once;
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            gst::init().unwrap();
            gstrsrtsp::plugin_register_static().expect("rtsp plugin register failed");
        });
    }

    #[test]
    fn test_element_with_user_credentials() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2");

        // Set username and password
        element.set_property("user-id", "testuser");
        element.set_property("user-pw", "testpass");

        // Get properties back
        let user_id: Option<String> = element.property("user-id");
        let user_pw: Option<String> = element.property("user-pw");

        assert_eq!(user_id, Some("testuser".to_string()));
        assert_eq!(user_pw, Some("testpass".to_string()));
    }

    #[test]
    fn test_location_with_credentials() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2");

        // Set location with credentials
        let location = "rtsp://user:pass@example.com:554/stream";
        element.set_property("location", location);

        // Credentials should be parsed from URL
        let user_id: Option<String> = element.property("user-id");
        let user_pw: Option<String> = element.property("user-pw");

        assert_eq!(user_id, Some("user".to_string()));
        assert_eq!(user_pw, Some("pass".to_string()));
    }

    #[test]
    fn test_basic_auth_generation() {
        let auth_state = AuthState::default();
        let auth_header = auth_state.generate_basic_auth("user", "pass");

        // Basic auth should be base64 encoded "user:pass"
        assert_eq!(auth_header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_digest_auth_generation() {
        let mut auth_state = AuthState {
            method: Some(AuthMethod::Digest),
            realm: Some("test".to_string()),
            nonce: Some("abc123".to_string()),
            opaque: Some("xyz".to_string()),
            qop: Some(vec!["auth".to_string()]),
            algorithm: Some("MD5".to_string()),
            stale: false,
            nc: 0,
        };

        let auth_header =
            auth_state.generate_digest_auth("user", "pass", &Method::Describe, "/stream");

        // Should generate a valid Digest auth header
        assert!(auth_header.starts_with("Digest "));
        assert!(auth_header.contains("username=\"user\""));
        assert!(auth_header.contains("realm=\"test\""));
        assert!(auth_header.contains("nonce=\"abc123\""));
        assert!(auth_header.contains("uri=\"/stream\""));
        assert!(auth_header.contains("response="));
    }

    #[test]
    fn test_auth_state_reset() {
        let mut auth_state = AuthState {
            method: Some(AuthMethod::Basic),
            realm: Some("test".to_string()),
            nonce: Some("abc".to_string()),
            opaque: None,
            qop: None,
            algorithm: None,
            stale: false,
            nc: 5,
        };

        auth_state.reset();

        assert_eq!(auth_state.method, None);
        assert_eq!(auth_state.realm, None);
        assert_eq!(auth_state.nonce, None);
        assert_eq!(auth_state.nc, 0);
    }

    #[test]
    fn test_auth_with_special_characters() {
        let auth_state = AuthState::default();

        // Test with special characters in password
        let auth_header = auth_state.generate_basic_auth("user", "p@ss!word#123");

        // Should properly encode special characters
        assert!(auth_header.starts_with("Basic "));

        // Decode and verify
        let encoded = auth_header.strip_prefix("Basic ").unwrap();
        let decoded_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded).unwrap();
        let decoded = String::from_utf8(decoded_bytes).unwrap();
        assert_eq!(decoded, "user:p@ss!word#123");
    }

    #[test]
    fn test_priority_of_property_over_url_credentials() {
        init();

        let element = gst::ElementFactory::make("rtspsrc2")
            .build()
            .expect("Failed to create rtspsrc2");

        // First set via URL
        element.set_property("location", "rtsp://urluser:urlpass@example.com/stream");

        // Then override via properties
        element.set_property("user-id", "propuser");
        element.set_property("user-pw", "proppass");

        // Properties should take precedence
        let user_id: Option<String> = element.property("user-id");
        let user_pw: Option<String> = element.property("user-pw");

        assert_eq!(user_id, Some("propuser".to_string()));
        assert_eq!(user_pw, Some("proppass".to_string()));
    }
}
