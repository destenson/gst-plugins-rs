use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use gst::prelude::*;
use gst::{self, glib, ClockTime, StateChangeSuccess};
use rand::{rng, Rng};

/// Holds the elements and signal handler IDs associated with the current rtspsrc2 chain.
struct SourceBinState {
    bin: gst::Bin,
    src: gst::Element,
    decodebin: gst::Element,
    output_pad: Arc<Mutex<Option<gst::GhostPad>>>,
    src_pad_added_id: RefCell<Option<glib::SignalHandlerId>>,
    decode_pad_added_id: RefCell<Option<glib::SignalHandlerId>>,
}

impl SourceBinState {
    fn disconnect(&self) {
        if let Some(id) = self.src_pad_added_id.borrow_mut().take() {
            self.src.disconnect(id);
        }
        if let Some(id) = self.decode_pad_added_id.borrow_mut().take() {
            self.decodebin.disconnect(id);
        }
        let ghost = match self.output_pad.lock() {
            Ok(mut pad) => pad.take(),
            Err(err) => {
                eprintln!("Failed to lock ghost pad during disconnect: {err}");
                None
            }
        };
        if let Some(ghost) = ghost {
            let pad: gst::Pad = ghost.clone().upcast();
            if let Some(peer) = pad.peer() {
                unlink_pads(&pad, &peer);
            }
            let _ = pad.set_active(false);
            let _ = self.bin.remove_pad(&ghost);
        }
    }
}

#[derive(Default)]
struct AppState {
    source: Option<SourceBinState>,
    restarts: u32,
}

#[derive(Clone, Debug)]
struct Config {
    url: String,
    restart_interval: Option<u32>,
    max_restarts: Option<u32>,
    restart_jitter: Option<f64>,
    use_autovideosink: bool,
}

impl Config {
    fn parse() -> Result<Self> {
        let mut url =
            std::env::var("RTSP_URL").unwrap_or_else(|_| "rtsp://127.0.0.1:8554/test-h264-udp".to_string());
        let mut restart_interval = None;
        let mut max_restarts = None;
        let mut restart_jitter = None;
        let mut use_autovideosink = false;

        let args = std::env::args().skip(1).collect::<Vec<_>>();
        if args.iter().any(|a| a == "--help" || a == "-h") {
            Self::print_help();
            std::process::exit(0);
        }

        let mut idx = 0usize;
        while idx < args.len() {
            let arg = &args[idx];
            match arg.as_str() {
                val if val.starts_with("--url=") => {
                    url = val.trim_start_matches("--url=").to_string();
                }
                "--url" => {
                    idx += 1;
                    url = args
                        .get(idx)
                        .cloned()
                        .ok_or_else(|| anyhow!("--url expects a value"))?;
                }
                val if val.starts_with("--restart-interval=") => {
                    let raw = val.trim_start_matches("--restart-interval=");
                    restart_interval = Some(raw.parse().context("Invalid restart interval")?);
                }
                "--restart-interval" => {
                    idx += 1;
                    let raw = args
                        .get(idx)
                        .ok_or_else(|| anyhow!("--restart-interval expects a value"))?;
                    restart_interval = Some(raw.parse().context("Invalid restart interval")?);
                }
                val if val.starts_with("--max-restarts=") => {
                    let raw = val.trim_start_matches("--max-restarts=");
                    max_restarts = Some(raw.parse().context("Invalid max restart count")?);
                }
                "--max-restarts" => {
                    idx += 1;
                    let raw = args
                        .get(idx)
                        .ok_or_else(|| anyhow!("--max-restarts expects a value"))?;
                    max_restarts = Some(raw.parse().context("Invalid max restart count")?);
                }
                val if val.starts_with("--restart-jitter=") => {
                    let raw = val.trim_start_matches("--restart-jitter=");
                    restart_jitter = Some(raw.parse().context("Invalid restart jitter value")?);
                }
                "--restart-jitter" => {
                    idx += 1;
                    let raw = args
                        .get(idx)
                        .ok_or_else(|| anyhow!("--restart-jitter expects a value"))?;
                    restart_jitter = Some(raw.parse().context("Invalid restart jitter value")?);
                }
                "--autovideosink" => {
                    use_autovideosink = true;
                }
                other => {
                    return Err(anyhow!("Unknown argument: {other}"));
                }
            }
            idx += 1;
        }

        Ok(Self {
            url,
            restart_interval,
            max_restarts,
            restart_jitter,
            use_autovideosink,
        })
    }

