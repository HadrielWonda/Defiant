use std::sync::Arc;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use redis::aio::ConnectionManager;
use tracing::{info, warn, error};

use crate::{models::{CreatePaymentRequest, PaymentResponse, PaymentStatus, PaymentMethod}, errors::DefiantError, db::Database};

pub struct PaymentService {
    db: Arc<Database>,
    redis: Arc<ConnectionManager>,
}

impl PaymentService {
    pub fn new(db: Arc<Database>, redis: Arc<ConnectionManager>) -> Self {
        Self { db, redis }
    }
    
    pub async fn create_payment(
        &self,
        request: CreatePaymentRequest,
        api_key: &str,
    ) -> Result<PaymentResponse, DefiantError> {
        // Start transaction
        let mut tx = self.db.pool.begin().await?;
        
        // Validate API key and get merchant
        let merchant = self.validate_api_key(api_key, &mut tx).await?;
        
        // Check fraud
        self.check_fraud(&request, &merchant.id, &mut tx).await?;
        
        // Create payment record
        let payment_id = Uuid::new_v4();
        let now = Utc::now();
        
        let payment = sqlx::query_as!(
            Payment,
            r#"
            INSERT INTO payments (
                id, amount, currency, status, payment_method,
                merchant_id, customer_id, description, metadata,
                created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING *
            "#,
            payment_id,
            request.amount,
            request.currency.to_uppercase(),
            PaymentStatus::Pending as PaymentStatus,
            request.payment_method as PaymentMethod,
            merchant.id,
            request.customer_id,
            request.description,
            request.metadata,
            now,
            now,
        )
        .fetch_one(&mut *tx)
        .await?;
        
        // Process payment based on method
        let processed_payment = match request.payment_method {
            PaymentMethod::Card => self.process_card_payment(payment, &request, &mut tx).await?,
            PaymentMethod::Crypto => self.process_crypto_payment(payment, &mut tx).await?,
            _ => payment,
        };
        
        // Commit transaction
        tx.commit().await?;
        
        // Emit event
        self.emit_payment_event(&processed_payment, "payment.created").await;
        
        // Convert to response
        Ok(PaymentResponse {
            id: processed_payment.id,
            amount: processed_payment.amount,
            currency: processed_payment.currency,
            status: processed_payment.status,
            payment_method: processed_payment.payment_method,
            customer_id: processed_payment.customer_id,
            description: processed_payment.description,
            metadata: processed_payment.metadata,
            created_at: processed_payment.created_at,
            client_secret: Some(format!("pi_{}_secret_{}", processed_payment.id, Uuid::new_v4())),
            next_action: None,
        })
    }
    
    pub async fn get_payment(
        &self,
        payment_id: Uuid,
        api_key: &str,
    ) -> Result<PaymentResponse, DefiantError> {
        let merchant = self.get_merchant_by_api_key(api_key).await?;
        
        let payment = sqlx::query_as!(
            Payment,
            r#"
            SELECT * FROM payments 
            WHERE id = $1 AND merchant_id = $2
            "#,
            payment_id,
            merchant.id,
        )
        .fetch_optional(&self.db.pool)
        .await?
        .ok_or_else(|| DefiantError::NotFound("Payment not found".into()))?;
        
        self.payment_to_response(payment).await
    }
    
    async fn process_card_payment(
        &self,
        payment: Payment,
        request: &CreatePaymentRequest,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Payment, DefiantError> {
        // Simulate payment processing
        info!("Processing card payment: {}", payment.id);
        
        // In real implementation, integrate with payment processor
        // For now, simulate success
        let status = if rand::random::<f32>() > 0.1 {
            PaymentStatus::Succeeded
        } else {
            PaymentStatus::Failed
        };
        
        let updated_payment = sqlx::query_as!(
            Payment,
            r#"
            UPDATE payments 
            SET status = $1, updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
            status as PaymentStatus,
            Utc::now(),
            payment.id,
        )
        .fetch_one(&mut **tx)
        .await?;
        
        Ok(updated_payment)
    }
    
    async fn process_crypto_payment(
        &self,
        payment: Payment,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Payment, DefiantError> {
        // Generate crypto address for payment
        let crypto_address = self.generate_crypto_address(&payment).await?;
        
        // Update payment with crypto details
        let updated_payment = sqlx::query_as!(
            Payment,
            r#"
            UPDATE payments 
            SET metadata = jsonb_set(
                COALESCE(metadata, '{}'::jsonb),
                '{crypto_address}',
                $1::jsonb
            ),
            updated_at = $2
            WHERE id = $3
            RETURNING *
            "#,
            serde_json::json!(crypto_address),
            Utc::now(),
            payment.id,
        )
        .fetch_one(&mut **tx)
        .await?;
        
        Ok(updated_payment)
    }
    
    async fn generate_crypto_address(&self, payment: &Payment) -> Result<String, DefiantError> {
        // Generate unique crypto address for this payment
        let address = format!("0x{}{}", 
            hex::encode(payment.id.as_bytes()),
            hex::encode(&payment.created_at.timestamp().to_be_bytes())
        );
        
        Ok(address)
    }
    
    async fn validate_api_key(
        &self,
        api_key: &str,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<Merchant, DefiantError> {
        let merchant = sqlx::query_as!(
            Merchant,
            r#"
            SELECT m.* FROM merchants m
            JOIN api_keys ak ON m.id = ak.merchant_id
            WHERE ak.key = $1 AND ak.active = true
            AND m.active = true
            "#,
            api_key,
        )
        .fetch_optional(&mut **tx)
        .await?
        .ok_or_else(|| DefiantError::AuthenticationError("Invalid API key".into()))?;
        
        Ok(merchant)
    }
    
    async fn check_fraud(
        &self,
        request: &CreatePaymentRequest,
        merchant_id: &Uuid,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), DefiantError> {
        // Simple fraud check
        // In production, use machine learning
        if request.amount > 1_000_000_00 { // $10,000
            warn!("Large payment detected: {}", request.amount);
            
            // Check if merchant is allowed for large payments
            let allowed = sqlx::query_scalar!(
                r#"SELECT allow_large_payments FROM merchants WHERE id = $1"#,
                merchant_id,
            )
            .fetch_one(&mut **tx)
            .await
            .unwrap_or(false);
            
            if !allowed {
                return Err(DefiantError::PaymentError("Large payments not allowed".into()));
            }
        }
        
        Ok(())
    }
    
    async fn emit_payment_event(&self, payment: &Payment, event_type: &str) {
        // Publish event to Redis for WebSocket clients
        let event = serde_json::json!({
            "type": event_type,
            "data": payment,
            "created_at": Utc::now(),
        });
        
        if let Err(e) = redis::cmd("PUBLISH")
            .arg("payments")
            .arg(event.to_string())
            .query_async::<_, ()>(&mut self.redis.clone())
            .await
        {
            error!("Failed to publish event: {}", e);
        }
    }
    
    async fn payment_to_response(&self, payment: Payment) -> Result<PaymentResponse, DefiantError> {
        Ok(PaymentResponse {
            id: payment.id,
            amount: payment.amount,
            currency: payment.currency,
            status: payment.status,
            payment_method: payment.payment_method,
            customer_id: payment.customer_id,
            description: payment.description,
            metadata: payment.metadata,
            created_at: payment.created_at,
            client_secret: None, // Only for initial creation
            next_action: None,
        })
    }
}

// Internal types
struct Merchant {
    id: Uuid,
    name: String,
    email: String,
    active: bool,
    allow_large_payments: bool,
}