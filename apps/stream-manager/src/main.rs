use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, error};
use tracing_subscriber::EnvFilter;
use stream_manager::{
    config::ConfigManager,
    gst_utils,
    manager::StreamManager,
    recovery::{RecoveryManager, RecoveryConfig},
    service::ServiceManager,
    storage::{DiskRotationManager, DiskRotationConfig},
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/stream-manager/config.toml")]
    config: PathBuf,

    /// Enable debug logging
    #[arg(short, long)]
    debug: bool,

    /// Bind address for REST API
    #[arg(long, default_value = "0.0.0.0:8080")]
    bind: String,
    
    /// Check available GStreamer plugins and exit
    #[arg(long)]
    check_plugins: bool,
    
    /// Run as systemd service
    #[arg(long)]
    service: bool,
    
    /// Run in foreground (don't daemonize)
    #[arg(long)]
    foreground: bool,
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

    // Use different logging depending on if we're running as a service
    if args.service && !args.foreground {
        // When running as systemd service, use simpler output without timestamps
        // (systemd adds its own timestamps)
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .without_time()
            .init();
    } else {
        // Full logging for interactive use
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true)
            .init();
    }

    info!("Starting Stream Manager v{}", env!("CARGO_PKG_VERSION"));
    info!("Configuration file: {:?}", args.config);
    info!("API bind address: {}", args.bind);

    // Initialize GStreamer and discover plugins
    let gst_capabilities = gst_utils::initialize()?;
    info!("GStreamer initialized successfully");
    
    // If --check-plugins flag is set, print info and exit
    if args.check_plugins {
        gst_utils::print_plugin_info(&gst_capabilities);
        return Ok(());
    }

    // Load configuration
    let mut config_manager = ConfigManager::new(args.config.clone()).await?;
    let config = Arc::new(config_manager.get().await.clone());
    info!("Configuration loaded successfully");
    info!("App name: {}", config.app.name);
    
    // Initialize recovery manager
    let recovery_config = RecoveryConfig::default();
    let recovery_manager = Arc::new(RecoveryManager::new(recovery_config));
    info!("Recovery manager initialized");
    
    // Initialize components
    let stream_manager = Arc::new(StreamManager::new(config.clone())?);
    info!("Stream manager initialized");
    
    // Register recovery handlers for streams
    {
        let sm = stream_manager.clone();
        recovery_manager.register_recovery_handler(
            "stream".to_string(),
            Box::new(move |snapshot| {
                // Recovery logic for streams will be implemented later
                stream_manager::recovery::RecoveryResult::Recovered
            }),
        ).await;
    }
    
    let disk_rotation_config = if let Some(ref storage_config) = config.storage {
        DiskRotationConfig {
            auto_rotate_on_unmount: storage_config.auto_rotate.unwrap_or(true),
            buffer_size_mb: storage_config.buffer_size_mb.unwrap_or(512) as usize,
            migration_timeout_secs: storage_config.migration_timeout_secs.unwrap_or(30),
            poll_interval_secs: storage_config.poll_interval_secs.unwrap_or(5),
            min_free_space_gb: storage_config.min_free_space_gb as f64,
        }
    } else {
        DiskRotationConfig::default()
    };
    
    let disk_rotation_manager = Arc::new(DiskRotationManager::new(disk_rotation_config));
    info!("Disk rotation manager initialized");
    
    if args.service {
        // Run as systemd service
        info!("Running as systemd service");
        
        let service_manager = ServiceManager::new(
            config.clone(),
            stream_manager,
            disk_rotation_manager,
        )?.with_recovery_manager(recovery_manager.clone());
        
        // Run the service (blocks until shutdown)
        if let Err(e) = service_manager.run().await {
            error!("Service error: {}", e);
            return Err(e.into());
        }
    } else {
        // Run in standalone mode
        info!("Running in standalone mode");
        
        // Start configuration file watching only if config file exists
        if args.config.exists() {
            config_manager.start_watching().await?;
            info!("Configuration hot-reload enabled");
        } else {
            info!("Running with default configuration (no hot-reload)");
        }
        
        // Start disk rotation monitoring
        disk_rotation_manager.start_monitoring().await?;
        
        // Start API server (actix-web spawns its own runtime)
        let api_config = config.clone();
        let api_manager = stream_manager.clone();
        std::thread::spawn(move || {
            let runtime = actix_rt::System::new();
            runtime.block_on(async move {
                if let Err(e) = stream_manager::api::start_server(
                    api_config,
                    api_manager,
                ).await {
                    error!("API server error: {}", e);
                }
            });
        });
        
        // Wait for shutdown signal
        tokio::signal::ctrl_c().await?;
        info!("Received Ctrl+C, shutting down");
        
        info!("Shutting down Stream Manager");
        
        // Cleanup
        let streams = stream_manager.list_streams().await;
        for stream in streams {
            if let Err(e) = stream_manager.remove_stream(&stream.id).await {
                error!("Failed to stop stream {}: {}", stream.id, e);
            }
        }
    }

    Ok(())
}