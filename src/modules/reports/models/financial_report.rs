use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Financial report aggregating service fees and taxes over a date range
/// Implements FR-012 (financial reporting), FR-063 (breakdowns), FR-064 (currency separation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    /// Start date of the reporting period (inclusive)
    pub start_date: NaiveDate,
    /// End date of the reporting period (inclusive)
    pub end_date: NaiveDate,
    /// Service fee breakdown by gateway and currency
    pub service_fees: Vec<ServiceFeeBreakdown>,
    /// Tax breakdown by rate and currency
    pub taxes: Vec<TaxBreakdown>,
}

/// Service fee aggregation grouped by gateway and currency (FR-063)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceFeeBreakdown {
    /// Gateway identifier (e.g., "xendit", "midtrans")
    pub gateway: String,
    /// Currency code (e.g., "IDR", "USD")
    pub currency: String,
    /// Total service fees collected in this currency for this gateway
    pub total_amount: Decimal,
    /// Number of transactions that contributed to this total
    pub transaction_count: i64,
}

/// Tax aggregation grouped by rate and currency (FR-063, FR-064)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaxBreakdown {
    /// Tax rate as decimal (e.g., 0.11 for 11% VAT)
    pub tax_rate: Decimal,
    /// Currency code (e.g., "IDR", "USD")
    pub currency: String,
    /// Total tax collected at this rate in this currency
    pub total_amount: Decimal,
    /// Number of line items that contributed to this total
    pub transaction_count: i64,
}

impl FinancialReport {
    /// Create a new financial report for the specified date range
    pub fn new(
        start_date: NaiveDate,
        end_date: NaiveDate,
        service_fees: Vec<ServiceFeeBreakdown>,
        taxes: Vec<TaxBreakdown>,
    ) -> Self {
        Self {
            start_date,
            end_date,
            service_fees,
            taxes,
        }
    }

    /// Check if the report is empty (no service fees or taxes)
    pub fn is_empty(&self) -> bool {
        self.service_fees.is_empty() && self.taxes.is_empty()
    }

    /// Get total service fees across all gateways and currencies
    pub fn total_service_fees(&self) -> Decimal {
        self.service_fees
            .iter()
            .map(|sf| sf.total_amount)
            .sum()
    }

    /// Get total taxes across all rates and currencies
    pub fn total_taxes(&self) -> Decimal {
        self.taxes
            .iter()
            .map(|t| t.total_amount)
            .sum()
    }
}

impl ServiceFeeBreakdown {
    /// Create a new service fee breakdown
    pub fn new(gateway: String, currency: String, total_amount: Decimal, transaction_count: i64) -> Self {
        Self {
            gateway,
            currency,
            total_amount,
            transaction_count,
        }
    }
}

impl TaxBreakdown {
    /// Create a new tax breakdown
    pub fn new(tax_rate: Decimal, currency: String, total_amount: Decimal, transaction_count: i64) -> Self {
        Self {
            tax_rate,
            currency,
            total_amount,
            transaction_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_financial_report_creation() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        
        let service_fees = vec![
            ServiceFeeBreakdown::new("xendit".to_string(), "IDR".to_string(), dec!(100000), 5),
        ];
        let taxes = vec![
            TaxBreakdown::new(dec!(0.11), "IDR".to_string(), dec!(55000), 5),
        ];

        let report = FinancialReport::new(start, end, service_fees, taxes);
        
        assert_eq!(report.start_date, start);
        assert_eq!(report.end_date, end);
        assert_eq!(report.service_fees.len(), 1);
        assert_eq!(report.taxes.len(), 1);
        assert!(!report.is_empty());
    }

    #[test]
    fn test_empty_report() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        
        let report = FinancialReport::new(start, end, vec![], vec![]);
        
        assert!(report.is_empty());
        assert_eq!(report.total_service_fees(), dec!(0));
        assert_eq!(report.total_taxes(), dec!(0));
    }

    #[test]
    fn test_total_calculations() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
        
        let service_fees = vec![
            ServiceFeeBreakdown::new("xendit".to_string(), "IDR".to_string(), dec!(100000), 5),
            ServiceFeeBreakdown::new("midtrans".to_string(), "IDR".to_string(), dec!(50000), 3),
            ServiceFeeBreakdown::new("xendit".to_string(), "USD".to_string(), dec!(10.50), 2),
        ];
        let taxes = vec![
            TaxBreakdown::new(dec!(0.11), "IDR".to_string(), dec!(55000), 5),
            TaxBreakdown::new(dec!(0.10), "USD".to_string(), dec!(5.00), 2),
        ];

        let report = FinancialReport::new(start, end, service_fees, taxes);
        
        // Total service fees: 100000 + 50000 + 10.50 = 150010.50
        assert_eq!(report.total_service_fees(), dec!(150010.50));
        // Total taxes: 55000 + 5.00 = 55005.00
        assert_eq!(report.total_taxes(), dec!(55005.00));
    }

    #[test]
    fn test_service_fee_breakdown_creation() {
        let breakdown = ServiceFeeBreakdown::new("xendit".to_string(), "IDR".to_string(), dec!(100000), 5);
        
        assert_eq!(breakdown.gateway, "xendit");
        assert_eq!(breakdown.currency, "IDR");
        assert_eq!(breakdown.total_amount, dec!(100000));
        assert_eq!(breakdown.transaction_count, 5);
    }

    #[test]
    fn test_tax_breakdown_creation() {
        let breakdown = TaxBreakdown::new(dec!(0.11), "IDR".to_string(), dec!(55000), 5);
        
        assert_eq!(breakdown.tax_rate, dec!(0.11));
        assert_eq!(breakdown.currency, "IDR");
        assert_eq!(breakdown.total_amount, dec!(55000));
        assert_eq!(breakdown.transaction_count, 5);
    }
}
