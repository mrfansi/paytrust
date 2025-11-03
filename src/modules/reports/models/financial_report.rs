use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub service_fee_breakdown: Vec<ServiceFeeBreakdown>,
    pub tax_breakdown: Vec<TaxBreakdown>,
    pub total_revenue: Vec<CurrencyTotal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceFeeBreakdown {
    pub currency: String,
    pub gateway_name: String,
    pub total_amount: Decimal,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxBreakdown {
    pub currency: String,
    pub tax_rate: Decimal,
    pub total_amount: Decimal,
    pub transaction_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyTotal {
    pub currency: String,
    pub total_amount: Decimal,
    pub transaction_count: i64,
}
