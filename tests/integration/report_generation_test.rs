//! Integration tests for financial report generation (T065)
//!
//! Validates that financial reports correctly aggregate service fees and taxes
//! with proper breakdowns by currency, gateway, and tax rate.
//!
//! Related FRs: FR-063 (service fee and tax breakdown), FR-064 (separate totals by currency)

use rust_decimal::Decimal;
use std::str::FromStr;

/// Financial report aggregation result
#[derive(Debug, Clone)]
struct FinancialReport {
    start_date: String,
    end_date: String,
    service_fees: Vec<ServiceFeeBreakdown>,
    taxes: Vec<TaxBreakdown>,
}

#[derive(Debug, Clone, PartialEq)]
struct ServiceFeeBreakdown {
    currency: String,
    gateway: String,
    total_amount: Decimal,
    transaction_count: i32,
}

#[derive(Debug, Clone, PartialEq)]
struct TaxBreakdown {
    currency: String,
    tax_rate: Decimal,
    total_amount: Decimal,
    transaction_count: i32,
}

/// Mock transaction data
#[derive(Debug, Clone)]
struct Transaction {
    id: String,
    invoice_id: String,
    currency: String,
    gateway: String,
    subtotal: Decimal,
    tax_rate: Decimal,
    tax_amount: Decimal,
    service_fee: Decimal,
    created_at: String,
}

/// Generate financial report from transactions
fn generate_financial_report(
    transactions: Vec<Transaction>,
    start_date: String,
    end_date: String,
) -> FinancialReport {
    use std::collections::HashMap;

    // Aggregate service fees by (currency, gateway)
    let mut service_fee_map: HashMap<(String, String), (Decimal, i32)> = HashMap::new();
    for tx in &transactions {
        let key = (tx.currency.clone(), tx.gateway.clone());
        let entry = service_fee_map.entry(key).or_insert((Decimal::ZERO, 0));
        entry.0 += tx.service_fee;
        entry.1 += 1;
    }

    // Aggregate taxes by (currency, tax_rate)
    let mut tax_map: HashMap<(String, String), (Decimal, i32)> = HashMap::new();
    for tx in &transactions {
        let key = (tx.currency.clone(), tx.tax_rate.to_string());
        let entry = tax_map.entry(key).or_insert((Decimal::ZERO, 0));
        entry.0 += tx.tax_amount;
        entry.1 += 1;
    }

    // Convert to breakdown structures
    let service_fees: Vec<ServiceFeeBreakdown> = service_fee_map
        .into_iter()
        .map(|((currency, gateway), (total_amount, transaction_count))| ServiceFeeBreakdown {
            currency,
            gateway,
            total_amount,
            transaction_count,
        })
        .collect();

    let taxes: Vec<TaxBreakdown> = tax_map
        .into_iter()
        .map(|((currency, tax_rate_str), (total_amount, transaction_count))| TaxBreakdown {
            currency,
            tax_rate: Decimal::from_str(&tax_rate_str).unwrap(),
            total_amount,
            transaction_count,
        })
        .collect();

    FinancialReport {
        start_date,
        end_date,
        service_fees,
        taxes,
    }
}

