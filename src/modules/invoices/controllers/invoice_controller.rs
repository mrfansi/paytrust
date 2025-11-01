// T043: InvoiceController implementation
// HTTP handlers for invoice endpoints
//
// Endpoints:
// - POST /invoices - Create new invoice
// - GET /invoices/{id} - Get invoice by ID
// - GET /invoices - List invoices with pagination

use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use crate::core::{AppError, Currency, Result};
use crate::modules::invoices::{
    models::{Invoice, LineItem},
    services::InvoiceService,
};

/// Request to create an invoice
#[derive(Debug, Deserialize)]
pub struct CreateInvoiceRequest {
    /// Merchant's reference ID (must be unique)
    pub external_id: String,
    
    /// Payment gateway ID to use
    pub gateway_id: String,
    
    /// Invoice currency (IDR, MYR, USD)
    pub currency: Currency,
    
    /// Line items (must have at least one)
    pub line_items: Vec<CreateLineItemRequest>,
}

/// Request to create a line item
#[derive(Debug, Deserialize)]
pub struct CreateLineItemRequest {
    /// Product/service description
    pub description: String,
    
    /// Quantity (must be positive)
    pub quantity: i32,
    
    /// Price per unit
    pub unit_price: rust_decimal::Decimal,
    
    /// Tax rate (e.g., 0.10 for 10%, optional)
    #[serde(default)]
    pub tax_rate: Option<rust_decimal::Decimal>,
    
    /// Tax category (e.g., "VAT", "GST", optional)
    #[serde(default)]
    pub tax_category: Option<String>,
}

