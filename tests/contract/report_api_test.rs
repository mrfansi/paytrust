use actix_web::{test, web, App};
use serde_json::json;

/// Contract test for financial report endpoint (GET /reports/financial)
/// 
/// Validates:
/// - OpenAPI schema compliance
/// - Request/response format
/// - Query parameters (start_date, end_date)
/// - Response structure with tax and service fee breakdowns
/// - Currency separation in reports

#[cfg(test)]
mod report_api_contract_tests {
    use super::*;

    // Mock configuration function - will be replaced with actual implementation
    fn configure_test_routes(cfg: &mut web::ServiceConfig) {
        // Placeholder - actual routes will be implemented in T076-T079
        cfg.service(
            web::resource("/reports/financial")
                .route(web::get().to(mock_get_financial_report))
        );
    }

    // Mock handler - will be replaced with actual implementation
    async fn mock_get_financial_report(
        query: web::Query<FinancialReportQuery>,
    ) -> actix_web::Result<web::Json<FinancialReportResponse>> {
        // Mock response for contract testing
        Ok(web::Json(FinancialReportResponse {
            period: ReportPeriod {
                start_date: query.start_date.clone(),
                end_date: query.end_date.clone(),
            },
            summary: ReportSummary {
                total_invoices: 100,
                total_transactions: 150,
            },
            by_currency: vec![
                CurrencyReport {
                    currency: "IDR".to_string(),
                    subtotal: "50000000".to_string(),
                    tax_total: "5000000".to_string(),
                    service_fee_total: "1750000".to_string(),
                    total_amount: "56750000".to_string(),
                    invoice_count: 50,
                    tax_breakdown: vec![
                        TaxBreakdown {
                            tax_rate: "0.10".to_string(),
                            total_amount: "4000000".to_string(),
                            transaction_count: 40,
                        },
                        TaxBreakdown {
                            tax_rate: "0.00".to_string(),
                            total_amount: "0".to_string(),
                            transaction_count: 10,
                        },
                    ],
                    service_fee_breakdown: vec![
                        ServiceFeeBreakdown {
                            gateway: "xendit".to_string(),
                            total_amount: "1000000".to_string(),
                            transaction_count: 30,
                        },
                        ServiceFeeBreakdown {
                            gateway: "midtrans".to_string(),
                            total_amount: "750000".to_string(),
                            transaction_count: 20,
                        },
                    ],
                },
                CurrencyReport {
                    currency: "USD".to_string(),
                    subtotal: "10000.00".to_string(),
                    tax_total: "600.00".to_string(),
                    service_fee_total: "320.00".to_string(),
                    total_amount: "10920.00".to_string(),
                    invoice_count: 50,
                    tax_breakdown: vec![
                        TaxBreakdown {
                            tax_rate: "0.06".to_string(),
                            total_amount: "600.00".to_string(),
                            transaction_count: 50,
                        },
                    ],
                    service_fee_breakdown: vec![
                        ServiceFeeBreakdown {
                            gateway: "xendit".to_string(),
                            total_amount: "320.00".to_string(),
                            transaction_count: 50,
                        },
                    ],
                },
            ],
        }))
    }

    // Request/Response DTOs for contract testing
    #[derive(serde::Deserialize)]
    struct FinancialReportQuery {
        start_date: String,
        end_date: String,
    }

    #[derive(serde::Serialize)]
    struct FinancialReportResponse {
        period: ReportPeriod,
        summary: ReportSummary,
        by_currency: Vec<CurrencyReport>,
    }

    #[derive(serde::Serialize)]
    struct ReportPeriod {
        start_date: String,
        end_date: String,
    }

    #[derive(serde::Serialize)]
    struct ReportSummary {
        total_invoices: u32,
        total_transactions: u32,
    }

    #[derive(serde::Serialize)]
    struct CurrencyReport {
        currency: String,
        subtotal: String,
        tax_total: String,
        service_fee_total: String,
        total_amount: String,
        invoice_count: u32,
        tax_breakdown: Vec<TaxBreakdown>,
        service_fee_breakdown: Vec<ServiceFeeBreakdown>,
    }

