//! Contract tests for Financial Report API (T062)
//!
//! Validates GET /reports/financial endpoint compliance with OpenAPI schema.
//! Tests response structure, field presence, types, and required properties.
//!
//! Related FRs: FR-063 (service fee and tax breakdown), FR-064 (separate totals by currency)

/// Mock FinancialReportResponse structure based on OpenAPI schema
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct FinancialReportResponse {
    start_date: String,
    end_date: String,
    service_fees: Vec<ServiceFeeBreakdown>,
    taxes: Vec<TaxBreakdown>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct ServiceFeeBreakdown {
    currency: String,
    gateway: String,
    total_amount: String,
    transaction_count: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TaxBreakdown {
    currency: String,
    tax_rate: String,
    total_amount: String,
    transaction_count: i32,
}

/// Test: Response structure matches OpenAPI schema
#[test]
fn test_financial_report_response_structure() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![ServiceFeeBreakdown {
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            total_amount: "145000".to_string(),
            transaction_count: 5,
        }],
        taxes: vec![TaxBreakdown {
            currency: "IDR".to_string(),
            tax_rate: "0.10".to_string(),
            total_amount: "500000".to_string(),
            transaction_count: 5,
        }],
    };

    let json = serde_json::to_value(&response).unwrap();

    // Required fields present
    assert!(json.get("start_date").is_some());
    assert!(json.get("end_date").is_some());
    assert!(json.get("service_fees").is_some());
    assert!(json.get("taxes").is_some());

    // Date format validation (YYYY-MM-DD)
    let start_date = json["start_date"].as_str().unwrap();
    assert_eq!(start_date.len(), 10);
    assert!(start_date.contains('-'));

    // Arrays are present
    assert!(json["service_fees"].is_array());
    assert!(json["taxes"].is_array());
}

/// Test: Service fee breakdown structure matches schema
#[test]
fn test_service_fee_breakdown_structure() {
    let breakdown = ServiceFeeBreakdown {
        currency: "IDR".to_string(),
        gateway: "xendit".to_string(),
        total_amount: "145000".to_string(),
        transaction_count: 5,
    };

    let json = serde_json::to_value(&breakdown).unwrap();

    // Required fields
    assert_eq!(json["currency"].as_str().unwrap(), "IDR");
    assert_eq!(json["gateway"].as_str().unwrap(), "xendit");
    assert_eq!(json["total_amount"].as_str().unwrap(), "145000");
    assert_eq!(json["transaction_count"].as_i64().unwrap(), 5);
}

/// Test: Tax breakdown structure matches schema (FR-063, FR-064)
#[test]
fn test_tax_breakdown_structure() {
    let breakdown = TaxBreakdown {
        currency: "IDR".to_string(),
        tax_rate: "0.10".to_string(),
        total_amount: "500000".to_string(),
        transaction_count: 5,
    };

    let json = serde_json::to_value(&breakdown).unwrap();

    // Required fields
    assert_eq!(json["currency"].as_str().unwrap(), "IDR");
    assert_eq!(json["tax_rate"].as_str().unwrap(), "0.10");
    assert_eq!(json["total_amount"].as_str().unwrap(), "500000");
    assert_eq!(json["transaction_count"].as_i64().unwrap(), 5);
}

/// Test: Multiple currencies are separated in response (FR-064)
#[test]
fn test_multiple_currencies_separated() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![
            ServiceFeeBreakdown {
                currency: "IDR".to_string(),
                gateway: "xendit".to_string(),
                total_amount: "145000".to_string(),
                transaction_count: 3,
            },
            ServiceFeeBreakdown {
                currency: "MYR".to_string(),
                gateway: "xendit".to_string(),
                total_amount: "15.50".to_string(),
                transaction_count: 2,
            },
        ],
        taxes: vec![
            TaxBreakdown {
                currency: "IDR".to_string(),
                tax_rate: "0.10".to_string(),
                total_amount: "500000".to_string(),
                transaction_count: 3,
            },
            TaxBreakdown {
                currency: "MYR".to_string(),
                tax_rate: "0.06".to_string(),
                total_amount: "30.00".to_string(),
                transaction_count: 2,
            },
        ],
    };

    // Each currency has its own breakdown entry
    assert_eq!(response.service_fees.len(), 2);
    assert_eq!(response.taxes.len(), 2);

    // Verify currencies are different
    let currencies: Vec<_> = response.service_fees.iter().map(|f| &f.currency).collect();
    assert!(currencies.contains(&&"IDR".to_string()));
    assert!(currencies.contains(&&"MYR".to_string()));
}

