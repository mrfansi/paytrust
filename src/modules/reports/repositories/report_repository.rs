use chrono::NaiveDate;
use rust_decimal::Decimal;
use sqlx::MySqlPool;

use crate::core::Result;
use crate::modules::reports::models::{ServiceFeeBreakdown, TaxBreakdown};

/// Repository for aggregating financial report data
/// Implements FR-012 (financial reporting), FR-063 (breakdowns), FR-064 (currency separation)
pub struct ReportRepository {
    pool: MySqlPool,
}

impl ReportRepository {
    /// Create a new report repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Get service fee breakdown aggregated by gateway and currency
    /// 
    /// Queries invoices table to sum service_fee grouped by gateway_id and currency.
    /// Only includes invoices with status 'paid' or 'partially_paid' within the date range.
    /// 
    /// # Arguments
    /// * `start_date` - Start of reporting period (inclusive)
    /// * `end_date` - End of reporting period (inclusive)
    /// * `currency_filter` - Optional currency filter (e.g., Some("IDR"))
    /// 
    /// # Returns
    /// Vector of ServiceFeeBreakdown ordered by gateway and currency
    pub async fn get_service_fee_breakdown(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        currency_filter: Option<&str>,
    ) -> Result<Vec<ServiceFeeBreakdown>> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();

        let query = if let Some(currency) = currency_filter {
            sqlx::query_as::<_, ServiceFeeBreakdownRow>(
                r#"
                SELECT 
                    gateway_id as gateway,
                    currency,
                    SUM(service_fee) as total_amount,
                    COUNT(DISTINCT id) as transaction_count
                FROM invoices
                WHERE created_at >= ?
                  AND created_at <= ?
                  AND currency = ?
                  AND status IN ('paid', 'partially_paid')
                  AND gateway_id IS NOT NULL
                GROUP BY gateway_id, currency
                ORDER BY gateway_id, currency
                "#,
            )
            .bind(start_datetime)
            .bind(end_datetime)
            .bind(currency)
        } else {
            sqlx::query_as::<_, ServiceFeeBreakdownRow>(
                r#"
                SELECT 
                    gateway_id as gateway,
                    currency,
                    SUM(service_fee) as total_amount,
                    COUNT(DISTINCT id) as transaction_count
                FROM invoices
                WHERE created_at >= ?
                  AND created_at <= ?
                  AND status IN ('paid', 'partially_paid')
                  AND gateway_id IS NOT NULL
                GROUP BY gateway_id, currency
                ORDER BY gateway_id, currency
                "#,
            )
            .bind(start_datetime)
            .bind(end_datetime)
        };

        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| ServiceFeeBreakdown::new(
                row.gateway,
                row.currency,
                row.total_amount,
                row.transaction_count,
            ))
            .collect())
    }

    /// Get tax breakdown aggregated by rate and currency
    /// 
    /// Queries line_items joined with invoices to sum tax_amount grouped by tax_rate and currency.
    /// Only includes line items from invoices with status 'paid' or 'partially_paid' within the date range.
    /// Excludes line items with zero tax rate.
    /// 
    /// # Arguments
    /// * `start_date` - Start of reporting period (inclusive)
    /// * `end_date` - End of reporting period (inclusive)
    /// * `currency_filter` - Optional currency filter (e.g., Some("IDR"))
    /// 
    /// # Returns
    /// Vector of TaxBreakdown ordered by tax_rate and currency
    pub async fn get_tax_breakdown(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        currency_filter: Option<&str>,
    ) -> Result<Vec<TaxBreakdown>> {
        let start_datetime = start_date.and_hms_opt(0, 0, 0).unwrap();
        let end_datetime = end_date.and_hms_opt(23, 59, 59).unwrap();

        let query = if let Some(currency) = currency_filter {
            sqlx::query_as::<_, TaxBreakdownRow>(
                r#"
                SELECT 
                    li.tax_rate,
                    i.currency,
                    SUM(li.tax_amount) as total_amount,
                    COUNT(li.id) as transaction_count
                FROM line_items li
                JOIN invoices i ON li.invoice_id = i.id
                WHERE i.created_at >= ?
                  AND i.created_at <= ?
                  AND i.currency = ?
                  AND i.status IN ('paid', 'partially_paid')
                  AND li.tax_rate > 0
                GROUP BY li.tax_rate, i.currency
                ORDER BY li.tax_rate, i.currency
                "#,
            )
            .bind(start_datetime)
            .bind(end_datetime)
            .bind(currency)
        } else {
            sqlx::query_as::<_, TaxBreakdownRow>(
                r#"
                SELECT 
                    li.tax_rate,
                    i.currency,
                    SUM(li.tax_amount) as total_amount,
                    COUNT(li.id) as transaction_count
                FROM line_items li
                JOIN invoices i ON li.invoice_id = i.id
                WHERE i.created_at >= ?
                  AND i.created_at <= ?
                  AND i.status IN ('paid', 'partially_paid')
                  AND li.tax_rate > 0
                GROUP BY li.tax_rate, i.currency
                ORDER BY li.tax_rate, i.currency
                "#,
            )
            .bind(start_datetime)
            .bind(end_datetime)
        };

        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| TaxBreakdown::new(
                row.tax_rate,
                row.currency,
                row.total_amount,
                row.transaction_count,
            ))
            .collect())
    }
}

/// Internal struct for mapping service fee query results
#[derive(sqlx::FromRow)]
struct ServiceFeeBreakdownRow {
    gateway: String,
    currency: String,
    total_amount: Decimal,
    transaction_count: i64,
}

/// Internal struct for mapping tax query results
#[derive(sqlx::FromRow)]
struct TaxBreakdownRow {
    tax_rate: Decimal,
    currency: String,
    total_amount: Decimal,
    transaction_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_struct_size() {
        // Simple compile-time test - actual DB tests are in integration tests
        assert!(std::mem::size_of::<ReportRepository>() > 0);
    }
}
