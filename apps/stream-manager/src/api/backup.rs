//! Backup API endpoints
//!
//! This module provides REST API endpoints for backup and recovery operations.

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, error, debug};
use crate::api::{AppState, ApiError};
use std::sync::Arc;

/// Configure backup API routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/backup")
            .route("/recovery/status", web::get().to(get_recovery_status))
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

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, App};
    
    #[actix_rt::test]
    async fn test_backup_endpoints() {
        // TODO: Add comprehensive tests for backup endpoints
    }
}
