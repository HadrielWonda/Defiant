use actix_web::{dev::ServiceRequest, error::ErrorUnauthorized, Error, HttpMessage};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::extractors::AuthenticationError;
use jsonwebtoken::{decode, Validation, Algorithm, DecodingKey};
use serde::{Deserialize, Serialize};
use std::future::{ready, Ready};
use std::pin::Pin;
use actix_web::dev::{forward_ready, Service, Transform};
use futures_util::future::LocalBoxFuture;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // User ID
    pub exp: usize, // Expiration time
    pub role: String,
    pub merchant_id: Option<String>,
}

pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = actix_web::dev::ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthenticationMiddleware { service }))
    }
}

pub struct AuthenticationMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = actix_web::dev::ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        // Skip auth for certain paths
        let path = req.path();
        if path.starts_with("/health") 
            || path.starts_with("/api/v1/webhooks")
            || path == "/metrics" {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        // Extract token
        let token = req.headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .or_else(|| {
                // Check query parameter
                req.query_string()
                    .split('&')
                    .find(|param| param.starts_with("token="))
                    .and_then(|param| param.split('=').nth(1))
            });

        match token {
            Some(token) => {
                // Validate token
                match validate_token(token) {
                    Ok(claims) => {
                        // Insert claims into request extensions
                        req.extensions_mut().insert(claims);
                        let fut = self.service.call(req);
                        Box::pin(async move { fut.await })
                    }
                    Err(_) => Box::pin(async move {
                        Err(ErrorUnauthorized("Invalid token"))
                    }),
                }
            }
            None => Box::pin(async move {
                Err(ErrorUnauthorized("Missing authentication token"))
            }),
        }
    }
}

fn validate_token(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());
    
    decode::<Claims>(
        token,
        &decoding_key,
        &Validation::new(Algorithm::HS256),
    )
    .map(|data| data.claims)
}

// For routes that require authentication
pub struct AuthenticatedUser;

impl actix_web::guard::Guard for AuthenticatedUser {
    fn check(&self, req: &actix_web::HttpRequest) -> bool {
        req.extensions().get::<Claims>().is_some()
    }
}