/// Response for invoice operations
#[derive(Debug, Serialize)]
pub struct InvoiceResponse {
    pub id: String,
    pub external_id: String,
    pub gateway_id: String,
    pub currency: Currency,
    pub subtotal: rust_decimal::Decimal,
    pub tax_total: rust_decimal::Decimal,
    pub service_fee: rust_decimal::Decimal,
    pub total: rust_decimal::Decimal,
    pub status: String,
    pub line_items: Vec<LineItemResponse>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct LineItemResponse {
    pub id: String,
    pub description: String,
    pub quantity: i32,
    pub unit_price: rust_decimal::Decimal,
    pub currency: Currency,
    pub subtotal: rust_decimal::Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<rust_decimal::Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_amount: Option<rust_decimal::Decimal>,
}

impl From<Invoice> for InvoiceResponse {
    fn from(mut invoice: Invoice) -> Self {
        let subtotal = invoice.get_subtotal();
        let tax_total = invoice.tax_total.unwrap_or_default();
        let service_fee = invoice.service_fee.unwrap_or_default();
        let total = invoice.get_total();
        
        Self {
            id: invoice.id.unwrap_or_default(),
            external_id: invoice.external_id.clone(),
            gateway_id: invoice.gateway_id.clone(),
            currency: invoice.currency,
            subtotal,
            tax_total,
            service_fee,
            total,
            status: invoice.status.to_string(),
            line_items: invoice
                .line_items
                .into_iter()
                .map(LineItemResponse::from)
                .collect(),
            expires_at: invoice.expires_at.unwrap_or_else(chrono::Utc::now),
            created_at: invoice.created_at.unwrap_or_else(chrono::Utc::now),
            updated_at: invoice.updated_at.unwrap_or_else(chrono::Utc::now),
        }
    }
}

impl From<LineItem> for LineItemResponse {
    fn from(mut line_item: LineItem) -> Self {
        let subtotal = line_item.get_subtotal();
        Self {
            id: line_item.id.unwrap_or_default(),
            description: line_item.description.clone(),
            quantity: line_item.quantity,
            unit_price: line_item.unit_price,
            currency: line_item.currency,
            subtotal,
            tax_rate: line_item.tax_rate,
            tax_category: line_item.tax_category.clone(),
            tax_amount: line_item.tax_amount,
        }
    }
}

/// POST /invoices - Create a new invoice
/// 
/// # Request Body
/// ```json
/// {
///   "external_id": "INV-001",
///   "gateway_id": "xendit",
///   "currency": "IDR",
///   "line_items": [
///     {
///       "description": "Product A",
///       "quantity": 2,
///       "unit_price": 50000
///     }
///   ]
/// }
/// ```
/// 
/// # Response
/// - 201 Created: Invoice created successfully
/// - 400 Bad Request: Validation error
/// - 500 Internal Server Error: Database error
pub async fn create_invoice(
    pool: web::Data<MySqlPool>,
    request: web::Json<CreateInvoiceRequest>,
) -> Result<HttpResponse> {
    let service = InvoiceService::new(pool.get_ref().clone());

    // Convert request line items to domain models with tax information (FR-057, FR-058)
    let line_items: Result<Vec<LineItem>> = request
        .line_items
        .iter()
        .map(|req| {
            // If tax_rate is provided, create line item with tax
            if let Some(tax_rate) = req.tax_rate {
                LineItem::new_with_tax(
                    req.description.clone(),
                    req.quantity,
                    req.unit_price,
                    request.currency,
                    tax_rate,
                    req.tax_category.clone(),
                )
            } else {
                // Otherwise create regular line item (zero tax)
                LineItem::new(
                    req.description.clone(),
                    req.quantity,
                    req.unit_price,
                    request.currency,
                )
            }
        })
        .collect();

    let line_items = line_items?;

    // Create invoice
    let invoice = service
        .create_invoice(
            request.external_id.clone(),
            request.gateway_id.clone(),
            request.currency,
            line_items,
        )
        .await?;

    let response = InvoiceResponse::from(invoice);

    Ok(HttpResponse::Created().json(response))
}

/// GET /invoices/{id} - Get invoice by ID
/// 
/// # Path Parameters
/// - `id`: Invoice ID (UUID)
/// 
/// # Response
/// - 200 OK: Invoice found
/// - 404 Not Found: Invoice not found
/// - 500 Internal Server Error: Database error
pub async fn get_invoice(
    pool: web::Data<MySqlPool>,
    id: web::Path<String>,
) -> Result<HttpResponse> {
    let service = InvoiceService::new(pool.get_ref().clone());

    let invoice = service.get_invoice(&id).await?;
    let response = InvoiceResponse::from(invoice);

    Ok(HttpResponse::Ok().json(response))
}

/// Query parameters for listing invoices
#[derive(Debug, Deserialize)]
pub struct ListInvoicesQuery {
    /// Maximum results (default: 20, max: 100)
    pub limit: Option<i64>,
    
    /// Results to skip (default: 0)
    pub offset: Option<i64>,
}

/// GET /invoices - List invoices with pagination
/// 
/// # Query Parameters
/// - `limit`: Maximum results (default: 20, max: 100)
/// - `offset`: Results to skip (default: 0)
/// 
/// # Response
/// - 200 OK: List of invoices
/// - 500 Internal Server Error: Database error
pub async fn list_invoices(
    pool: web::Data<MySqlPool>,
    query: web::Query<ListInvoicesQuery>,
) -> Result<HttpResponse> {
    let service = InvoiceService::new(pool.get_ref().clone());

    let invoices = service.list_invoices(query.limit, query.offset).await?;
    
    let response: Vec<InvoiceResponse> = invoices
        .into_iter()
        .map(InvoiceResponse::from)
        .collect();

    Ok(HttpResponse::Ok().json(response))
}

/// Configure invoice routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/invoices")
            .route("", web::post().to(create_invoice))
            .route("", web::get().to(list_invoices))
            .route("/{id}", web::get().to(get_invoice)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_deserialization() {
        let json = r#"{
            "external_id": "INV-001",
            "gateway_id": "xendit",
            "currency": "IDR",
            "line_items": [
                {
                    "description": "Product A",
                    "quantity": 2,
                    "unit_price": 50000
                }
            ]
        }"#;

        let request: CreateInvoiceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.external_id, "INV-001");
        assert_eq!(request.line_items.len(), 1);
    }
}
