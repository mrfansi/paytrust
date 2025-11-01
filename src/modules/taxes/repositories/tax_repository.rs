//! Tax repository for database operations
//!
//! Handles CRUD operations and aggregation queries for tax data.

use rust_decimal::Decimal;
use sqlx::{MySqlPool, Row};
use std::str::FromStr;

use crate::modules::taxes::models::Tax;

/// Repository for tax-related database operations
pub struct TaxRepository {
    pool: MySqlPool,
}

impl TaxRepository {
    /// Create a new tax repository
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }

    /// Find active tax rate by currency and category
    ///
    /// Returns the most recent active tax rate for the given criteria.
    pub async fn find_active_by_currency_and_category(
        &self,
        currency: &str,
        category: &str,
    ) -> Result<Option<Tax>, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT id, category, rate, currency, effective_from, is_active, created_at, updated_at
            FROM taxes
            WHERE currency = ? AND category = ? AND is_active = true
            ORDER BY effective_from DESC
            LIMIT 1
            "#,
        )
        .bind(currency)
        .bind(category)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| Tax {
            id: row.get("id"),
            category: row.get("category"),
            rate: Decimal::from_str(row.get::<String, _>("rate").as_str()).unwrap_or(Decimal::ZERO),
            currency: row.get("currency"),
            effective_from: row.get("effective_from"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    /// Get tax breakdown aggregated by currency and rate
    ///
    /// Used for financial reporting (FR-063, FR-064).
    /// Returns aggregated tax amounts grouped by currency and tax rate.
    pub async fn get_tax_breakdown(
        &self,
        start_date: &str,
        end_date: &str,
        currency_filter: Option<&str>,
    ) -> Result<Vec<TaxBreakdown>, sqlx::Error> {
        let mut query = String::from(
            r#"
            SELECT 
                i.currency,
                li.tax_rate,
                SUM(li.tax_amount) as total_amount,
                COUNT(DISTINCT i.id) as transaction_count
            FROM invoices i
            JOIN line_items li ON li.invoice_id = i.id
            WHERE i.created_at >= ? AND i.created_at <= ?
            "#,
        );

        if currency_filter.is_some() {
            query.push_str(" AND i.currency = ?");
        }

        query.push_str(" GROUP BY i.currency, li.tax_rate ORDER BY i.currency, li.tax_rate");

        let mut sql_query = sqlx::query(&query).bind(start_date).bind(end_date);

        if let Some(currency) = currency_filter {
            sql_query = sql_query.bind(currency);
        }

        let rows = sql_query.fetch_all(&self.pool).await?;

        let breakdowns = rows
            .iter()
            .map(|row| TaxBreakdown {
                currency: row.get("currency"),
                tax_rate: Decimal::from_str(row.get::<String, _>("tax_rate").as_str())
                    .unwrap_or(Decimal::ZERO),
                total_amount: Decimal::from_str(row.get::<String, _>("total_amount").as_str())
                    .unwrap_or(Decimal::ZERO),
                transaction_count: row.get("transaction_count"),
            })
            .collect();

        Ok(breakdowns)
    }

    /// Create a new tax rate configuration
    pub async fn create(&self, tax: &Tax) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO taxes (id, category, rate, currency, effective_from, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&tax.id)
        .bind(&tax.category)
        .bind(tax.rate.to_string())
        .bind(&tax.currency)
        .bind(&tax.effective_from)
        .bind(tax.is_active)
        .bind(&tax.created_at)
        .bind(&tax.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update tax rate active status
    pub async fn update_active_status(&self, id: &str, is_active: bool) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            UPDATE taxes
            SET is_active = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(is_active)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Find tax by ID
    pub async fn find_by_id(&self, id: &str) -> Result<Option<Tax>, sqlx::Error> {
        let result = sqlx::query(
            r#"
            SELECT id, category, rate, currency, effective_from, is_active, created_at, updated_at
            FROM taxes
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(|row| Tax {
            id: row.get("id"),
            category: row.get("category"),
            rate: Decimal::from_str(row.get::<String, _>("rate").as_str()).unwrap_or(Decimal::ZERO),
            currency: row.get("currency"),
            effective_from: row.get("effective_from"),
            is_active: row.get("is_active"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }))
    }

    /// List all active tax rates
    pub async fn list_active(&self) -> Result<Vec<Tax>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT id, category, rate, currency, effective_from, is_active, created_at, updated_at
            FROM taxes
            WHERE is_active = true
            ORDER BY currency, category, effective_from DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let taxes = rows
            .iter()
            .map(|row| Tax {
                id: row.get("id"),
                category: row.get("category"),
                rate: Decimal::from_str(row.get::<String, _>("rate").as_str())
                    .unwrap_or(Decimal::ZERO),
                currency: row.get("currency"),
                effective_from: row.get("effective_from"),
                is_active: row.get("is_active"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(taxes)
    }
}

/// Tax breakdown for financial reporting
#[derive(Debug, Clone)]
pub struct TaxBreakdown {
    pub currency: String,
    pub tax_rate: Decimal,
    pub total_amount: Decimal,
    pub transaction_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tax_breakdown_creation() {
        let breakdown = TaxBreakdown {
            currency: "IDR".to_string(),
            tax_rate: Decimal::from_str("0.10").unwrap(),
            total_amount: Decimal::from_str("50000").unwrap(),
            transaction_count: 5,
        };

        assert_eq!(breakdown.currency, "IDR");
        assert_eq!(breakdown.tax_rate, Decimal::from_str("0.10").unwrap());
        assert_eq!(breakdown.total_amount, Decimal::from_str("50000").unwrap());
        assert_eq!(breakdown.transaction_count, 5);
    }
}