    fn print_help() {
        println!("Usage: cargo run -p gst-plugin-rtsp --example rtspsrc_cleanup -- [OPTIONS]\n");
        println!("Options:");
        println!("  --url <RTSP-URL>              RTSP source to connect to (default: env RTSP_URL or rtsp://127.0.0.1:8554/test)");
        println!("  --restart-interval <seconds>  Force periodic removal & re-creation of rtspsrc2 every N seconds");
        println!("  --max-restarts <count>        Stop after the source has been rebuilt this many times");
        println!(
            "  --restart-jitter <fraction>   Jitter to apply around the restart interval (default: 0.25 = ±25%)"
        );
        println!("  --autovideosink               Use autovideosink instead of fakesink (allows visual frame verification)");
        println!();
        println!("Environment:");
        println!("  RTSP_URL                      Default RTSP URL if --url is omitted");
        println!(
            "  GST_PLUGIN_PATH               Must include the build output containing rtspsrc2"
        );
    }
}

fn sanitize_jitter(jitter: f64) -> f64 {
    jitter.abs().min(0.5)
}

fn wait_for_state<E>(element: &E, desired: gst::State, label: &str)
where
    E: IsA<gst::Element>,
{
    let timeout = ClockTime::from_seconds(2);
    let (result, state, pending) = element.upcast_ref::<gst::Element>().state(timeout);
    match result {
        Ok(StateChangeSuccess::Success) | Ok(StateChangeSuccess::NoPreroll) if state == desired => {
        }
        Ok(other) => {
            eprintln!(
                "{label}: state change returned {:?} (current {:?}, pending {:?})",
                other, state, pending
            );
        }
        Err(err) => {
            eprintln!("{label}: error while waiting for {:?}: {err:?}", desired);
        }
    }
}

fn next_restart_delay(base_interval: u32, jitter: f64) -> (f64, Duration) {
    let base_secs = base_interval.max(1) as f64;
    let jitter = sanitize_jitter(jitter);
    if jitter == 0.0 {
        return (base_secs, Duration::from_secs_f64(base_secs));
    }

    let spread = base_secs * jitter;
    let min_delay = (base_secs - spread).max(0.25);
    let max_delay = base_secs + spread;
    let mut rng = rng();
    let selected = if (max_delay - min_delay).abs() < f64::EPSILON {
        min_delay
    } else {
        rng.random_range(min_delay..=max_delay)
    };
    (selected, Duration::from_secs_f64(selected))
}

fn unlink_pads(pad_a: &gst::Pad, pad_b: &gst::Pad) {
    let (src, sink) = match pad_a.direction() {
        gst::PadDirection::Src => (pad_a, pad_b),
        _ => match pad_b.direction() {
            gst::PadDirection::Src => (pad_b, pad_a),
            _ => {
                eprintln!(
                    "Cannot unlink pads without a source: {:?} -> {:?}",
                    pad_a, pad_b
                );
                return;
            }
        },
    };

    let Some(current_peer) = src.peer() else {
        return;
    };

    if current_peer != *sink {
        eprintln!(
            "Pads are not linked to each other, cannot unlink: {} -> {}",
            src.path_string(),
            sink.path_string()
        );
        return;
    }
}

