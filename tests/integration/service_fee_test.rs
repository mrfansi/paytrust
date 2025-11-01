//! Integration tests for service fee calculation per gateway (T064)
//!
//! Validates that service fees are calculated correctly based on gateway-specific
//! fee structures (percentage + fixed components).
//!
//! Related FRs: FR-009 (service fee calculation), FR-047 (gateway-specific fees)

use rust_decimal::Decimal;
use std::str::FromStr;

/// Gateway fee configuration
#[derive(Debug, Clone)]
struct GatewayFeeConfig {
    gateway_name: String,
    percentage_rate: Decimal, // e.g., 0.029 for 2.9%
    fixed_fee: Decimal,       // e.g., 2200 for IDR 2,200
}

/// Mock Invoice with service fee
#[derive(Debug, Clone)]
struct Invoice {
    id: String,
    gateway_id: String,
    subtotal: Decimal,
    tax_total: Decimal,
    service_fee: Decimal,
    total: Decimal,
    currency: String,
}

/// Calculate service fee: (subtotal × percentage) + fixed_fee
fn calculate_service_fee(
    subtotal: Decimal,
    percentage_rate: Decimal,
    fixed_fee: Decimal,
) -> Decimal {
    let percentage_component = subtotal * percentage_rate;
    let total_fee = percentage_component + fixed_fee;
    total_fee.round_dp(2)
}

/// Test: Xendit fee structure (2.9% + IDR 2,200) for IDR transactions
#[test]
fn test_xendit_idr_fee_structure() {
    let gateway = GatewayFeeConfig {
        gateway_name: "xendit".to_string(),
        percentage_rate: Decimal::from_str("0.029").unwrap(), // 2.9%
        fixed_fee: Decimal::from_str("2200").unwrap(),        // IDR 2,200
    };

    let subtotal = Decimal::from_str("100000").unwrap(); // IDR 100,000
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (100,000 × 0.029) + 2,200 = 2,900 + 2,200 = 5,100
    assert_eq!(service_fee, Decimal::from_str("5100").unwrap());

    let invoice = Invoice {
        id: "inv-001".to_string(),
        gateway_id: "gateway-xendit-123".to_string(),
        subtotal,
        tax_total: Decimal::ZERO,
        service_fee,
        total: subtotal + service_fee,
        currency: "IDR".to_string(),
    };

    assert_eq!(invoice.service_fee, Decimal::from_str("5100").unwrap());
    assert_eq!(invoice.total, Decimal::from_str("105100").unwrap());
}

/// Test: Midtrans fee structure (2.0% + IDR 0) for IDR transactions
#[test]
fn test_midtrans_idr_fee_structure() {
    let gateway = GatewayFeeConfig {
        gateway_name: "midtrans".to_string(),
        percentage_rate: Decimal::from_str("0.020").unwrap(), // 2.0%
        fixed_fee: Decimal::ZERO,                             // No fixed fee
    };

    let subtotal = Decimal::from_str("100000").unwrap(); // IDR 100,000
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (100,000 × 0.020) + 0 = 2,000
    assert_eq!(service_fee, Decimal::from_str("2000").unwrap());

    let invoice = Invoice {
        id: "inv-002".to_string(),
        gateway_id: "gateway-midtrans-456".to_string(),
        subtotal,
        tax_total: Decimal::ZERO,
        service_fee,
        total: subtotal + service_fee,
        currency: "IDR".to_string(),
    };

    assert_eq!(invoice.service_fee, Decimal::from_str("2000").unwrap());
    assert_eq!(invoice.total, Decimal::from_str("102000").unwrap());
}

/// Test: Same subtotal, different gateways = different service fees
#[test]
fn test_same_subtotal_different_gateways() {
    let subtotal = Decimal::from_str("500000").unwrap(); // IDR 500,000

    // Xendit: 2.9% + 2,200
    let xendit_fee = calculate_service_fee(
        subtotal,
        Decimal::from_str("0.029").unwrap(),
        Decimal::from_str("2200").unwrap(),
    );

    // Midtrans: 2.0% + 0
    let midtrans_fee =
        calculate_service_fee(subtotal, Decimal::from_str("0.020").unwrap(), Decimal::ZERO);

    // Xendit: (500,000 × 0.029) + 2,200 = 14,500 + 2,200 = 16,700
    assert_eq!(xendit_fee, Decimal::from_str("16700").unwrap());

    // Midtrans: (500,000 × 0.020) = 10,000
    assert_eq!(midtrans_fee, Decimal::from_str("10000").unwrap());

    // Different fees for same subtotal
    assert_ne!(xendit_fee, midtrans_fee);

    // Xendit more expensive due to fixed fee
    assert!(xendit_fee > midtrans_fee);
}

