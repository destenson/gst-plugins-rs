//! Backup API endpoints
//!
//! This module provides REST API endpoints for backup and recovery operations.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, error, debug};
use crate::api::{AppState, ApiError};
use crate::backup::{BackupType, BackupManager};
use std::sync::Arc;

/// Configure backup API routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/backup")
            .route("", web::get().to(list_backups))
            .route("", web::post().to(create_backup))
            .route("/{backup_id}", web::get().to(get_backup))
            .route("/{backup_id}", web::delete().to(delete_backup))
            .route("/{backup_id}/verify", web::post().to(verify_backup))
            .route("/{backup_id}/restore", web::post().to(restore_backup))
            .route("/status", web::get().to(get_backup_status))
            .route("/recovery/status", web::get().to(get_recovery_status))
    );
}

/// Request to create a backup
#[derive(Debug, Deserialize)]
struct CreateBackupRequest {
    backup_type: Option<String>,
    description: Option<String>,
}

/// Response for backup creation
#[derive(Debug, Serialize)]
struct CreateBackupResponse {
    backup_id: String,
    message: String,
    timestamp: String,
}

/// List all backups
async fn list_backups(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    debug!("Listing all backups");
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    let history = backup_manager.get_backup_history().await;
    
    Ok(HttpResponse::Ok().json(json!({
        "backups": history,
        "total": history.len()
    })))
}

/// Create a new backup
async fn create_backup(
    state: web::Data<AppState>,
    req: web::Json<CreateBackupRequest>,
) -> Result<HttpResponse, ApiError> {
    info!("Creating backup with type: {:?}", req.backup_type);
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    let backup_type = match req.backup_type.as_deref() {
        Some("full") | None => BackupType::Full,
        Some("incremental") => BackupType::Incremental,
        Some("configuration") => BackupType::Configuration,
        Some("database") => BackupType::Database,
        Some("recovery") => BackupType::Recovery,
        Some(t) => return Err(ApiError::BadRequest(format!("Invalid backup type: {}", t))),
    };
    
    match backup_manager.trigger_backup(backup_type).await {
        Ok(backup_id) => {
            info!("Backup {} created successfully", backup_id);
            Ok(HttpResponse::Created().json(CreateBackupResponse {
                backup_id: backup_id.clone(),
                message: format!("Backup {} created successfully", backup_id),
                timestamp: chrono::Utc::now().to_rfc3339(),
            }))
        }
        Err(e) => {
            error!("Failed to create backup: {}", e);
            Err(ApiError::InternalError(format!("Failed to create backup: {}", e)))
        }
    }
}

/// Get backup details
async fn get_backup(
    state: web::Data<AppState>,
    backup_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    debug!("Getting backup details for {}", backup_id);
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    let history = backup_manager.get_backup_history().await;
    
    let backup = history.iter()
        .find(|b| b.id == backup_id.as_str())
        .ok_or_else(|| ApiError::NotFound(format!("Backup {} not found", backup_id)))?;
    
    Ok(HttpResponse::Ok().json(backup))
}

/// Delete a backup
async fn delete_backup(
    _state: web::Data<AppState>,
    backup_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    // TODO: Implement backup deletion
    info!("Delete backup request for {}", backup_id);
    
    Err(ApiError::NotImplemented("Backup deletion not yet implemented".to_string()))
}

/// Verify backup integrity
async fn verify_backup(
    state: web::Data<AppState>,
    backup_id: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    info!("Verifying backup {}", backup_id);
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    match backup_manager.verify_backup(&backup_id).await {
        Ok(verified) => {
            if verified {
                Ok(HttpResponse::Ok().json(json!({
                    "backup_id": backup_id.as_str(),
                    "verified": true,
                    "message": "Backup integrity verified successfully",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })))
            } else {
                Ok(HttpResponse::Ok().json(json!({
                    "backup_id": backup_id.as_str(),
                    "verified": false,
                    "message": "Backup integrity verification failed",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                })))
            }
        }
        Err(e) => {
            error!("Failed to verify backup {}: {}", backup_id, e);
            Err(ApiError::InternalError(format!("Failed to verify backup: {}", e)))
        }
    }
}

/// Request to restore from backup
#[derive(Debug, Deserialize)]
struct RestoreBackupRequest {
    confirm: bool,
    items: Option<Vec<String>>,
}

/// Restore from backup
async fn restore_backup(
    state: web::Data<AppState>,
    backup_id: web::Path<String>,
    req: web::Json<RestoreBackupRequest>,
) -> Result<HttpResponse, ApiError> {
    if !req.confirm {
        return Err(ApiError::BadRequest("Restore must be confirmed with confirm=true".to_string()));
    }
    
    info!("Restoring from backup {}", backup_id);
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    match backup_manager.restore_from_backup(&backup_id).await {
        Ok(_) => {
            info!("Successfully restored from backup {}", backup_id);
            Ok(HttpResponse::Ok().json(json!({
                "backup_id": backup_id.as_str(),
                "message": "Backup restored successfully",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Failed to restore from backup {}: {}", backup_id, e);
            Err(ApiError::InternalError(format!("Failed to restore from backup: {}", e)))
        }
    }
}

/// Get backup system status
async fn get_backup_status(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    debug!("Getting backup system status");
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    let history = backup_manager.get_backup_history().await;
    let latest_backup = history.first();
    
    let config = state.config.read().await;
    let backup_config = config.backup.as_ref();
    
    Ok(HttpResponse::Ok().json(json!({
        "enabled": backup_config.map(|c| c.enabled).unwrap_or(false),
        "total_backups": history.len(),
        "latest_backup": latest_backup.map(|b| json!({
            "id": b.id,
            "timestamp": b.timestamp,
            "type": b.backup_type,
            "size_bytes": b.size_bytes,
            "verified": b.verified
        })),
        "configuration": backup_config.map(|c| json!({
            "interval_secs": c.interval_secs,
            "retention_count": c.retention_count,
            "compress": c.compress,
            "verify_after_backup": c.verify_after_backup,
            "include_recordings": c.include_recordings
        }))
    })))
}

/// Get recovery status
async fn get_recovery_status(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    debug!("Getting recovery status");
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Backup manager not configured".to_string()))?;
    
    let status = backup_manager.get_recovery_status().await;
    
    Ok(HttpResponse::Ok().json(json!({
        "in_progress": status.in_progress,
        "backup_id": status.backup_id,
        "items_total": status.items_total,
        "items_restored": status.items_restored,
        "errors": status.errors,
        "started_at": status.started_at.map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
        "completed_at": status.completed_at.map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_rt::test]
    async fn test_backup_endpoints() {
        // TODO: Add comprehensive tests for backup endpoints
    }
}