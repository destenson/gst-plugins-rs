// Performance optimization tests for buffer pool, connection pool, and RTCP enhancements

#[cfg(test)]
mod buffer_pool_tests {
    use bytes::BytesMut;
    use gst_plugin_rtsp::rtspsrc::buffer_pool::*;

    #[test]
    fn test_memory_usage_tests() {
        let pool = BufferPool::new(10 * 1024 * 1024); // 10MB limit

        // Acquire buffers of different sizes
        let buf1 = pool.acquire(512);
        assert!(buf1.capacity() >= 512);

        let buf2 = pool.acquire(1024);
        assert!(buf2.capacity() >= 1024);

        let buf3 = pool.acquire(1500);
        assert!(buf3.capacity() >= 1500);

        // Release them back
        pool.release(buf1);
        pool.release(buf2);
        pool.release(buf3);

        // Check memory usage
        assert!(pool.memory_usage() > 0);

        // Clear and verify
        pool.clear();
        assert_eq!(pool.memory_usage(), 0);
    }

    #[test]
    fn test_buffer_perf() {
        let pool = BufferPool::new(64 * 1024 * 1024);
        let iterations = 1000;

        // Benchmark allocation and release
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let buffer = pool.acquire(1500);
            // Simulate some work
            let mut _data = vec![0u8; 100];
            pool.release(buffer);
        }

        let duration = start.elapsed();
        let avg_time_us = duration.as_micros() / iterations;

        // Should be very fast (< 10 microseconds per operation)
        assert!(
            avg_time_us < 10,
            "Buffer pool operations too slow: {}us",
            avg_time_us
        );

