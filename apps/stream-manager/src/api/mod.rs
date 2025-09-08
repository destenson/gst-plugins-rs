// REST API implementation
// TODO: Implement in PRP-11 and PRP-12

use actix_web::HttpResponse;

pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "healthy"
    }))
}