fn schedule_periodic_restart(
    base_interval: u32,
    jitter: f64,
    pipeline_weak: glib::WeakRef<gst::Pipeline>,
    sink_queue: gst::Element,
    config: Rc<Config>,
    state: Rc<RefCell<AppState>>,
    replacing: Rc<Cell<bool>>,
    main_loop: glib::MainLoop,
) {
    let (delay_seconds, delay_duration) = next_restart_delay(base_interval, jitter);
    println!(
        "Scheduling periodic restart in {:.1}s (base {}s, jitter {:.0}%)",
        delay_seconds,
        base_interval,
        jitter * 100.0
    );

    let sink_queue_for_cb = sink_queue.clone();
    let config_for_cb = config.clone();
    let state_for_cb = state.clone();
    let replacing_for_cb = replacing.clone();
    let loop_for_cb = main_loop.clone();
    let pipeline_weak_for_timeout = pipeline_weak.clone();

    glib::timeout_add_local(delay_duration, move || {
        let Some(pipeline) = pipeline_weak_for_timeout.upgrade() else {
            return glib::ControlFlow::Break;
        };

        if replacing_for_cb.get() {
            schedule_periodic_restart(
                base_interval,
                jitter,
                pipeline.downgrade(),
                sink_queue_for_cb.clone(),
                config_for_cb.clone(),
                state_for_cb.clone(),
                replacing_for_cb.clone(),
                loop_for_cb.clone(),
            );
            return glib::ControlFlow::Break;
        }

        replacing_for_cb.set(true);
        if let Err(e) = replace_source(
            &pipeline,
            &sink_queue_for_cb,
            &config_for_cb.url,
            &state_for_cb,
            "periodic restart",
        ) {
            eprintln!("Periodic rebuild failed: {e:?}");
            loop_for_cb.quit();
            return glib::ControlFlow::Break;
        }
        replacing_for_cb.set(false);

        if let Some(limit) = config_for_cb.max_restarts {
            if state_for_cb.borrow().restarts >= limit {
                println!("Reached max restarts ({limit}) via timer, stopping main loop");
                loop_for_cb.quit();
                return glib::ControlFlow::Break;
            }
        }

        schedule_periodic_restart(
            base_interval,
            jitter,
            pipeline.downgrade(),
            sink_queue_for_cb.clone(),
            config_for_cb.clone(),
            state_for_cb.clone(),
            replacing_for_cb.clone(),
            loop_for_cb.clone(),
        );

        glib::ControlFlow::Break
    });
}

fn create_source_bin(url: &str, sink_queue: &gst::Element, id: u32) -> Result<SourceBinState> {
    let bin = gst::Bin::new();

    let src = gst::ElementFactory::make("rtspsrc2")
        .name(&format!("rtspsrc2-source-{id}"))
        .property("location", url)
        // .property("protocols", "tcp")
        .property("protocols", "udp")
        .property("async-handling", true)
        .property("buffer-mode", "slave")
        .property("do-retransmission", false)
        .property("latency", 200u32)
        .property("max-reconnection-attempts", 5i32)
        // .property("max-reconnection-attempts", -1i32)
        .property("retry-strategy", "auto")
        .property("reconnection-timeout", 3_000_000_000u64)
        .property("select-streams", "video")
        // .property("tcp-timeout", 3_000_000u64)
        .property("timeout", 3_000_000_000u64)
        .property("user-agent", "rtspsrc_cleanup_example/1.0")
        .build()
        .context(
            "Failed to create rtspsrc2 element. Did you build the plugin and set GST_PLUGIN_PATH?",
        )?;

    let decodebin = gst::ElementFactory::make("decodebin")
        .name(&format!("decodebin-{id}"))
        .build()
        .context("Failed to create decodebin element")?;

    bin.add_many(&[&src, &decodebin])?;

    let output_pad: Arc<Mutex<Option<gst::GhostPad>>> = Arc::new(Mutex::new(None));

    let decodebin_for_src = decodebin.clone();
    let src_pad_added_id = src.connect_pad_added(move |_src, pad| {
        let Some(sink_pad) = decodebin_for_src.static_pad("sink") else {
            eprintln!("decodebin sink pad not available");
            return;
        };
        if sink_pad.is_linked() {
            println!("decodebin sink already linked, ignoring pad {}", pad.name());
            return;
        }
        if let Err(err) = pad.link(&sink_pad) {
            eprintln!(
                "Failed to link rtspsrc2 pad {} to decodebin sink: {:?}",
                pad.name(),
                err
            );
        }
    });

    let queue_for_decode = sink_queue.clone();
    let bin_for_decode = bin.clone();
    let output_pad_for_decode = output_pad.clone();
    let decode_pad_added_id = decodebin.connect_pad_added(move |_db, pad| {
        let Some(sink_pad) = queue_for_decode.static_pad("sink") else {
            eprintln!("Output queue sink pad not available");
            return;
        };

        let existing = match output_pad_for_decode.lock() {
            Ok(mut pad) => pad.take(),
            Err(err) => {
                eprintln!("Failed to lock existing ghost pad storage: {err}");
                None
            }
        };

        if let Some(existing) = existing {
            let existing_pad: gst::Pad = existing.clone().upcast();
            if let Some(peer) = existing_pad.peer() {
                unlink_pads(&existing_pad, &peer);
            }
            let _ = existing_pad.set_active(false);
            let _ = bin_for_decode.remove_pad(&existing);
        }

        if sink_pad.is_linked() {
            let Some(peer) = sink_pad.peer() else {
                println!(
                    "Queue sink pad appears linked but no peer reported, refusing to override"
                );
                return;
            };
            unlink_pads(&sink_pad, &peer);
        }

        let ghost_name = format!("src-{}", pad.name());
        let ghost_pad = match gst::GhostPad::builder_with_target(pad) {
            Ok(builder) => builder.name(&ghost_name).build(),
            Err(err) => {
                eprintln!(
                    "Failed to create ghost pad for decodebin target {}: {:?}",
                    pad.name(),
                    err
                );
                return;
            }
        };

        if let Err(err) = ghost_pad.set_active(true) {
            eprintln!("Failed to activate ghost pad {ghost_name}: {err:?}");
            return;
        }

        if let Err(err) = bin_for_decode.add_pad(&ghost_pad) {
            eprintln!("Failed to add ghost pad {ghost_name} to source bin: {err:?}");
            let _ = ghost_pad.set_active(false);
            return;
        }

        let ghost_pad_ref: gst::Pad = ghost_pad.clone().upcast();
        match ghost_pad_ref.link(&sink_pad) {
            Ok(_) => {
                println!(
                    "Linked decodebin pad {} to output queue via ghost pad",
                    pad.name()
                );
                if let Ok(mut slot) = output_pad_for_decode.lock() {
                    *slot = Some(ghost_pad);
                }
            }
            Err(err) => {
                eprintln!(
                    "Failed to link source ghost pad to queue sink for pad {}: {:?}",
                    pad.name(),
                    err
                );
                let _ = ghost_pad_ref.set_active(false);
                let _ = bin_for_decode.remove_pad(&ghost_pad);
            }
            
        }
    });

    Ok(SourceBinState {
        bin,
        src,
        decodebin,
        output_pad,
        src_pad_added_id: RefCell::new(Some(src_pad_added_id)),
        decode_pad_added_id: RefCell::new(Some(decode_pad_added_id)),
    })
}

