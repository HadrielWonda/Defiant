pub mod v1;
pub mod auth;
pub mod admin;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(v1::configure)
            .configure(auth::configure)
            .configure(admin::configure)
    );
}