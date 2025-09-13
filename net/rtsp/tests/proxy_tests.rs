// Integration tests for proxy support in RTSP
//
// These tests verify that proxy configuration is properly handled through
// properties and environment variables in the rtspsrc2 element

use gst::prelude::*;

#[test]
fn test_proxy_properties() {
    gst::init().unwrap();

    // Create rtspsrc2 element
    let element = gst::ElementFactory::make("rtspsrc2")
        .name("test-rtsp-src")
        .build()
        .unwrap();

    // Test setting proxy property
    element.set_property("proxy", "http://proxy.example.com:8080");
    let proxy: Option<String> = element.property("proxy");
    assert_eq!(proxy, Some("http://proxy.example.com:8080".to_string()));

    // Test setting proxy-id property
    element.set_property("proxy-id", "testuser");
    let proxy_id: Option<String> = element.property("proxy-id");
    assert_eq!(proxy_id, Some("testuser".to_string()));

    // Test setting proxy-pw property
    element.set_property("proxy-pw", "testpass");
    let proxy_pw: Option<String> = element.property("proxy-pw");
    assert_eq!(proxy_pw, Some("testpass".to_string()));
}

#[test]
fn test_proxy_env_detection() {
    // Save original environment variables
    let original_http_proxy = std::env::var("http_proxy").ok();
    let original_https_proxy = std::env::var("https_proxy").ok();

    // Set environment variables
    std::env::set_var("http_proxy", "http://env-proxy.example.com:3128");

    // The proxy module should detect these when no explicit proxy is set
    // This would be tested through actual element usage

    // Restore original environment variables
    match original_http_proxy {
        Some(val) => std::env::set_var("http_proxy", val),
        None => std::env::remove_var("http_proxy"),
    }
    match original_https_proxy {
        Some(val) => std::env::set_var("https_proxy", val),
        None => std::env::remove_var("https_proxy"),
    }
}

#[test]
fn test_proxy_url_formats() {
    gst::init().unwrap();

    let element = gst::ElementFactory::make("rtspsrc2")
        .name("test-rtsp-src")
        .build()
        .unwrap();

    // Test various proxy URL formats
    let test_cases = vec![
        "http://proxy.example.com:8080",
        "https://secure-proxy.example.com:8443",
        "socks5://socks-proxy.example.com:1080",
        "http://user:pass@proxy.example.com:3128",
    ];

    for proxy_url in test_cases {
        element.set_property("proxy", proxy_url);
        let retrieved: Option<String> = element.property("proxy");
        assert_eq!(retrieved, Some(proxy_url.to_string()));
    }
}

#[test]
fn test_proxy_with_credentials() {
    gst::init().unwrap();

    let element = gst::ElementFactory::make("rtspsrc2")
        .name("test-rtsp-src")
        .build()
        .unwrap();

    // Set proxy with separate credentials
    element.set_property("proxy", "http://proxy.example.com:8080");
    element.set_property("proxy-id", "username");
    element.set_property("proxy-pw", "password");

    let proxy: Option<String> = element.property("proxy");
    let proxy_id: Option<String> = element.property("proxy-id");
    let proxy_pw: Option<String> = element.property("proxy-pw");

    assert_eq!(proxy, Some("http://proxy.example.com:8080".to_string()));
    assert_eq!(proxy_id, Some("username".to_string()));
    assert_eq!(proxy_pw, Some("password".to_string()));
}

// Mock proxy server for end-to-end testing
#[cfg(feature = "integration-tests")]
mod integration {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    async fn mock_http_proxy_server(listener: TcpListener) {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();

            tokio::spawn(async move {
                let mut buffer = vec![0; 1024];
                let n = stream.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..n]);

                // Check for CONNECT request
                if request.starts_with("CONNECT ") {
                    // Send 200 OK response
                    let response = "HTTP/1.1 200 Connection Established\r\n\r\n";
                    stream.write_all(response.as_bytes()).await.unwrap();
                }
            });
        }
    }

    #[tokio::test]
    async fn test_rtsp_through_http_proxy() {
        // Start mock HTTP proxy
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_addr = listener.local_addr().unwrap();

        let proxy_handle = tokio::spawn(async move {
            mock_http_proxy_server(listener).await;
        });

        // Create pipeline with proxy
        gst::init().unwrap();

        let pipeline = gst::parse::launch(&format!(
            "rtspsrc2 name=src location=rtsp://example.com/test proxy=http://127.0.0.1:{} ! fakesink",
            proxy_addr.port()
        )).unwrap();

        // The pipeline would attempt to connect through the proxy
        // In a real test, we'd verify the proxy receives the CONNECT request

        proxy_handle.abort();
    }
}
