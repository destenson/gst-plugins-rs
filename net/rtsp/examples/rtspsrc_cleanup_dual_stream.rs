//! rtspsrc_cleanup_dual_stream.rs
//! 
//! Example demonstrating two independent RTSP streams in the same pipeline.
//! Each stream can be reconnected independently without affecting the other.
//! 
//! Setup for testing:
//! - Configure mediamtx to serve on both 127.0.0.1 and 127.0.0.2 (or use different ports)
//! - Use iptables to selectively drop packets for testing:
//!   `sudo iptables -A INPUT -s 127.0.0.2 -p udp -j DROP`
//!   `sudo iptables -D INPUT -s 127.0.0.2 -p udp -j DROP`
//! 
//! Alternatively, use different streams or ports instead of different IPs.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use gst::prelude::*;
use gst::{self, glib};

/// Frame counter for one stream
struct StreamStats {
    id: usize,
    counter: Arc<AtomicU64>,
    start_time: Instant,
}

impl StreamStats {
    fn new(id: usize) -> Self {
        Self {
            id,
            counter: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
        }
    }

    fn setup_probe(&self, sink: &gst::Element) -> Result<()> {
        let counter_clone = self.counter.clone();
        let stream_id = self.id;
        let sink_pad = sink
            .static_pad("sink")
            .context(format!("Stream {} sink has no sink pad", stream_id))?;
        
        sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, _info| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            gst::PadProbeReturn::Ok
        });
        
        Ok(())
    }

    fn get_stats(&self) -> (u64, f64) {
        let count = self.counter.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let fps = if elapsed > 0.0 {
            count as f64 / elapsed
        } else {
            0.0
        };
        (count, fps)
    }
}

fn create_stream_bin(stream_id: usize, url: &str) -> Result<gst::Bin> {
    let bin = gst::Bin::builder()
        .name(&format!("stream{}-bin", stream_id))
        .build();

    let src = gst::ElementFactory::make("rtspsrc2")
        .name(&format!("rtspsrc2-stream{}", stream_id))
        .property("location", url)
        .property("protocols", "udp")
        .property("async-handling", true)
        .property("buffer-mode", "slave")
        .property("do-retransmission", false)
        .property("latency", 200u32)
        .property("max-reconnection-attempts", 5i32)
        .property("retry-strategy", "auto")
        .property("reconnection-timeout", 3_000_000_000u64)
        .property("select-streams", "video")
        .property("timeout", 3_000_000_000u64)
        .property("user-agent", &format!("dual_stream_example/stream{}", stream_id))
        .build()
        .context("Failed to create rtspsrc2")?;

    let decodebin = gst::ElementFactory::make("decodebin")
        .name(&format!("decodebin-stream{}", stream_id))
        .build()
        .context("Failed to create decodebin")?;

    let queue = gst::ElementFactory::make("queue")
        .name(&format!("queue-stream{}", stream_id))
        .build()
        .context("Failed to create queue")?;

    let fakesink = gst::ElementFactory::make("fakesink")
        .name(&format!("sink-stream{}", stream_id))
        .property("sync", false)
        .property("async", true)
        .build()
        .context("Failed to create fakesink")?;

    bin.add_many(&[&src, &decodebin, &queue, &fakesink])?;
    queue.link(&fakesink)?;

    // Link rtspsrc2 -> decodebin dynamically
    let decodebin_clone = decodebin.clone();
    src.connect_pad_added(move |_src, pad| {
        println!("[Stream {stream_id}] rtspsrc2 pad added: {}", pad.name());
        let Some(sink_pad) = decodebin_clone.static_pad("sink") else {
            eprintln!("[Stream {stream_id}] decodebin sink pad not available");
            return;
        };
        if sink_pad.is_linked() {
            println!("[Stream {stream_id}] decodebin already linked");
            return;
        }
        if let Err(err) = pad.link(&sink_pad) {
            eprintln!("[Stream {stream_id}] Failed to link rtspsrc2 to decodebin: {err:?}");
        } else {
            println!("[Stream {stream_id}] Linked rtspsrc2 -> decodebin");
        }
    });

    // Link decodebin -> queue dynamically
    let queue_clone = queue.clone();
    decodebin.connect_pad_added(move |_db, pad| {
        println!("[Stream {stream_id}] decodebin pad added: {}", pad.name());
        let Some(sink_pad) = queue_clone.static_pad("sink") else {
            eprintln!("[Stream {stream_id}] queue sink pad not available");
            return;
        };
        if sink_pad.is_linked() {
            println!("[Stream {stream_id}] queue already linked");
            return;
        }
        if let Err(err) = pad.link(&sink_pad) {
            eprintln!("[Stream {stream_id}] Failed to link decodebin to queue: {err:?}");
        } else {
            println!("[Stream {stream_id}] Linked decodebin -> queue -> sink");
        }
    });

    Ok(bin)
}

