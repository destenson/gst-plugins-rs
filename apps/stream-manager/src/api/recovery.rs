//! Recovery API endpoints
//!
//! This module provides REST API endpoints for recovery and maintenance operations.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, error, debug};
use crate::api::{AppState, ApiError};

/// Configure recovery API routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/recovery")
            .route("/status", web::get().to(get_recovery_status))
            .route("/check-integrity", web::post().to(check_integrity))
            .route("/sync-recordings", web::post().to(sync_recordings))
            .route("/reset-recordings", web::post().to(reset_recordings))
            .route("/rebuild-database", web::post().to(rebuild_database))
    );
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

/// Check database integrity
async fn check_integrity(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    info!("Checking database integrity");
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Recovery manager not configured".to_string()))?;
    
    match backup_manager.check_database_integrity().await {
        Ok(is_ok) => {
            Ok(HttpResponse::Ok().json(json!({
                "status": if is_ok { "ok" } else { "recovered" },
                "message": if is_ok { 
                    "Database integrity check passed" 
                } else { 
                    "Database was corrupted but has been recovered" 
                },
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Database integrity check failed: {}", e);
            Err(ApiError::InternalError(format!("Database integrity check failed: {}", e)))
        }
    }
}

/// Sync recordings with database
async fn sync_recordings(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    info!("Syncing recordings with database");
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Recovery manager not configured".to_string()))?;
    
    match backup_manager.sync_recordings().await {
        Ok(result) => {
            info!("Recording sync completed: {:?}", result);
            Ok(HttpResponse::Ok().json(json!({
                "status": "success",
                "files_removed": result.files_removed,
                "files_added": result.files_added,
                "db_entries_removed": result.db_entries_removed,
                "has_changes": result.has_changes(),
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Recording sync failed: {}", e);
            Err(ApiError::InternalError(format!("Recording sync failed: {}", e)))
        }
    }
}

/// Request to reset recordings
#[derive(Debug, Deserialize)]
struct ResetRecordingsRequest {
    confirm: bool,
    #[serde(default)]
    delete_files: bool,
}

/// Reset all recordings
async fn reset_recordings(
    state: web::Data<AppState>,
    req: web::Json<ResetRecordingsRequest>,
) -> Result<HttpResponse, ApiError> {
    if !req.confirm {
        return Err(ApiError::BadRequest("Reset must be confirmed with confirm=true".to_string()));
    }
    
    info!("Resetting all recordings (delete_files={})", req.delete_files);
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Recovery manager not configured".to_string()))?;
    
    match backup_manager.reset_recordings(true).await {
        Ok(result) => {
            info!("Reset complete: {} files deleted, {} DB entries removed", 
                  result.files_deleted, result.db_entries_deleted);
            
            Ok(HttpResponse::Ok().json(json!({
                "status": "success",
                "files_deleted": result.files_deleted,
                "db_entries_deleted": result.db_entries_deleted,
                "message": "All recordings have been reset",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Reset recordings failed: {}", e);
            Err(ApiError::InternalError(format!("Reset recordings failed: {}", e)))
        }
    }
}

/// Rebuild database from filesystem
async fn rebuild_database(state: web::Data<AppState>) -> Result<HttpResponse, ApiError> {
    info!("Rebuilding database from filesystem");
    
    let backup_manager = state.backup_manager.as_ref()
        .ok_or_else(|| ApiError::NotFound("Recovery manager not configured".to_string()))?;
    
    match backup_manager.rebuild_database_from_filesystem().await {
        Ok(()) => {
            info!("Database rebuilt successfully from filesystem");
            Ok(HttpResponse::Ok().json(json!({
                "status": "success",
                "message": "Database rebuilt from filesystem",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Database rebuild failed: {}", e);
            Err(ApiError::InternalError(format!("Database rebuild failed: {}", e)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[actix_rt::test]
    async fn test_recovery_endpoints() {
        // TODO: Add tests for recovery endpoints
    }
}
