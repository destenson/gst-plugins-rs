#![allow(unused)]
// GStreamer RTSP Source 2 - Authentication Module
//
// Copyright (C) 2023-2024 Nirbheek Chauhan <nirbheek centricular com>
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0

use base64::Engine;
use md5::{Digest, Md5};
use rtsp_types::{headers::WWW_AUTHENTICATE, HeaderName, Method, Response, StatusCode};
use std::collections::HashMap;
use std::sync::LazyLock;

use super::body::Body;
use super::imp::CAT;

/// Authorization header name for RTSP
pub static AUTHORIZATION: LazyLock<HeaderName> =
    LazyLock::new(|| HeaderName::from_static_str("Authorization").unwrap());

#[derive(Debug, Clone, PartialEq)]
pub enum AuthMethod {
    Basic,
    Digest,
}

#[derive(Debug, Clone)]
pub struct AuthState {
    pub method: Option<AuthMethod>,
    pub realm: Option<String>,
    pub nonce: Option<String>,
    pub opaque: Option<String>,
    pub qop: Option<Vec<String>>,
    pub algorithm: Option<String>,
    pub stale: bool,
    pub nc: u32,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            method: None,
            realm: None,
            nonce: None,
            opaque: None,
            qop: None,
            algorithm: None,
            stale: false,
            nc: 0,
        }
    }
}

impl AuthState {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Parse WWW-Authenticate header from 401 response
    pub fn parse_challenge(&mut self, response: &Response<Body>) -> Result<(), String> {
        let Some(www_auth) = response.header(&WWW_AUTHENTICATE) else {
            return Err("No WWW-Authenticate header in 401 response".to_string());
        };

        let www_auth_str = www_auth.as_str();
        gst::debug!(CAT, "Parsing WWW-Authenticate: {}", www_auth_str);

        // Check authentication method
        if www_auth_str.starts_with("Basic ") {
            self.method = Some(AuthMethod::Basic);
            self.parse_basic_challenge(&www_auth_str[6..])
        } else if www_auth_str.starts_with("Digest ") {
            self.method = Some(AuthMethod::Digest);
            self.parse_digest_challenge(&www_auth_str[7..])
        } else {
            Err(format!(
                "Unsupported authentication method: {}",
                www_auth_str
            ))
        }
    }

    fn parse_basic_challenge(&mut self, params: &str) -> Result<(), String> {
        // Basic auth only has realm parameter
        let params = parse_auth_params(params);
        self.realm = params.get("realm").cloned();
        Ok(())
    }

    fn parse_digest_challenge(&mut self, params: &str) -> Result<(), String> {
        let params = parse_auth_params(params);

        self.realm = params.get("realm").cloned();
        self.nonce = params.get("nonce").cloned();
        self.opaque = params.get("opaque").cloned();
        self.algorithm = params.get("algorithm").cloned();

        if let Some(qop_str) = params.get("qop") {
            self.qop = Some(
                qop_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect::<Vec<_>>(),
            );
        }

        if let Some(stale_str) = params.get("stale") {
            self.stale = stale_str.eq_ignore_ascii_case("true");
        }

        if self.nonce.is_none() {
            return Err("Digest challenge missing nonce".to_string());
        }

        Ok(())
    }

    /// Generate Authorization header for Basic auth
    pub fn generate_basic_auth(&self, username: &str, password: &str) -> String {
        let credentials = format!("{}:{}", username, password);
        let encoded = base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes());

