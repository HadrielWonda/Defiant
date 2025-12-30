use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Arc;
use libc::{size_t, int64_t};

use defiant_backend::{
    models::{CreatePaymentRequest, PaymentResponse, CreateCustomerRequest, CustomerResponse},
    services::{payment_service::PaymentService, customer_service::CustomerService},
    db::Database,
    errors::DefiantError as RustDefiantError,
};

// Re-export from backend
use defiant_backend as backend;

// ==================== Error Handling ====================

#[repr(C)]
pub struct CDefiantError {
    pub message: *mut c_char,
    pub code: c_int,
    pub details: *mut c_char,
}

impl From<RustDefiantError> for CDefiantError {
    fn from(err: RustDefiantError) -> Self {
        let message = CString::new(err.to_string()).unwrap_or_default();
        let details = CString::new("").unwrap_or_default();
        
        let code = match err {
            RustDefiantError::DatabaseError(_) => 1,
            RustDefiantError::ValidationError(_) => 2,
            RustDefiantError::AuthenticationError(_) => 3,
            RustDefiantError::AuthorizationError(_) => 4,
            RustDefiantError::PaymentError(_) => 5,
            RustDefiantError::RateLimitError => 6,
            RustDefiantError::NotFound(_) => 7,
            RustDefiantError::BadRequest(_) => 8,
            RustDefiantError::Conflict(_) => 9,
            _ => 0,
        };
        
        CDefiantError {
            message: message.into_raw(),
            code,
            details: details.into_raw(),
        }
    }
}

// ==================== Core Types ====================

#[repr(C)]
pub struct CDefiantPayment {
    pub id: *mut c_char,
    pub amount: int64_t,
    pub currency: *mut c_char,
    pub status: *mut c_char,
    pub payment_method: *mut c_char,
    pub customer_id: *mut c_char,
    pub description: *mut c_char,
    pub metadata: *mut c_char,
    pub created_at: *mut c_char,
    pub client_secret: *mut c_char,
}

