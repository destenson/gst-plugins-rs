use actix_web::{
    web, App, HttpServer,
    middleware::{Logger, NormalizePath},
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;
use crate::{Config, manager::StreamManager, config::ConfigReloader, storage::DiskRotationManager};

pub mod routes;
pub mod dto;
pub mod error;
pub mod middleware;
pub mod streams;
pub mod websocket;
pub mod event_integration;
pub mod rotation;

pub use error::ApiError;
pub use dto::*;

#[derive(Clone)]
pub struct AppState {
    pub stream_manager: Arc<StreamManager>,
    pub config: Arc<RwLock<Config>>,
    pub config_reloader: Option<Arc<RwLock<ConfigReloader>>>,
    pub metrics_collector: Arc<crate::metrics::MetricsCollector>,
    pub event_broadcaster: Arc<websocket::EventBroadcaster>,
    pub disk_rotation_manager: Arc<DiskRotationManager>,
}

impl AppState {
    pub fn new(stream_manager: Arc<StreamManager>, config: Arc<Config>) -> Self {
        let metrics_collector = Arc::new(
            crate::metrics::MetricsCollector::new(
                stream_manager.clone(),
                Some(&config.monitoring.metrics),
            )
            .expect("Failed to create metrics collector")
        );
        
        let event_broadcaster = Arc::new(websocket::EventBroadcaster::new());
        event_broadcaster.start();
        
        let disk_rotation_manager = Arc::new(
            DiskRotationManager::new(crate::storage::DiskRotationConfig::default())
        );
        
        Self {
            stream_manager,
            config: Arc::new(RwLock::new((*config).clone())),
            config_reloader: None,
            metrics_collector,
            event_broadcaster,
            disk_rotation_manager,
        }
    }
    
    pub fn with_metrics(
        stream_manager: Arc<StreamManager>,
        config: Arc<Config>,
        metrics_collector: Arc<crate::metrics::MetricsCollector>,
    ) -> Self {
        let event_broadcaster = Arc::new(websocket::EventBroadcaster::new());
        event_broadcaster.start();
        
        let disk_rotation_manager = Arc::new(
            DiskRotationManager::new(crate::storage::DiskRotationConfig::default())
        );
        
        Self {
            stream_manager,
            config: Arc::new(RwLock::new((*config).clone())),
            config_reloader: None,
            metrics_collector,
            event_broadcaster,
            disk_rotation_manager,
        }
    }
    
    pub fn with_reloader(mut self, config_path: std::path::PathBuf) -> Self {
        if let Ok(reloader) = ConfigReloader::new(self.config.clone(), config_path) {
            self.config_reloader = Some(Arc::new(RwLock::new(reloader)));
        }
        self
    }
}

pub async fn start_server(
    config: Arc<Config>,
    stream_manager: Arc<StreamManager>,
) -> std::io::Result<()> {
    let bind_address = format!("{}:{}", config.api.host, config.api.port);
    info!("Starting API server on {}", bind_address);
    
    let app_state = AppState::new(stream_manager, config.clone());
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(app_state.event_broadcaster.clone()))
            .app_data(web::Data::new(app_state.disk_rotation_manager.clone()))
            .wrap(Logger::default())
            .wrap(NormalizePath::trim())
            .wrap(middleware::error_handler())
            .wrap(middleware::request_logger())
            .configure(routes::configure_routes)
    })
    .bind(&bind_address)?
    .workers(config.api.worker_threads.unwrap_or(4))
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_web::test]
    async fn test_app_state_creation() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let app_state = AppState::new(stream_manager, config);
        
        assert!(Arc::strong_count(&app_state.stream_manager) > 0);
        assert!(Arc::strong_count(&app_state.config) > 0);
    }
    
    #[actix_web::test]
    async fn test_server_configuration() {
        let config = Arc::new(Config::default());
        let stream_manager = Arc::new(StreamManager::new(config.clone()).unwrap());
        let app_state = AppState::new(stream_manager, config);
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .configure(routes::configure_routes)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/api/v1/health")
            .to_request();
        
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}