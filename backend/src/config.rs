use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiration: i64,
    pub cors_origin: String,
    pub workers: usize,
    pub log_level: String,
    pub environment: Environment,
    pub stripe_secret_key: Option<String>,
    pub stripe_webhook_secret: Option<String>,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_email: String,
    pub rate_limit_requests: u32,
    pub rate_limit_period: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub enum Environment {
    Development,
    Production,
    Staging,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        let environment = env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".into());
        
        let mut cfg = config::Config::builder()
            .add_source(config::File::with_name("config/default"))
            .add_source(config::File::with_name(&format!("config/{}", environment)).required(false))
            .add_source(config::Environment::with_prefix("DEFIANT").separator("__"));
        
        if environment == "development" {
            cfg = cfg.add_source(config::File::with_name("config/development").required(false));
        }
        
        let config = cfg.build()?.try_deserialize()?;
        
        Ok(config)
    }
    
    pub fn is_development(&self) -> bool {
        matches!(self.environment, Environment::Development)
    }
    
    pub fn is_production(&self) -> bool {
        matches!(self.environment, Environment::Production)
    }
}