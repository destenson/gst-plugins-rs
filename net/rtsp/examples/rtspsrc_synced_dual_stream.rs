//! rtspsrc_synced_dual_stream.rs
//! 
//! Example demonstrating two synchronized RTSP streams in the same pipeline.
//! Uses compositor/aggregator to keep frames from both streams time-aligned.
//! Similar to nvstreammux behavior in DeepStream.
//! 
//! Each stream can be reconnected independently without affecting the other,
//! but the compositor ensures frames are processed in sync.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use gst::prelude::*;
use gst::{self, glib};

/// Frame counter for tracking stream statistics
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

    fn setup_probe(&self, pad: &gst::Pad) {
        let counter_clone = self.counter.clone();
        let stream_id = self.id;
        
        pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
            if let Some(gst::PadProbeData::Buffer(ref buffer)) = info.data {
                counter_clone.fetch_add(1, Ordering::Relaxed);
                
                // Log PTS for debugging sync issues
                if counter_clone.load(Ordering::Relaxed) % 100 == 0 {
                    gst::debug!(
                        gst::CAT_DEFAULT,
                        "[Stream {}] Frame {} PTS: {:?}",
                        stream_id,
                        counter_clone.load(Ordering::Relaxed),
                        buffer.pts()
                    );
                }
            }
            gst::PadProbeReturn::Ok
        });
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

fn create_stream_source(stream_id: usize, url: &str) -> Result<(gst::Bin, gst::Pad)> {
    let bin = gst::Bin::builder()
        .name(&format!("stream{}-source", stream_id))
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
        .property("user-agent", &format!("synced_dual_stream/stream{}", stream_id))
        .build()
        .context("Failed to create rtspsrc2")?;

    let decodebin = gst::ElementFactory::make("decodebin")
        .name(&format!("decodebin-stream{}", stream_id))
        .build()
        .context("Failed to create decodebin")?;

    // videoconvert to ensure consistent format for compositor
    let convert = gst::ElementFactory::make("videoconvert")
        .name(&format!("convert-stream{}", stream_id))
        .build()
        .context("Failed to create videoconvert")?;

    // videoscale to normalize resolution if needed
    let scale = gst::ElementFactory::make("videoscale")
        .name(&format!("scale-stream{}", stream_id))
        .build()
        .context("Failed to create videoscale")?;

    // capsfilter to set consistent output format
    let capsfilter = gst::ElementFactory::make("capsfilter")
        .name(&format!("caps-stream{}", stream_id))
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .field("format", "I420")
                .field("width", 640)
                .field("height", 480)
                .build(),
        )
        .build()
        .context("Failed to create capsfilter")?;

    // queue for buffering before compositor
    let queue = gst::ElementFactory::make("queue")
        .name(&format!("queue-stream{}", stream_id))
        .property("max-size-buffers", 3u32)
        .property("max-size-time", 100_000_000u64) // 100ms
        .build()
        .context("Failed to create queue")?;

    bin.add_many(&[&src, &decodebin, &convert, &scale, &capsfilter, &queue])?;
    
    // Static link: convert -> scale -> capsfilter -> queue
    gst::Element::link_many(&[&convert, &scale, &capsfilter, &queue])?;

    // Dynamic link: rtspsrc2 -> decodebin
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

    // Dynamic link: decodebin -> convert
    let convert_clone = convert.clone();
    decodebin.connect_pad_added(move |_db, pad| {
        println!("[Stream {stream_id}] decodebin pad added: {}", pad.name());
        let Some(sink_pad) = convert_clone.static_pad("sink") else {
            eprintln!("[Stream {stream_id}] convert sink pad not available");
            return;
        };
        if sink_pad.is_linked() {
            println!("[Stream {stream_id}] convert already linked");
            return;
        }
        if let Err(err) = pad.link(&sink_pad) {
            eprintln!("[Stream {stream_id}] Failed to link decodebin to convert: {err:?}");
        } else {
            println!("[Stream {stream_id}] Linked decodebin -> convert -> scale -> caps -> queue");
        }
    });

    // Create ghost pad from queue's src pad
    let queue_src = queue
        .static_pad("src")
        .context("Queue has no src pad")?;
    let ghost_pad = gst::GhostPad::builder_with_target(&queue_src)
        .context("Failed to create ghost pad")?
        .name(&format!("src_{}", stream_id))
        .build();
    
    ghost_pad.set_active(true)?;
    bin.add_pad(&ghost_pad)?;

    Ok((bin, ghost_pad.upcast()))
}

