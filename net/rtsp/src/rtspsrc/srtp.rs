// SRTP Support Module
//
// Copyright (C) 2024
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at
// <https://mozilla.org/MPL/2.0/>.
//
// SPDX-License-Identifier: MPL-2.0


#[derive(Debug, Clone, Default)]
pub struct SrtpInfo {
    pub use_srtp: bool,
    pub crypto_suite: Option<String>,
    pub key_info: Option<String>,
    pub session_params: Option<String>,
    pub profile: SrtpProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SrtpProfile {
    #[default]
    None,
    Savp,  // RTP/SAVP - Secure AVP
    Savpf, // RTP/SAVPF - Secure AVP with feedback
}

impl SrtpProfile {
    pub fn from_str(s: &str) -> Self {
        match s {
            "RTP/SAVP" => Self::Savp,
            "RTP/SAVPF" => Self::Savpf,
            _ => Self::None,
        }
    }

    pub fn is_secure(&self) -> bool {
        match self {
            Self::Savp | Self::Savpf => true,
            Self::None => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CryptoAttribute {
    pub tag: u32,
    pub crypto_suite: String,
    pub key_params: String,
    pub session_params: Option<String>,
}

impl CryptoAttribute {
    /// Parse a crypto attribute from SDP
    /// Format: a=crypto:<tag> <crypto-suite> <key-params> [<session-params>]
    /// Example: a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:WVNfX19zZW1jdGwgKCkgewkyMjA7fQp9CnVubGVz
    pub fn parse(value: &str) -> Option<Self> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 3 {
            return None;
        }

        let tag = parts[0].parse::<u32>().ok()?;
        let crypto_suite = parts[1].to_string();
        let key_params = parts[2].to_string();
        let session_params = if parts.len() > 3 {
            Some(parts[3..].join(" "))
        } else {
            None
        };

        Some(Self {
            tag,
            crypto_suite,
            key_params,
            session_params,
        })
    }
}

/// Parse crypto attribute and update SRTP info
pub fn parse_crypto_attribute(value: &str, srtp_info: &mut SrtpInfo) -> Result<(), String> {
    let crypto = CryptoAttribute::parse(value)
        .ok_or_else(|| format!("Failed to parse crypto attribute: {}", value))?;

    srtp_info.use_srtp = true;
    srtp_info.crypto_suite = Some(crypto.crypto_suite);
    srtp_info.key_info = Some(crypto.key_params);
    srtp_info.session_params = crypto.session_params;

    Ok(())
}

/// Parse key-mgmt attribute for MIKEY support
/// Format: a=key-mgmt:<key-mgmt-protocol> <key-mgmt-data>
/// Example: a=key-mgmt:mikey AQAFgM...
pub fn parse_key_mgmt_attribute(value: &str, srtp_info: &mut SrtpInfo) -> Result<(), String> {
    let parts: Vec<&str> = value.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid key-mgmt attribute: {}", value));
    }

    let protocol = parts[0];
    let key_data = parts[1];

    if protocol == "mikey" {
        srtp_info.use_srtp = true;
        srtp_info.key_info = Some(format!("mikey:{}", key_data));
    }

    Ok(())
}

/// Check if the media protocol indicates SRTP
pub fn is_srtp_protocol(protocol: &str) -> bool {
    matches!(
        protocol,
        "RTP/SAVP" | "RTP/SAVPF" | "UDP/TLS/RTP/SAVP" | "UDP/TLS/RTP/SAVPF"
    )
}

/// Get the appropriate srtpdec caps for a given crypto suite
pub fn get_srtp_caps(crypto_suite: &str) -> Option<gst::Caps> {
    let srtp_cipher = match crypto_suite {
        "AES_CM_128_HMAC_SHA1_80" => "aes-128-icm",
        "AES_CM_128_HMAC_SHA1_32" => "aes-128-icm",
        "AES_256_CM_HMAC_SHA1_80" => "aes-256-icm",
        "AES_256_CM_HMAC_SHA1_32" => "aes-256-icm",
        "AEAD_AES_128_GCM" => "aes-128-gcm",
        "AEAD_AES_256_GCM" => "aes-256-gcm",
        _ => return None,
    };

    let srtp_auth = match crypto_suite {
        "AES_CM_128_HMAC_SHA1_80" | "AES_256_CM_HMAC_SHA1_80" => "hmac-sha1-80",
        "AES_CM_128_HMAC_SHA1_32" | "AES_256_CM_HMAC_SHA1_32" => "hmac-sha1-32",
        "AEAD_AES_128_GCM" | "AEAD_AES_256_GCM" => "null",
        _ => return None,
    };

    Some(
        gst::Caps::builder("application/x-srtp")
            .field("srtp-cipher", srtp_cipher)
            .field("srtp-auth", srtp_auth)
            .field("srtcp-cipher", srtp_cipher)
            .field("srtcp-auth", srtp_auth)
            .build(),
    )
}