fn main() -> Result<()> {
    gst::init()?;

    let url1 = std::env::var("RTSP_URL1")
        .unwrap_or_else(|_| "rtsp://127.0.0.1:8554/stream1".to_string());
    let url2 = std::env::var("RTSP_URL2")
        .unwrap_or_else(|_| "rtsp://127.0.0.2:8554/stream2".to_string());

    println!("Dual RTSP Stream Example");
    println!("Stream 1: {}", url1);
    println!("Stream 2: {}", url2);
    println!();

    let pipeline = gst::Pipeline::new();

    // Create two independent stream bins
    let stream1_bin = create_stream_bin(1, &url1)?;
    let stream2_bin = create_stream_bin(2, &url2)?;

    pipeline.add_many(&[&stream1_bin, &stream2_bin])?;

    // Setup frame counters
    let stats1 = StreamStats::new(1);
    let stats2 = StreamStats::new(2);

    let sink1 = stream1_bin
        .by_name("sink-stream1")
        .context("Sink 1 not found")?;
    let sink2 = stream2_bin
        .by_name("sink-stream2")
        .context("Sink 2 not found")?;

    stats1.setup_probe(&sink1)?;
    stats2.setup_probe(&sink2)?;

    // Setup bus watch
    let bus = pipeline.bus().context("Pipeline has no bus")?;
    let main_loop = glib::MainLoop::new(None, false);
    let loop_clone = main_loop.clone();

    let _bus_watch = bus.add_watch_local(move |_, msg| {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => {
                let src_name = err
                    .src()
                    .map(|s| s.path_string())
                    .unwrap_or_else(|| "unknown".into());
                eprintln!(
                    "Error from {}: {} ({:?})",
                    src_name,
                    err.error(),
                    err.debug()
                );
                // In a real app, you might want to restart just the failing stream
                // For now, we'll just log the error and continue
            }
            MessageView::Warning(warning) => {
                let src_name = warning
                    .src()
                    .map(|s| s.path_string())
                    .unwrap_or_else(|| "unknown".into());
                eprintln!(
                    "Warning from {}: {} ({:?})",
                    src_name,
                    warning.error(),
                    warning.debug()
                );
            }
            MessageView::Eos(..) => {
                println!("EOS received, stopping");
                loop_clone.quit();
                return glib::ControlFlow::Break;
            }
            MessageView::StateChanged(sc)
                if sc.src().map(|s| s.is::<gst::Pipeline>()).unwrap_or(false) =>
            {
                println!("Pipeline state: {:?} -> {:?}", sc.old(), sc.current());
            }
            _ => {}
        }

        glib::ControlFlow::Continue
    })?;

    // Start the pipeline
    pipeline.set_state(gst::State::Playing)?;
    println!("Pipeline started, waiting for streams...\n");

    // Periodic stats reporting
    let stats1_clone = Arc::new(stats1);
    let stats2_clone = Arc::new(stats2);
    let stats1_timer = stats1_clone.clone();
    let stats2_timer = stats2_clone.clone();

    glib::timeout_add_local(Duration::from_secs(2), move || {
        let (count1, fps1) = stats1_timer.get_stats();
        let (count2, fps2) = stats2_timer.get_stats();
        
        println!(
            "[Stats] Stream 1: {} frames ({:.1} fps) | Stream 2: {} frames ({:.1} fps)",
            count1, fps1, count2, fps2
        );
        
        glib::ControlFlow::Continue
    });

    println!("Press Ctrl+C to stop");
    main_loop.run();

    pipeline.set_state(gst::State::Null)?;
    println!("Shut down cleanly");

    Ok(())
}
