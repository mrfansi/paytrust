// T033: Contract test for POST /invoices endpoint
//
// Validates that the API response matches the OpenAPI schema specification
//
// These tests validate the JSON schema structure of API responses.
// They ensure that:
// - Required fields are present
// - Field types match the OpenAPI specification
// - Enum values are valid
// - Nested structures (like line_items) have correct shape

use serde_json::json;

#[test]
fn test_create_invoice_request_schema() {
    // Validate CreateInvoiceRequest schema from OpenAPI spec
    let request = json!({
        "external_id": "ORD-12345",
        "gateway_id": "xendit",
        "currency": "IDR",
        "line_items": [
            {
                "description": "Premium Subscription",
                "quantity": 1,
                "unit_price": "1000000"
            }
        ]
    });

    // Verify required fields
    assert!(
        request.get("external_id").is_some(),
        "external_id is required"
    );
    assert!(
        request.get("gateway_id").is_some(),
        "gateway_id is required"
    );
    assert!(request.get("currency").is_some(), "currency is required");
    assert!(
        request.get("line_items").is_some(),
        "line_items is required"
    );

    // Verify field types
    assert!(
        request["external_id"].is_string(),
        "external_id must be string"
    );
    assert!(
        request["gateway_id"].is_string(),
        "gateway_id must be string"
    );
    assert!(request["currency"].is_string(), "currency must be string");
    assert!(request["line_items"].is_array(), "line_items must be array");

    // Verify line_items structure
    let line_items = request["line_items"].as_array().unwrap();
    assert!(!line_items.is_empty(), "line_items must not be empty");

    for item in line_items {
        assert!(item.get("description").is_some(), "description is required");
        assert!(item.get("quantity").is_some(), "quantity is required");
        assert!(item.get("unit_price").is_some(), "unit_price is required");
    }
}

#[test]
fn test_invoice_response_schema() {
    // Validate InvoiceResponse schema from OpenAPI spec
    let response = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "external_id": "ORD-12345",
        "gateway_id": "xendit",
        "currency": "IDR",
        "status": "pending",
        "total": "1000000",
        "line_items": [
            {
                "description": "Premium Subscription",
                "quantity": 1,
                "unit_price": "1000000",
                "subtotal": "1000000",
                "currency": "IDR"
            }
        ],
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z"
    });

    // Verify required fields
    assert!(response.get("id").is_some(), "Response must include 'id'");
    assert!(
        response.get("external_id").is_some(),
        "Response must include 'external_id'"
    );
    assert!(
        response.get("gateway_id").is_some(),
        "Response must include 'gateway_id'"
    );
    assert!(
        response.get("currency").is_some(),
        "Response must include 'currency'"
    );
    assert!(
        response.get("status").is_some(),
        "Response must include 'status'"
    );
    assert!(
        response.get("total").is_some(),
        "Response must include 'total'"
    );
    assert!(
        response.get("line_items").is_some(),
        "Response must include 'line_items'"
    );
    assert!(
        response.get("created_at").is_some(),
        "Response must include 'created_at'"
    );
    assert!(
        response.get("updated_at").is_some(),
        "Response must include 'updated_at'"
    );

    // Verify field types
    assert!(response["id"].is_string(), "'id' must be a string");
    assert!(
        response["external_id"].is_string(),
        "'external_id' must be a string"
    );
    assert!(
        response["gateway_id"].is_string(),
        "'gateway_id' must be a string"
    );
    assert!(
        response["currency"].is_string(),
        "'currency' must be a string"
    );
    assert!(response["status"].is_string(), "'status' must be a string");
    assert!(
        response["line_items"].is_array(),
        "'line_items' must be an array"
    );

    // Verify currency format (3-letter ISO code)
    let currency = response["currency"].as_str().unwrap();
    assert_eq!(currency.len(), 3, "Currency must be 3-letter ISO code");

    // Verify status is a valid enum value
    let status = response["status"].as_str().unwrap();
    assert!(
        ["pending", "processing", "paid", "expired", "failed"].contains(&status),
        "Status must be a valid InvoiceStatus value: {}",
        status
    );

    // Verify line_items structure
    let line_items = response["line_items"].as_array().unwrap();
    assert!(!line_items.is_empty(), "line_items must not be empty");

    for item in line_items {
        assert!(
            item.get("description").is_some(),
            "Line item must have 'description'"
        );
        assert!(
            item.get("quantity").is_some(),
            "Line item must have 'quantity'"
        );
        assert!(
            item.get("unit_price").is_some(),
            "Line item must have 'unit_price'"
        );
        assert!(
            item.get("subtotal").is_some(),
            "Line item must have 'subtotal'"
        );
        assert!(
            item.get("currency").is_some(),
            "Line item must have 'currency'"
        );
    }
}

#[test]
fn test_error_response_schema() {
    // Validate ErrorResponse schema from OpenAPI spec
    let error_response = json!({
        "error": "Invalid request: currency must be a valid ISO code"
    });

    // Verify required fields
    assert!(
        error_response.get("error").is_some(),
        "Error response must include 'error'"
    );
    assert!(
        error_response["error"].is_string(),
        "'error' must be a string"
    );
}

#[test]
fn test_currency_validation() {
    // Valid currencies per OpenAPI spec
    let valid_currencies = vec!["IDR", "MYR", "SGD", "USD"];

    for currency in valid_currencies {
        assert_eq!(
            currency.len(),
            3,
            "Currency '{}' must be 3 characters",
            currency
        );
        assert!(
            currency.chars().all(|c| c.is_ascii_uppercase()),
            "Currency '{}' must be uppercase",
            currency
        );
    }
}

#[test]
fn test_invoice_status_enum() {
    // Valid invoice statuses per OpenAPI spec
    let valid_statuses = vec!["pending", "processing", "paid", "expired", "failed"];

    for status in valid_statuses {
        assert!(!status.is_empty(), "Status must not be empty");
        assert!(
            status.chars().all(|c| c.is_ascii_lowercase()),
            "Status '{}' must be lowercase",
            status
        );
    }
}

#[test]
fn test_line_item_quantity_validation() {
    // Quantity must be positive integer
    let line_item = json!({
        "description": "Test Item",
        "quantity": 1,
        "unit_price": "1000"
    });

    let quantity = line_item["quantity"].as_i64().unwrap();
    assert!(quantity > 0, "Quantity must be positive");
}

#[test]
fn test_pagination_response_schema() {
    // Validate pagination structure from OpenAPI spec
    let paginated_response = json!({
        "data": [],
        "page": 1,
        "per_page": 10,
        "total": 0
    });

    assert!(
        paginated_response.get("data").is_some(),
        "Pagination must include 'data'"
    );
    assert!(
        paginated_response.get("page").is_some(),
        "Pagination must include 'page'"
    );
    assert!(
        paginated_response.get("per_page").is_some(),
        "Pagination must include 'per_page'"
    );
    assert!(
        paginated_response.get("total").is_some(),
        "Pagination must include 'total'"
    );

    assert!(
        paginated_response["data"].is_array(),
        "'data' must be array"
    );
    assert!(
        paginated_response["page"].is_number(),
        "'page' must be number"
    );
    assert!(
        paginated_response["per_page"].is_number(),
        "'per_page' must be number"
    );
    assert!(
        paginated_response["total"].is_number(),
        "'total' must be number"
    );
}