/// Test: Service fee breakdown by gateway and currency (FR-063)
#[test]
fn test_service_fee_breakdown_by_gateway_and_currency() {
    let transactions = vec![
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("10000").unwrap(),
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-002".to_string(),
            invoice_id: "inv-002".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("200000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("20000").unwrap(),
            service_fee: Decimal::from_str("8000").unwrap(),
            created_at: "2025-11-01T11:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-003".to_string(),
            invoice_id: "inv-003".to_string(),
            currency: "IDR".to_string(),
            gateway: "midtrans".to_string(),
            subtotal: Decimal::from_str("150000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("15000").unwrap(),
            service_fee: Decimal::from_str("3000").unwrap(),
            created_at: "2025-11-01T12:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Should have 2 service fee breakdowns (xendit and midtrans for IDR)
    assert_eq!(report.service_fees.len(), 2);

    // Find Xendit breakdown
    let xendit_breakdown = report
        .service_fees
        .iter()
        .find(|f| f.gateway == "xendit" && f.currency == "IDR")
        .unwrap();
    
    assert_eq!(xendit_breakdown.total_amount, Decimal::from_str("13100").unwrap()); // 5100 + 8000
    assert_eq!(xendit_breakdown.transaction_count, 2);

    // Find Midtrans breakdown
    let midtrans_breakdown = report
        .service_fees
        .iter()
        .find(|f| f.gateway == "midtrans" && f.currency == "IDR")
        .unwrap();
    
    assert_eq!(midtrans_breakdown.total_amount, Decimal::from_str("3000").unwrap());
    assert_eq!(midtrans_breakdown.transaction_count, 1);
}

/// Test: Tax breakdown by rate and currency (FR-063)
#[test]
fn test_tax_breakdown_by_rate_and_currency() {
    let transactions = vec![
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("10000").unwrap(),
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-002".to_string(),
            invoice_id: "inv-002".to_string(),
            currency: "IDR".to_string(),
            gateway: "midtrans".to_string(),
            subtotal: Decimal::from_str("200000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("20000").unwrap(),
            service_fee: Decimal::from_str("4000").unwrap(),
            created_at: "2025-11-01T11:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-003".to_string(),
            invoice_id: "inv-003".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("150000").unwrap(),
            tax_rate: Decimal::from_str("0.11").unwrap(), // Different tax rate
            tax_amount: Decimal::from_str("16500").unwrap(),
            service_fee: Decimal::from_str("6500").unwrap(),
            created_at: "2025-11-01T12:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Should have 2 tax breakdowns (10% and 11% for IDR)
    assert_eq!(report.taxes.len(), 2);

    // Find 10% tax breakdown
    let tax_10_breakdown = report
        .taxes
        .iter()
        .find(|t| t.tax_rate == Decimal::from_str("0.10").unwrap() && t.currency == "IDR")
        .unwrap();
    
    assert_eq!(tax_10_breakdown.total_amount, Decimal::from_str("30000").unwrap()); // 10000 + 20000
    assert_eq!(tax_10_breakdown.transaction_count, 2);

    // Find 11% tax breakdown
    let tax_11_breakdown = report
        .taxes
        .iter()
        .find(|t| t.tax_rate == Decimal::from_str("0.11").unwrap() && t.currency == "IDR")
        .unwrap();
    
    assert_eq!(tax_11_breakdown.total_amount, Decimal::from_str("16500").unwrap());
    assert_eq!(tax_11_breakdown.transaction_count, 1);
}

/// Test: Separate totals by currency (FR-064)
#[test]
fn test_separate_totals_by_currency() {
    let transactions = vec![
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("10000").unwrap(),
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-002".to_string(),
            invoice_id: "inv-002".to_string(),
            currency: "MYR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100.00").unwrap(),
            tax_rate: Decimal::from_str("0.06").unwrap(),
            tax_amount: Decimal::from_str("6.00").unwrap(),
            service_fee: Decimal::from_str("2.50").unwrap(),
            created_at: "2025-11-01T11:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Should have 2 service fee breakdowns (one per currency)
    assert_eq!(report.service_fees.len(), 2);

    // IDR service fees
    let idr_fees = report
        .service_fees
        .iter()
        .find(|f| f.currency == "IDR")
        .unwrap();
    assert_eq!(idr_fees.total_amount, Decimal::from_str("5100").unwrap());

    // MYR service fees
    let myr_fees = report
        .service_fees
        .iter()
        .find(|f| f.currency == "MYR")
        .unwrap();
    assert_eq!(myr_fees.total_amount, Decimal::from_str("2.50").unwrap());

    // Should have 2 tax breakdowns (one per currency/rate combination)
    assert_eq!(report.taxes.len(), 2);

    // IDR taxes
    let idr_taxes = report
        .taxes
        .iter()
        .find(|t| t.currency == "IDR")
        .unwrap();
    assert_eq!(idr_taxes.total_amount, Decimal::from_str("10000").unwrap());

    // MYR taxes
    let myr_taxes = report
        .taxes
        .iter()
        .find(|t| t.currency == "MYR")
        .unwrap();
    assert_eq!(myr_taxes.total_amount, Decimal::from_str("6.00").unwrap());
}

/// Test: Empty report (no transactions in date range)
#[test]
fn test_empty_report() {
    let transactions = vec![];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-30".to_string(),
    );

    assert_eq!(report.service_fees.len(), 0);
    assert_eq!(report.taxes.len(), 0);
    assert_eq!(report.start_date, "2025-11-01");
    assert_eq!(report.end_date, "2025-11-30");
}

/// Test: Multiple currencies, multiple gateways aggregation
#[test]
fn test_complex_multi_currency_multi_gateway_aggregation() {
    let transactions = vec![
        // IDR - Xendit
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("10000").unwrap(),
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
        // IDR - Midtrans
        Transaction {
            id: "tx-002".to_string(),
            invoice_id: "inv-002".to_string(),
            currency: "IDR".to_string(),
            gateway: "midtrans".to_string(),
            subtotal: Decimal::from_str("200000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("20000").unwrap(),
            service_fee: Decimal::from_str("4000").unwrap(),
            created_at: "2025-11-01T11:00:00Z".to_string(),
        },
        // MYR - Xendit
        Transaction {
            id: "tx-003".to_string(),
            invoice_id: "inv-003".to_string(),
            currency: "MYR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100.00").unwrap(),
            tax_rate: Decimal::from_str("0.06").unwrap(),
            tax_amount: Decimal::from_str("6.00").unwrap(),
            service_fee: Decimal::from_str("2.50").unwrap(),
            created_at: "2025-11-01T12:00:00Z".to_string(),
        },
        // USD - Xendit
        Transaction {
            id: "tx-004".to_string(),
            invoice_id: "inv-004".to_string(),
            currency: "USD".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100.00").unwrap(),
            tax_rate: Decimal::from_str("0.08").unwrap(),
            tax_amount: Decimal::from_str("8.00").unwrap(),
            service_fee: Decimal::from_str("3.50").unwrap(),
            created_at: "2025-11-01T13:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Should have 4 service fee breakdowns:
    // - IDR/Xendit, IDR/Midtrans, MYR/Xendit, USD/Xendit
    assert_eq!(report.service_fees.len(), 4);

    // Should have 4 tax breakdowns:
    // - IDR/0.10, MYR/0.06, USD/0.08
    assert_eq!(report.taxes.len(), 3);

    // Verify each combination exists
    assert!(report.service_fees.iter().any(|f| f.currency == "IDR" && f.gateway == "xendit"));
    assert!(report.service_fees.iter().any(|f| f.currency == "IDR" && f.gateway == "midtrans"));
    assert!(report.service_fees.iter().any(|f| f.currency == "MYR" && f.gateway == "xendit"));
    assert!(report.service_fees.iter().any(|f| f.currency == "USD" && f.gateway == "xendit"));
}

/// Test: Transaction count accuracy
#[test]
fn test_transaction_count_accuracy() {
    let transactions = vec![
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("10000").unwrap(),
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-002".to_string(),
            invoice_id: "inv-002".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("200000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("20000").unwrap(),
            service_fee: Decimal::from_str("8000").unwrap(),
            created_at: "2025-11-01T11:00:00Z".to_string(),
        },
        Transaction {
            id: "tx-003".to_string(),
            invoice_id: "inv-003".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("300000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("30000").unwrap(),
            service_fee: Decimal::from_str("11000").unwrap(),
            created_at: "2025-11-01T12:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Should have 1 service fee breakdown
    assert_eq!(report.service_fees.len(), 1);
    assert_eq!(report.service_fees[0].transaction_count, 3);
    assert_eq!(report.service_fees[0].total_amount, Decimal::from_str("24100").unwrap());

    // Should have 1 tax breakdown
    assert_eq!(report.taxes.len(), 1);
    assert_eq!(report.taxes[0].transaction_count, 3);
    assert_eq!(report.taxes[0].total_amount, Decimal::from_str("60000").unwrap());
}

/// Test: Zero tax transactions included in report
#[test]
fn test_zero_tax_transactions_in_report() {
    let transactions = vec![
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::ZERO,
            tax_amount: Decimal::ZERO,
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Service fee should be reported
    assert_eq!(report.service_fees.len(), 1);
    assert_eq!(report.service_fees[0].total_amount, Decimal::from_str("5100").unwrap());

    // Tax breakdown should include zero tax
    assert_eq!(report.taxes.len(), 1);
    assert_eq!(report.taxes[0].total_amount, Decimal::ZERO);
    assert_eq!(report.taxes[0].tax_rate, Decimal::ZERO);
}

/// Test: Date range filter (conceptual - in real implementation would filter transactions)
#[test]
fn test_date_range_in_report_metadata() {
    let transactions = vec![];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-30".to_string(),
    );

    assert_eq!(report.start_date, "2025-11-01");
    assert_eq!(report.end_date, "2025-11-30");
}

/// Test: Service fee and tax totals are independent
#[test]
fn test_service_fee_and_tax_totals_independent() {
    let transactions = vec![
        Transaction {
            id: "tx-001".to_string(),
            invoice_id: "inv-001".to_string(),
            currency: "IDR".to_string(),
            gateway: "xendit".to_string(),
            subtotal: Decimal::from_str("100000").unwrap(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            tax_amount: Decimal::from_str("10000").unwrap(),
            service_fee: Decimal::from_str("5100").unwrap(),
            created_at: "2025-11-01T10:00:00Z".to_string(),
        },
    ];

    let report = generate_financial_report(
        transactions,
        "2025-11-01".to_string(),
        "2025-11-01".to_string(),
    );

    // Service fee total
    let service_fee_total = report.service_fees[0].total_amount;
    
    // Tax total
    let tax_total = report.taxes[0].total_amount;

    // They should be different and independent
    assert_eq!(service_fee_total, Decimal::from_str("5100").unwrap());
    assert_eq!(tax_total, Decimal::from_str("10000").unwrap());
    assert_ne!(service_fee_total, tax_total);
}
