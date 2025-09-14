// version_detection.rs - RTSP version detection and negotiation
//
// This module provides functionality for detecting and negotiating RTSP protocol versions
// in preparation for future RTSP 2.0 support.

use rtsp_types::{Method, Request, Response, StatusCode, Version, HeaderName};
use std::fmt;
use gst::glib;

/// Supported RTSP protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVersion {
    /// RTSP 1.0 (RFC 2326)
    V1_0,
    /// RTSP 2.0 (RFC 7826) - Not yet supported
    V2_0,
}

impl ProtocolVersion {
    /// Convert from rtsp_types::Version
    pub fn from_rtsp_version(version: Version) -> Self {
        match version {
            Version::V1_0 => ProtocolVersion::V1_0,
            Version::V2_0 => ProtocolVersion::V2_0,
            // rtsp_types might add more versions in the future
        }
    }

    /// Convert to rtsp_types::Version
    pub fn to_rtsp_version(self) -> Version {
        match self {
            ProtocolVersion::V1_0 => Version::V1_0,
            ProtocolVersion::V2_0 => Version::V2_0,
        }
    }

    /// Get the version string for protocol messages
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtocolVersion::V1_0 => "RTSP/1.0",
            ProtocolVersion::V2_0 => "RTSP/2.0",
        }
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Version negotiation state machine
#[derive(Debug, Clone)]
pub struct VersionNegotiator {
    /// Client's preferred version
    preferred_version: ProtocolVersion,
    /// Server's detected version
    server_version: Option<ProtocolVersion>,
    /// Negotiated version for the session
    negotiated_version: Option<ProtocolVersion>,
    /// Whether the server supports RTSP 2.0 features
    supports_v2_features: Vec<String>,
}

impl VersionNegotiator {
    /// Create a new version negotiator
    pub fn new(preferred_version: ProtocolVersion) -> Self {
        Self {
            preferred_version,
            server_version: None,
            negotiated_version: None,
            supports_v2_features: Vec::new(),
        }
    }