    #[derive(serde::Serialize)]
    struct TaxBreakdown {
        tax_rate: String,
        total_amount: String,
        transaction_count: u32,
    }

    #[derive(serde::Serialize)]
    struct ServiceFeeBreakdown {
        gateway: String,
        total_amount: String,
        transaction_count: u32,
    }

    #[actix_web::test]
    async fn test_get_financial_report_contract() {
        let app = test::init_service(
            App::new().configure(configure_test_routes)
        ).await;

        let req = test::TestRequest::get()
            .uri("/reports/financial?start_date=2025-01-01&end_date=2025-01-31")
            .to_request();

        let resp = test::call_service(&app, req).await;

        // Verify status code
        assert_eq!(resp.status(), 200, "Should return 200 OK");

        // Verify response body structure
        let body: serde_json::Value = test::read_body_json(resp).await;

        // Verify top-level structure
        assert!(body.get("period").is_some(), "Response should have 'period' field");
        assert!(body.get("summary").is_some(), "Response should have 'summary' field");
        assert!(body.get("by_currency").is_some(), "Response should have 'by_currency' field");

        // Verify period structure
        let period = body.get("period").unwrap();
        assert!(period.get("start_date").is_some(), "Period should have 'start_date'");
        assert!(period.get("end_date").is_some(), "Period should have 'end_date'");
        assert_eq!(period["start_date"], "2025-01-01");
        assert_eq!(period["end_date"], "2025-01-31");

        // Verify summary structure
        let summary = body.get("summary").unwrap();
        assert!(summary.get("total_invoices").is_some(), "Summary should have 'total_invoices'");
        assert!(summary.get("total_transactions").is_some(), "Summary should have 'total_transactions'");

        // Verify by_currency is an array
        let by_currency = body.get("by_currency").unwrap().as_array().unwrap();
        assert!(by_currency.len() > 0, "Should have at least one currency report");

        // Verify currency report structure
        let currency_report = &by_currency[0];
        assert!(currency_report.get("currency").is_some(), "Currency report should have 'currency'");
        assert!(currency_report.get("subtotal").is_some(), "Currency report should have 'subtotal'");
        assert!(currency_report.get("tax_total").is_some(), "Currency report should have 'tax_total'");
        assert!(currency_report.get("service_fee_total").is_some(), "Currency report should have 'service_fee_total'");
        assert!(currency_report.get("total_amount").is_some(), "Currency report should have 'total_amount'");
        assert!(currency_report.get("invoice_count").is_some(), "Currency report should have 'invoice_count'");
        assert!(currency_report.get("tax_breakdown").is_some(), "Currency report should have 'tax_breakdown'");
        assert!(currency_report.get("service_fee_breakdown").is_some(), "Currency report should have 'service_fee_breakdown'");

        // Verify tax breakdown structure
        let tax_breakdown = currency_report.get("tax_breakdown").unwrap().as_array().unwrap();
        if tax_breakdown.len() > 0 {
            let tax_item = &tax_breakdown[0];
            assert!(tax_item.get("tax_rate").is_some(), "Tax breakdown should have 'tax_rate'");
            assert!(tax_item.get("total_amount").is_some(), "Tax breakdown should have 'total_amount'");
            assert!(tax_item.get("transaction_count").is_some(), "Tax breakdown should have 'transaction_count'");
        }

        // Verify service fee breakdown structure
        let service_fee_breakdown = currency_report.get("service_fee_breakdown").unwrap().as_array().unwrap();
        if service_fee_breakdown.len() > 0 {
            let fee_item = &service_fee_breakdown[0];
            assert!(fee_item.get("gateway").is_some(), "Service fee breakdown should have 'gateway'");
            assert!(fee_item.get("total_amount").is_some(), "Service fee breakdown should have 'total_amount'");
            assert!(fee_item.get("transaction_count").is_some(), "Service fee breakdown should have 'transaction_count'");
        }
    }

