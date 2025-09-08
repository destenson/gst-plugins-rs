use actix_web::{web, HttpResponse};
use crate::api::{AppState, ApiError};
use serde_json::json;
use tracing::debug;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/health", web::get().to(health_check))
            .route("/health/liveness", web::get().to(liveness_check))
            .route("/health/readiness", web::get().to(readiness_check))
            .service(
                web::scope("/streams")
                    .route("", web::get().to(list_streams))
                    .route("", web::post().to(add_stream))
                    .route("/{stream_id}", web::get().to(get_stream))
                    .route("/{stream_id}", web::delete().to(remove_stream))
                    .route("/{stream_id}/start", web::post().to(start_stream))
                    .route("/{stream_id}/stop", web::post().to(stop_stream))
            )
            .service(
                web::scope("/config")
                    .route("", web::get().to(get_config))
                    .route("", web::put().to(update_config))
                    .route("/reload", web::post().to(reload_config))
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

async fn list_streams(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    let streams = state.stream_manager.list_streams().await;
    Ok(HttpResponse::Ok().json(streams))
}

async fn add_stream(
    state: web::Data<AppState>,
    req: web::Json<crate::api::dto::AddStreamRequest>,
) -> Result<HttpResponse, ApiError> {
    let config: crate::config::StreamConfig = req.into_inner().into();
    state.stream_manager
        .add_stream(config.id.clone(), config)
        .await
        .map_err(|e| ApiError::BadRequest(e.to_string()))?;
    
    Ok(HttpResponse::Created().json(json!({
        "message": "Stream added successfully"
    })))
}

async fn get_stream(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let stream_id = path.into_inner();
    let _stream = state.stream_manager
        .get_stream(&stream_id)
        .await
        .ok_or_else(|| ApiError::NotFound(format!("Stream not found: {}", stream_id)))?;
    
    // TODO: Convert ManagedStream to StreamResponse
    Ok(HttpResponse::Ok().json(json!({
        "id": stream_id,
        "status": "active"
    })))
}

async fn remove_stream(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let stream_id = path.into_inner();
    state.stream_manager
        .remove_stream(&stream_id)
        .await
        .map_err(|e| ApiError::NotFound(e.to_string()))?;
    
    Ok(HttpResponse::Ok().json(json!({
        "message": "Stream removed successfully"
    })))
}

async fn start_stream(
    _state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let stream_id = path.into_inner();
    // TODO: Implement individual stream start/stop in future PRP
    Ok(HttpResponse::Ok().json(json!({
        "message": format!("Stream {} start requested", stream_id),
        "note": "Individual stream control will be implemented in a future PRP"
    })))
}

async fn stop_stream(
    _state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let stream_id = path.into_inner();
    // TODO: Implement individual stream start/stop in future PRP
    Ok(HttpResponse::Ok().json(json!({
        "message": format!("Stream {} stop requested", stream_id),
        "note": "Individual stream control will be implemented in a future PRP"
    })))
}

async fn get_config(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    Ok(HttpResponse::Ok().json(&*state.config))
}

async fn update_config(
    _state: web::Data<AppState>,
    _config: web::Json<crate::Config>,
) -> Result<HttpResponse, ApiError> {
    // TODO: Implement config update logic in PRP-15
    Ok(HttpResponse::NotImplemented().json(json!({
        "message": "Config update will be implemented in PRP-15"
    })))
}

async fn reload_config(_state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // TODO: Implement config reload logic in PRP-15
    Ok(HttpResponse::NotImplemented().json(json!({
        "message": "Config reload will be implemented in PRP-15"
    })))
}

async fn get_metrics(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // TODO: Implement actual metrics collection
    let streams = state.stream_manager.list_streams().await;
    Ok(HttpResponse::Ok().json(json!({
        "total_streams": streams.len(),
        "active_streams": 0 // TODO: count active streams properly
    })))
}

async fn get_prometheus_metrics(_state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // TODO: Implement Prometheus metrics in PRP-13
    Ok(HttpResponse::NotImplemented().body("Prometheus metrics will be implemented in PRP-13"))
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
        let app_state = AppState::new(stream_manager, config);
        
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