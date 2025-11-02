use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};

/// Application-wide Result type
pub type Result<T> = std::result::Result<T, AppError>;

/// Main application error type
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    /// Validation errors for business rules
    #[error("Validation error: {0}")]
    Validation(String),

    /// Database operation errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Payment gateway errors
    #[error("Gateway error: {0}")]
    Gateway(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Unauthorized access
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Conflict - resource state conflict (e.g., concurrent modifications)
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// HTTP client errors
    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Internal server errors
    #[error("Internal error: {0}")]
    Internal(String),
}

impl ResponseError for AppError {
    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        
        // Sanitize error messages to prevent information disclosure
        let error_message = match self {
            // Return detailed messages for client errors (4xx)
            AppError::Validation(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::RateLimitExceeded(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            
            // Sanitize server errors (5xx) - don't expose internal details
            AppError::Database(_) => "A database error occurred".to_string(),
            AppError::Configuration(_) => "A configuration error occurred".to_string(),
            AppError::Internal(_) => "An internal server error occurred".to_string(),
            
            // Gateway errors may contain sensitive API details
            AppError::Gateway(_) => "A payment gateway error occurred".to_string(),
            AppError::HttpClient(_) => "An external service error occurred".to_string(),
            
            // JSON errors usually contain request details (safe to return)
            AppError::Json(err) => format!("Invalid JSON: {}", err),
        };

        // Log full error for debugging (with tracing)
        match self {
            AppError::Database(e) => {
                tracing::error!(error = %e, "Database error occurred");
            }
            AppError::Gateway(e) => {
                tracing::error!(error = %e, "Gateway error occurred");
            }
            AppError::Internal(e) => {
                tracing::error!(error = %e, "Internal error occurred");
            }
            _ => {}
        }

        HttpResponse::build(status_code).json(serde_json::json!({
            "error": {
                "message": error_message,
                "code": status_code.as_u16(),
            }
        }))
    }

    fn status_code(&self) -> StatusCode {
        match self {
            AppError::Validation(_) => StatusCode::BAD_REQUEST,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Gateway(_) => StatusCode::BAD_GATEWAY,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::RateLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Configuration(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::HttpClient(_) => StatusCode::BAD_GATEWAY,
            AppError::Json(_) => StatusCode::BAD_REQUEST,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

// Helper functions for common error scenarios
impl AppError {
    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::Validation(msg.into())
    }

    pub fn not_found(resource: impl Into<String>) -> Self {
        AppError::NotFound(resource.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError::Unauthorized(msg.into())
    }

    pub fn gateway(msg: impl Into<String>) -> Self {
        AppError::Gateway(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::Internal(msg.into())
    }
}
