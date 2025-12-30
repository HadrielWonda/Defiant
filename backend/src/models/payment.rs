use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Payment {
    pub id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub payment_method: PaymentMethod,
    pub customer_id: Uuid,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub refunded_amount: i64,
    pub refund_reason: Option<String>,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Processing,
    RequiresAction,
    RequiresConfirmation,
    RequiresCapture,
    Canceled,
    Succeeded,
    Failed,
    Refunded,
    PartiallyRefunded,
    Disputed,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
pub enum PaymentMethod {
    Card,
    BankTransfer,
    Crypto,
    ApplePay,
    GooglePay,
    PayPal,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreatePaymentRequest {
    #[validate(range(min = 50, message = "Amount must be at least $0.50"))]
    pub amount: i64,
    
    #[validate(length(min = 3, max = 3))]
    pub currency: String,
    
    pub payment_method: PaymentMethod,
    
    #[validate(length(max = 500))]
    pub description: Option<String>,
    
    pub metadata: Option<serde_json::Value>,
    
    pub customer_id: Option<Uuid>,
    
    pub source: Option<PaymentSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentSource {
    pub token: String,
    pub card: Option<CardDetails>,
    pub billing_details: Option<BillingDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CardDetails {
    #[validate(length(equal = 16))]
    pub number: String,
    
    #[validate(range(min = 1, max = 12))]
    pub exp_month: u8,
    
    #[validate(range(min = 2024, max = 2100))]
    pub exp_year: u16,
    
    #[validate(length(min = 3, max = 4))]
    pub cvc: String,
    
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingDetails {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub address: Option<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub line1: String,
    pub line2: Option<String>,
    pub city: String,
    pub state: Option<String>,
    pub postal_code: String,
    pub country: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub amount: i64,
    pub currency: String,
    pub status: PaymentStatus,
    pub payment_method: PaymentMethod,
    pub customer_id: Option<Uuid>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub client_secret: Option<String>,
    pub next_action: Option<NextAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NextAction {
    Redirect { url: String },
    ThreeDSecure { url: String },
    VerifyWithAmounts { amounts: Vec<i64> },
}