fn replace_source(
    pipeline: &gst::Pipeline,
    sink_queue: &gst::Element,
    url: &str,
    state: &Rc<RefCell<AppState>>,
    reason: &str,
) -> Result<()> {
    println!("\n=== Rebuilding rtspsrc2 chain ({reason}) ===");

    let change = pipeline
        .set_state(gst::State::Ready)
        .context("Failed to set pipeline to READY")?;
    if change == StateChangeSuccess::Async {
        wait_for_state(pipeline, gst::State::Ready, "pipeline -> READY");
    }
    println!("Pipeline transitioned to READY: {:?}", change);

    if let Some(sink_pad) = sink_queue.static_pad("sink") {
        if let Some(peer) = sink_pad.peer() {
            unlink_pads(&sink_pad, &peer);
        }
    }
    sink_queue.send_event(gst::event::FlushStart::new());
    sink_queue.send_event(gst::event::FlushStop::builder(true).build());

    {
        let mut data = state.borrow_mut();
        if let Some(old) = data.source.take() {
            old.disconnect();
            let old_change = old
                .bin
                .set_state(gst::State::Null)
                .context("Failed to set previous source bin to NULL")?;
            if old_change == StateChangeSuccess::Async {
                wait_for_state(&old.bin, gst::State::Null, "source bin -> NULL");
            }
            pipeline
                .remove(&old.bin)
                .context("Failed to remove previous source bin")?;
        }

        let next_id = data.restarts + 1;
        drop(data);

        let new_state = create_source_bin(url, sink_queue, next_id)?;
        pipeline
            .add(&new_state.bin)
            .context("Failed to add new source bin to pipeline")?;
        new_state
            .bin
            .sync_state_with_parent()
            .context("Failed to sync new source bin with pipeline state")?;

        let mut data = state.borrow_mut();
        data.source = Some(new_state);
        data.restarts = next_id;
        println!("Rebuild count: {}", data.restarts);
    }

    let change = pipeline
        .set_state(gst::State::Playing)
        .context("Failed to set pipeline to PLAYING")?;
    if change == StateChangeSuccess::Async {
        wait_for_state(pipeline, gst::State::Playing, "pipeline -> PLAYING");
    }
    println!("Pipeline transitioned to PLAYING: {:?}", change);

    Ok(())
}

