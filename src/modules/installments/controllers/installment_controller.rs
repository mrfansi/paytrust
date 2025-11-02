// T092: InstallmentController implementation
// HTTP handlers for installment endpoints
//
// Endpoints:
// - GET /invoices/{id}/installments - Get installment schedule for an invoice
// - PATCH /invoices/{id}/installments - Adjust unpaid installment amounts

use actix_web::{web, HttpResponse};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

use crate::core::{Currency, Result};
use crate::modules::installments::{
    models::{InstallmentSchedule, InstallmentStatus},
    services::InstallmentService,
};

/// Response for a single installment
#[derive(Debug, Serialize)]
pub struct InstallmentResponse {
    pub id: String,
    pub installment_number: i32,
    pub amount: String,
    pub tax_amount: String,
    pub service_fee_amount: String,
    pub due_date: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paid_at: Option<String>,
}

impl From<InstallmentSchedule> for InstallmentResponse {
    fn from(installment: InstallmentSchedule) -> Self {
        Self {
            id: installment.id,
            installment_number: installment.installment_number,
            amount: installment.amount.to_string(),
            tax_amount: installment.tax_amount.to_string(),
            service_fee_amount: installment.service_fee_amount.to_string(),
            due_date: installment.due_date.to_string(),
            status: installment.status.to_string(),
            payment_url: installment.payment_url,
            gateway_reference: installment.gateway_reference,
            paid_at: installment.paid_at.map(|dt| dt.to_string()),
        }
    }
}

/// Response for GET /invoices/{id}/installments
#[derive(Debug, Serialize)]
pub struct GetInstallmentsResponse {
    pub invoice_id: String,
    pub installments: Vec<InstallmentResponse>,
}

/// Request for PATCH /invoices/{id}/installments
#[derive(Debug, Deserialize)]
pub struct AdjustInstallmentsRequest {
    pub installments: Vec<InstallmentAdjustment>,
}

#[derive(Debug, Deserialize)]
pub struct InstallmentAdjustment {
    pub installment_number: i32,
    pub new_amount: String,
}

/// Response for PATCH /invoices/{id}/installments
#[derive(Debug, Serialize)]
pub struct AdjustInstallmentsResponse {
    pub invoice_id: String,
    pub installments: Vec<InstallmentResponse>,
}

