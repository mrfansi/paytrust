use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::core::{AppError, Currency, Result};

/// Installment payment schedule entry for an invoice
/// Implements FR-014 to FR-020, FR-059, FR-060, FR-068 to FR-072
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InstallmentSchedule {
    pub id: String,
    pub invoice_id: String,
    /// Sequential number (1, 2, 3...) - FR-014
    pub installment_number: i32,
    /// Payment amount for this installment
    pub amount: Decimal,
    /// Proportionally distributed tax (FR-059)
    pub tax_amount: Decimal,
    /// Proportionally distributed service fee (FR-060)
    pub service_fee_amount: Decimal,
    /// Payment due date
    pub due_date: NaiveDate,
    /// Current status
    #[sqlx(try_from = "String")]
    pub status: InstallmentStatus,
    /// Gateway-generated payment URL
    pub payment_url: Option<String>,
    /// Gateway transaction reference
    pub gateway_reference: Option<String>,
    /// Payment completion timestamp
    pub paid_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

/// Installment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallmentStatus {
    /// Not yet paid
    Unpaid,
    /// Payment received
    Paid,
    /// Due date passed without payment
    Overdue,
}

impl InstallmentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unpaid => "unpaid",
            Self::Paid => "paid",
            Self::Overdue => "overdue",
        }
    }
}

impl std::fmt::Display for InstallmentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<String> for InstallmentStatus {
    type Error = String;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.as_str() {
            "unpaid" => Ok(Self::Unpaid),
            "paid" => Ok(Self::Paid),
            "overdue" => Ok(Self::Overdue),
            _ => Err(format!("Invalid installment status: {}", value)),
        }
    }
}

impl InstallmentSchedule {
    /// Create a new installment schedule entry
    /// 
    /// # Arguments
    /// * `invoice_id` - Parent invoice ID
    /// * `installment_number` - Sequential number (1-based)
    /// * `amount` - Payment amount for this installment
    /// * `tax_amount` - Proportionally distributed tax
    /// * `service_fee_amount` - Proportionally distributed service fee
    /// * `due_date` - Payment due date
    pub fn new(
        invoice_id: String,
        installment_number: i32,
        amount: Decimal,
        tax_amount: Decimal,
        service_fee_amount: Decimal,
        due_date: NaiveDate,
    ) -> Result<Self> {
        // Validate installment number (FR-014: 2-12 installments)
        if installment_number < 1 || installment_number > 12 {
            return Err(AppError::validation(
                format!("Installment number must be between 1 and 12, got {}", installment_number)
            ));
        }

        // Validate amounts
        if amount <= Decimal::ZERO {
            return Err(AppError::validation("Installment amount must be positive"));
        }

        if tax_amount < Decimal::ZERO {
            return Err(AppError::validation("Tax amount cannot be negative"));
        }

        if service_fee_amount < Decimal::ZERO {
            return Err(AppError::validation("Service fee amount cannot be negative"));
        }

        let now = chrono::Utc::now().naive_utc();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            invoice_id,
            installment_number,
            amount,
            tax_amount,
            service_fee_amount,
            due_date,
            status: InstallmentStatus::Unpaid,
            payment_url: None,
            gateway_reference: None,
            paid_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// Check if this installment can be paid (FR-068: sequential enforcement)
    /// 
    /// An installment can be paid if:
    /// - It is the first installment (number 1), OR
    /// - All previous installments have been paid
    pub fn can_be_paid(&self, previous_installments: &[InstallmentSchedule]) -> bool {
        // First installment can always be paid
        if self.installment_number == 1 {
            return true;
        }

        // Check that all previous installments are paid
        for i in 1..self.installment_number {
            let prev_paid = previous_installments
                .iter()
                .any(|inst| inst.installment_number == i && inst.status == InstallmentStatus::Paid);
            
            if !prev_paid {
                return false;
            }
        }

        true
    }

    /// Check if this installment can be adjusted (FR-077)
    /// 
    /// Only unpaid installments can be adjusted after first payment
    pub fn can_be_adjusted(&self) -> bool {
        self.status == InstallmentStatus::Unpaid
    }

    /// Mark installment as paid
    pub fn mark_as_paid(&mut self, gateway_reference: String) -> Result<()> {
        if self.status == InstallmentStatus::Paid {
            return Err(AppError::validation(
                format!("Installment {} is already paid", self.installment_number)
            ));
        }

        self.status = InstallmentStatus::Paid;
        self.gateway_reference = Some(gateway_reference);
        self.paid_at = Some(chrono::Utc::now().naive_utc());
        self.updated_at = chrono::Utc::now().naive_utc();

        Ok(())
    }

    /// Mark installment as overdue
    pub fn mark_as_overdue(&mut self) -> Result<()> {
        if self.status == InstallmentStatus::Paid {
            return Err(AppError::validation(
                "Cannot mark paid installment as overdue"
            ));
        }

        self.status = InstallmentStatus::Overdue;
        self.updated_at = chrono::Utc::now().naive_utc();

        Ok(())
    }

    /// Update payment URL from gateway
    pub fn set_payment_url(&mut self, url: String) {
        self.payment_url = Some(url);
        self.updated_at = chrono::Utc::now().naive_utc();
    }

    /// Get total amount including tax and service fee
    pub fn total_amount(&self) -> Decimal {
        self.amount + self.tax_amount + self.service_fee_amount
    }

    /// Check if installment is past due date
    pub fn is_past_due(&self) -> bool {
        if self.status == InstallmentStatus::Paid {
            return false;
        }

        let today = chrono::Utc::now().date_naive();
        self.due_date < today
    }
}

/// Configuration for creating installment schedules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallmentConfig {
    /// Number of installments (2-12 per FR-014)
    pub installment_count: i32,
    /// Optional custom amounts per installment
    /// If provided, must sum to invoice total (FR-017)
    pub custom_amounts: Option<Vec<Decimal>>,
}

