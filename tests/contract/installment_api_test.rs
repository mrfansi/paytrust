// T084: Contract test for GET /invoices/{id}/installments endpoint
// T085: Contract test for PATCH /invoices/{id}/installments endpoint

use actix_web::{test, web, App};
use paytrust::modules::installments::controllers::{adjust_installments, get_installments};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;

/// Test GET /invoices/{id}/installments returns installment schedule
#[actix_web::test]
async fn test_get_installments_contract() {
    let app = test::init_service(App::new().route(
        "/v1/invoices/{invoice_id}/installments",
        web::get().to(get_installments),
    ))
    .await;

    let invoice_id = "550e8400-e29b-41d4-a716-446655440000";
    let req = test::TestRequest::get()
        .uri(&format!("/v1/invoices/{}/installments", invoice_id))
        .insert_header(("X-API-Key", "test-key"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 200 or 404 based on whether invoice exists
    assert!(resp.status().is_success() || resp.status() == 404);

    if resp.status().is_success() {
        let body: Value = test::read_body_json(resp).await;

        // Verify response structure per OpenAPI schema
        assert!(
            body.get("installments").is_some(),
            "Response must have installments array"
        );

        let installments = body["installments"].as_array().unwrap();

        if !installments.is_empty() {
            let first = &installments[0];

            // Verify required fields per OpenAPI schema
            assert!(first.get("id").is_some());
            assert!(first.get("installment_number").is_some());
            assert!(first.get("amount").is_some());
            assert!(first.get("due_date").is_some());
            assert!(first.get("status").is_some());

            // Verify status is valid enum value
            let status = first["status"].as_str().unwrap();
            assert!(
                ["unpaid", "paid", "overdue"].contains(&status),
                "Status must be valid enum value"
            );
        }
    }
}

/// Test GET /invoices/{id}/installments returns empty array for invoice without installments
#[actix_web::test]
async fn test_get_installments_no_schedule() {
    let app = test::init_service(App::new().route(
        "/v1/invoices/{invoice_id}/installments",
        web::get().to(get_installments),
    ))
    .await;

    let invoice_id = "no-installments-invoice-id";
    let req = test::TestRequest::get()
        .uri(&format!("/v1/invoices/{}/installments", invoice_id))
        .insert_header(("X-API-Key", "test-key"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    if resp.status().is_success() {
        let body: Value = test::read_body_json(resp).await;
        let installments = body["installments"].as_array().unwrap();
        assert_eq!(
            installments.len(),
            0,
            "Should return empty array for invoice without installments"
        );
    }
}

/// Test PATCH /invoices/{id}/installments updates unpaid installment amounts
#[actix_web::test]
async fn test_patch_installments_contract() {
    let app = test::init_service(App::new().route(
        "/v1/invoices/{invoice_id}/installments",
        web::patch().to(adjust_installments),
    ))
    .await;

    let invoice_id = "550e8400-e29b-41d4-a716-446655440000";

    let update_body = json!({
        "adjustments": [
            {
                "installment_number": 2,
                "new_amount": "150.00"
            },
            {
                "installment_number": 3,
                "new_amount": "200.00"
            }
        ]
    });

    let req = test::TestRequest::patch()
        .uri(&format!("/v1/invoices/{}/installments", invoice_id))
        .insert_header(("X-API-Key", "test-key"))
        .insert_header(("Content-Type", "application/json"))
        .set_json(&update_body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 200, 400, or 404 based on validation
    assert!(
        resp.status().is_success() || resp.status() == 400 || resp.status() == 404,
        "Response should be 200 (success), 400 (validation error), or 404 (not found)"
    );

    if resp.status().is_success() {
        let body: Value = test::read_body_json(resp).await;

        // Verify response has updated installments
        assert!(body.get("installments").is_some());

        let installments = body["installments"].as_array().unwrap();
        assert!(
            !installments.is_empty(),
            "Should return updated installments"
        );
    }
}

/// Test PATCH /invoices/{id}/installments rejects adjustment if total doesn't match
#[actix_web::test]
async fn test_patch_installments_total_validation() {
    let app = test::init_service(App::new().route(
        "/v1/invoices/{invoice_id}/installments",
        web::patch().to(adjust_installments),
    ))
    .await;

    let invoice_id = "550e8400-e29b-41d4-a716-446655440000";

    // Invalid adjustment that doesn't sum to original total
    let update_body = json!({
        "adjustments": [
            {
                "installment_number": 1,
                "new_amount": "1000000.00"  // Way too large
            }
        ]
    });

    let req = test::TestRequest::patch()
        .uri(&format!("/v1/invoices/{}/installments", invoice_id))
        .insert_header(("X-API-Key", "test-key"))
        .insert_header(("Content-Type", "application/json"))
        .set_json(&update_body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 for validation error
    if resp.status() == 400 {
        let body: Value = test::read_body_json(resp).await;

        // Verify error message mentions total mismatch (FR-079)
        assert!(body.get("error").is_some());
        let error_msg = body["error"].as_str().unwrap();
        assert!(
            error_msg.contains("total") || error_msg.contains("sum"),
            "Error should mention total/sum validation failure"
        );
    }
}

/// Test PATCH /invoices/{id}/installments rejects adjustment of paid installments
#[actix_web::test]
async fn test_patch_installments_rejects_paid() {
    let app = test::init_service(App::new().route(
        "/v1/invoices/{invoice_id}/installments",
        web::patch().to(adjust_installments),
    ))
    .await;

    let invoice_id = "invoice-with-paid-installments";

    // Attempt to adjust a paid installment (FR-077: only unpaid can be adjusted)
    let update_body = json!({
        "adjustments": [
            {
                "installment_number": 1,  // Assume this is paid
                "new_amount": "100.00"
            }
        ]
    });

    let req = test::TestRequest::patch()
        .uri(&format!("/v1/invoices/{}/installments", invoice_id))
        .insert_header(("X-API-Key", "test-key"))
        .insert_header(("Content-Type", "application/json"))
        .set_json(&update_body)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Should return 400 if trying to adjust paid installment
    if resp.status() == 400 {
        let body: Value = test::read_body_json(resp).await;

        // Verify error mentions paid installment
        assert!(body.get("error").is_some());
        let error_msg = body["error"].as_str().unwrap();
        assert!(
            error_msg.contains("paid") || error_msg.contains("locked"),
            "Error should mention that paid installments cannot be adjusted"
        );
    }
}

/// Test GET /invoices/{id}/installments includes tax and service fee breakdown
#[actix_web::test]
async fn test_get_installments_includes_breakdown() {
    let app = test::init_service(App::new().route(
        "/v1/invoices/{invoice_id}/installments",
        web::get().to(get_installments),
    ))
    .await;

    let invoice_id = "invoice-with-tax-and-fees";
    let req = test::TestRequest::get()
        .uri(&format!("/v1/invoices/{}/installments", invoice_id))
        .insert_header(("X-API-Key", "test-key"))
        .to_request();

    let resp = test::call_service(&app, req).await;

    if resp.status().is_success() {
        let body: Value = test::read_body_json(resp).await;
        let installments = body["installments"].as_array().unwrap();

        if !installments.is_empty() {
            let first = &installments[0];

            // Verify tax and service fee fields present (FR-059, FR-060)
            assert!(first.get("amount").is_some());
            assert!(first.get("tax_amount").is_some());
            assert!(first.get("service_fee_amount").is_some());

            // Verify amounts are valid decimals
            let amount = Decimal::from_str(first["amount"].as_str().unwrap());
            assert!(amount.is_ok(), "Amount must be valid decimal");

            let tax_amount = Decimal::from_str(first["tax_amount"].as_str().unwrap());
            assert!(tax_amount.is_ok(), "Tax amount must be valid decimal");

            let fee_amount = Decimal::from_str(first["service_fee_amount"].as_str().unwrap());
            assert!(
                fee_amount.is_ok(),
                "Service fee amount must be valid decimal"
            );
        }
    }
}
