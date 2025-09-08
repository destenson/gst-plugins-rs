use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;
use stream_manager::config::ConfigManager;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.toml")]
    config: PathBuf,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Bind address for REST API
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.debug {
        EnvFilter::new("debug")
    } else {
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"))
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("Starting Stream Manager");
    info!("Configuration file: {:?}", args.config);
    info!("API bind address: {}", args.bind);

    // Initialize GStreamer
    gst::init()?;
    info!("GStreamer initialized successfully");

    // Load configuration
    let mut config_manager = ConfigManager::new(args.config).await?;
    let config = config_manager.get().await;
    info!("Configuration loaded successfully");
    info!("App name: {}", config.app.name);
    info!("Configured {} streams", config.streams.len());
    
    // Start configuration file watching
    config_manager.start_watching().await?;
    info!("Configuration hot-reload enabled");
    
    // TODO: Initialize pipeline manager (PRP-04)
    // TODO: Start stream manager (PRP-09)
    // TODO: Start REST API server (PRP-11)
    // TODO: Start health monitoring (PRP-10)

    // For now, just run a simple event loop
    tokio::signal::ctrl_c().await?;
    info!("Shutting down Stream Manager");

    Ok(())
}