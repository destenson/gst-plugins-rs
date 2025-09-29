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

    // Configure recovery endpoints
    crate::api::recovery::configure(cfg);

    // Configure database endpoints (development mode only)
    crate::api::database::register_routes(cfg);

    // Configure WebRTC endpoints from PRP-26
    // WebRTC configuration is handled by webrtc module directly

    // Configure WHIP/WHEP endpoints from PRP-27
    crate::webrtc::whip_whep::configure(cfg);

    cfg.service(
        web::scope("/api/v1")
            .route("/metrics", web::get().to(get_metrics))
            .route("/metrics/prometheus", web::get().to(get_prometheus_metrics))
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
        // TODO: The reloader will detect and broadcast changes
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
    let recording_streams = streams.iter()
        .filter(|s| s.recording_state.is_recording)
        .count();

    // Get system resource usage
    let cpu_usage = get_cpu_usage();
    let memory_usage = get_memory_usage();
    let disk_usage = get_disk_usage();

    Ok(HttpResponse::Ok().json(json!({
        "active_streams": active_streams,
        "total_streams": streams.len(),
        "recording_streams": recording_streams,
        "cpu_usage": cpu_usage,
        "memory_usage": memory_usage,
        "disk_usage": disk_usage,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

// Helper functions to get system metrics
fn get_cpu_usage() -> f64 {
    // TODO: Implement actual CPU usage collection
    // For now return mock data
    25.5
}

fn get_memory_usage() -> f64 {
    // TODO: Implement actual memory usage collection
    // For now return mock data
    512.0 // MB
}

fn get_disk_usage() -> serde_json::Value {
    // TODO: Implement actual disk usage collection
    // For now return mock data
    json!({
        "used": 15000000000u64,  // 15 GB in bytes
        "total": 100000000000u64  // 100 GB in bytes
    })
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
        // Initialize GStreamer for tests
        gst::init().ok();
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