/// Test: Custom gateway with MYR currency
#[test]
fn test_custom_gateway_myr_currency() {
    let gateway = GatewayFeeConfig {
        gateway_name: "custom-gateway".to_string(),
        percentage_rate: Decimal::from_str("0.015").unwrap(), // 1.5%
        fixed_fee: Decimal::from_str("1.00").unwrap(),        // MYR 1.00
    };

    let subtotal = Decimal::from_str("100.00").unwrap(); // MYR 100.00
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (100 × 0.015) + 1.00 = 1.50 + 1.00 = 2.50
    assert_eq!(service_fee, Decimal::from_str("2.50").unwrap());

    let invoice = Invoice {
        id: "inv-003".to_string(),
        gateway_id: "gateway-custom-789".to_string(),
        subtotal,
        tax_total: Decimal::ZERO,
        service_fee,
        total: subtotal + service_fee,
        currency: "MYR".to_string(),
    };

    assert_eq!(invoice.service_fee, Decimal::from_str("2.50").unwrap());
    assert_eq!(invoice.total, Decimal::from_str("102.50").unwrap());
}

/// Test: Zero percentage rate (only fixed fee)
#[test]
fn test_zero_percentage_only_fixed_fee() {
    let gateway = GatewayFeeConfig {
        gateway_name: "fixed-only-gateway".to_string(),
        percentage_rate: Decimal::ZERO,                // 0%
        fixed_fee: Decimal::from_str("5000").unwrap(), // IDR 5,000
    };

    let subtotal = Decimal::from_str("1000000").unwrap();
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (1,000,000 × 0) + 5,000 = 5,000
    assert_eq!(service_fee, Decimal::from_str("5000").unwrap());

    // Fee same regardless of subtotal
    let small_subtotal = Decimal::from_str("10000").unwrap();
    let small_fee =
        calculate_service_fee(small_subtotal, gateway.percentage_rate, gateway.fixed_fee);
    assert_eq!(small_fee, service_fee);
}

/// Test: Zero fixed fee (only percentage)
#[test]
fn test_zero_fixed_only_percentage() {
    let gateway = GatewayFeeConfig {
        gateway_name: "percentage-only-gateway".to_string(),
        percentage_rate: Decimal::from_str("0.025").unwrap(), // 2.5%
        fixed_fee: Decimal::ZERO,
    };

    let subtotal = Decimal::from_str("200000").unwrap();
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (200,000 × 0.025) = 5,000
    assert_eq!(service_fee, Decimal::from_str("5000").unwrap());

    // Scales linearly with subtotal
    let double_subtotal = Decimal::from_str("400000").unwrap();
    let double_fee =
        calculate_service_fee(double_subtotal, gateway.percentage_rate, gateway.fixed_fee);
    assert_eq!(double_fee, service_fee * Decimal::from_str("2").unwrap());
}

/// Test: Service fee calculation with invoice including taxes
#[test]
fn test_service_fee_with_taxes() {
    let subtotal = Decimal::from_str("100000").unwrap();
    let tax_rate = Decimal::from_str("0.10").unwrap();
    let tax_total = (subtotal * tax_rate).round_dp(2);

    // Service fee calculated on SUBTOTAL only, not including tax (FR-055)
    let gateway = GatewayFeeConfig {
        gateway_name: "xendit".to_string(),
        percentage_rate: Decimal::from_str("0.029").unwrap(),
        fixed_fee: Decimal::from_str("2200").unwrap(),
    };

    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    let invoice = Invoice {
        id: "inv-004".to_string(),
        gateway_id: "gateway-xendit-123".to_string(),
        subtotal,
        tax_total,
        service_fee,
        total: subtotal + tax_total + service_fee,
        currency: "IDR".to_string(),
    };

    // Service fee: (100,000 × 0.029) + 2,200 = 5,100
    assert_eq!(invoice.service_fee, Decimal::from_str("5100").unwrap());

    // Tax: 100,000 × 0.10 = 10,000
    assert_eq!(invoice.tax_total, Decimal::from_str("10000").unwrap());

    // Total: 100,000 + 10,000 + 5,100 = 115,100
    assert_eq!(invoice.total, Decimal::from_str("115100").unwrap());
}