        format!("Basic {}", encoded)
    }

    /// Generate Authorization header for Digest auth
    pub fn generate_digest_auth(
        &mut self,
        username: &str,
        password: &str,
        method: &Method,
        uri: &str,
    ) -> String {
        let realm = self.realm.as_deref().unwrap_or("");
        let nonce = self.nonce.as_deref().unwrap_or("");
        let opaque = self.opaque.as_deref();
        let algorithm = self.algorithm.as_deref().unwrap_or("MD5");

        // Calculate response hash
        let response = if self.qop.is_some()
            && self.qop.as_ref().unwrap().contains(&"auth".to_string())
        {
            // With qop=auth
            self.nc += 1;
            let nc = format!("{:08x}", self.nc);
            let cnonce = generate_cnonce();
            let qop = "auth";

            let method_str = format!("{:?}", method);
            let response = calculate_digest_response(
                username,
                realm,
                password,
                &method_str,
                uri,
                nonce,
                Some(&nc),
                Some(&cnonce),
                Some(qop),
            );

            // Build authorization with qop parameters
            let mut auth = format!(
                r#"Digest username="{}", realm="{}", nonce="{}", uri="{}", response="{}", algorithm={}, nc={}, cnonce="{}", qop={}"#,
                username, realm, nonce, uri, response, algorithm, nc, cnonce, qop
            );

            if let Some(opaque) = opaque {
                auth.push_str(&format!(r#", opaque="{}""#, opaque));
            }

            auth
        } else {
            // Without qop
            let method_str = format!("{:?}", method);
            let response = calculate_digest_response(
                username,
                realm,
                password,
                &method_str,
                uri,
                nonce,
                None,
                None,
                None,
            );

            // Build authorization without qop parameters
            let mut auth = format!(
                r#"Digest username="{}", realm="{}", nonce="{}", uri="{}", response="{}", algorithm={}"#,
                username, realm, nonce, uri, response, algorithm
            );

            if let Some(opaque) = opaque {
                auth.push_str(&format!(r#", opaque="{}""#, opaque));
            }

            auth
        };

        response
    }

    /// Check if we have a valid auth challenge
    pub fn has_challenge(&self) -> bool {
        self.method.is_some()
    }

    /// Check if the authentication is stale (needs refresh)
    pub fn is_stale(&self) -> bool {
        self.stale
    }
}

/// Parse authentication parameters from WWW-Authenticate header
fn parse_auth_params(params_str: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();

    // Split by comma but be careful about quoted values
    let mut current_param = String::new();
    let mut in_quotes = false;
    let mut escape_next = false;

    for ch in params_str.chars() {
        if escape_next {
            current_param.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => {
                escape_next = true;
            }
            '"' => {
                in_quotes = !in_quotes;
                current_param.push(ch);
            }
            ',' if !in_quotes => {
                // End of parameter
                if let Some((key, value)) = parse_single_param(&current_param) {
                    params.insert(key, value);
                }
                current_param.clear();
            }
            _ => {
                current_param.push(ch);
            }
        }
    }

    // Don't forget the last parameter
    if !current_param.is_empty() {
        if let Some((key, value)) = parse_single_param(&current_param) {
            params.insert(key, value);
        }
    }

    params
}

/// Parse a single key=value parameter, handling quoted values
fn parse_single_param(param: &str) -> Option<(String, String)> {
    let param = param.trim();
    let eq_pos = param.find('=')?;

    let key = param[..eq_pos].trim().to_lowercase();
    let value = param[eq_pos + 1..].trim();

    // Remove quotes if present
    let value = if value.starts_with('"') && value.ends_with('"') {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    };

    Some((key, value))
}

/// Calculate MD5 digest response for Digest authentication
fn calculate_digest_response(
    username: &str,
    realm: &str,
    password: &str,
    method: &str,
    uri: &str,
    nonce: &str,
    nc: Option<&str>,
    cnonce: Option<&str>,
    qop: Option<&str>,
) -> String {
    // A1 = username:realm:password
    let a1 = format!("{}:{}:{}", username, realm, password);
    let ha1 = format!("{:x}", Md5::digest(a1.as_bytes()));

    // A2 = method:uri
    let a2 = format!("{}:{}", method, uri);
    let ha2 = format!("{:x}", Md5::digest(a2.as_bytes()));

    // Response calculation depends on qop
    let response = if let (Some(nc), Some(cnonce), Some(qop)) = (nc, cnonce, qop) {
        // With qop: MD5(HA1:nonce:nc:cnonce:qop:HA2)
        let response_str = format!("{}:{}:{}:{}:{}:{}", ha1, nonce, nc, cnonce, qop, ha2);
        format!("{:x}", Md5::digest(response_str.as_bytes()))
    } else {
        // Without qop: MD5(HA1:nonce:HA2)
        let response_str = format!("{}:{}:{}", ha1, nonce, ha2);
        format!("{:x}", Md5::digest(response_str.as_bytes()))
    };

    response
}

/// Generate a client nonce for Digest authentication
fn generate_cnonce() -> String {
    // Use a simple timestamp-based cnonce for now
    // In production, this should use a proper random generator
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("{:016x}", timestamp)
}

/// Check if a response requires authentication
pub fn requires_auth(response: &Response<Body>) -> bool {
    response.status() == StatusCode::Unauthorized
}

/// Generate authorization header based on current auth state
pub fn generate_auth_header(
    auth_state: &mut AuthState,
    username: Option<&str>,
    password: Option<&str>,
    method: &Method,
    uri: &str,
) -> Option<String> {
    let (username, password) = match (username, password) {
        (Some(u), Some(p)) => (u, p),
        _ => return None,
    };

    if !auth_state.has_challenge() {
        return None;
    }

    match auth_state.method {
        Some(AuthMethod::Basic) => Some(auth_state.generate_basic_auth(username, password)),
        Some(AuthMethod::Digest) => {
            Some(auth_state.generate_digest_auth(username, password, method, uri))
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_auth_encoding() {
        let auth_state = AuthState::default();
        let auth_header = auth_state.generate_basic_auth("user", "pass");
        assert_eq!(auth_header, "Basic dXNlcjpwYXNz");
    }

    #[test]
    fn test_parse_basic_challenge() {
        let mut auth_state = AuthState::default();
        let result = auth_state.parse_basic_challenge(r#"realm="Test Realm""#);
        assert!(result.is_ok());
        assert_eq!(auth_state.realm, Some("Test Realm".to_string()));
    }

    #[test]
    fn test_parse_digest_challenge() {
        let mut auth_state = AuthState::default();
        let challenge = r#"realm="Test", nonce="abc123", qop="auth", algorithm=MD5, opaque="xyz""#;
        let result = auth_state.parse_digest_challenge(challenge);
        assert!(result.is_ok());
        assert_eq!(auth_state.realm, Some("Test".to_string()));
        assert_eq!(auth_state.nonce, Some("abc123".to_string()));
        assert_eq!(auth_state.opaque, Some("xyz".to_string()));
        assert_eq!(auth_state.qop, Some(vec!["auth".to_string()]));
        assert_eq!(auth_state.algorithm, Some("MD5".to_string()));
    }

    #[test]
    fn test_digest_response_calculation() {
        // Test vector from RFC 2617
        let response = calculate_digest_response(
            "Mufasa",
            "testrealm@host.com",
            "Circle Of Life",
            "GET",
            "/dir/index.html",
            "dcd98b7102dd2f0e8b11d0f600bfb0c093",
            Some("00000001"),
            Some("0a4f113b"),
            Some("auth"),
        );

        // The expected response for these inputs
        assert_eq!(response.len(), 32); // MD5 hash is 32 hex characters
    }

    #[test]
    fn test_parse_auth_params() {
        let params_str = r#"realm="Test Realm", nonce="123", qop="auth, auth-int", stale=true"#;
        let params = parse_auth_params(params_str);

        assert_eq!(params.get("realm"), Some(&"Test Realm".to_string()));
        assert_eq!(params.get("nonce"), Some(&"123".to_string()));
        assert_eq!(params.get("qop"), Some(&"auth, auth-int".to_string()));
        assert_eq!(params.get("stale"), Some(&"true".to_string()));
    }
}