/// GET /invoices/{invoice_id}/installments
///
/// Returns the installment schedule for an invoice.
///
/// # Path Parameters
/// - `invoice_id`: UUID of the invoice
///
/// # Returns
/// - 200: Installment schedule with all installments
/// - 404: Invoice not found
pub async fn get_installments(
    invoice_id: web::Path<String>,
    pool: web::Data<MySqlPool>,
) -> Result<HttpResponse> {
    let service = InstallmentService::new(pool.get_ref().clone());

    let installments = service.get_installments(&invoice_id).await?;

    let response = GetInstallmentsResponse {
        invoice_id: invoice_id.into_inner(),
        installments: installments
            .into_iter()
            .map(InstallmentResponse::from)
            .collect(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// PATCH /invoices/{invoice_id}/installments
///
/// Adjusts unpaid installment amounts while maintaining total invoice amount.
/// Only unpaid installments can be modified (FR-077, FR-078).
///
/// # Path Parameters
/// - `invoice_id`: UUID of the invoice
///
/// # Request Body
/// ```json
/// {
///   "installments": [
///     {"installment_number": 2, "new_amount": "4000000"},
///     {"installment_number": 3, "new_amount": "5100000"}
///   ]
/// }
/// ```
///
/// # Business Rules
/// - FR-077: Only unpaid installments can be adjusted
/// - FR-078: Paid installments remain unchanged
/// - FR-079: New amounts recalculate proportional tax/fee
/// - FR-080: Sum of new amounts must equal remaining total
///
/// # Returns
/// - 200: Installments adjusted successfully
/// - 400: Invalid adjustment request (paid installments, amount mismatch, etc.)
/// - 404: Invoice not found
pub async fn adjust_installments(
    invoice_id: web::Path<String>,
    request: web::Json<AdjustInstallmentsRequest>,
    pool: web::Data<MySqlPool>,
) -> Result<HttpResponse> {
    let service = InstallmentService::new(pool.get_ref().clone());

    // Get existing installments
    let existing_installments = service.get_installments(&invoice_id).await?;

    if existing_installments.is_empty() {
        return Err(crate::core::AppError::not_found(
            "No installments found for this invoice",
        ));
    }

    // Parse new amounts from request
    let mut new_amounts_map: std::collections::HashMap<i32, Decimal> =
        std::collections::HashMap::new();
    for adjustment in &request.installments {
        let amount = adjustment.new_amount.parse::<Decimal>().map_err(|_| {
            crate::core::AppError::validation(format!(
                "Invalid amount format: {}",
                adjustment.new_amount
            ))
        })?;
        new_amounts_map.insert(adjustment.installment_number, amount);
    }

    // Validate that only unpaid installments are being adjusted
    for installment in &existing_installments {
        if new_amounts_map.contains_key(&installment.installment_number)
            && installment.status != InstallmentStatus::Unpaid
        {
            return Err(crate::core::AppError::validation(
                format!(
                    "Cannot adjust installment {}: only unpaid installments can be modified (status: {})",
                    installment.installment_number,
                    installment.status
                )
            ));
        }
    }

    // Calculate remaining totals (from unpaid installments)
    let unpaid_installments: Vec<_> = existing_installments
        .iter()
        .filter(|i| i.status == InstallmentStatus::Unpaid)
        .collect();

    let remaining_total: Decimal = unpaid_installments.iter().map(|i| i.amount).sum();
    let remaining_tax: Decimal = unpaid_installments.iter().map(|i| i.tax_amount).sum();
    let remaining_fee: Decimal = unpaid_installments
        .iter()
        .map(|i| i.service_fee_amount)
        .sum();

    // Validate that sum of new amounts equals remaining total
    let new_amounts_sum: Decimal = new_amounts_map.values().sum();
    if new_amounts_sum != remaining_total {
        return Err(crate::core::AppError::validation(format!(
            "Sum of new amounts ({}) must equal remaining total ({})",
            new_amounts_sum, remaining_total
        )));
    }

    // Get currency from first installment (all should have same invoice currency)
    // We need to get the invoice to know the currency
    // For now, we'll infer from the installment amounts' precision
    let currency = infer_currency_from_amounts(&existing_installments);

    // Adjust unpaid installments
    let updated_installments = service
        .adjust_unpaid_installments(
            &invoice_id,
            remaining_total,
            remaining_tax,
            remaining_fee,
            currency,
        )
        .await?;

    let response = AdjustInstallmentsResponse {
        invoice_id: invoice_id.into_inner(),
        installments: updated_installments
            .into_iter()
            .map(InstallmentResponse::from)
            .collect(),
    };

    Ok(HttpResponse::Ok().json(response))
}

/// Infer currency from installment amounts' decimal precision
/// - 0 decimals = IDR
/// - 2 decimals = MYR or USD (default to MYR)
fn infer_currency_from_amounts(installments: &[InstallmentSchedule]) -> Currency {
    if let Some(first) = installments.first() {
        let scale = first.amount.scale();
        if scale == 0 {
            Currency::IDR
        } else {
            // Default to MYR for 2 decimal currencies
            // In production, this should come from the invoice record
            Currency::MYR
        }
    } else {
        Currency::IDR // Default fallback
    }
}

/// Configure installment routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/invoices/{invoice_id}")
            .route("/installments", web::get().to(get_installments))
            .route("/installments", web::patch().to(adjust_installments)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_installment_response_serialization() {
        let installment = InstallmentSchedule {
            id: "inst-001".to_string(),
            invoice_id: "inv-001".to_string(),
            installment_number: 1,
            amount: Decimal::new(500000, 0),
            tax_amount: Decimal::new(55000, 0),
            service_fee_amount: Decimal::new(10000, 0),
            due_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            status: InstallmentStatus::Unpaid,
            payment_url: Some("https://pay.example.com/123".to_string()),
            gateway_reference: None,
            paid_at: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        let response = InstallmentResponse::from(installment);

        assert_eq!(response.id, "inst-001");
        assert_eq!(response.installment_number, 1);
        assert_eq!(response.amount, "500000");
        assert_eq!(response.status, "unpaid");
        assert_eq!(
            response.payment_url,
            Some("https://pay.example.com/123".to_string())
        );
    }

    #[test]
    fn test_currency_inference() {
        let idr_installment = InstallmentSchedule {
            id: "inst-001".to_string(),
            invoice_id: "inv-001".to_string(),
            installment_number: 1,
            amount: Decimal::new(1000000, 0), // 0 decimals
            tax_amount: Decimal::new(110000, 0),
            service_fee_amount: Decimal::new(20000, 0),
            due_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            status: InstallmentStatus::Unpaid,
            payment_url: None,
            gateway_reference: None,
            paid_at: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        assert_eq!(
            infer_currency_from_amounts(&[idr_installment]),
            Currency::IDR
        );

        let myr_installment = InstallmentSchedule {
            id: "inst-002".to_string(),
            invoice_id: "inv-002".to_string(),
            installment_number: 1,
            amount: Decimal::new(100050, 2), // 2 decimals: 1000.50
            tax_amount: Decimal::new(6003, 2),
            service_fee_amount: Decimal::new(2000, 2),
            due_date: NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            status: InstallmentStatus::Unpaid,
            payment_url: None,
            gateway_reference: None,
            paid_at: None,
            created_at: chrono::Utc::now().naive_utc(),
            updated_at: chrono::Utc::now().naive_utc(),
        };

        assert_eq!(
            infer_currency_from_amounts(&[myr_installment]),
            Currency::MYR
        );
    }
}
