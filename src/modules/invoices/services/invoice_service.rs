use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use rust_decimal::Decimal;

use crate::core::error::AppError;
use crate::modules::gateways::repositories::gateway_repository::GatewayRepository;
use crate::modules::invoices::models::{
    CreateInvoiceRequest, CreateLineItemRequest, Invoice, InvoiceResponse, InvoiceStatus,
    LineItem, LineItemResponse,
};
use crate::modules::invoices::repositories::invoice_repository::InvoiceRepository;

/// Service for invoice business logic
pub struct InvoiceService {
    invoice_repo: Arc<dyn InvoiceRepository>,
    gateway_repo: Arc<dyn GatewayRepository>,
}

impl InvoiceService {
    pub fn new(
        invoice_repo: Arc<dyn InvoiceRepository>,
        gateway_repo: Arc<dyn GatewayRepository>,
    ) -> Self {
        Self {
            invoice_repo,
            gateway_repo,
        }
    }

    /// Create a new invoice with line items
    /// Implements FR-001, FR-004, FR-007, FR-044, FR-044a, FR-046, FR-051
    pub async fn create_invoice(
        &self,
        request: CreateInvoiceRequest,
        tenant_id: &str,
    ) -> Result<InvoiceResponse, AppError> {
        // Validate gateway exists and supports currency (FR-007, FR-046)
        let gateway = self
            .gateway_repo
            .find_by_id(request.gateway_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Gateway not found".to_string()))?;

        // FR-046: Validate gateway supports invoice currency
        let currency_str = request.currency.to_string();
        if !gateway.supports_currency(&currency_str) {
            return Err(AppError::Validation(format!(
                "Gateway does not support currency {}",
                request.currency
            )));
        }

        // Validate line items exist
        if request.line_items.is_empty() {
            return Err(AppError::Validation(
                "Invoice must have at least one line item".to_string(),
            ));
        }

        // Calculate totals
        let (line_items, subtotal, tax_total) =
            self.calculate_line_items_and_totals(&request.line_items)?;

        // Calculate service fee (FR-009, FR-047)
        let service_fee = self.calculate_service_fee(&gateway, subtotal);

        // Calculate total amount (FR-055, FR-056)
        let total_amount = subtotal + tax_total + service_fee;

        // Validate and set expires_at (FR-044, FR-044a)
        let created_at = Utc::now();
        let expires_at = self.validate_and_set_expires_at(
            request.expires_at,
            created_at,
            request.installment_count,
            &request.installment_custom_amounts,
        )?;

        // Create invoice entity
        let invoice = Invoice {
            id: 0, // Will be set by database
            tenant_id: tenant_id.to_string(),
            external_id: request.external_id.clone(),
            currency: request.currency,
            subtotal,
            tax_total,
            service_fee,
            total_amount,
            status: InvoiceStatus::Draft,
            gateway_id: request.gateway_id,
            original_invoice_id: None,
            payment_initiated_at: None, // FR-051: Not set until payment initiated
            expires_at,
            created_at,
            updated_at: created_at,
        };

        // Save to database
        let created_invoice = self
            .invoice_repo
            .create(&invoice, &line_items, tenant_id)
            .await?;

        // Fetch line items for response
        let line_items_response = self
            .invoice_repo
            .find_line_items(created_invoice.id, tenant_id)
            .await?;

        Ok(self.to_response(created_invoice, line_items_response, None))
    }

    /// Get invoice by ID
    pub async fn get_invoice(
        &self,
        id: i64,
        tenant_id: &str,
    ) -> Result<InvoiceResponse, AppError> {
        let invoice = self
            .invoice_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        let line_items = self.invoice_repo.find_line_items(id, tenant_id).await?;

        Ok(self.to_response(invoice, line_items, None))
    }

    /// List invoices for tenant
    pub async fn list_invoices(
        &self,
        tenant_id: &str,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<InvoiceResponse>, AppError> {
        let invoices = self.invoice_repo.list(tenant_id, limit, offset).await?;

        let mut responses = Vec::new();
        for invoice in invoices {
            let line_items = self
                .invoice_repo
                .find_line_items(invoice.id, tenant_id)
                .await?;
            responses.push(self.to_response(invoice, line_items, None));
        }

        Ok(responses)
    }

    /// Set payment initiated timestamp (FR-051a)
    /// This makes the invoice immutable
    pub async fn set_payment_initiated(
        &self,
        invoice_id: i64,
        tenant_id: &str,
    ) -> Result<(), AppError> {
        // FR-051a: Set payment_initiated_at on first payment attempt
        let initiated_at = Utc::now();
        self.invoice_repo
            .set_payment_initiated(invoice_id, initiated_at, tenant_id)
            .await?;

        // Update status to pending if still draft
        let invoice = self
            .invoice_repo
            .find_by_id(invoice_id, tenant_id)
            .await?
            .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

        if invoice.status == InvoiceStatus::Draft {
            self.invoice_repo
                .update_status(invoice_id, InvoiceStatus::Pending, tenant_id)
                .await?;
        }

        Ok(())
    }

    /// Calculate line items with totals
    fn calculate_line_items_and_totals(
        &self,
        items: &[CreateLineItemRequest],
    ) -> Result<(Vec<LineItem>, Decimal, Decimal), AppError> {
        let mut line_items = Vec::new();
        let mut subtotal = Decimal::ZERO;
        let mut tax_total = Decimal::ZERO;

        for item in items {
            let line_item = LineItem::new(
                0, // invoice_id will be set later
                item.product_name.clone(),
                item.quantity,
                item.unit_price,
                item.tax_rate,
                item.tax_category.clone(),
            )?;

            subtotal += line_item.subtotal;
            tax_total += line_item.tax_amount;
            line_items.push(line_item);
        }

        Ok((line_items, subtotal, tax_total))
    }

    /// Calculate service fee based on gateway configuration (FR-009, FR-047)
    fn calculate_service_fee(
        &self,
        gateway: &crate::modules::gateways::models::gateway_config::GatewayConfig,
        subtotal: Decimal,
    ) -> Decimal {
        // FR-047: service_fee = (subtotal × fee_percentage) + fee_fixed
        (subtotal * gateway.fee_percentage) + gateway.fee_fixed
    }

    /// Validate and set expires_at timestamp (FR-044, FR-044a)
    fn validate_and_set_expires_at(
        &self,
        expires_at: Option<DateTime<Utc>>,
        created_at: DateTime<Utc>,
        _installment_count: Option<u32>,
        _installment_custom_amounts: &Option<Vec<Decimal>>,
    ) -> Result<DateTime<Utc>, AppError> {
        let expires_at = match expires_at {
            Some(exp) => {
                // FR-044a(b): Validate not in past
                if exp < Utc::now() {
                    return Err(AppError::Validation(
                        "Expiration time cannot be in the past".to_string(),
                    ));
                }

                // FR-044a(c): Validate >= created_at + 1 hour
                let min_expiry = created_at + Duration::hours(1);
                if exp < min_expiry {
                    return Err(AppError::Validation(
                        "Expiration must be at least 1 hour from now".to_string(),
                    ));
                }

                // FR-044a(d): Validate <= created_at + 30 days
                let max_expiry = created_at + Duration::days(30);
                if exp > max_expiry {
                    return Err(AppError::Validation(
                        "Expiration must be within 30 days from now".to_string(),
                    ));
                }

                // TODO FR-044a(e): If invoice has installments, validate expires_at >= last_installment.due_date
                // This will be implemented in Phase 5 (User Story 3) when installments are added

                exp
            }
            None => {
                // FR-044: Default to created_at + 24 hours
                created_at + Duration::hours(24)
            }
        };

        Ok(expires_at)
    }

    /// Convert invoice and line items to response DTO
    fn to_response(
        &self,
        invoice: Invoice,
        line_items: Vec<LineItem>,
        payment_url: Option<String>,
    ) -> InvoiceResponse {
        let is_immutable = invoice.is_immutable();
        
        InvoiceResponse {
            id: invoice.id,
            external_id: invoice.external_id,
            tenant_id: invoice.tenant_id,
            currency: invoice.currency,
            subtotal: invoice.subtotal.to_string(),
            tax_total: invoice.tax_total.to_string(),
            service_fee: invoice.service_fee.to_string(),
            total_amount: invoice.total_amount.to_string(),
            status: invoice.status,
            payment_url,
            is_immutable,
            expires_at: invoice.expires_at.to_rfc3339(),
            created_at: invoice.created_at.to_rfc3339(),
            updated_at: invoice.updated_at.to_rfc3339(),
            line_items: line_items
                .into_iter()
                .map(|li| LineItemResponse {
                    id: li.id,
                    product_name: li.product_name,
                    quantity: li.quantity.to_string(),
                    unit_price: li.unit_price.to_string(),
                    subtotal: li.subtotal.to_string(),
                    tax_rate: li.tax_rate.to_string(),
                    tax_category: li.tax_category,
                    tax_amount: li.tax_amount.to_string(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests with real database are in tests/integration/
    // These are unit tests for calculation logic only

    #[test]
    fn test_service_compiles() {
        // This test ensures the service compiles
        // Actual business logic tests are in integration tests
    }
}