fn main() -> Result<()> {
    gst::init()?;

    let config = Rc::new(Config::parse()?);
    println!("Using RTSP URL: {}", config.url);
    if let Some(interval) = config.restart_interval {
        let jitter = sanitize_jitter(config.restart_jitter.unwrap_or(0.25));
        if jitter == 0.0 {
            println!("Periodic restart interval: {interval}s (no jitter)");
        } else {
            println!(
                "Periodic restart interval: {interval}s with ±{:.0}% jitter",
                jitter * 100.0
            );
        }
    } else {
        println!("Periodic restarts disabled (waiting for network errors)");
    }
    if let Some(limit) = config.max_restarts {
        println!("Will stop after {limit} rebuilds");
    }

    let pipeline = gst::Pipeline::new();

    let sink_queue = gst::ElementFactory::make("queue")
        .name("output-queue")
        .build()
        .context("Failed to create queue element")?;
    
    let (sink, frame_counter, last_pts) = if config.use_autovideosink {
        println!("Using autovideosink (visual output)");
        let sink = gst::ElementFactory::make("autovideosink")
            .name("video-sink")
            .property("sync", false)
            .build()
            .context("Failed to create autovideosink element")?;
        (sink, None, None)
    } else {
        println!("Using fakesink (frame counter enabled)");
        let frame_counter = Arc::new(AtomicU64::new(0));
        let last_pts = Arc::new(Mutex::new(None::<gst::ClockTime>));
        let sink = gst::ElementFactory::make("fakesink")
            .name("test-sink")
            .property("sync", false)
            .property("async", true)
            .build()
            .context("Failed to create fakesink element")?;
        
        // Add a probe to count frames and track PTS
        let counter_clone = frame_counter.clone();
        let pts_clone = last_pts.clone();
        let sink_pad = sink.static_pad("sink").context("fakesink has no sink pad")?;
        sink_pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
            counter_clone.fetch_add(1, Ordering::Relaxed);
            
            // Track last PTS for accurate FPS calculation
            if let Some(gst::PadProbeData::Buffer(ref buffer)) = info.data {
                if let Some(pts) = buffer.pts() {
                    if let Ok(mut last) = pts_clone.lock() {
                        *last = Some(pts);
                    }
                }
            }
            
            gst::PadProbeReturn::Ok
        });
        
        (sink, Some(frame_counter), Some(last_pts))
    };

    pipeline.add_many(&[&sink_queue, &sink])?;
    sink_queue.link(&sink)?;

    let state = Rc::new(RefCell::new(AppState::default()));
    replace_source(&pipeline, &sink_queue, &config.url, &state, "initial start")?;

    let main_loop = glib::MainLoop::new(None, false);

    let bus = pipeline.bus().context("Pipeline does not have a bus")?;

    let loop_clone = main_loop.clone();
    let pipeline_weak = pipeline.downgrade();
    let sink_queue_clone = sink_queue.clone();
    let config_clone = config.clone();
    let state_clone = state.clone();
    let replacing = Rc::new(Cell::new(false));
    let replacing_bus = replacing.clone();

    let _bus_watch = bus.add_watch_local(move |_, msg| {
        use gst::MessageView;

        match msg.view() {
            MessageView::Error(err) => {
                eprintln!(
                    "Error from {:?}: {} ({:?})",
                    err.src().map(|s| s.path_string()),
                    err.error(),
                    err.debug()
                );

                // Check if error is from autovideosink (window closed)
                if let Some(src) = err.src() {
                    if src.name().starts_with("autovideosink") 
                        || src.name().starts_with("video-sink") {
                        println!("Video sink window closed, exiting");
                        loop_clone.quit();
                        return glib::ControlFlow::Break;
                    }
                }

                if replacing_bus.get() {
                    return glib::ControlFlow::Continue;
                }

                if let Some(pipeline) = pipeline_weak.upgrade() {
                    replacing_bus.set(true);
                    if let Err(e) = replace_source(
                        &pipeline,
                        &sink_queue_clone,
                        &config_clone.url,
                        &state_clone,
                        "bus error",
                    ) {
                        eprintln!("Failed to rebuild source after error: {e:?}");
                        loop_clone.quit();
                        return glib::ControlFlow::Break;
                    }
                    replacing_bus.set(false);

                    if let Some(limit) = config_clone.max_restarts {
                        if state_clone.borrow().restarts >= limit {
                            println!("Reached max restarts ({limit}), stopping main loop");
                            loop_clone.quit();
                            return glib::ControlFlow::Break;
                        }
                    }
                }
            }
            MessageView::Warning(warning) => {
                eprintln!(
                    "Warning from {:?}: {} ({:?})",
                    warning.src().map(|s| s.path_string()),
                    warning.error(),
                    warning.debug()
                );
            }
            MessageView::Info(info) => {
                println!(
                    "Info from {:?}: {} ({:?})",
                    info.src().map(|s| s.path_string()),
                    info.error(),
                    info.debug()
                );
            }
            MessageView::Progress(progress) => {
                let (ty, _code, text) = progress.get();
                println!("Progress {:?}: {}", ty, text.replace("\"", ""));
            }
            MessageView::Element(element) => {
                if let Some(structure) = element.structure() {
                    match structure.name().as_str() {
                        "rtsp-connection-retry" => {
                            let attempt = structure.get::<u32>("attempt").unwrap_or_default();
                            let delay_ms =
                                structure.get::<u64>("next-delay-ms").unwrap_or_default();
                            let err = structure
                                .get::<String>("error")
                                .unwrap_or_else(|_| "(unknown error)".into());
                            println!(
                                "Connection retry #{attempt} scheduled in {delay_ms}ms: {err}"
                            );
                        }
                        "rtsp-reconnection-attempt" => {
                            let reason = structure
                                .get::<String>("reason")
                                .unwrap_or_else(|_| "unknown".into());
                            println!("Reconnection attempt starting ({reason})");
                        }
                        "rtsp-reconnection-success" => {
                            let elapsed_ms = structure.get::<u64>("elapsed-ms").unwrap_or_default();
                            println!("Reconnection succeeded in {elapsed_ms}ms");
                        }
                        "rtsp-reconnection-failed" => {
                            let err = structure
                                .get::<String>("error")
                                .unwrap_or_else(|_| "(unknown error)".into());
                            eprintln!("Reconnection failed: {err}");
                        }
                        "rtsp-success" | "rtsp-error" | "rtsp-parameters" => {
                            println!("RTSP message: {structure:?}");
                        }
                        other => {
                            println!("Element message {other}: {structure:?}");
                        }
                    }
                }
            }
            MessageView::Eos(..) => {
                println!("EOS received, stopping main loop");
                loop_clone.quit();
                return glib::ControlFlow::Break;
            }
            MessageView::StateChanged(state_changed)
                if state_changed
                    .src()
                    .map(|s| s.is::<gst::Pipeline>())
                    .unwrap_or(false) =>
            {
                let new_state = state_changed.current();
                println!("Pipeline state changed to {new_state:?}");
            }
            _ => {}
        }

        glib::ControlFlow::Continue
    })?;

    if let Some(interval) = config.restart_interval {
        let jitter = sanitize_jitter(config.restart_jitter.unwrap_or(0.25));
        schedule_periodic_restart(
            interval,
            jitter,
            pipeline.downgrade(),
            sink_queue.clone(),
            config.clone(),
            state.clone(),
            replacing.clone(),
            main_loop.clone(),
        );
    }

    // Add periodic frame counter reporting
    if let (Some(counter), Some(pts_tracker)) = (frame_counter, last_pts) {
        let start_time = Instant::now();
        let mut last_count = 0u64;
        let mut last_time = start_time;
        
        glib::timeout_add_local(Duration::from_secs(1), move || {
            let current_count = counter.load(Ordering::Relaxed);
            let now = Instant::now();
            let wall_elapsed = now.duration_since(last_time).as_secs_f64();
            let total_wall_elapsed = now.duration_since(start_time).as_secs_f64();
            
            let frames_in_period = current_count.saturating_sub(last_count);
            
            // Instantaneous FPS based on wall-clock (shows actual delivery rate over last period)
            let instant_fps = if wall_elapsed > 0.0 && frames_in_period > 0 {
                frames_in_period as f64 / wall_elapsed
            } else {
                0.0
            };
            
            // Average FPS based on total wall-clock time since start
            let avg_fps = if total_wall_elapsed > 0.0 && current_count > 0 {
                current_count as f64 / total_wall_elapsed
            } else {
                0.0
            };
            
            println!(
                "Frame stats: {} total frames, {:.1} fps (last 1s), {:.1} fps (avg)",
                current_count, instant_fps, avg_fps
            );
            
            last_count = current_count;
            last_time = now;
            
            glib::ControlFlow::Continue
        });
    }

    println!("Press Ctrl+C to stop the test. Waiting for reconnections...");
    main_loop.run();

    println!("Shutting down pipeline");
    pipeline.set_state(gst::State::Null).ok();

    Ok(())
}