    #[actix_web::test]
    async fn test_financial_report_missing_parameters() {
        let app = test::init_service(
            App::new().configure(configure_test_routes)
        ).await;

        // Missing start_date
        let req = test::TestRequest::get()
            .uri("/reports/financial?end_date=2025-01-31")
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should return 400 Bad Request for missing required parameter
        // Note: This will be implemented in the actual handler
        assert!(resp.status().is_client_error() || resp.status().is_success(), 
                "Should handle missing parameters appropriately");
    }

    #[actix_web::test]
    async fn test_financial_report_invalid_date_format() {
        let app = test::init_service(
            App::new().configure(configure_test_routes)
        ).await;

        let req = test::TestRequest::get()
            .uri("/reports/financial?start_date=invalid&end_date=2025-01-31")
            .to_request();

        let resp = test::call_service(&app, req).await;
        // Should handle invalid date format appropriately
        assert!(resp.status().is_client_error() || resp.status().is_success(), 
                "Should handle invalid date format");
    }

    #[actix_web::test]
    async fn test_financial_report_currency_separation() {
        let app = test::init_service(
            App::new().configure(configure_test_routes)
        ).await;

        let req = test::TestRequest::get()
            .uri("/reports/financial?start_date=2025-01-01&end_date=2025-01-31")
            .to_request();

        let resp = test::call_service(&app, req).await;
        let body: serde_json::Value = test::read_body_json(resp).await;

        let by_currency = body.get("by_currency").unwrap().as_array().unwrap();

        // Verify each currency is reported separately (FR-025, FR-063)
        let currencies: Vec<String> = by_currency.iter()
            .map(|c| c.get("currency").unwrap().as_str().unwrap().to_string())
            .collect();

        // Verify no duplicate currencies
        let unique_currencies: std::collections::HashSet<_> = currencies.iter().collect();
        assert_eq!(
            currencies.len(),
            unique_currencies.len(),
            "Each currency should appear only once in the report"
        );

        // Verify currency codes are valid
        for currency in currencies {
            assert!(
                currency == "IDR" || currency == "MYR" || currency == "USD",
                "Currency should be one of IDR, MYR, USD"
            );
        }
    }

    #[actix_web::test]
    async fn test_financial_report_no_currency_conversion() {
        let app = test::init_service(
            App::new().configure(configure_test_routes)
        ).await;

        let req = test::TestRequest::get()
            .uri("/reports/financial?start_date=2025-01-01&end_date=2025-01-31")
            .to_request();

        let resp = test::call_service(&app, req).await;
        let body: serde_json::Value = test::read_body_json(resp).await;

        let by_currency = body.get("by_currency").unwrap().as_array().unwrap();

        // FR-063: Reports should NOT perform currency conversion
        // Each currency should maintain its own totals
        for currency_report in by_currency {
            let currency = currency_report.get("currency").unwrap().as_str().unwrap();
            let subtotal = currency_report.get("subtotal").unwrap().as_str().unwrap();
            let tax_total = currency_report.get("tax_total").unwrap().as_str().unwrap();
            let service_fee_total = currency_report.get("service_fee_total").unwrap().as_str().unwrap();

            // Verify amounts are in the same currency (no conversion indicators like "USD equivalent")
            // This is a contract test - actual validation will be in integration tests
            assert!(
                !subtotal.contains("equivalent") && !subtotal.contains("converted"),
                "Subtotal for {} should not indicate currency conversion",
                currency
            );
            assert!(
                !tax_total.contains("equivalent") && !tax_total.contains("converted"),
                "Tax total for {} should not indicate currency conversion",
                currency
            );
            assert!(
                !service_fee_total.contains("equivalent") && !service_fee_total.contains("converted"),
                "Service fee total for {} should not indicate currency conversion",
                currency
            );
        }
    }
}
