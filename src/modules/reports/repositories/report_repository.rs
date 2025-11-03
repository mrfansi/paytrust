use async_trait::async_trait;
use sqlx::MySqlPool;
use crate::core::error::AppError;
use crate::modules::reports::models::{ServiceFeeBreakdown, TaxBreakdown, CurrencyTotal};

/// Repository for financial report aggregation queries
#[async_trait]
pub trait ReportRepository: Send + Sync {
    /// Get service fee breakdown by gateway and currency
    async fn get_service_fee_breakdown(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<Vec<ServiceFeeBreakdown>, AppError>;

    /// Get tax breakdown by currency and rate
    async fn get_tax_breakdown(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<Vec<TaxBreakdown>, AppError>;

    /// Get total revenue by currency
    async fn get_revenue_by_currency(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<Vec<CurrencyTotal>, AppError>;
}

pub struct MySqlReportRepository {
    pool: MySqlPool,
}

impl MySqlReportRepository {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReportRepository for MySqlReportRepository {
    /// FR-012: Service fee breakdown by gateway and currency
    async fn get_service_fee_breakdown(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<Vec<ServiceFeeBreakdown>, AppError> {
        let query = r#"
            SELECT 
                i.currency,
                gc.gateway_name,
                SUM(i.service_fee) as total_amount,
                COUNT(*) as transaction_count
            FROM invoices i
            INNER JOIN gateway_configs gc ON i.gateway_id = gc.id
            WHERE i.created_at >= ? AND i.created_at <= ?
            AND i.status = 'paid'
            GROUP BY i.currency, gc.gateway_name
            ORDER BY i.currency, gc.gateway_name
        "#;

        let results = sqlx::query_as::<_, (String, String, rust_decimal::Decimal, i64)>(query)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

        Ok(results
            .into_iter()
            .map(|(currency, gateway_name, total_amount, transaction_count)| {
                ServiceFeeBreakdown {
                    currency,
                    gateway_name,
                    total_amount,
                    transaction_count,
                }
            })
            .collect())
    }

    /// FR-064: Tax breakdown by currency and rate
    async fn get_tax_breakdown(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<Vec<TaxBreakdown>, AppError> {
        let query = r#"
            SELECT 
                i.currency,
                li.tax_rate,
                SUM(li.tax_amount) as total_amount,
                COUNT(DISTINCT i.id) as transaction_count
            FROM line_items li
            INNER JOIN invoices i ON li.invoice_id = i.id
            WHERE i.created_at >= ? AND i.created_at <= ?
            AND i.status = 'paid'
            GROUP BY i.currency, li.tax_rate
            ORDER BY i.currency, li.tax_rate
        "#;

        let results = sqlx::query_as::<_, (String, rust_decimal::Decimal, rust_decimal::Decimal, i64)>(query)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

        Ok(results
            .into_iter()
            .map(|(currency, tax_rate, total_amount, transaction_count)| {
                TaxBreakdown {
                    currency,
                    tax_rate,
                    total_amount,
                    transaction_count,
                }
            })
            .collect())
    }

    /// FR-013: Total revenue by currency (no conversion)
    async fn get_revenue_by_currency(
        &self,
        start_date: chrono::NaiveDateTime,
        end_date: chrono::NaiveDateTime,
    ) -> Result<Vec<CurrencyTotal>, AppError> {
        let query = r#"
            SELECT 
                currency,
                SUM(total_amount) as total_amount,
                COUNT(*) as transaction_count
            FROM invoices
            WHERE created_at >= ? AND created_at <= ?
            AND status = 'paid'
            GROUP BY currency
            ORDER BY currency
        "#;

        let results = sqlx::query_as::<_, (String, rust_decimal::Decimal, i64)>(query)
            .bind(start_date)
            .bind(end_date)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Internal(format!("Database error: {}", e)))?;

        Ok(results
            .into_iter()
            .map(|(currency, total_amount, transaction_count)| {
                CurrencyTotal {
                    currency,
                    total_amount,
                    transaction_count,
                }
            })
            .collect())
    }
}