        // Check reuse efficiency
        let stats = pool.stats();
        let mtu_stats = stats
            .iter()
            .find(|(size, _)| *size == 1500 || *size == 2048)
            .unwrap()
            .1
            .clone();
        assert!(mtu_stats.reuses > 0, "Buffer pool should reuse buffers");
    }

    #[test]
    fn test_stress_buffers() {
        use std::sync::Arc;
        use std::thread;

        let pool = Arc::new(BufferPool::new(128 * 1024 * 1024));
        let num_threads = 10;
        let ops_per_thread = 100;

        let mut handles = vec![];

        for _ in 0..num_threads {
            let pool_clone = Arc::clone(&pool);
            let handle = thread::spawn(move || {
                for i in 0..ops_per_thread {
                    // Vary buffer sizes
                    let size = 100 + (i * 100) % 4000;
                    let buffer = pool_clone.acquire(size);

                    // Simulate some work
                    thread::yield_now();

                    pool_clone.release(buffer);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify pool is still functional
        let final_buffer = pool.acquire(1000);
        assert!(final_buffer.capacity() >= 1000);
    }
}

#[cfg(test)]
mod connection_pool_tests {
    use gst_plugin_rtsp::rtspsrc::connection_pool::*;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn test_connection_pool() {
        let config = ConnectionPoolConfig {
            enabled: true,
            max_connections_per_server: 3,
            idle_timeout: Duration::from_secs(10),
        };

        let pool = ConnectionPool::new(config);

        // Start test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Accept connections in background
        tokio::spawn(async move {
            loop {
                if listener.accept().await.is_err() {
                    break;
                }
            }
        });

        // Test basic checkout/checkin
        let stream1 = TcpStream::connect(server_addr).await.unwrap();
        pool.add_new(stream1, server_addr).await.unwrap();

        let checked_out = pool.checkout(server_addr).await;
        assert!(checked_out.is_some());

        pool.checkin(checked_out.unwrap(), server_addr)
            .await
            .unwrap();

        // Verify stats
        let stats = pool.stats();
        assert!(stats.contains_key(&server_addr));
        assert_eq!(stats[&server_addr].total_connections, 1);
    }

    #[tokio::test]
    async fn test_pool_concurrent() {
        let config = ConnectionPoolConfig {
            enabled: true,
            max_connections_per_server: 5,
            idle_timeout: Duration::from_secs(10),
        };

        let pool = Arc::new(ConnectionPool::new(config));

        // Start test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Accept connections in background
        tokio::spawn(async move {
            loop {
                if listener.accept().await.is_err() {
                    break;
                }
            }
        });

        // Add multiple connections
        for _ in 0..3 {
            let stream = TcpStream::connect(server_addr).await.unwrap();
            pool.add_new(stream, server_addr).await.unwrap();
        }

        // Concurrent checkouts
        let mut handles = vec![];
        for _ in 0..3 {
            let pool_clone = Arc::clone(&pool);
            let handle = tokio::spawn(async move {
                if let Some(stream) = pool_clone.checkout(server_addr).await {
                    // Simulate some work
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    pool_clone.checkin(stream, server_addr).await.unwrap();
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        let stats = pool.stats();
        assert_eq!(stats[&server_addr].total_connections, 3);
    }

    #[tokio::test]
    async fn test_pool_reuse() {
        let config = ConnectionPoolConfig {
            enabled: true,
            max_connections_per_server: 2,
            idle_timeout: Duration::from_secs(5),
        };

        let pool = ConnectionPool::new(config);

        // Start test server
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server_addr = listener.local_addr().unwrap();

        // Accept connections in background
        tokio::spawn(async move {
            loop {
                if listener.accept().await.is_err() {
                    break;
                }
            }
        });

        // Add and reuse connection multiple times
        let stream = TcpStream::connect(server_addr).await.unwrap();
        pool.add_new(stream, server_addr).await.unwrap();

        for _ in 0..5 {
            let stream = pool.checkout(server_addr).await.unwrap();
            pool.checkin(stream, server_addr).await.unwrap();
        }

        // Should still have just one connection
        let stats = pool.stats();
        assert_eq!(stats[&server_addr].total_connections, 1);
        assert!(stats[&server_addr].total_uses >= 5);
    }

    use std::sync::Arc;
}

#[cfg(test)]
mod rtcp_enhanced_tests {
    use gst_plugin_rtsp::rtspsrc::rtcp_enhanced::*;

    #[test]
    fn test_rtcp_stats() {
        let handler = RtcpEnhancedHandler::new(true, true);

        // Create a mock SR packet
        let sr_packet = vec![
            0x80, 200, // V=2, P=0, RC=0, PT=SR
            0x00, 0x06, // Length
            0x12, 0x34, 0x56, 0x78, // SSRC
            0x00, 0x00, 0x00, 0x00, // NTP timestamp MSW
            0x00, 0x00, 0x00, 0x00, // NTP timestamp LSW
            0x00, 0x00, 0x00, 0x00, // RTP timestamp
            0x00, 0x00, 0x00, 0x64, // Packet count
            0x00, 0x00, 0x10, 0x00, // Octet count
        ];

        handler.process_rtcp_packet(&sr_packet).unwrap();

        let ssrc = 0x12345678;
        let stats = handler.get_statistics(ssrc);
        assert!(stats.is_some());
        assert!(stats.unwrap().last_sr_timestamp.is_some());
    }

    #[test]
    fn test_rtcp_xr() {
        let handler = RtcpEnhancedHandler::new(true, true);

        // Create a VoIP metrics XR block
        let xr_block = XrReportBlock {
            block_type: 7, // VoIP Metrics
            type_specific: 0,
            block_length: 8,
            ssrc: 0x12345678,
            data: vec![
                10, // loss_rate
                5,  // discard_rate
                20, // burst_density
                10, // gap_density
                0x00,
                0x64, // burst_duration
                0x00,
                0x32, // gap_duration
                0x00,
                0x0A, // round_trip_delay
                0x00,
                0x05,        // end_system_delay
                -60i8 as u8, // signal_level
                -80i8 as u8, // noise_level
                35,          // rerl
                16,          // gmin
                93,          // r_factor
                93,          // ext_r_factor
                40,          // mos_lq
                45,          // mos_cq
            ],
        };

        let metrics = xr_block.parse_voip_metrics().unwrap();
        assert_eq!(metrics.r_factor, 93);
        assert_eq!(metrics.loss_rate, 10);
    }

    #[test]
    fn test_rtcp_feedback() {
        // Test NACK creation
        let nack = FeedbackMessage::create_nack(0x11111111, 0x22222222, &[100, 101, 102]);
        assert_eq!(nack.message_type, 205); // RTCP_RTPFB
        assert_eq!(nack.fmt, 1); // RTPFB_NACK
        assert_eq!(nack.sender_ssrc, 0x11111111);
        assert_eq!(nack.media_ssrc, 0x22222222);
        assert!(nack.data.len() >= 6); // At least 3 sequence numbers

        // Test PLI creation
        let pli = FeedbackMessage::create_pli(0x33333333, 0x44444444);
        assert_eq!(pli.message_type, 206); // RTCP_PSFB
        assert_eq!(pli.fmt, 1); // PSFB_PLI

        // Test FIR creation
        let fir = FeedbackMessage::create_fir(0x55555555, 0x66666666, 42);
        assert_eq!(fir.message_type, 206); // RTCP_PSFB
        assert_eq!(fir.fmt, 4); // PSFB_FIR
        assert_eq!(fir.data[3], 42);

        // Test REMB creation
        let remb = FeedbackMessage::create_remb(0x77777777, 1_000_000, &[0x88888888, 0x99999999]);
        assert_eq!(remb.message_type, 206); // RTCP_PSFB
        assert_eq!(remb.fmt, 15); // PSFB_REMB
        assert_eq!(&remb.data[0..4], b"REMB");
    }
}