    /// Detect server version from response
    pub fn detect_server_version(&mut self, response: &Response<Vec<u8>>) -> ProtocolVersion {
        let version = ProtocolVersion::from_rtsp_version(response.version());
        self.server_version = Some(version);
        
        // Check for RTSP 2.0 specific headers
        let require_header = HeaderName::from_static_str("Require").unwrap();
        let proxy_require_header = HeaderName::from_static_str("Proxy-Require").unwrap();
        let supported_header = HeaderName::from_static_str("Supported").unwrap();
        
        if response.header(&require_header).is_some() 
            || response.header(&proxy_require_header).is_some() 
            || response.header(&supported_header).is_some() {
            // Server has RTSP 2.0 feature negotiation headers
            if let Some(supported) = response.header(&supported_header) {
                let supported_str = supported.as_str();
                self.supports_v2_features = supported_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
        }
        
        version
    }

    /// Negotiate version based on client preference and server capability
    pub fn negotiate(&mut self) -> ProtocolVersion {
        let negotiated = match (self.preferred_version, self.server_version) {
            // Both support 2.0
            (ProtocolVersion::V2_0, Some(ProtocolVersion::V2_0)) => ProtocolVersion::V2_0,
            // Client wants 2.0 but server only supports 1.0
            (ProtocolVersion::V2_0, Some(ProtocolVersion::V1_0)) => {
                glib::g_warning!("rtspsrc", "Server doesn't support RTSP 2.0, falling back to 1.0");
                ProtocolVersion::V1_0
            }
            // Server supports 2.0 but client prefers 1.0
            (ProtocolVersion::V1_0, Some(ProtocolVersion::V2_0)) => {
                glib::g_debug!("rtspsrc", "Server supports RTSP 2.0 but client prefers 1.0");
                ProtocolVersion::V1_0
            }
            // Default to 1.0
            _ => ProtocolVersion::V1_0,
        };
        
        self.negotiated_version = Some(negotiated);
        negotiated
    }

    /// Get the negotiated version
    pub fn get_negotiated_version(&self) -> Option<ProtocolVersion> {
        self.negotiated_version
    }

    /// Check if a specific RTSP 2.0 feature is supported
    pub fn supports_feature(&self, feature: &str) -> bool {
        self.supports_v2_features.iter().any(|f| f == feature)
    }

    /// Build request with appropriate version
    pub fn build_request(&self, method: Method, _uri: &str) -> Request<Vec<u8>> {
        let version = self.negotiated_version
            .unwrap_or(self.preferred_version)
            .to_rtsp_version();
        
        Request::builder(method, version)
            .build(Vec::new())
    }

    /// Add RTSP 2.0 feature requirements to request
    pub fn add_v2_features(&self, mut request: Request<Vec<u8>>, features: &[&str]) -> Request<Vec<u8>> {
        if self.negotiated_version == Some(ProtocolVersion::V2_0) && !features.is_empty() {
            let require_header_name = HeaderName::from_static_str("Require").unwrap();
            let require_header_value = features.join(", ");
            request.insert_header(require_header_name, require_header_value);
        }
        request
    }
}

/// Check if a response indicates version mismatch
pub fn check_version_error(response: &Response<Vec<u8>>) -> bool {
    match response.status() {
        // RTSP 2.0 specific status codes for version/feature negotiation failures
        StatusCode::NotImplemented => {
            // 501 - Method not implemented (could be version issue)
            true
        }
        StatusCode::OptionNotSupported => {
            // 551 - Option not supported
            true
        }
        _ => {
            // Check for other version-related errors (505 would be here if StatusCode had it)
            // For now, we'll check the raw status code value
            let status_value: u16 = response.status().into();
            status_value == 505
        }
    }
}

/// Parse version from RTSP version string (e.g., "RTSP/1.0")
pub fn parse_version_string(version_str: &str) -> Option<ProtocolVersion> {
    match version_str {
        "RTSP/1.0" => Some(ProtocolVersion::V1_0),
        "RTSP/2.0" => Some(ProtocolVersion::V2_0),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_detection() {
        let mut negotiator = VersionNegotiator::new(ProtocolVersion::V1_0);
        
        // Test detecting 1.0 server
        let response_v1 = Response::builder(Version::V1_0, StatusCode::Ok)
            .build(Vec::new());
        let detected = negotiator.detect_server_version(&response_v1);
        assert_eq!(detected, ProtocolVersion::V1_0);
    }

    #[test]
    fn test_version_negotiation() {
        // Test 1.0 client with 1.0 server
        let mut negotiator = VersionNegotiator::new(ProtocolVersion::V1_0);
        negotiator.server_version = Some(ProtocolVersion::V1_0);
        assert_eq!(negotiator.negotiate(), ProtocolVersion::V1_0);

        // Test 2.0 client with 1.0 server (fallback)
        let mut negotiator = VersionNegotiator::new(ProtocolVersion::V2_0);
        negotiator.server_version = Some(ProtocolVersion::V1_0);
        assert_eq!(negotiator.negotiate(), ProtocolVersion::V1_0);

        // Test 2.0 client with 2.0 server
        let mut negotiator = VersionNegotiator::new(ProtocolVersion::V2_0);
        negotiator.server_version = Some(ProtocolVersion::V2_0);
        assert_eq!(negotiator.negotiate(), ProtocolVersion::V2_0);
    }

    #[test]
    fn test_v2_feature_detection() {
        let mut negotiator = VersionNegotiator::new(ProtocolVersion::V2_0);
        
        // Test response with Supported header (RTSP 2.0 feature)
        let supported_header = HeaderName::from_static_str("Supported").unwrap();
        let mut response = Response::builder(Version::V2_0, StatusCode::Ok)
            .build(Vec::new());
        response.insert_header(supported_header, "play.basic, play.scale, play.speed");
        
        negotiator.detect_server_version(&response);
        assert!(negotiator.supports_feature("play.scale"));
        assert!(negotiator.supports_feature("play.speed"));
        assert!(!negotiator.supports_feature("nonexistent"));
    }

    #[test]
    fn test_version_error_detection() {
        // Test 505 Version Not Supported
        let response = Response::builder(Version::V1_0, StatusCode::from(505))
            .build(Vec::new());
        assert!(check_version_error(&response));

        // Test 501 Not Implemented
        let response = Response::builder(Version::V1_0, StatusCode::NotImplemented)
            .build(Vec::new());
        assert!(check_version_error(&response));

        // Test 200 OK (not a version error)
        let response = Response::builder(Version::V1_0, StatusCode::Ok)
            .build(Vec::new());
        assert!(!check_version_error(&response));
    }

    #[test]
    fn test_parse_version_string() {
        assert_eq!(parse_version_string("RTSP/1.0"), Some(ProtocolVersion::V1_0));
        assert_eq!(parse_version_string("RTSP/2.0"), Some(ProtocolVersion::V2_0));
        assert_eq!(parse_version_string("RTSP/3.0"), None);
        assert_eq!(parse_version_string("HTTP/1.1"), None);
    }

    #[test]
    fn test_build_request_with_version() {
        let negotiator = VersionNegotiator::new(ProtocolVersion::V2_0);
        let request = negotiator.build_request(Method::Options, "rtsp://example.com/test");
        
        assert_eq!(request.version(), Version::V2_0);
        assert_eq!(request.method(), &Method::Options);
    }

    #[test]
    fn test_add_v2_features_to_request() {
        let mut negotiator = VersionNegotiator::new(ProtocolVersion::V2_0);
        negotiator.negotiated_version = Some(ProtocolVersion::V2_0);
        
        let request = negotiator.build_request(Method::Setup, "rtsp://example.com/test");
        let request_with_features = negotiator.add_v2_features(request, &["play.scale", "play.speed"]);
        
        let require_header = HeaderName::from_static_str("Require").unwrap();
        let header_value = request_with_features.header(&require_header);
        assert!(header_value.is_some());
        let header_str = header_value.unwrap().as_str();
        assert_eq!(header_str, "play.scale, play.speed");
    }
}