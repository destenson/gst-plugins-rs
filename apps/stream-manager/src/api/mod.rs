use actix_web::{
    web, App, HttpServer,
    middleware::{Logger, NormalizePath},
};
use std::sync::Arc;
use tracing::info;
use crate::{Config, manager::StreamManager};

pub mod routes;
pub mod dto;
pub mod error;
pub mod middleware;
pub mod streams;

pub use error::ApiError;
pub use dto::*;

#[derive(Clone)]
pub struct AppState {
    pub stream_manager: Arc<StreamManager>,
    pub config: Arc<Config>,
}

impl AppState {
    pub fn new(stream_manager: Arc<StreamManager>, config: Arc<Config>) -> Self {
        Self {
            stream_manager,
            config,
        }
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