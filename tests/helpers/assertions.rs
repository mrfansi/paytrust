// Test Assertion Helpers
//
// Provides common assertions for HTTP responses in integration tests.

use actix_web::http::StatusCode;
use awc::ClientResponse;
use serde_json::Value;

/// Assert HTTP response is successful (2xx)
///
/// # Panics
/// If status code is not 2xx, panics with response details
///
/// # Example
/// ```no_run
/// # use actix_web::test;
/// # use paytrust::test_helpers::*;
/// #[actix_web::test]
/// async fn test_success() {
///     let srv = spawn_test_server().await;
///     let response = srv.get("/health").send().await.unwrap();
///     assert_success(&response);
/// }
/// ```
pub fn assert_success(response: &ClientResponse) {
    let status = response.status();
    assert!(
        status.is_success(),
        "Expected successful response (2xx), got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 201 Created
///
/// # Panics
/// If status code is not 201
///
/// # Example
/// ```no_run
/// # use actix_web::test;
/// # use paytrust::test_helpers::*;
/// #[actix_web::test]
/// async fn test_created() {
///     let srv = spawn_test_server().await;
///     let payload = TestDataFactory::create_invoice_payload();
///     let response = srv.post("/api/invoices").send_json(&payload).await.unwrap();
///     assert_created(&response);
/// }
/// ```
pub fn assert_created(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::CREATED,
        "Expected 201 Created, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 200 OK
///
/// # Panics
/// If status code is not 200
pub fn assert_ok(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::OK,
        "Expected 200 OK, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 400 Bad Request
///
/// # Panics
/// If status code is not 400
///
/// # Example
/// ```no_run
/// # use actix_web::test;
/// # use paytrust::test_helpers::*;
/// #[actix_web::test]
/// async fn test_invalid_payload() {
///     let srv = spawn_test_server().await;
///     let response = srv.post("/api/invoices")
///         .send_json(&serde_json::json!({"invalid": "data"}))
///         .await
///         .unwrap();
///     assert_bad_request(&response);
/// }
/// ```
pub fn assert_bad_request(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Expected 400 Bad Request, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 404 Not Found
///
/// # Panics
/// If status code is not 404
pub fn assert_not_found(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "Expected 404 Not Found, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 401 Unauthorized
///
/// # Panics
/// If status code is not 401
pub fn assert_unauthorized(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "Expected 401 Unauthorized, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 403 Forbidden
///
/// # Panics
/// If status code is not 403
pub fn assert_forbidden(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "Expected 403 Forbidden, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 422 Unprocessable Entity
///
/// # Panics
/// If status code is not 422
pub fn assert_unprocessable_entity(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "Expected 422 Unprocessable Entity, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert HTTP response is 500 Internal Server Error
///
/// # Panics
/// If status code is not 500
pub fn assert_server_error(response: &ClientResponse) {
    let status = response.status();
    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Expected 500 Internal Server Error, got {} {}",
        status.as_u16(),
        status.canonical_reason().unwrap_or("Unknown")
    );
}

/// Assert response body contains expected JSON field
///
/// # Parameters
/// - `body`: JSON response body
/// - `field`: Field name to check
///
/// # Panics
/// If field is not present in JSON
///
/// # Example
/// ```
/// # use serde_json::json;
/// # use paytrust::test_helpers::assert_json_field;
/// let body = json!({"id": "123", "status": "pending"});
/// assert_json_field(&body, "id");
/// assert_json_field(&body, "status");
/// ```
pub fn assert_json_field(body: &Value, field: &str) {
    assert!(
        body.get(field).is_some(),
        "Expected JSON field '{}' not found in response: {}",
        field,
        body
    );
}

/// Assert response body JSON field has expected value
///
/// # Parameters
/// - `body`: JSON response body
/// - `field`: Field name
/// - `expected`: Expected value
///
/// # Panics
/// If field value doesn't match expected
///
/// # Example
/// ```
/// # use serde_json::json;
/// # use paytrust::test_helpers::assert_json_field_eq;
/// let body = json!({"status": "pending", "amount": 100000});
/// assert_json_field_eq(&body, "status", "pending");
/// assert_json_field_eq(&body, "amount", 100000);
/// ```
pub fn assert_json_field_eq<T: PartialEq + std::fmt::Debug>(
    body: &Value,
    field: &str,
    expected: T,
) where
    T: serde::Serialize,
{
    let actual = body.get(field).expect(&format!(
        "Field '{}' not found in response: {}",
        field, body
    ));

    let expected_value = serde_json::to_value(expected).unwrap();

    assert_eq!(
        actual, &expected_value,
        "Field '{}' value mismatch. Expected {:?}, got {:?}",
        field, expected_value, actual
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_assert_json_field() {
        let body = json!({"id": "123", "status": "pending"});
        assert_json_field(&body, "id");
        assert_json_field(&body, "status");
    }

    #[test]
    #[should_panic(expected = "Expected JSON field 'missing'")]
    fn test_assert_json_field_missing() {
        let body = json!({"id": "123"});
        assert_json_field(&body, "missing");
    }

    #[test]
    fn test_assert_json_field_eq() {
        let body = json!({"status": "pending", "amount": 100000});
        assert_json_field_eq(&body, "status", "pending");
        assert_json_field_eq(&body, "amount", 100000);
    }

    #[test]
    #[should_panic(expected = "value mismatch")]
    fn test_assert_json_field_eq_mismatch() {
        let body = json!({"status": "pending"});
        assert_json_field_eq(&body, "status", "active");
    }
}
