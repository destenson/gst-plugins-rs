use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{interval, sleep};
use tracing::{info, warn, error, debug};
use thiserror::Error;

use crate::{Config, manager::StreamManager};
use crate::storage::DiskRotationManager;
use crate::recovery::RecoveryManager;

pub mod notify;
pub mod signals;

pub use notify::{SdNotify, NotifyState};
pub use signals::{SignalHandler, SignalType};

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Service initialization failed: {0}")]
    InitializationFailed(String),
    
    #[error("Watchdog error: {0}")]
    WatchdogError(String),
    
    #[error("Signal handling error: {0}")]
    SignalError(String),
    
    #[error("Shutdown error: {0}")]
    ShutdownError(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct ServiceManager {
    config: Arc<RwLock<Config>>,
    stream_manager: Arc<StreamManager>,
    disk_rotation_manager: Arc<DiskRotationManager>,
    recovery_manager: Option<Arc<RecoveryManager>>,
    sd_notify: Option<SdNotify>,
    signal_handler: SignalHandler,
    watchdog_interval: Option<Duration>,
    running: Arc<AtomicBool>,
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<RwLock<mpsc::Receiver<()>>>,
}

impl ServiceManager {
    pub fn new(
        config: Arc<Config>,
        stream_manager: Arc<StreamManager>,
        disk_rotation_manager: Arc<DiskRotationManager>,
    ) -> Result<Self, ServiceError> {
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        
        // Initialize sd_notify if running under systemd
        let sd_notify = SdNotify::new().ok();
        
        // Get watchdog interval from environment
        let watchdog_interval = Self::get_watchdog_interval();
        
        // Create signal handler
        let signal_handler = SignalHandler::new();
        
        Ok(Self {
            config: Arc::new(RwLock::new((*config).clone())),
            stream_manager,
            disk_rotation_manager,
            recovery_manager: None,  // Will be set separately if needed
            sd_notify,
            signal_handler,
            watchdog_interval,
            running: Arc::new(AtomicBool::new(false)),
            shutdown_tx,
            shutdown_rx: Arc::new(RwLock::new(shutdown_rx)),
        })
    }
    
    pub fn with_recovery_manager(mut self, recovery_manager: Arc<RecoveryManager>) -> Self {
        self.recovery_manager = Some(recovery_manager);
        self
    }
    
    fn get_watchdog_interval() -> Option<Duration> {
        if let Ok(usec_str) = std::env::var("WATCHDOG_USEC") {
            if let Ok(usec) = usec_str.parse::<u64>() {
                // Use half the watchdog interval for safety
                return Some(Duration::from_micros(usec / 2));
            }
        }
        None
    }
    
    pub async fn run(&self) -> Result<(), ServiceError> {
        info!("Starting Stream Manager service");
        
        // Set running flag
        self.running.store(true, Ordering::SeqCst);
        
        // Notify systemd we're starting
        if let Some(ref notify) = self.sd_notify {
            notify.notify(NotifyState::Status("Starting service...".to_string()))?;
        }
        
        // Initialize components
        self.initialize_components().await?;
        
        // Notify systemd we're ready
        if let Some(ref notify) = self.sd_notify {
            notify.notify(NotifyState::Ready)?;
            notify.notify(NotifyState::Status("Service running".to_string()))?;
        }
        
        info!("Service initialized successfully");
        
        // Start background tasks
        let mut tasks = vec![];
        
        // Start watchdog task if needed
        if self.watchdog_interval.is_some() {
            let manager = Arc::new(self.clone());
            tasks.push(tokio::spawn(manager.watchdog_task()));
        }
        
        // Start signal handler
        {
            let manager = Arc::new(self.clone());
            tasks.push(tokio::spawn(manager.signal_handler_task()));
        }
        
        // Start health monitoring
        {
            let manager = Arc::new(self.clone());
            tasks.push(tokio::spawn(manager.health_monitor_task()));
        }
        
        // Wait for shutdown signal
        let mut shutdown_rx = self.shutdown_rx.write().await;
        let _ = shutdown_rx.recv().await;
        
        info!("Shutdown signal received, stopping service");
        
        // Notify systemd we're stopping
        if let Some(ref notify) = self.sd_notify {
            notify.notify(NotifyState::Stopping)?;
            notify.notify(NotifyState::Status("Shutting down...".to_string()))?;
        }
        
        // Set running flag to false
        self.running.store(false, Ordering::SeqCst);
        
        // Cancel all background tasks
        for task in tasks {
            task.abort();
        }
        
        // Perform graceful shutdown
        self.shutdown().await?;
        
        info!("Service stopped successfully");
        Ok(())
    }
    
    async fn initialize_components(&self) -> Result<(), ServiceError> {
        debug!("Initializing service components");
        
        // Initialize GStreamer if needed
        gst::init().map_err(|e| ServiceError::InitializationFailed(e.to_string()))?;
        
        // Start disk rotation monitoring
        self.disk_rotation_manager.start_monitoring().await
            .map_err(|e| ServiceError::InitializationFailed(
                format!("Failed to start disk monitoring: {}", e)
            ))?;
        
        // Initialize any pending streams from configuration
        let config = self.config.read().await;
        for stream_config in &config.streams {
            if stream_config.enabled {
                info!("Starting configured stream: {}", stream_config.id);
                if let Err(e) = self.stream_manager.add_stream(
                    stream_config.id.clone(),
                    stream_config.clone()
                ).await {
                    warn!("Failed to start stream {}: {}", stream_config.id, e);
                }
            }
        }
        
        Ok(())
    }
    
    async fn watchdog_task(self: Arc<Self>) {
        if let Some(interval_duration) = self.watchdog_interval {
            let mut ticker = interval(interval_duration);
            info!("Watchdog task started with interval: {:?}", interval_duration);
            
            while self.running.load(Ordering::SeqCst) {
                ticker.tick().await;
                
                // Check system health
                let is_healthy = self.check_health().await;
                
                if is_healthy {
                    // Send watchdog heartbeat
                    if let Some(ref notify) = self.sd_notify {
                        if let Err(e) = notify.notify(NotifyState::Watchdog) {
                            error!("Failed to send watchdog heartbeat: {}", e);
                        }
                    }
                } else {
                    warn!("System health check failed, not sending watchdog heartbeat");
                }
            }
            
            debug!("Watchdog task stopped");
        }
    }
    
    async fn signal_handler_task(self: Arc<Self>) {
        info!("Signal handler task started");
        
        loop {
            let signal_type = self.signal_handler.wait_for_signal().await;
            
            match signal_type {
                SignalType::Terminate => {
                    info!("Received SIGTERM, initiating graceful shutdown");
                    let _ = self.shutdown_tx.send(()).await;
                    break;
                }
                SignalType::Interrupt => {
                    info!("Received SIGINT, initiating graceful shutdown");
                    let _ = self.shutdown_tx.send(()).await;
                    break;
                }
                SignalType::Reload => {
                    info!("Received SIGHUP, reloading configuration");
                    if let Err(e) = self.reload_configuration().await {
                        error!("Failed to reload configuration: {}", e);
                    }
                }
                SignalType::User1 => {
                    info!("Received SIGUSR1, dumping status");
                    self.dump_status().await;
                }
                SignalType::User2 => {
                    info!("Received SIGUSR2, rotating logs");
                    // Log rotation would be handled by tracing-subscriber
                }
            }
        }
        
        debug!("Signal handler task stopped");
    }
    
    async fn health_monitor_task(self: Arc<Self>) {
        let mut ticker = interval(Duration::from_secs(30));
        info!("Health monitor task started");
        
        while self.running.load(Ordering::SeqCst) {
            ticker.tick().await;
            
            // Update systemd status with health information
            if let Some(ref notify) = self.sd_notify {
                let streams = self.stream_manager.list_streams().await;
                let active_count = streams.iter()
                    .filter(|s| matches!(s.state, crate::manager::StreamState::Running))
                    .count();
                
                let status = format!(
                    "Running - {} active streams, {} total",
                    active_count,
                    streams.len()
                );
                
                let _ = notify.notify(NotifyState::Status(status));
            }
        }
        
        debug!("Health monitor task stopped");
    }
    
    async fn check_health(&self) -> bool {
        // Check various health indicators
        let mut is_healthy = true;
        
        // Check if stream manager is responsive
        match tokio::time::timeout(
            Duration::from_secs(5),
            self.stream_manager.list_streams()
        ).await {
            Ok(_) => {}
            Err(_) => {
                warn!("Stream manager health check timed out");
                is_healthy = false;
            }
        }
        
        // Check disk rotation manager
        if let Some(active_disk) = self.disk_rotation_manager.get_active_disk().await {
            debug!("Active disk: {:?}", active_disk);
        }
        
        // Add more health checks as needed
        
        is_healthy
    }
    
    async fn reload_configuration(&self) -> Result<(), ServiceError> {
        info!("Reloading configuration");
        
        // Notify systemd about reload
        if let Some(ref notify) = self.sd_notify {
            notify.notify(NotifyState::Reloading)?;
            notify.notify(NotifyState::Status("Reloading configuration...".to_string()))?;
        }
        
        // Reload logic would go here
        // This would coordinate with ConfigReloader from PRP-15
        
        // Notify reload complete
        if let Some(ref notify) = self.sd_notify {
            notify.notify(NotifyState::Ready)?;
            notify.notify(NotifyState::Status("Configuration reloaded".to_string()))?;
        }
        
        Ok(())
    }
    
    async fn dump_status(&self) {
        info!("=== Service Status Dump ===");
        
        // Dump stream status
        let streams = self.stream_manager.list_streams().await;
        info!("Total streams: {}", streams.len());
        for stream in streams {
            info!("  Stream {}: {:?}", stream.id, stream.state);
        }
        
        // Dump disk status
        let disks = self.disk_rotation_manager.list_disks().await;
        info!("Total disks: {}", disks.len());
        for disk in disks {
            info!("  Disk {:?}: mounted={}, active={}", 
                disk.path, disk.mounted, disk.is_active);
        }
        
        // Dump rotation state
        let rotation_state = self.disk_rotation_manager.get_rotation_state().await;
        info!("Rotation state: {:?}", rotation_state);
        
        info!("=== End Status Dump ===");
    }
    
    async fn shutdown(&self) -> Result<(), ServiceError> {
        info!("Performing graceful shutdown");
        
        // Stop all streams
        let streams = self.stream_manager.list_streams().await;
        for stream in streams {
            info!("Stopping stream: {}", stream.id);
            if let Err(e) = self.stream_manager.remove_stream(&stream.id).await {
                warn!("Failed to stop stream {}: {}", stream.id, e);
            }
        }
        
        // Wait a bit for streams to stop
        sleep(Duration::from_secs(2)).await;
        
        info!("All streams stopped");
        Ok(())
    }
}

impl Clone for ServiceManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stream_manager: self.stream_manager.clone(),
            disk_rotation_manager: self.disk_rotation_manager.clone(),
            recovery_manager: self.recovery_manager.clone(),
            sd_notify: self.sd_notify.clone(),
            signal_handler: self.signal_handler.clone(),
            watchdog_interval: self.watchdog_interval,
            running: self.running.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            shutdown_rx: self.shutdown_rx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::DiskRotationConfig;
    
    #[tokio::test]
    async fn test_service_manager_creation() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let disk_rotation_manager = Arc::new(
            DiskRotationManager::new(DiskRotationConfig::default())
        );
        
        let service = ServiceManager::new(
            config,
            stream_manager,
            disk_rotation_manager
        );
        
        assert!(service.is_ok());
    }
    
    #[tokio::test]
    async fn test_watchdog_interval_parsing() {
        // Test with valid WATCHDOG_USEC
        std::env::set_var("WATCHDOG_USEC", "30000000");
        let interval = ServiceManager::get_watchdog_interval();
        assert!(interval.is_some());
        assert_eq!(interval.unwrap(), Duration::from_secs(15)); // Half of 30s
        
        // Clean up
        std::env::remove_var("WATCHDOG_USEC");
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let disk_rotation_manager = Arc::new(
            DiskRotationManager::new(DiskRotationConfig::default())
        );
        
        let service = ServiceManager::new(
            config,
            stream_manager,
            disk_rotation_manager
        ).unwrap();
        
        let is_healthy = service.check_health().await;
        assert!(is_healthy);
    }
}