fn main() -> Result<()> {
    gst::init()?;

    let url1 = std::env::var("RTSP_URL1")
        .unwrap_or_else(|_| "rtsp://127.0.0.1:8554/stream1".to_string());
    let url2 = std::env::var("RTSP_URL2")
        .unwrap_or_else(|_| "rtsp://127.0.0.2:8554/stream2".to_string());

    println!("Synchronized Dual RTSP Stream Example");
    println!("Stream 1: {}", url1);
    println!("Stream 2: {}", url2);
    println!("Using compositor to keep streams synchronized");
    println!();

    let pipeline = gst::Pipeline::new();

    // Create two independent stream sources
    let (stream1_bin, stream1_src_pad) = create_stream_source(1, &url1)?;
    let (stream2_bin, stream2_src_pad) = create_stream_source(2, &url2)?;

    // Create compositor for synchronized mixing
    // Similar to nvstreammux - aggregates multiple video streams
    let compositor = gst::ElementFactory::make("compositor")
        .name("mux")
        // Key properties for sync behavior
        .property("start-time-selection", 0i32) // Use first buffer timestamp
        .build()
        .context("Failed to create compositor")?;

    // Output processing chain
    let queue_out = gst::ElementFactory::make("queue")
        .name("queue-output")
        .build()?;

    let convert_out = gst::ElementFactory::make("videoconvert")
        .name("convert-output")
        .build()?;

    let sink = gst::ElementFactory::make("fakesink")
        .name("sink")
        .property("sync", true) // Important: sync to clock for proper timing
        .property("async", false)
        .build()
        .context("Failed to create fakesink")?;

    pipeline.add_many(&[
        stream1_bin.upcast_ref::<gst::Element>(),
        stream2_bin.upcast_ref(),
        &compositor,
        queue_out.upcast_ref(),
        convert_out.upcast_ref(),
        sink.upcast_ref(),
    ])?;

    // Link compositor output chain
    gst::Element::link_many(&[&compositor, &queue_out, &convert_out, &sink])?;

    // Request sink pads from compositor and link sources
    let comp_sink1 = compositor
        .request_pad_simple("sink_0")
        .context("Failed to get compositor sink_0")?;
    let comp_sink2 = compositor
        .request_pad_simple("sink_1")
        .context("Failed to get compositor sink_1")?;

    // Configure compositor sink pads for side-by-side layout
    comp_sink1.set_property("xpos", 0i32);
    comp_sink1.set_property("ypos", 0i32);
    comp_sink1.set_property("width", 640i32);
    comp_sink1.set_property("height", 480i32);

    comp_sink2.set_property("xpos", 640i32); // Side by side
    comp_sink2.set_property("ypos", 0i32);
    comp_sink2.set_property("width", 640i32);
    comp_sink2.set_property("height", 480i32);

    // Link source pads to compositor
    stream1_src_pad.link(&comp_sink1)?;
    stream2_src_pad.link(&comp_sink2)?;

    println!("Compositor configured for side-by-side layout (1280x480)");

    // Setup frame counters on the source pads
    let stats1 = Arc::new(StreamStats::new(1));
    let stats2 = Arc::new(StreamStats::new(2));

    stats1.setup_probe(&stream1_src_pad);
    stats2.setup_probe(&stream2_src_pad);

    // Also count synchronized output frames
    let output_counter = Arc::new(AtomicU64::new(0));
    let output_counter_clone = output_counter.clone();
    let sink_pad = sink.static_pad("sink").context("Sink has no sink pad")?;
    sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, _info| {
        output_counter_clone.fetch_add(1, Ordering::Relaxed);
        gst::PadProbeReturn::Ok
    });

    // Setup bus watch
    let bus = pipeline.bus().context("Pipeline has no bus")?;
    let main_loop = glib::MainLoop::new(None, false);
    let loop_clone = main_loop.clone();

    let recalculate_latency = |pipeline: &gst::Pipeline| {
        if let Err(err) = pipeline.recalculate_latency() {
            eprintln!("Failed to recalculate latency: {err}");
        }
    };

    let pipeline_clone = pipeline.clone();
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
                // In production, you'd want to restart just the failing stream
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
            MessageView::StreamStatus(ss) => {
                println!("Stream status from {:?}: {:?}", ss.src(), ss.get().0);
            }
            MessageView::Latency(_) => {
                // Recalculate latency when streams change
                recalculate_latency(pipeline_clone.as_ref());
            }
            _ => {}
        }

        glib::ControlFlow::Continue
    })?;

    // Start pipeline
    pipeline.set_state(gst::State::Playing)?;
    println!("Pipeline started, waiting for streams...\n");

    // Periodic stats reporting
    let stats1_timer = stats1.clone();
    let stats2_timer = stats2.clone();
    let output_counter_timer = output_counter.clone();
    let start_time = Instant::now();

    glib::timeout_add_local(Duration::from_secs(2), move || {
        let (count1, fps1) = stats1_timer.get_stats();
        let (count2, fps2) = stats2_timer.get_stats();
        let output_count = output_counter_timer.load(Ordering::Relaxed);
        let elapsed = start_time.elapsed().as_secs_f64();
        let output_fps = if elapsed > 0.0 {
            output_count as f64 / elapsed
        } else {
            0.0
        };
        
        println!(
            "[Stats] Stream 1: {} frames ({:.1} fps) | Stream 2: {} frames ({:.1} fps) | Synced Output: {} frames ({:.1} fps)",
            count1, fps1, count2, fps2, output_count, output_fps
        );
        
        glib::ControlFlow::Continue
    });

    println!("Press Ctrl+C to stop");
    println!("Note: Compositor will wait for frames from BOTH streams before outputting");
    println!("      If one stream is blocked, output will pause until it recovers\n");
    
    main_loop.run();

    pipeline.set_state(gst::State::Null)?;
    println!("Shut down cleanly");

    Ok(())
}
