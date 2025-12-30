use actix_web::{web, HttpResponse, HttpRequest};
use serde_json::json;
use tracing::{info, error};
use uuid::Uuid;

use crate::{models::{CreatePaymentRequest, PaymentResponse}, errors::DefiantError, AppState, services::payment_service::PaymentService};

#[utoipa::path(
    post,
    path = "/api/v1/payments",
    request_body = CreatePaymentRequest,
    responses(
        (status = 201, description = "Payment created successfully", body = PaymentResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Unauthorized"),
        (status = 402, description = "Payment required"),
        (status = 429, description = "Rate limit exceeded"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_payment(
    req: HttpRequest,
    data: web::Json<CreatePaymentRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, DefiantError> {
    info!("Creating payment for amount: {}", data.amount);
    
    // Validate input
    data.validate()?;
    
    // Check rate limiting
    check_rate_limit(&req, &state).await?;
    
    // Get API key from headers
    let api_key = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| DefiantError::AuthenticationError("Missing API key".into()))?;
    
    // Create payment service
    let payment_service = PaymentService::new(state.db.clone(), state.redis.clone());
    
    // Create payment
    let payment = payment_service.create_payment(data.into_inner(), api_key).await?;
    
    info!("Payment created: {}", payment.id);
    
    Ok(HttpResponse::Created().json(payment))
}

#[utoipa::path(
    get,
    path = "/api/v1/payments/{payment_id}",
    params(
        ("payment_id" = Uuid, Path, description = "Payment ID")
    ),
    responses(
        (status = 200, description = "Payment retrieved successfully", body = PaymentResponse),
        (status = 404, description = "Payment not found"),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_payment(
    req: HttpRequest,
    path: web::Path<Uuid>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, DefiantError> {
    let payment_id = path.into_inner();
    info!("Getting payment: {}", payment_id);
    
    // Get API key
    let api_key = req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| DefiantError::AuthenticationError("Missing API key".into()))?;
    
    let payment_service = PaymentService::new(state.db.clone(), state.redis.clone());
    let payment = payment_service.get_payment(payment_id, api_key).await?;
    
    Ok(HttpResponse::Ok().json(payment))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/{payment_id}/capture",
    params(
        ("payment_id" = Uuid, Path, description = "Payment ID")
    ),
    responses(
        (status = 200, description = "Payment captured successfully", body = PaymentResponse),
        (status = 400, description = "Cannot capture payment"),
        (status = 404, description = "Payment not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn capture_payment(
    req: HttpRequest,
    path: web::Path<Uuid>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, DefiantError> {
    let payment_id = path.into_inner();
    info!("Capturing payment: {}", payment_id);
    
    let api_key = get_api_key(&req)?;
    let payment_service = PaymentService::new(state.db.clone(), state.redis.clone());
    let payment = payment_service.capture_payment(payment_id, api_key).await?;
    
    Ok(HttpResponse::Ok().json(payment))
}

#[utoipa::path(
    post,
    path = "/api/v1/payments/{payment_id}/refund",
    params(
        ("payment_id" = Uuid, Path, description = "Payment ID")
    ),
    responses(
        (status = 200, description = "Payment refunded successfully", body = PaymentResponse),
        (status = 400, description = "Cannot refund payment"),
        (status = 404, description = "Payment not found"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn refund_payment(
    req: HttpRequest,
    path: web::Path<Uuid>,
    data: web::Json<RefundRequest>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, DefiantError> {
    let payment_id = path.into_inner();
    info!("Refunding payment: {}", payment_id);
    
    let api_key = get_api_key(&req)?;
    let payment_service = PaymentService::new(state.db.clone(), state.redis.clone());
    let payment = payment_service.refund_payment(payment_id, data.into_inner(), api_key).await?;
    
    Ok(HttpResponse::Ok().json(payment))
}

#[utoipa::path(
    get,
    path = "/api/v1/payments",
    params(
        ("limit" = Option<i64>, Query, description = "Number of payments to return"),
        ("starting_after" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("ending_before" = Option<Uuid>, Query, description = "Cursor for pagination"),
        ("customer" = Option<Uuid>, Query, description = "Filter by customer"),
        ("status" = Option<String>, Query, description = "Filter by status"),
    ),
    responses(
        (status = 200, description = "List of payments", body = PaymentsListResponse),
        (status = 401, description = "Unauthorized"),
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_payments(
    req: HttpRequest,
    query: web::Query<PaymentListQuery>,
    state: web::Data<AppState>,
) -> Result<HttpResponse, DefiantError> {
    let api_key = get_api_key(&req)?;
    let payment_service = PaymentService::new(state.db.clone(), state.redis.clone());
    let payments = payment_service.list_payments(query.into_inner(), api_key).await?;
    
    Ok(HttpResponse::Ok().json(payments))
}

// Helper functions
async fn check_rate_limit(req: &HttpRequest, state: &web::Data<AppState>) -> Result<(), DefiantError> {
    // Implement rate limiting using Redis
    // This is a simplified version
    let client_ip = req.connection_info().realip_remote_addr().unwrap_or("unknown");
    let key = format!("rate_limit:{}", client_ip);
    
    let mut conn = state.redis.get_async_connection().await
        .map_err(|_| DefiantError::InternalError)?;
    
    let count: i64 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await
        .map_err(|_| DefiantError::InternalError)?;
    
    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(60) // 1 minute
            .query_async(&mut conn)
            .await
            .map_err(|_| DefiantError::InternalError)?;
    }
    
    if count > 100 { // 100 requests per minute
        return Err(DefiantError::RateLimitError);
    }
    
    Ok(())
}

fn get_api_key(req: &HttpRequest) -> Result<&str, DefiantError> {
    req.headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| DefiantError::AuthenticationError("Missing API key".into()))
}

// Request/Response types
#[derive(Debug, serde::Deserialize)]
pub struct RefundRequest {
    pub amount: Option<i64>,
    pub reason: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct PaymentListQuery {
    pub limit: Option<i64>,
    pub starting_after: Option<Uuid>,
    pub ending_before: Option<Uuid>,
    pub customer: Option<Uuid>,
    pub status: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct PaymentsListResponse {
    pub data: Vec<PaymentResponse>,
    pub has_more: bool,
    pub total: i64,
    pub url: String,
}