impl InstallmentConfig {
    /// Validate installment configuration
    pub fn validate(&self, invoice_total: Decimal) -> Result<()> {
        // FR-014: 2-12 installments
        if self.installment_count < 2 || self.installment_count > 12 {
            return Err(AppError::validation(
                format!("Installment count must be between 2 and 12, got {}", self.installment_count)
            ));
        }

        // If custom amounts provided, validate
        if let Some(ref amounts) = self.custom_amounts {
            if amounts.len() != self.installment_count as usize {
                return Err(AppError::validation(
                    format!(
                        "Custom amounts count ({}) must match installment count ({})",
                        amounts.len(),
                        self.installment_count
                    )
                ));
            }

            // FR-017: Sum must equal invoice total
            let sum: Decimal = amounts.iter().sum();
            if sum != invoice_total {
                return Err(AppError::validation(
                    format!(
                        "Sum of custom amounts ({}) must equal invoice total ({})",
                        sum, invoice_total
                    )
                ));
            }

            // All amounts must be positive
            for (i, amount) in amounts.iter().enumerate() {
                if *amount <= Decimal::ZERO {
                    return Err(AppError::validation(
                        format!("Installment {} amount must be positive", i + 1)
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_installment_schedule_creation() {
        let installment = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(100000),
            dec!(11000),
            dec!(5000),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(installment.is_ok());
        let inst = installment.unwrap();
        assert_eq!(inst.installment_number, 1);
        assert_eq!(inst.amount, dec!(100000));
        assert_eq!(inst.tax_amount, dec!(11000));
        assert_eq!(inst.service_fee_amount, dec!(5000));
        assert_eq!(inst.total_amount(), dec!(116000));
        assert_eq!(inst.status, InstallmentStatus::Unpaid);
    }

    #[test]
    fn test_installment_validation_negative_number() {
        let result = InstallmentSchedule::new(
            "inv-123".to_string(),
            0,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("between 1 and 12"));
    }

    #[test]
    fn test_installment_validation_too_high() {
        let result = InstallmentSchedule::new(
            "inv-123".to_string(),
            13,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_installment_validation_negative_amount() {
        let result = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(-100),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be positive"));
    }

    #[test]
    fn test_can_be_paid_first_installment() {
        let inst = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        ).unwrap();

        assert!(inst.can_be_paid(&[]));
    }

    #[test]
    fn test_can_be_paid_sequential_enforcement() {
        let mut inst1 = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        ).unwrap();

        let inst2 = InstallmentSchedule::new(
            "inv-123".to_string(),
            2,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 15).unwrap(),
        ).unwrap();

        // Second installment cannot be paid before first
        assert!(!inst2.can_be_paid(&[inst1.clone()]));

        // Mark first as paid
        inst1.mark_as_paid("gw-ref-1".to_string()).unwrap();

        // Now second can be paid
        assert!(inst2.can_be_paid(&[inst1]));
    }

    #[test]
    fn test_mark_as_paid() {
        let mut inst = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        ).unwrap();

        assert_eq!(inst.status, InstallmentStatus::Unpaid);
        assert!(inst.paid_at.is_none());

        inst.mark_as_paid("gw-ref-123".to_string()).unwrap();

        assert_eq!(inst.status, InstallmentStatus::Paid);
        assert!(inst.paid_at.is_some());
        assert_eq!(inst.gateway_reference, Some("gw-ref-123".to_string()));
    }

    #[test]
    fn test_cannot_double_pay() {
        let mut inst = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        ).unwrap();

        inst.mark_as_paid("ref-1".to_string()).unwrap();
        let result = inst.mark_as_paid("ref-2".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already paid"));
    }

    #[test]
    fn test_can_be_adjusted() {
        let mut inst = InstallmentSchedule::new(
            "inv-123".to_string(),
            1,
            dec!(100000),
            dec!(0),
            dec!(0),
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        ).unwrap();

        assert!(inst.can_be_adjusted());

        inst.mark_as_paid("ref".to_string()).unwrap();
        assert!(!inst.can_be_adjusted());
    }

    #[test]
    fn test_installment_config_validation() {
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: None,
        };

        assert!(config.validate(dec!(300000)).is_ok());
    }

    #[test]
    fn test_installment_config_count_validation() {
        let config = InstallmentConfig {
            installment_count: 1,
            custom_amounts: None,
        };

        assert!(config.validate(dec!(300000)).is_err());

        let config = InstallmentConfig {
            installment_count: 13,
            custom_amounts: None,
        };

        assert!(config.validate(dec!(300000)).is_err());
    }

    #[test]
    fn test_installment_config_custom_amounts_validation() {
        // Valid custom amounts
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: Some(vec![dec!(100000), dec!(100000), dec!(100000)]),
        };

        assert!(config.validate(dec!(300000)).is_ok());

        // Sum mismatch
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: Some(vec![dec!(100000), dec!(100000), dec!(50000)]),
        };

        assert!(config.validate(dec!(300000)).is_err());

        // Count mismatch
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: Some(vec![dec!(150000), dec!(150000)]),
        };

        assert!(config.validate(dec!(300000)).is_err());
    }
}
