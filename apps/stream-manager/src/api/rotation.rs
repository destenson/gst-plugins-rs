use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

use crate::storage::{DiskRotationManager, RotationState, DiskInfo};
use crate::api::error::ApiError;

#[derive(Debug, Serialize)]
pub struct DiskListResponse {
    pub disks: Vec<DiskInfo>,
    pub active_disk: Option<PathBuf>,
    pub rotation_state: RotationState,
}

#[derive(Debug, Deserialize)]
pub struct TriggerRotationRequest {
    pub target_disk: Option<PathBuf>,
    pub force: bool,
}

#[derive(Debug, Serialize)]
pub struct RotationStatusResponse {
    pub state: RotationState,
    pub active_disk: Option<PathBuf>,
    pub queue_length: usize,
}

#[derive(Debug, Deserialize)]
pub struct MarkDiskRequest {
    pub disk_path: PathBuf,
    pub action: DiskAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DiskAction {
    Activate,
    Deactivate,
    Remove,
}

pub async fn list_disks(
    rotation_manager: web::Data<Arc<DiskRotationManager>>,
) -> Result<HttpResponse, ApiError> {
    info!("Listing available disks");
    
    let disks = rotation_manager.list_disks().await;
    let active_disk = rotation_manager.get_active_disk().await;
    let rotation_state = rotation_manager.get_rotation_state().await;
    
    Ok(HttpResponse::Ok().json(DiskListResponse {
        disks,
        active_disk,
        rotation_state,
    }))
}

pub async fn trigger_rotation(
    rotation_manager: web::Data<Arc<DiskRotationManager>>,
    req: web::Json<TriggerRotationRequest>,
) -> Result<HttpResponse, ApiError> {
    info!("Triggering disk rotation: {:?}", req.target_disk);
    
    match rotation_manager.trigger_rotation(req.target_disk.clone()).await {
        Ok(()) => {
            info!("Disk rotation initiated successfully");
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "rotation_started",
                "target": req.target_disk
            })))
        }
        Err(e) => {
            warn!("Failed to trigger rotation: {}", e);
            Err(ApiError::BadRequest(e.to_string()))
        }
    }
}

pub async fn rotation_status(
    rotation_manager: web::Data<Arc<DiskRotationManager>>,
) -> Result<HttpResponse, ApiError> {
    let state = rotation_manager.get_rotation_state().await;
    let active_disk = rotation_manager.get_active_disk().await;
    
    // TODO: Get queue length from rotation manager
    let queue_length = 0;
    
    Ok(HttpResponse::Ok().json(RotationStatusResponse {
        state,
        active_disk,
        queue_length,
    }))
}

pub async fn mark_disk(
    rotation_manager: web::Data<Arc<DiskRotationManager>>,
    req: web::Json<MarkDiskRequest>,
) -> Result<HttpResponse, ApiError> {
    info!("Marking disk {:?} for action: {:?}", req.disk_path, req.action);
    
    match req.action {
        DiskAction::Activate => {
            // Trigger rotation to this disk
            match rotation_manager.trigger_rotation(Some(req.disk_path.clone())).await {
                Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
                    "status": "disk_activated",
                    "path": req.disk_path
                }))),
                Err(e) => Err(ApiError::BadRequest(e.to_string()))
            }
        }
        DiskAction::Deactivate => {
            // Mark disk for rotation away
            info!("Marking disk for deactivation: {:?}", req.disk_path);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "status": "disk_marked_for_deactivation",
                "path": req.disk_path
            })))
        }
        DiskAction::Remove => {
            // Safe removal - trigger rotation if this is active disk
            let active = rotation_manager.get_active_disk().await;
            if active == Some(req.disk_path.clone()) {
                match rotation_manager.trigger_rotation(None).await {
                    Ok(()) => Ok(HttpResponse::Ok().json(serde_json::json!({
                        "status": "safe_removal_initiated",
                        "path": req.disk_path
                    }))),
                    Err(e) => Err(ApiError::BadRequest(e.to_string()))
                }
            } else {
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "status": "disk_can_be_removed",
                    "path": req.disk_path
                })))
            }
        }
    }
}

pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/rotation")
            .route("/disks", web::get().to(list_disks))
            .route("/trigger", web::post().to(trigger_rotation))
            .route("/status", web::get().to(rotation_status))
            .route("/mark", web::post().to(mark_disk))
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    use crate::storage::DiskRotationConfig;
    
    #[actix_rt::test]
    async fn test_list_disks_endpoint() {
        let rotation_manager = Arc::new(DiskRotationManager::new(DiskRotationConfig::default()));
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(rotation_manager))
                .configure(configure_routes)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/rotation/disks")
            .to_request();
            
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
    
    #[actix_rt::test]
    async fn test_rotation_status_endpoint() {
        let rotation_manager = Arc::new(DiskRotationManager::new(DiskRotationConfig::default()));
        
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(rotation_manager))
                .configure(configure_routes)
        ).await;
        
        let req = test::TestRequest::get()
            .uri("/rotation/status")
            .to_request();
            
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