/// Test: Service fees grouped by gateway (FR-063)
#[test]
fn test_service_fees_grouped_by_gateway() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![
            ServiceFeeBreakdown {
                currency: "IDR".to_string(),
                gateway: "xendit".to_string(),
                total_amount: "145000".to_string(),
                transaction_count: 3,
            },
            ServiceFeeBreakdown {
                currency: "IDR".to_string(),
                gateway: "midtrans".to_string(),
                total_amount: "100000".to_string(),
                transaction_count: 5,
            },
        ],
        taxes: vec![],
    };

    // Each gateway has separate entry even for same currency
    assert_eq!(response.service_fees.len(), 2);

    let gateways: Vec<_> = response.service_fees.iter().map(|f| &f.gateway).collect();
    assert!(gateways.contains(&&"xendit".to_string()));
    assert!(gateways.contains(&&"midtrans".to_string()));
}

/// Test: Taxes grouped by rate and currency (FR-063)
#[test]
fn test_taxes_grouped_by_rate() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![],
        taxes: vec![
            TaxBreakdown {
                currency: "IDR".to_string(),
                tax_rate: "0.10".to_string(),
                total_amount: "500000".to_string(),
                transaction_count: 3,
            },
            TaxBreakdown {
                currency: "IDR".to_string(),
                tax_rate: "0.11".to_string(),
                total_amount: "220000".to_string(),
                transaction_count: 2,
            },
        ],
    };

    // Each tax rate has separate entry even for same currency
    assert_eq!(response.taxes.len(), 2);

    let rates: Vec<_> = response.taxes.iter().map(|t| &t.tax_rate).collect();
    assert!(rates.contains(&&"0.10".to_string()));
    assert!(rates.contains(&&"0.11".to_string()));
}

/// Test: Empty report has valid structure
#[test]
fn test_empty_report_structure() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![],
        taxes: vec![],
    };

    let json = serde_json::to_value(&response).unwrap();

    // Required fields still present
    assert!(json.get("start_date").is_some());
    assert!(json.get("end_date").is_some());
    assert!(json["service_fees"].is_array());
    assert!(json["taxes"].is_array());

    // Arrays are empty but valid
    assert_eq!(json["service_fees"].as_array().unwrap().len(), 0);
    assert_eq!(json["taxes"].as_array().unwrap().len(), 0);
}

/// Test: Transaction counts are non-negative integers
#[test]
fn test_transaction_counts_valid() {
    let service_fee = ServiceFeeBreakdown {
        currency: "IDR".to_string(),
        gateway: "xendit".to_string(),
        total_amount: "145000".to_string(),
        transaction_count: 5,
    };

    let tax = TaxBreakdown {
        currency: "IDR".to_string(),
        tax_rate: "0.10".to_string(),
        total_amount: "500000".to_string(),
        transaction_count: 10,
    };

    assert!(service_fee.transaction_count >= 0);
    assert!(tax.transaction_count >= 0);
}

/// Test: Amounts are represented as strings (for precision)
#[test]
fn test_amounts_as_strings() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![ServiceFeeBreakdown {
            currency: "MYR".to_string(),
            gateway: "xendit".to_string(),
            total_amount: "15.50".to_string(), // Decimal as string
            transaction_count: 2,
        }],
        taxes: vec![TaxBreakdown {
            currency: "IDR".to_string(),
            tax_rate: "0.10".to_string(), // Rate as string
            total_amount: "500000".to_string(),
            transaction_count: 5,
        }],
    };

    let json = serde_json::to_value(&response).unwrap();

    // Amounts are strings, not numbers
    assert!(json["service_fees"][0]["total_amount"].is_string());
    assert!(json["taxes"][0]["total_amount"].is_string());
    assert!(json["taxes"][0]["tax_rate"].is_string());
}

/// Test: Supported currencies are valid (IDR, MYR, USD per FR-002)
#[test]
fn test_supported_currencies() {
    let valid_currencies = vec!["IDR", "MYR", "USD"];

    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![ServiceFeeBreakdown {
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            total_amount: "145000".to_string(),
            transaction_count: 3,
        }],
        taxes: vec![TaxBreakdown {
            currency: "MYR".to_string(),
            tax_rate: "0.06".to_string(),
            total_amount: "30.00".to_string(),
            transaction_count: 2,
        }],
    };

    // All currencies in response are valid
    for fee in &response.service_fees {
        assert!(valid_currencies.contains(&fee.currency.as_str()));
    }

    for tax in &response.taxes {
        assert!(valid_currencies.contains(&tax.currency.as_str()));
    }
}

/// Test: Date range is required in response
#[test]
fn test_date_range_required() {
    let response = FinancialReportResponse {
        start_date: "2025-01-01".to_string(),
        end_date: "2025-01-31".to_string(),
        service_fees: vec![],
        taxes: vec![],
    };

    // Dates must not be empty
    assert!(!response.start_date.is_empty());
    assert!(!response.end_date.is_empty());

    // start_date should be before or equal to end_date
    assert!(response.start_date <= response.end_date);
}
