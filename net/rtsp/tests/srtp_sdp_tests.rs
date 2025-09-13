// SRTP SDP Parsing Tests

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        gst::init().unwrap();
    });
}

#[test]
fn test_srtp_sdp_parsing() {
    init();

    // Test SDP with crypto attribute
    let sdp_with_crypto = br#"v=0
o=- 0 0 IN IP4 127.0.0.1
s=Test Stream
c=IN IP4 127.0.0.1
t=0 0
m=video 5004 RTP/SAVP 96
a=rtpmap:96 H264/90000
a=crypto:1 AES_CM_128_HMAC_SHA1_80 inline:WVNfX19zZW1jdGwgKCkgewkyMjA7fQp9CnVubGVz
a=control:track1"#;

    let session = sdp_types::Session::parse(sdp_with_crypto).unwrap();

    // Check that media has RTP/SAVP profile
    assert_eq!(session.medias[0].proto, "RTP/SAVP");

    // Check that crypto attribute is present
    let crypto_attr = session.medias[0]
        .attributes
        .iter()
        .find(|a| a.attribute == "crypto");
    assert!(crypto_attr.is_some());

    let crypto_value = crypto_attr.unwrap().value.as_ref().unwrap();
    assert!(crypto_value.starts_with("1 AES_CM_128_HMAC_SHA1_80"));
}

#[test]
fn test_srtp_crypto_detection() {
    init();

    // Test SDP with different crypto suites
    let sdp_aes256 = br#"v=0
o=- 0 0 IN IP4 127.0.0.1
s=Test Stream
c=IN IP4 127.0.0.1
t=0 0
m=audio 5004 RTP/SAVPF 0
a=rtpmap:0 PCMU/8000
a=crypto:1 AES_256_CM_HMAC_SHA1_80 inline:base64keystring
a=control:track1"#;

    let session = sdp_types::Session::parse(sdp_aes256).unwrap();
    assert_eq!(session.medias[0].proto, "RTP/SAVPF");

    let crypto_attr = session.medias[0]
        .attributes
        .iter()
        .find(|a| a.attribute == "crypto");
    assert!(crypto_attr.is_some());
}

#[test]
fn test_srtp_detect() {
    init();

    // Test various SRTP profiles in SDP
    let profiles = vec![
        ("RTP/SAVP", true),
        ("RTP/SAVPF", true),
        ("UDP/TLS/RTP/SAVP", true),
        ("UDP/TLS/RTP/SAVPF", true),
        ("RTP/AVP", false),
        ("RTP/AVPF", false),
    ];

    for (profile, should_be_srtp) in profiles {
        let sdp = format!(
            r#"v=0
o=- 0 0 IN IP4 127.0.0.1
s=Test
c=IN IP4 127.0.0.1
t=0 0
m=video 5004 {} 96
a=rtpmap:96 H264/90000"#,
            profile
        );

        let session = sdp_types::Session::parse(sdp.as_bytes()).unwrap();
        let is_secure_profile = session.medias[0].proto.contains("SAVP");
        assert_eq!(
            is_secure_profile, should_be_srtp,
            "Failed for profile: {}",
            profile
        );
    }
}

#[test]
fn test_key_mgmt_parsing() {
    init();

    // Test SDP with key-mgmt attribute
    let sdp_with_mikey = br#"v=0
o=- 0 0 IN IP4 127.0.0.1
s=Test Stream
c=IN IP4 127.0.0.1
t=0 0
m=video 5004 RTP/SAVP 96
a=rtpmap:96 H264/90000
a=key-mgmt:mikey AQAFgM0AAAAAAAAAAAAAA...
a=control:track1"#;

    let session = sdp_types::Session::parse(sdp_with_mikey).unwrap();

    // Check that media has RTP/SAVP profile
    assert_eq!(session.medias[0].proto, "RTP/SAVP");

    // Check that key-mgmt attribute is present
    let key_mgmt_attr = session.medias[0]
        .attributes
        .iter()
        .find(|a| a.attribute == "key-mgmt");
    assert!(key_mgmt_attr.is_some());

    let key_mgmt_value = key_mgmt_attr.unwrap().value.as_ref().unwrap();
    assert!(key_mgmt_value.starts_with("mikey"));
}
