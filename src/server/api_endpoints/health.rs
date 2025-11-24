use crate::error::AybError;
use crate::http::structs::HealthResponse;
use actix_web::{get, HttpResponse};

#[get("/health")]
async fn health() -> Result<HttpResponse, AybError> {
    Ok(HttpResponse::Ok().json(HealthResponse {
        status: "ok".to_string(),
    }))
}