impl From<PaymentResponse> for CDefiantPayment {
    fn from(payment: PaymentResponse) -> Self {
        CDefiantPayment {
            id: CString::new(payment.id.to_string()).unwrap().into_raw(),
            amount: payment.amount,
            currency: CString::new(payment.currency).unwrap().into_raw(),
            status: CString::new(payment.status.to_string()).unwrap().into_raw(),
            payment_method: CString::new(payment.payment_method.to_string()).unwrap().into_raw(),
            customer_id: payment.customer_id
                .map(|id| CString::new(id.to_string()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            description: payment.description
                .map(|desc| CString::new(desc).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            metadata: payment.metadata
                .map(|meta| CString::new(meta.to_string()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            created_at: CString::new(payment.created_at.to_rfc3339()).unwrap().into_raw(),
            client_secret: payment.client_secret
                .map(|secret| CString::new(secret).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
        }
    }
}

#[repr(C)]
pub struct CDefiantCustomer {
    pub id: *mut c_char,
    pub email: *mut c_char,
    pub name: *mut c_char,
    pub balance: int64_t,
    pub currency: *mut c_char,
    pub delinquent: bool,
    pub created_at: *mut c_char,
}

impl From<CustomerResponse> for CDefiantCustomer {
    fn from(customer: CustomerResponse) -> Self {
        CDefiantCustomer {
            id: CString::new(customer.id.to_string()).unwrap().into_raw(),
            email: CString::new(customer.email).unwrap().into_raw(),
            name: customer.name
                .map(|name| CString::new(name).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            balance: customer.balance,
            currency: customer.currency
                .map(|curr| CString::new(curr).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            delinquent: customer.delinquent,
            created_at: CString::new(customer.created_at.to_rfc3339()).unwrap().into_raw(),
        }
    }
}

#[repr(C)]
pub struct CDefiantPaymentList {
    pub payments: *mut CDefiantPayment,
    pub count: size_t,
    pub has_more: bool,
    pub total: int64_t,
    pub url: *mut c_char,
}

// ==================== Global State ====================

struct DefiantState {
    db: Option<Arc<Database>>,
    redis: Option<Arc<redis::aio::ConnectionManager>>,
}

static mut STATE: Option<DefiantState> = None;

fn get_state() -> Result<&'static mut DefiantState, RustDefiantError> {
    unsafe {
        STATE.as_mut().ok_or_else(|| RustDefiantError::InternalError)
    }
}

// ==================== Initialization ====================

#[no_mangle]
pub extern "C" fn defiant_init(config_path: *const c_char, error: *mut CDefiantError) {
    let config_path_str = unsafe {
        if config_path.is_null() {
            "config/default.toml"
        } else {
            CStr::from_ptr(config_path).to_str().unwrap_or("config/default.toml")
        }
    };
    
    let result = || -> Result<(), RustDefiantError> {
        // Load configuration
        let config = backend::config::Config::from_file(config_path_str)?;
        
        // Initialize database
        let db = Database::new(&config.database_url).await?;
        
        // Initialize Redis
        let redis_client = redis::Client::open(config.redis_url.clone())?;
        let redis = redis_client.get_tokio_connection_manager().await?;
        
        // Create state
        unsafe {
            STATE = Some(DefiantState {
                db: Some(Arc::new(db)),
                redis: Some(Arc::new(redis)),
            });
        }
        
        Ok(())
    };
    
    match result() {
        Ok(_) => {
            if !error.is_null() {
                unsafe {
                    (*error).message = ptr::null_mut();
                    (*error).code = 0;
                    (*error).details = ptr::null_mut();
                }
            }
        }
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = e.into();
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn defiant_cleanup() {
    unsafe {
        STATE = None;
    }
}

// ==================== Payment API ====================

#[no_mangle]
pub extern "C" fn defiant_create_payment(
    api_key: *const c_char,
    amount: int64_t,
    currency: *const c_char,
    payment_method: *const c_char,
    customer_id: *const c_char,
    description: *const c_char,
    metadata: *const c_char,
    error: *mut CDefiantError,
) -> *mut CDefiantPayment {
    let result = || -> Result<CDefiantPayment, RustDefiantError> {
        let state = get_state()?;
        let db = state.db.as_ref().ok_or(RustDefiantError::InternalError)?;
        let redis = state.redis.as_ref().ok_or(RustDefiantError::InternalError)?;
        
        let api_key_str = unsafe { CStr::from_ptr(api_key).to_str()? };
        let currency_str = unsafe { CStr::from_ptr(currency).to_str()? };
        let payment_method_str = unsafe { CStr::from_ptr(payment_method).to_str()? };
        
        let customer_id_uuid = if !customer_id.is_null() {
            Some(unsafe { CStr::from_ptr(customer_id).to_str()?.parse()? })
        } else {
            None
        };
        
        let description_str = if !description.is_null() {
            Some(unsafe { CStr::from_ptr(description).to_str()?.to_string() })
        } else {
            None
        };
        
        let metadata_json = if !metadata.is_null() {
            let metadata_str = unsafe { CStr::from_ptr(metadata).to_str()? };
            Some(serde_json::from_str(metadata_str)?)
        } else {
            None
        };
        
        // Create payment request
        let request = CreatePaymentRequest {
            amount,
            currency: currency_str.to_string(),
            payment_method: payment_method_str.parse()?,
            description: description_str,
            metadata: metadata_json,
            customer_id: customer_id_uuid,
            source: None,
        };
        
        // Validate request
        request.validate()?;
        
        // Create payment service
        let payment_service = PaymentService::new(db.clone(), redis.clone());
        
        // Create payment
        let payment = tokio::runtime::Runtime::new()?
            .block_on(payment_service.create_payment(request, api_key_str))?;
        
        Ok(payment.into())
    };
    
    match result() {
        Ok(payment) => Box::into_raw(Box::new(payment)),
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = e.into();
                }
            }
            ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn defiant_get_payment(
    api_key: *const c_char,
    payment_id: *const c_char,
    error: *mut CDefiantError,
) -> *mut CDefiantPayment {
    let result = || -> Result<CDefiantPayment, RustDefiantError> {
        let state = get_state()?;
        let db = state.db.as_ref().ok_or(RustDefiantError::InternalError)?;
        let redis = state.redis.as_ref().ok_or(RustDefiantError::InternalError)?;
        
        let api_key_str = unsafe { CStr::from_ptr(api_key).to_str()? };
        let payment_id_str = unsafe { CStr::from_ptr(payment_id).to_str()? };
        let payment_id_uuid = payment_id_str.parse()?;
        
        let payment_service = PaymentService::new(db.clone(), redis.clone());
        let payment = tokio::runtime::Runtime::new()?
            .block_on(payment_service.get_payment(payment_id_uuid, api_key_str))?;
        
        Ok(payment.into())
    };
    
    match result() {
        Ok(payment) => Box::into_raw(Box::new(payment)),
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = e.into();
                }
            }
            ptr::null_mut()
        }
    }
}

// ==================== Customer API ====================

#[no_mangle]
pub extern "C" fn defiant_create_customer(
    api_key: *const c_char,
    email: *const c_char,
    name: *const c_char,
    phone: *const c_char,
    description: *const c_char,
    metadata: *const c_char,
    error: *mut CDefiantError,
) -> *mut CDefiantCustomer {
    let result = || -> Result<CDefiantCustomer, RustDefiantError> {
        let state = get_state()?;
        let db = state.db.as_ref().ok_or(RustDefiantError::InternalError)?;
        let redis = state.redis.as_ref().ok_or(RustDefiantError::InternalError)?;
        
        let api_key_str = unsafe { CStr::from_ptr(api_key).to_str()? };
        let email_str = unsafe { CStr::from_ptr(email).to_str()? };
        
        let name_str = if !name.is_null() {
            Some(unsafe { CStr::from_ptr(name).to_str()?.to_string() })
        } else {
            None
        };
        
        let phone_str = if !phone.is_null() {
            Some(unsafe { CStr::from_ptr(phone).to_str()?.to_string() })
        } else {
            None
        };
        
        let description_str = if !description.is_null() {
            Some(unsafe { CStr::from_ptr(description).to_str()?.to_string() })
        } else {
            None
        };
        
        let metadata_json = if !metadata.is_null() {
            let metadata_str = unsafe { CStr::from_ptr(metadata).to_str()? };
            Some(serde_json::from_str(metadata_str)?)
        } else {
            None
        };
        
        let request = CreateCustomerRequest {
            email: email_str.to_string(),
            name: name_str,
            phone: phone_str,
            description: description_str,
            metadata: metadata_json,
            payment_method: None,
            address: None,
        };
        
        request.validate()?;
        
        let customer_service = CustomerService::new(db.clone(), redis.clone());
        let customer = tokio::runtime::Runtime::new()?
            .block_on(customer_service.create_customer(request, api_key_str))?;
        
        Ok(customer.into())
    };
    
    match result() {
        Ok(customer) => Box::into_raw(Box::new(customer)),
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = e.into();
                }
            }
            ptr::null_mut()
        }
    }
}

// ==================== Memory Management ====================

#[no_mangle]
pub extern "C" fn defiant_free_payment(payment: *mut CDefiantPayment) {
    if payment.is_null() {
        return;
    }
    
    unsafe {
        let payment = Box::from_raw(payment);
        
        if !payment.id.is_null() {
            drop(CString::from_raw(payment.id));
        }
        if !payment.currency.is_null() {
            drop(CString::from_raw(payment.currency));
        }
        if !payment.status.is_null() {
            drop(CString::from_raw(payment.status));
        }
        if !payment.payment_method.is_null() {
            drop(CString::from_raw(payment.payment_method));
        }
        if !payment.customer_id.is_null() {
            drop(CString::from_raw(payment.customer_id));
        }
        if !payment.description.is_null() {
            drop(CString::from_raw(payment.description));
        }
        if !payment.metadata.is_null() {
            drop(CString::from_raw(payment.metadata));
        }
        if !payment.created_at.is_null() {
            drop(CString::from_raw(payment.created_at));
        }
        if !payment.client_secret.is_null() {
            drop(CString::from_raw(payment.client_secret));
        }
    }
}

#[no_mangle]
pub extern "C" fn defiant_free_customer(customer: *mut CDefiantCustomer) {
    if customer.is_null() {
        return;
    }
    
    unsafe {
        let customer = Box::from_raw(customer);
        
        if !customer.id.is_null() {
            drop(CString::from_raw(customer.id));
        }
        if !customer.email.is_null() {
            drop(CString::from_raw(customer.email));
        }
        if !customer.name.is_null() {
            drop(CString::from_raw(customer.name));
        }
        if !customer.currency.is_null() {
            drop(CString::from_raw(customer.currency));
        }
        if !customer.created_at.is_null() {
            drop(CString::from_raw(customer.created_at));
        }
    }
}

#[no_mangle]
pub extern "C" fn defiant_free_error(error: *mut CDefiantError) {
    if error.is_null() {
        return;
    }
    
    unsafe {
        let error = Box::from_raw(error);
        
        if !error.message.is_null() {
            drop(CString::from_raw(error.message));
        }
        if !error.details.is_null() {
            drop(CString::from_raw(error.details));
        }
    }
}

#[no_mangle]
pub extern "C" fn defiant_free_string(str_ptr: *mut c_char) {
    if str_ptr.is_null() {
        return;
    }
    
    unsafe {
        drop(CString::from_raw(str_ptr));
    }
}

// ==================== Utility Functions ====================

#[no_mangle]
pub extern "C" fn defiant_validate_api_key(
    api_key: *const c_char,
    error: *mut CDefiantError,
) -> bool {
    let result = || -> Result<bool, RustDefiantError> {
        let state = get_state()?;
        let db = state.db.as_ref().ok_or(RustDefiantError::InternalError)?;
        
        let api_key_str = unsafe { CStr::from_ptr(api_key).to_str()? };
        
        let valid = tokio::runtime::Runtime::new()?
            .block_on(async {
                let merchant = sqlx::query!(
                    "SELECT m.id FROM merchants m
                     JOIN api_keys ak ON m.id = ak.merchant_id
                     WHERE ak.key = $1 AND ak.active = true
                     AND m.active = true",
                    api_key_str
                )
                .fetch_optional(&db.pool)
                .await?;
                
                Ok::<_, sqlx::Error>(merchant.is_some())
            })?;
        
        Ok(valid)
    };
    
    match result() {
        Ok(valid) => valid,
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = e.into();
                }
            }
            false
        }
    }
}

// Crypto functions
#[no_mangle]
pub extern "C" fn defiant_generate_crypto_address(
    currency: *const c_char,
    network: *const c_char,
    error: *mut CDefiantError,
) -> *mut c_char {
    let result = || -> Result<CString, RustDefiantError> {
        let currency_str = unsafe { CStr::from_ptr(currency).to_str()? };
        let network_str = unsafe { CStr::from_ptr(network).to_str()? };
        
        // Generate deterministic address from currency and network
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(currency_str);
        hasher.update(network_str);
        hasher.update(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos().to_string());
        
        let result = hasher.finalize();
        let address = format!("0x{}", hex::encode(&result[..20]));
        
        Ok(CString::new(address)?)
    };
    
    match result() {
        Ok(address) => address.into_raw(),
        Err(e) => {
            if !error.is_null() {
                unsafe {
                    *error = e.into();
                }
            }
            ptr::null_mut()
        }
    }
}