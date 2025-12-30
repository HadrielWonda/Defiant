use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Postgres};
use std::time::Duration;
use tracing::info;

pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        info!("Connecting to database...");
        
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(10))
            .idle_timeout(Duration::from_secs(300))
            .max_lifetime(Duration::from_secs(3600))
            .connect(database_url)
            .await?;
        
        info!("Database connection established");
        
        Ok(Self { pool })
    }
    
    pub async fn run_migrations(&self) -> Result<(), sqlx::Error> {
        info!("Running database migrations...");
        
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        
        info!("Migrations completed");
        Ok(())
    }
    
    pub fn get_pool(&self) -> &Pool<Postgres> {
        &self.pool
    }
}

// Connection pool extractor for Actix handlers
impl actix_web::FromRequest for Database {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;
    
    fn from_request(req: &actix_web::HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let data = req.app_data::<actix_web::web::Data<crate::AppState>>().unwrap();
        let db = Database {
            pool: data.db.pool.clone(),
        };
        std::future::ready(Ok(db))
    }
}