/// Test: Large subtotal with Xendit fee
#[test]
fn test_large_subtotal_xendit() {
    let gateway = GatewayFeeConfig {
        gateway_name: "xendit".to_string(),
        percentage_rate: Decimal::from_str("0.029").unwrap(),
        fixed_fee: Decimal::from_str("2200").unwrap(),
    };

    let subtotal = Decimal::from_str("10000000").unwrap(); // IDR 10,000,000
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (10,000,000 × 0.029) + 2,200 = 290,000 + 2,200 = 292,200
    assert_eq!(service_fee, Decimal::from_str("292200").unwrap());
}

/// Test: Small subtotal with Xendit fee (fixed fee dominates)
#[test]
fn test_small_subtotal_xendit() {
    let gateway = GatewayFeeConfig {
        gateway_name: "xendit".to_string(),
        percentage_rate: Decimal::from_str("0.029").unwrap(),
        fixed_fee: Decimal::from_str("2200").unwrap(),
    };

    let subtotal = Decimal::from_str("10000").unwrap(); // IDR 10,000
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (10,000 × 0.029) + 2,200 = 290 + 2,200 = 2,490
    assert_eq!(service_fee, Decimal::from_str("2490").unwrap());

    // Fixed fee is larger than percentage component
    let percentage_component = subtotal * gateway.percentage_rate;
    assert!(gateway.fixed_fee > percentage_component);
}

/// Test: Service fee precision (2 decimal places)
#[test]
fn test_service_fee_precision() {
    let gateway = GatewayFeeConfig {
        gateway_name: "xendit".to_string(),
        percentage_rate: Decimal::from_str("0.029").unwrap(),
        fixed_fee: Decimal::from_str("2200.00").unwrap(),
    };

    let subtotal = Decimal::from_str("33333.33").unwrap();
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    // (33,333.33 × 0.029) + 2,200 = 966.67 + 2,200 = 3,166.67
    assert_eq!(service_fee, Decimal::from_str("3166.67").unwrap());
}

/// Test: Gateway selection affects final invoice total
#[test]
fn test_gateway_selection_affects_total() {
    let subtotal = Decimal::from_str("300000").unwrap();
    let tax_total = Decimal::from_str("30000").unwrap(); // 10% tax

    // Invoice with Xendit
    let xendit_fee = calculate_service_fee(
        subtotal,
        Decimal::from_str("0.029").unwrap(),
        Decimal::from_str("2200").unwrap(),
    );

    let invoice_xendit = Invoice {
        id: "inv-xendit".to_string(),
        gateway_id: "gateway-xendit-123".to_string(),
        subtotal,
        tax_total,
        service_fee: xendit_fee,
        total: subtotal + tax_total + xendit_fee,
        currency: "IDR".to_string(),
    };

    // Invoice with Midtrans
    let midtrans_fee =
        calculate_service_fee(subtotal, Decimal::from_str("0.020").unwrap(), Decimal::ZERO);

    let invoice_midtrans = Invoice {
        id: "inv-midtrans".to_string(),
        gateway_id: "gateway-midtrans-456".to_string(),
        subtotal,
        tax_total,
        service_fee: midtrans_fee,
        total: subtotal + tax_total + midtrans_fee,
        currency: "IDR".to_string(),
    };

    // Xendit: (300,000 × 0.029) + 2,200 = 8,700 + 2,200 = 10,900
    assert_eq!(
        invoice_xendit.service_fee,
        Decimal::from_str("10900").unwrap()
    );
    assert_eq!(invoice_xendit.total, Decimal::from_str("340900").unwrap());

    // Midtrans: (300,000 × 0.020) = 6,000
    assert_eq!(
        invoice_midtrans.service_fee,
        Decimal::from_str("6000").unwrap()
    );
    assert_eq!(invoice_midtrans.total, Decimal::from_str("336000").unwrap());

    // Different totals due to gateway fees
    assert_ne!(invoice_xendit.total, invoice_midtrans.total);
}

/// Test: Zero service fee configuration (free gateway)
#[test]
fn test_zero_service_fee() {
    let gateway = GatewayFeeConfig {
        gateway_name: "free-gateway".to_string(),
        percentage_rate: Decimal::ZERO,
        fixed_fee: Decimal::ZERO,
    };

    let subtotal = Decimal::from_str("100000").unwrap();
    let service_fee = calculate_service_fee(subtotal, gateway.percentage_rate, gateway.fixed_fee);

    assert_eq!(service_fee, Decimal::ZERO);

    let invoice = Invoice {
        id: "inv-005".to_string(),
        gateway_id: "gateway-free-999".to_string(),
        subtotal,
        tax_total: Decimal::ZERO,
        service_fee,
        total: subtotal,
        currency: "IDR".to_string(),
    };

    assert_eq!(invoice.total, invoice.subtotal);
}
