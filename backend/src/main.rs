use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware};
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber;

mod api;
mod models;
mod services;
mod middleware as custom_middleware;
mod config;
mod db;
mod errors;
mod websocket;

use config::Config;
use db::Database;
use custom_middleware::auth::Authentication;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    // Load configuration
    let config = Config::from_env().expect("Failed to load configuration");
    
    // Initialize database
    let db = Database::new(&config.database_url)
        .await
        .expect("Failed to connect to database");
    
    // Run migrations
    db.run_migrations().await.expect("Failed to run migrations");
    
    // Create Redis connection for WebSockets and rate limiting
    let redis_client = redis::Client::open(config.redis_url.clone())
        .expect("Failed to create Redis client");
    
    // Create application state
    let app_state = web::Data::new(AppState {
        db: Arc::new(db),
        config: Arc::new(config.clone()),
        redis: Arc::new(redis_client),
    });

    // Start WebSocket server
    let ws_server = websocket::server::WebSocketServer::new(app_state.clone());
    let ws_server = Arc::new(ws_server);
    
    tracing::info!("Starting Defiant backend on {}:{}", config.host, config.port);
    
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);
        
        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(middleware::NormalizePath::trim())
            .wrap(Authentication)
            .configure(api::configure)
            .service(
                web::scope("/ws")
                    .service(websocket::handler::websocket_route)
            )
            .route("/health", web::get().to(health_check))
            .route("/metrics", web::get().to(metrics))
    })
    .bind((config.host.clone(), config.port))?
    .workers(config.workers)
    .run()
    .await
}

async fn health_check() -> &'static str {
    "🛡️ Defiant is running and ready for battle!"
}

async fn metrics() -> String {
    // TODO: Implement Prometheus metrics
    "metrics_endpoint".to_string()
}

pub struct AppState {
    pub db: Arc<Database>,
    pub config: Arc<Config>,
    pub redis: Arc<redis::Client>,
}