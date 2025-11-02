use actix_web::{
    body::BoxBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use futures_util::future::LocalBoxFuture;
use sqlx::MySqlPool;
use std::future::{ready, Ready};
use std::rc::Rc;

/// Middleware for API key authentication
/// Validates X-API-Key header and extracts tenant_id for multi-tenant isolation
pub struct ApiKeyAuth;

impl<S> Transform<S, ServiceRequest> for ApiKeyAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type InitError = ();
    type Transform = ApiKeyAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ApiKeyAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct ApiKeyAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S> Service<ServiceRequest> for ApiKeyAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse, Error = Error> + 'static,
    S::Future: 'static,
{
    type Response = ServiceResponse;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            // Skip auth for health check and public endpoints
            let path = req.path();
            if path == "/health" || path == "/ready" || path.starts_with("/docs") || path == "/openapi.json" {
                return service.call(req).await;
            }

            // Extract API key from header
            let api_key = match req.headers().get("X-API-Key") {
                Some(value) => match value.to_str() {
                    Ok(key) => key,
                    Err(_) => {
                        let response = HttpResponse::Unauthorized()
                            .json(serde_json::json!({
                                "error": {
                                    "code": 401,
                                    "message": "Invalid API key format"
                                }
                            }));
                        return Ok(req.into_response(response).map_into_boxed_body());
                    }
                },
                None => {
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": {
                                "code": 401,
                                "message": "Missing X-API-Key header"
                            }
                        }));
                    return Ok(req.into_response(response).map_into_boxed_body());
                }
            };

            // Get database pool from app data
            let pool = match req.app_data::<actix_web::web::Data<MySqlPool>>() {
                Some(pool) => pool.get_ref().clone(),
                None => {
                    let response = HttpResponse::InternalServerError()
                        .json(serde_json::json!({
                            "error": {
                                "code": 500,
                                "message": "Database pool not available"
                            }
                        }));
                    return Ok(req.into_response(response).map_into_boxed_body());
                }
            };

            // Verify API key against database
            match verify_api_key(&pool, api_key).await {
                Ok(tenant_id) => {
                    // Store tenant_id in request extensions for downstream use
                    req.extensions_mut().insert(TenantId(tenant_id.clone()));
                    
                    // Update last_used_at timestamp (fire and forget)
                    let pool_clone = pool.clone();
                    let api_key_clone = api_key.to_string();
                    actix_web::rt::spawn(async move {
                        let _ = update_last_used(&pool_clone, &api_key_clone).await;
                    });

                    service.call(req).await
                }
                Err(e) => {
                    let response = HttpResponse::Unauthorized()
                        .json(serde_json::json!({
                            "error": {
                                "code": 401,
                                "message": format!("Invalid API key: {}", e)
                            }
                        }));
                    Ok(req.into_response(response).map_into_boxed_body())
                }
            }
        })
    }
}

/// Tenant ID extracted from authenticated API key
#[derive(Debug, Clone)]
pub struct TenantId(pub String);

/// Verify API key against database using argon2
async fn verify_api_key(pool: &MySqlPool, api_key: &str) -> Result<String, String> {
    // Query database for API key hash (runtime query)
    let result = sqlx::query_as::<_, (String, String, bool)>(
        "SELECT key_hash, tenant_id, is_active FROM api_keys WHERE is_active = TRUE"
    )
    .fetch_all(pool)
    .await
    .map_err(|e| format!("Database error: {}", e))?;

    // Verify against each active key (argon2 verification)
    let argon2 = Argon2::default();
    for (key_hash, tenant_id, _is_active) in result {
        if let Ok(parsed_hash) = PasswordHash::new(&key_hash) {
            if argon2
                .verify_password(api_key.as_bytes(), &parsed_hash)
                .is_ok()
            {
                return Ok(tenant_id);
            }
        }
    }

    Err("Invalid API key".to_string())
}

/// Update last_used_at timestamp for API key
async fn update_last_used(pool: &MySqlPool, api_key: &str) -> Result<(), sqlx::Error> {
    // This is a best-effort update, we don't need to verify the key again
    // We'll update based on the hash matching
    let argon2 = Argon2::default();
    
    let keys = sqlx::query_as::<_, (i64, String)>(
        "SELECT id, key_hash FROM api_keys WHERE is_active = TRUE"
    )
    .fetch_all(pool)
    .await?;

    for (id, key_hash) in keys {
        if let Ok(parsed_hash) = PasswordHash::new(&key_hash) {
            if argon2
                .verify_password(api_key.as_bytes(), &parsed_hash)
                .is_ok()
            {
                sqlx::query("UPDATE api_keys SET last_used_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_id_extraction() {
        let tenant_id = TenantId("tenant-123".to_string());
        assert_eq!(tenant_id.0, "tenant-123");
    }
}
