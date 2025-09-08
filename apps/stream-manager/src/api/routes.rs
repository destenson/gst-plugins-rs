use actix_web::{web, HttpResponse};
use crate::api::{AppState, ApiError};
use serde_json::json;
use tracing::debug;

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    // Configure the stream control API endpoints from PRP-12
    crate::api::streams::configure(cfg);
    
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

async fn get_metrics(_state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    // TODO: Implement actual metrics collection in PRP-13
    Ok(HttpResponse::Ok().json(json!({
        "total_streams": 0,
        "active_streams": 0
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