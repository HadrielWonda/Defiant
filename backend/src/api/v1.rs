use actix_web::web;
use crate::middleware::auth::AuthenticatedUser;

pub mod payments;
pub mod customers;
pub mod webhooks;
pub mod subscriptions;
pub mod invoices;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .service(
                web::scope("/payments")
                    .route("", web::post().to(payments::create_payment))
                    .route("/{payment_id}", web::get().to(payments::get_payment))
                    .route("/{payment_id}/capture", web::post().to(payments::capture_payment))
                    .route("/{payment_id}/refund", web::post().to(payments::refund_payment))
                    .route("", web::get().to(payments::list_payments))
            )
            .service(
                web::scope("/customers")
                    .wrap(AuthenticatedUser)
                    .route("", web::post().to(customers::create_customer))
                    .route("/{customer_id}", web::get().to(customers::get_customer))
                    .route("/{customer_id}", web::put().to(customers::update_customer))
                    .route("/{customer_id}", web::delete().to(customers::delete_customer))
                    .route("", web::get().to(customers::list_customers))
                    .route("/{customer_id}/payment_methods", web::get().to(customers::list_payment_methods))
                    .route("/{customer_id}/balance_transactions", web::get().to(customers::get_balance_transactions))
            )
            .service(
                web::scope("/webhooks")
                    .route("/stripe", web::post().to(webhooks::handle_stripe_webhook))
                    .route("/{webhook_id}", web::get().to(webhooks::get_webhook))
                    .route("", web::post().to(webhooks::create_webhook))
                    .route("", web::get().to(webhooks::list_webhooks))
            )
            .service(
                web::scope("/subscriptions")
                    .wrap(AuthenticatedUser)
                    .route("", web::post().to(subscriptions::create_subscription))
                    .route("/{subscription_id}", web::get().to(subscriptions::get_subscription))
                    .route("/{subscription_id}", web::put().to(subscriptions::update_subscription))
                    .route("/{subscription_id}/cancel", web::post().to(subscriptions::cancel_subscription))
                    .route("", web::get().to(subscriptions::list_subscriptions))
            )
            .service(
                web::scope("/invoices")
                    .wrap(AuthenticatedUser)
                    .route("", web::post().to(invoices::create_invoice))
                    .route("/{invoice_id}", web::get().to(invoices::get_invoice))
                    .route("/{invoice_id}/pay", web::post().to(invoices::pay_invoice))
                    .route("/{invoice_id}/void", web::post().to(invoices::void_invoice))
                    .route("", web::get().to(invoices::list_invoices))
                    .route("/upcoming", web::get().to(invoices::get_upcoming_invoice))
            )
    );
}