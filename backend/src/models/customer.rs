use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub default_payment_method: Option<Uuid>,
    pub currency: Option<String>,
    pub balance: i64,
    pub delinquent: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(email)]
    pub email: String,
    
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    
    #[validate(length(min = 10, max = 15))]
    pub phone: Option<String>,
    
    #[validate(length(max = 500))]
    pub description: Option<String>,
    
    pub metadata: Option<serde_json::Value>,
    
    pub payment_method: Option<String>,
    
    pub address: Option<Address>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateCustomerRequest {
    #[validate(email)]
    pub email: Option<String>,
    
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,
    
    #[validate(length(min = 10, max = 15))]
    pub phone: Option<String>,
    
    #[validate(length(max = 500))]
    pub description: Option<String>,
    
    pub metadata: Option<serde_json::Value>,
    
    pub default_payment_method: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Address {
    #[validate(length(min = 1, max = 200))]
    pub line1: Option<String>,
    
    #[validate(length(max = 200))]
    pub line2: Option<String>,
    
    #[validate(length(min = 1, max = 100))]
    pub city: Option<String>,
    
    #[validate(length(max = 100))]
    pub state: Option<String>,
    
    #[validate(length(min = 3, max = 10))]
    pub postal_code: Option<String>,
    
    #[validate(length(min = 2, max = 2))]
    pub country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomerResponse {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub phone: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub default_payment_method: Option<String>,
    pub currency: Option<String>,
    pub balance: i64,
    pub delinquent: bool,
    pub created_at: DateTime<Utc>,
    pub payment_methods: Vec<PaymentMethodResponse>,
    pub subscriptions: Vec<SubscriptionResponse>,
    pub invoices: Vec<InvoiceResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodResponse {
    pub id: String,
    pub brand: String,
    pub last4: String,
    pub exp_month: u8,
    pub exp_year: u16,
    pub country: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
}