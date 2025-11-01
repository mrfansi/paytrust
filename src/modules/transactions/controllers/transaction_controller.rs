use super::super::models::{PaymentTransaction, TransactionStatus};
use super::super::services::{PaymentStats, TransactionService};
use crate::core::Result;
use actix_web::{get, web, HttpResponse};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Transaction controller for transaction history endpoints
///
/// Provides endpoints for:
/// - Listing transactions for an invoice
/// - Getting payment statistics
pub struct TransactionController {
    transaction_service: TransactionService,
}

impl TransactionController {
    /// Create a new TransactionController
    ///
    /// # Arguments
    /// * `transaction_service` - Transaction service
    pub fn new(transaction_service: TransactionService) -> Self {
        Self {
            transaction_service,
        }
    }

    /// Configure transaction routes
    ///
    /// # Arguments
    /// * `cfg` - Service configuration
    pub fn configure(cfg: &mut web::ServiceConfig, transaction_service: TransactionService) {
        let controller = web::Data::new(Self::new(transaction_service));

        cfg.service(
            web::scope("/invoices")
                .app_data(controller)
                .service(get_invoice_transactions)
                .service(get_invoice_payment_stats),
        );
    }
}

/// Transaction response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionResponse {
    pub id: String,
    pub invoice_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installment_id: Option<String>,
    pub gateway_transaction_ref: String,
    pub gateway_id: String,
    pub amount_paid: Decimal,
    pub currency: String,
    pub payment_method: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_response: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PaymentTransaction> for TransactionResponse {
    fn from(transaction: PaymentTransaction) -> Self {
        Self {
            id: transaction.id.unwrap_or_default(),
            invoice_id: transaction.invoice_id,
            installment_id: transaction.installment_id,
            gateway_transaction_ref: transaction.gateway_transaction_ref,
            gateway_id: transaction.gateway_id,
            amount_paid: transaction.amount_paid,
            currency: transaction.currency.to_string(),
            payment_method: transaction.payment_method,
            status: transaction.status,
            gateway_response: transaction.gateway_response,
            created_at: transaction
                .created_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
            updated_at: transaction
                .updated_at
                .map(|t| t.to_rfc3339())
                .unwrap_or_default(),
        }
    }
}

/// Payment statistics response
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentStatsResponse {
    pub invoice_total: Decimal,
    pub total_paid: Decimal,
    pub balance: Decimal,
    pub is_fully_paid: bool,
    pub transaction_count: usize,
    pub completed_count: usize,
    pub pending_count: usize,
    pub failed_count: usize,
}

impl From<PaymentStats> for PaymentStatsResponse {
    fn from(stats: PaymentStats) -> Self {
        Self {
            invoice_total: stats.invoice_total,
            total_paid: stats.total_paid,
            balance: stats.balance,
            is_fully_paid: stats.is_fully_paid,
            transaction_count: stats.transaction_count,
            completed_count: stats.completed_count,
            pending_count: stats.pending_count,
            failed_count: stats.failed_count,
        }
    }
}

/// List response for transactions
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionListResponse {
    pub transactions: Vec<TransactionResponse>,
    pub count: usize,
}

/// Get all transactions for an invoice
///
/// GET /invoices/{id}/transactions
///
/// Returns a list of all payment transactions for the specified invoice,
/// including their status, amounts, and gateway information.
///
/// # Path Parameters
/// * `id` - Invoice ID
///
/// # Returns
/// * `200 OK` - List of transactions
/// * `404 Not Found` - Invoice not found
#[get("/{id}/transactions")]
async fn get_invoice_transactions(
    path: web::Path<String>,
    controller: web::Data<TransactionController>,
) -> Result<HttpResponse> {
    let invoice_id = path.into_inner();

    info!(invoice_id = invoice_id.as_str(), "Fetching invoice transactions");

    let transactions = controller
        .transaction_service
        .list_invoice_transactions(&invoice_id)
        .await?;

    let response = TransactionListResponse {
        count: transactions.len(),
        transactions: transactions
            .into_iter()
            .map(TransactionResponse::from)
            .collect(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Get payment statistics for an invoice
///
/// GET /invoices/{id}/payment-stats
///
/// Returns payment statistics including total paid, balance remaining,
/// and transaction counts by status.
///
/// # Path Parameters
/// * `id` - Invoice ID
///
/// # Returns
/// * `200 OK` - Payment statistics
/// * `404 Not Found` - Invoice not found
#[get("/{id}/payment-stats")]
async fn get_invoice_payment_stats(
    path: web::Path<String>,
    controller: web::Data<TransactionController>,
) -> Result<HttpResponse> {
    let invoice_id = path.into_inner();

    info!(
        invoice_id = invoice_id.as_str(),
        "Fetching invoice payment statistics"
    );

    let stats = controller
        .transaction_service
        .get_payment_stats(&invoice_id)
        .await?;

    let response = PaymentStatsResponse::from(stats);

    Ok(HttpResponse::Ok().json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Currency;

    #[test]
    fn test_transaction_response_structure() {
        let transaction = PaymentTransaction {
            id: Some("txn-123".to_string()),
            invoice_id: "INV-001".to_string(),
            installment_id: Some("inst-456".to_string()),
            gateway_transaction_ref: "gateway-ref-789".to_string(),
            gateway_id: "xendit".to_string(),
            amount_paid: Decimal::new(100000, 0),
            currency: "IDR".to_string(),
            payment_method: "bank_transfer".to_string(),
            status: TransactionStatus::Completed.to_string(),
            gateway_response: Some(serde_json::json!({"status": "success"})),
            created_at: None,
            updated_at: None,
        };

        let response = TransactionResponse::from(transaction);
        assert_eq!(response.id, "txn-123");
        assert_eq!(response.invoice_id, "INV-001");
        assert_eq!(response.installment_id, Some("inst-456".to_string()));
        assert_eq!(response.gateway_transaction_ref, "gateway-ref-789");
        assert_eq!(response.gateway_id, "xendit");
        assert_eq!(response.amount_paid, Decimal::new(100000, 0));
        assert_eq!(response.currency, "IDR");
        assert_eq!(response.payment_method, "bank_transfer");
        assert_eq!(response.status, "completed"); // Status is stored lowercase in DB
    }

    #[test]
    fn test_payment_stats_response_structure() {
        let stats = PaymentStats {
            invoice_total: Decimal::new(300000, 0),
            total_paid: Decimal::new(200000, 0),
            balance: Decimal::new(100000, 0),
            is_fully_paid: false,
            transaction_count: 3,
            completed_count: 2,
            pending_count: 1,
            failed_count: 0,
        };

        let response = PaymentStatsResponse::from(stats);
        assert_eq!(response.invoice_total, Decimal::new(300000, 0));
        assert_eq!(response.total_paid, Decimal::new(200000, 0));
        assert_eq!(response.balance, Decimal::new(100000, 0));
        assert!(!response.is_fully_paid);
        assert_eq!(response.transaction_count, 3);
        assert_eq!(response.completed_count, 2);
        assert_eq!(response.pending_count, 1);
        assert_eq!(response.failed_count, 0);
    }

    #[test]
    fn test_transaction_list_response_structure() {
        let transactions = vec![];
        let response = TransactionListResponse {
            transactions,
            count: 0,
        };

        assert_eq!(response.count, 0);
        assert_eq!(response.transactions.len(), 0);
    }
}
