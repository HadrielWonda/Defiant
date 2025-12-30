#pragma once

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// ==================== Error Handling ====================
typedef struct {
    char* message;
    int code;
    char* details;
} DefiantError;

// ==================== Core Types ====================
typedef struct {
    char* id;
    int64_t amount;
    char* currency;
    char* status;
    char* payment_method;
    char* customer_id;
    char* description;
    char* metadata;
    char* created_at;
    char* client_secret;
} DefiantPayment;

typedef struct {
    char* id;
    char* email;
    char* name;
    int64_t balance;
    char* currency;
    bool delinquent;
    char* created_at;
} DefiantCustomer;

typedef struct {
    DefiantPayment* payments;
    size_t count;
    bool has_more;
    int64_t total;
    char* url;
} DefiantPaymentList;

typedef struct {
    char* id;
    char* name;
    char* api_key;
    bool active;
    char* created_at;
} DefiantMerchant;

// ==================== Initialization ====================
void defiant_init(const char* config_path, DefiantError* error);
void defiant_cleanup();

// ==================== Payment API ====================
DefiantPayment* defiant_create_payment(
    const char* api_key,
    int64_t amount,
    const char* currency,
    const char* payment_method,
    const char* customer_id,
    const char* description,
    const char* metadata,
    DefiantError* error
);

DefiantPayment* defiant_get_payment(
    const char* api_key,
    const char* payment_id,
    DefiantError* error
);

DefiantPaymentList* defiant_list_payments(
    const char* api_key,
    const char* cursor,
    int limit,
    const char* customer_id,
    const char* status,
    DefiantError* error
);

DefiantPayment* defiant_refund_payment(
    const char* api_key,
    const char* payment_id,
    int64_t amount,
    const char* reason,
    DefiantError* error
);

DefiantPayment* defiant_capture_payment(
    const char* api_key,
    const char* payment_id,
    DefiantError* error
);

// ==================== Customer API ====================
DefiantCustomer* defiant_create_customer(
    const char* api_key,
    const char* email,
    const char* name,
    const char* phone,
    const char* description,
    const char* metadata,
    DefiantError* error
);

DefiantCustomer* defiant_get_customer(
    const char* api_key,
    const char* customer_id,
    DefiantError* error
);

DefiantCustomer* defiant_update_customer(
    const char* api_key,
    const char* customer_id,
    const char* email,
    const char* name,
    const char* phone,
    const char* description,
    const char* metadata,
    DefiantError* error
);

bool defiant_delete_customer(
    const char* api_key,
    const char* customer_id,
    DefiantError* error
);

// ==================== Webhook API ====================
bool defiant_verify_webhook_signature(
    const char* payload,
    const char* signature_header,
    const char* webhook_secret,
    DefiantError* error
);

char* defiant_process_webhook(
    const char* payload,
    const char* signature_header,
    const char* webhook_secret,
    DefiantError* error
);

// ==================== Crypto API ====================
char* defiant_generate_crypto_address(
    const char* currency,
    const char* network,
    DefiantError* error
);

bool defiant_validate_crypto_transaction(
    const char* tx_hash,
    const char* currency,
    DefiantError* error
);

char* defiant_estimate_crypto_fee(
    const char* currency,
    int64_t amount,
    DefiantError* error
);

// ==================== Utility API ====================
char* defiant_generate_api_key(
    const char* merchant_id,
    const char* name,
    const char* permissions,
    DefiantError* error
);

bool defiant_validate_api_key(
    const char* api_key,
    DefiantError* error
);

char* defiant_encrypt_data(
    const char* data,
    const char* key,
    DefiantError* error
);

char* defiant_decrypt_data(
    const char* encrypted_data,
    const char* key,
    DefiantError* error
);

// ==================== Memory Management ====================
void defiant_free_payment(DefiantPayment* payment);
void defiant_free_customer(DefiantCustomer* customer);
void defiant_free_payment_list(DefiantPaymentList* list);
void defiant_free_error(DefiantError* error);
void defiant_free_string(char* str);

// ==================== Streaming API ====================
typedef void (*DefiantStreamCallback)(const char* event_type, const char* data, void* user_data);

bool defiant_stream_payments(
    const char* api_key,
    DefiantStreamCallback callback,
    void* user_data,
    DefiantError* error
);

bool defiant_stream_events(
    const char* api_key,
    const char* event_type,
    DefiantStreamCallback callback,
    void* user_data,
    DefiantError* error
);

// ==================== Analytics API ====================
typedef struct {
    int64_t total_amount;
    int64_t total_count;
    int64_t successful_count;
    int64_t failed_count;
    int64_t refunded_amount;
    char* start_date;
    char* end_date;
} DefiantAnalyticsSummary;

DefiantAnalyticsSummary* defiant_get_analytics(
    const char* api_key,
    const char* start_date,
    const char* end_date,
    const char* currency,
    DefiantError* error
);

void defiant_free_analytics(DefiantAnalyticsSummary* analytics);

#ifdef __cplusplus
}
#endif