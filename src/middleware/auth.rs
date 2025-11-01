use crate::core::AppError;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use futures_util::future::LocalBoxFuture;
use sqlx::MySqlPool;
use std::future::{ready, Ready};
use std::rc::Rc;

/// API Key authentication middleware
pub struct ApiKeyAuth {
    pool: MySqlPool,
}

impl ApiKeyAuth {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

impl<S, B> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ApiKeyAuthMiddleware<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiKeyAuthMiddleware {
            service: Rc::new(service),
            pool: self.pool.clone(),
        }))
    }
}

pub struct ApiKeyAuthMiddleware<S> {
    service: Rc<S>,
    pool: MySqlPool,
}

impl<S, B> Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let pool = self.pool.clone();

        Box::pin(async move {
            // Skip authentication for health check and public endpoints
            let path = req.path();
            if path == "/health" || path == "/" {
                return svc.call(req).await;
            }

            // Extract API key from X-API-Key header
            let api_key = req
                .headers()
                .get("X-API-Key")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| Error::from(AppError::unauthorized("Missing X-API-Key header")))?;

            // Validate API key against database
            let api_key_record = validate_api_key(&pool, api_key)
                .await
                .map_err(Error::from)?;

            // Store merchant_id in request extensions for use in handlers
            req.extensions_mut()
                .insert(api_key_record.merchant_id.clone());
            req.extensions_mut().insert(api_key_record);

            // Continue to the next middleware/handler
            svc.call(req).await
        })
    }
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ApiKeyRecord {
    pub id: String,
    pub merchant_id: String,
    pub rate_limit: i32,
    pub is_active: bool,
}

async fn validate_api_key(pool: &MySqlPool, api_key: &str) -> crate::core::Result<ApiKeyRecord> {
    // Fetch all active API keys to verify against Argon2 hashes
    // Note: This is necessary because Argon2 hashes include random salts,
    // so we can't do a direct database comparison. We must verify each hash.
    
    #[derive(sqlx::FromRow)]
    struct ApiKeyWithHash {
        id: String,
        merchant_id: String,
        key_hash: String,
        rate_limit: i32,
        is_active: bool,
    }

    let candidates = sqlx::query_as::<_, ApiKeyWithHash>(
        r#"
        SELECT id, merchant_id, key_hash, rate_limit, is_active
        FROM api_keys
        WHERE is_active = TRUE
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    // Verify API key against each hash until we find a match
    let mut matched_record: Option<ApiKeyRecord> = None;
    
    for candidate in candidates {
        if verify_api_key(api_key, &candidate.key_hash)? {
            matched_record = Some(ApiKeyRecord {
                id: candidate.id,
                merchant_id: candidate.merchant_id,
                rate_limit: candidate.rate_limit,
                is_active: candidate.is_active,
            });
            break;
        }
    }
    
    let record = matched_record
        .ok_or_else(|| AppError::unauthorized("Invalid API key"))?;

    // Update last_used_at timestamp (fire and forget)
    let _ = sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = ?")
        .bind(&record.id)
        .execute(pool)
        .await;

    Ok(record)
}

/// Helper function to hash API keys using Argon2
pub fn hash_api_key(api_key: &str) -> crate::core::Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(api_key.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| AppError::internal(format!("Failed to hash API key: {}", e)))
}

/// Helper function to verify API keys using Argon2
pub fn verify_api_key(api_key: &str, hash: &str) -> crate::core::Result<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::internal(format!("Invalid hash format: {}", e)))?;

    let argon2 = Argon2::default();

    Ok(argon2
        .verify_password(api_key.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_api_key() {
        let api_key = "test_key_123";
        let hash = hash_api_key(api_key).unwrap();

        assert!(verify_api_key(api_key, &hash).unwrap());
        assert!(!verify_api_key("wrong_key", &hash).unwrap());
    }
}
