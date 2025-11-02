// Payment Gateway Sandbox Helpers
//
// Provides helpers for interacting with payment gateway sandbox APIs.
// This module will be fully implemented in Phase 3 (User Story 1).

// Placeholder structs - will be implemented in T017-T018

/// Xendit sandbox API helper
///
/// Provides methods to interact with Xendit test mode API.
/// Full implementation in T017.
pub struct XenditSandbox {
    api_key: String,
    base_url: String,
}

impl XenditSandbox {
    /// Create new Xendit sandbox instance
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: "https://api.xendit.co".to_string(),
        }
    }

    // Methods will be added in T017:
    // - create_invoice()
    // - get_invoice()
    // - simulate_payment()
}

/// Midtrans sandbox API helper
///
/// Provides methods to interact with Midtrans sandbox API.
/// Full implementation in T018.
pub struct MidtransSandbox {
    server_key: String,
    base_url: String,
}

impl MidtransSandbox {
    /// Create new Midtrans sandbox instance
    pub fn new(server_key: String) -> Self {
        Self {
            server_key,
            base_url: "https://api.sandbox.midtrans.com".to_string(),
        }
    }

    // Methods will be added in T018:
    // - charge()
    // - get_status()
    // - cancel_transaction()
}
