use actix_web::{
    error::ResponseError,
    http::StatusCode,
    HttpResponse,
};
use serde_json::json;
use std::fmt;
use tracing::error;

#[derive(Debug)]
pub enum ApiError {
    BadRequest(String),
    NotFound(String),
    InternalError(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    ValidationError(String),
    ServiceUnavailable(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            ApiError::InternalError(msg) => write!(f, "Internal Server Error: {}", msg),
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            ApiError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            ApiError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            ApiError::ValidationError(msg) => write!(f, "Validation Error: {}", msg),
            ApiError::ServiceUnavailable(msg) => write!(f, "Service Unavailable: {}", msg),
        }
    }
}

impl ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match self {
            ApiError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            ApiError::Forbidden(_) => StatusCode::FORBIDDEN,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::ValidationError(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_type = match self {
            ApiError::BadRequest(_) => "bad_request",
            ApiError::NotFound(_) => "not_found",
            ApiError::InternalError(_) => "internal_error",
            ApiError::Unauthorized(_) => "unauthorized",
            ApiError::Forbidden(_) => "forbidden",
            ApiError::Conflict(_) => "conflict",
            ApiError::ValidationError(_) => "validation_error",
            ApiError::ServiceUnavailable(_) => "service_unavailable",
        };

        let message = self.to_string();
        
        // Log the error
        match self {
            ApiError::InternalError(_) => error!("{}", message),
            ApiError::ServiceUnavailable(_) => error!("{}", message),
            _ => tracing::warn!("{}", message),
        }

        HttpResponse::build(status).json(json!({
            "error": error_type,
            "message": message,
            "status_code": status.as_u16(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        }))
    }
}

impl From<crate::StreamManagerError> for ApiError {
    fn from(err: crate::StreamManagerError) -> Self {
        match err {
            crate::StreamManagerError::StreamNotFound(msg) => ApiError::NotFound(msg),
            crate::StreamManagerError::ConfigError(msg) => ApiError::BadRequest(msg),
            crate::StreamManagerError::StorageError(msg) => ApiError::InternalError(msg),
            crate::StreamManagerError::ApiError(msg) => ApiError::BadRequest(msg),
            _ => ApiError::InternalError(err.to_string()),
        }
    }
}

impl From<actix_web::error::Error> for ApiError {
    fn from(err: actix_web::error::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        ApiError::InternalError(err.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("JSON error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn test_api_error_status_codes() {
        assert_eq!(ApiError::BadRequest("test".to_string()).status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::NotFound("test".to_string()).status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ApiError::InternalError("test".to_string()).status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(ApiError::Unauthorized("test".to_string()).status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::Forbidden("test".to_string()).status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ApiError::Conflict("test".to_string()).status_code(), StatusCode::CONFLICT);
        assert_eq!(ApiError::ValidationError("test".to_string()).status_code(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(ApiError::ServiceUnavailable("test".to_string()).status_code(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn test_api_error_display() {
        let error = ApiError::BadRequest("Invalid input".to_string());
        assert_eq!(format!("{}", error), "Bad Request: Invalid input");
        
        let error = ApiError::NotFound("Stream not found".to_string());
        assert_eq!(format!("{}", error), "Not Found: Stream not found");
    }

    #[test]
    fn test_stream_manager_error_conversion() {
        let stream_error = crate::StreamManagerError::StreamNotFound("test-stream".to_string());
        let api_error: ApiError = stream_error.into();
        assert!(matches!(api_error, ApiError::NotFound(_)));
        
        let config_error = crate::StreamManagerError::ConfigError("Invalid config".to_string());
        let api_error: ApiError = config_error.into();
        assert!(matches!(api_error, ApiError::BadRequest(_)));
    }
}