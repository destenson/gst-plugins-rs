use actix_web::{web, HttpResponse};
use crate::api::{AppState, ApiError};
use serde_json::json;
use tracing::debug;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Configure the stream control API endpoints from PRP-12
    crate::api::streams::configure(cfg);
    
    // Configure WebSocket endpoint from PRP-14
    crate::api::websocket::configure(cfg);
    
    // Configure disk rotation endpoints from PRP-17
    crate::api::rotation::configure_routes(cfg);
    
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health_check))
            .route("/health/liveness", web::get().to(liveness_check))
            .route("/health/readiness", web::get().to(readiness_check))
            .service(
                web::scope("/config")
                    .route("", web::get().to(get_config))
                    .route("", web::put().to(update_config))
                    .route("/reload", web::post().to(reload_config))
                    .route("/reload/status", web::get().to(get_reload_status))
            )
            .service(
                web::scope("/metrics")
                    .route("", web::get().to(get_metrics))
                    .route("/prometheus", web::get().to(get_prometheus_metrics))
            )
    );
}

async fn health_check(_state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    debug!("Health check requested");
    Ok(HttpResponse::Ok().json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "service": "stream-manager"
    })))
}

async fn liveness_check() -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(json!({
        "status": "alive"
    })))
}

async fn readiness_check(_state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // TODO: Implement actual readiness check
    Ok(HttpResponse::Ok().json(json!({
        "status": "ready"
    })))
}


async fn get_config(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let config = state.config.read().await;
    Ok(HttpResponse::Ok().json(&*config))
}

async fn update_config(
    state: web::Data<AppState>,
    new_config: web::Json<crate::Config>,
) -> Result<HttpResponse, ApiError> {
    // Validate the new configuration
    new_config.validate().map_err(|e| ApiError::BadRequest(e.to_string()))?;
    
    // Update the configuration
    let mut config = state.config.write().await;
    *config = new_config.into_inner();
    
    // Notify components about config change if reloader is available
    if let Some(reloader) = &state.config_reloader {
        let reloader = reloader.read().await;
        // The reloader will detect and broadcast changes
    }
    
    Ok(HttpResponse::Ok().json(json!({
        "message": "Configuration updated successfully",
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

async fn reload_config(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    if let Some(reloader) = &state.config_reloader {
        let reloader = reloader.read().await;
        match reloader.reload_now().await {
            Ok(reload_event) => {
                Ok(HttpResponse::Ok().json(json!({
                    "message": "Configuration reloaded successfully",
                    "timestamp": reload_event.timestamp.to_rfc3339(),
                    "changes": reload_event.changes.len(),
                    "requires_restart": reload_event.requires_restart
                })))
            }
            Err(e) => {
                Err(ApiError::InternalError(format!("Failed to reload configuration: {}", e)))
            }
        }
    } else {
        Err(ApiError::InternalError("Configuration hot-reload is not enabled".to_string()))
    }
}

async fn get_reload_status(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    if let Some(reloader) = &state.config_reloader {
        let reloader = reloader.read().await;
        let status = reloader.get_status().await;
        Ok(HttpResponse::Ok().json(status))
    } else {
        Ok(HttpResponse::Ok().json(json!({
            "watching": false,
            "message": "Configuration hot-reload is not enabled"
        })))
    }
}

pub async fn get_metrics(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // Get stream statistics
    let streams = state.stream_manager.list_streams().await;
    let active_streams = streams.iter()
        .filter(|s| matches!(s.state, crate::manager::StreamState::Running))
        .count();
    
    
    Ok(HttpResponse::Ok().json(json!({
        "total_streams": streams.len(),
        "active_streams": active_streams,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

pub async fn get_prometheus_metrics(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    match state.metrics_collector.export_prometheus().await {
        Ok(metrics) => {
            Ok(HttpResponse::Ok()
                .content_type("text/plain; version=0.0.4")
                .body(metrics))
        }
        Err(e) => {
            tracing::error!("Failed to export Prometheus metrics: {}", e);
            Err(ApiError::InternalError(format!("Failed to export metrics: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_web::test]
    async fn test_route_registration() {
        let config = std::sync::Arc::new(crate::Config::default());
        let stream_manager = std::sync::Arc::new(
            crate::manager::StreamManager::new(config.clone()).unwrap()
        );
        let app_state = AppState::new(stream_manager, config.clone());
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(app_state))
                .configure(configure_routes)
        ).await;
        
        // Test health endpoint
        let req = test::TestRequest::get()
            .uri("/api/v1/health")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        
        // Test liveness endpoint
        let req = test::TestRequest::get()
            .uri("/api/v1/health/liveness")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}