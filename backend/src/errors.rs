use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DefiantError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Authentication error: {0}")]
    AuthenticationError(String),
    
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    
    #[error("Payment error: {0}")]
    PaymentError(String),
    
    #[error("Webhook error: {0}")]
    WebhookError(String),
    
    #[error("Rate limit exceeded")]
    RateLimitError,
    
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Internal server error")]
    InternalError,
    
    #[error("Bad request: {0}")]
    BadRequest(String),
    
    #[error("Conflict: {0}")]
    Conflict(String),
}

impl ResponseError for DefiantError {
    fn error_response(&self) -> HttpResponse {
        match self {
            DefiantError::DatabaseError(_) => {
                HttpResponse::InternalServerError().json(json!({
                    "error": "Database error",
                    "code": "DB_ERROR"
                }))
            }
            DefiantError::ValidationError(msg) => {
                HttpResponse::BadRequest().json(json!({
                    "error": msg,
                    "code": "VALIDATION_ERROR"
                }))
            }
            DefiantError::AuthenticationError(msg) => {
                HttpResponse::Unauthorized().json(json!({
                    "error": msg,
                    "code": "AUTH_ERROR"
                }))
            }
            DefiantError::AuthorizationError(msg) => {
                HttpResponse::Forbidden().json(json!({
                    "error": msg,
                    "code": "FORBIDDEN"
                }))
            }
            DefiantError::PaymentError(msg) => {
                HttpResponse::PaymentRequired().json(json!({
                    "error": msg,
                    "code": "PAYMENT_ERROR"
                }))
            }
            DefiantError::RateLimitError => {
                HttpResponse::TooManyRequests().json(json!({
                    "error": "Rate limit exceeded",
                    "code": "RATE_LIMIT"
                }))
            }
            DefiantError::NotFound(msg) => {
                HttpResponse::NotFound().json(json!({
                    "error": msg,
                    "code": "NOT_FOUND"
                }))
            }
            DefiantError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(json!({
                    "error": msg,
                    "code": "BAD_REQUEST"
                }))
            }
            DefiantError::Conflict(msg) => {
                HttpResponse::Conflict().json(json!({
                    "error": msg,
                    "code": "CONFLICT"
                }))
            }
            _ => HttpResponse::InternalServerError().json(json!({
                "error": "Internal server error",
                "code": "INTERNAL_ERROR"
            }))
        }
    }
}

impl From<validator::ValidationErrors> for DefiantError {
    fn from(err: validator::ValidationErrors) -> Self {
        let errors = err
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                let messages: Vec<String> = errors
                    .iter()
                    .map(|e| e.message.clone().unwrap_or_default().to_string())
                    .collect();
                format!("{}: {}", field, messages.join(", "))
            })
            .collect::<Vec<String>>()
            .join("; ");
        
        DefiantError::ValidationError(errors)
    }
}