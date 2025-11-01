use chrono::NaiveDate;
use rust_decimal::Decimal;
use tracing::{info, warn};

use crate::core::{AppError, Currency, Result};
use crate::modules::installments::models::{InstallmentConfig, InstallmentSchedule};

/// Calculator for installment payment schedules
/// Implements FR-059 (proportional tax distribution), FR-060 (proportional service fee distribution),
/// FR-071 (rounding handling), FR-072 (last installment absorption)
pub struct InstallmentCalculator;

impl InstallmentCalculator {
    /// Calculate installment schedules for an invoice
    /// 
    /// Distributes the invoice total, taxes, and service fees proportionally across installments.
    /// The last installment absorbs any rounding differences to ensure exact total match.
    /// 
    /// # Arguments
    /// * `invoice_id` - Parent invoice ID
    /// * `invoice_total` - Total invoice amount (subtotal only, excluding tax and fees for proportion calculation)
    /// * `tax_total` - Total tax amount to distribute
    /// * `service_fee_total` - Total service fee to distribute
    /// * `config` - Installment configuration (count and optional custom amounts)
    /// * `currency` - Invoice currency for precision handling
    /// * `start_date` - First installment due date
    /// 
    /// # Returns
    /// Vector of InstallmentSchedule objects with proportionally distributed amounts
    pub fn calculate_schedules(
        invoice_id: String,
        invoice_total: Decimal,
        tax_total: Decimal,
        service_fee_total: Decimal,
        config: &InstallmentConfig,
        currency: Currency,
        start_date: NaiveDate,
    ) -> Result<Vec<InstallmentSchedule>> {
        // Validate configuration
        config.validate(invoice_total)?;

        info!(
            "Calculating {} installment schedules for invoice {} (total: {}, tax: {}, fee: {})",
            config.installment_count, invoice_id, invoice_total, tax_total, service_fee_total
        );

        let mut schedules = Vec::new();
        let count = config.installment_count as usize;

        // Calculate base amounts per installment
        let base_amounts = if let Some(ref custom) = config.custom_amounts {
            custom.clone()
        } else {
            // Equal division
            Self::calculate_equal_amounts(invoice_total, count, currency)?
        };

        // Calculate proportional tax and fee for each installment (FR-059, FR-060)
        let mut total_distributed_amount = Decimal::ZERO;
        let mut total_distributed_tax = Decimal::ZERO;
        let mut total_distributed_fee = Decimal::ZERO;

        for i in 0..count {
            let installment_number = (i + 1) as i32;
            let base_amount = base_amounts[i];

            // Calculate proportional amounts (FR-059, FR-060)
            // Formula: installment_portion = item_total × (installment_amount / invoice_total)
            let proportion = base_amount / invoice_total;
            let tax_amount = if i == count - 1 {
                // Last installment absorbs rounding differences (FR-071, FR-072)
                tax_total - total_distributed_tax
            } else {
                let calculated = (tax_total * proportion).round_dp(currency.scale());
                calculated
            };

            let service_fee_amount = if i == count - 1 {
                // Last installment absorbs rounding differences (FR-071, FR-072)
                service_fee_total - total_distributed_fee
            } else {
                let calculated = (service_fee_total * proportion).round_dp(currency.scale());
                calculated
            };

            // Calculate due date (monthly intervals)
            let due_date = start_date
                .checked_add_months(chrono::Months::new(i as u32))
                .ok_or_else(|| AppError::validation("Failed to calculate due date"))?;

            let schedule = InstallmentSchedule::new(
                invoice_id.clone(),
                installment_number,
                base_amount,
                tax_amount,
                service_fee_amount,
                due_date,
            )?;

            total_distributed_amount += base_amount;
            total_distributed_tax += tax_amount;
            total_distributed_fee += service_fee_amount;

            schedules.push(schedule);
        }

        // Verify totals match exactly (FR-017)
        if total_distributed_amount != invoice_total {
            warn!(
                "Installment amount mismatch: distributed {} vs invoice {}",
                total_distributed_amount, invoice_total
            );
            return Err(AppError::validation(
                format!(
                    "Installment amounts ({}) do not sum to invoice total ({})",
                    total_distributed_amount, invoice_total
                )
            ));
        }

        if total_distributed_tax != tax_total {
            warn!(
                "Tax distribution mismatch: distributed {} vs total {}",
                total_distributed_tax, tax_total
            );
            return Err(AppError::validation(
                format!(
                    "Distributed tax ({}) does not match total tax ({})",
                    total_distributed_tax, tax_total
                )
            ));
        }

        if total_distributed_fee != service_fee_total {
            warn!(
                "Fee distribution mismatch: distributed {} vs total {}",
                total_distributed_fee, service_fee_total
            );
            return Err(AppError::validation(
                format!(
                    "Distributed service fee ({}) does not match total ({})",
                    total_distributed_fee, service_fee_total
                )
            ));
        }

        info!(
            "Successfully calculated {} installments with exact total match",
            count
        );

        Ok(schedules)
    }

    /// Calculate equal amounts for installments with last absorbing rounding (FR-071, FR-072)
    fn calculate_equal_amounts(
        total: Decimal,
        count: usize,
        currency: Currency,
    ) -> Result<Vec<Decimal>> {
        if count == 0 {
            return Err(AppError::validation("Installment count cannot be zero"));
        }

        let mut amounts = Vec::with_capacity(count);
        let base_amount = (total / Decimal::from(count)).round_dp(currency.scale());
        let mut distributed = Decimal::ZERO;

        for i in 0..count {
            let amount = if i == count - 1 {
                // Last installment absorbs rounding difference (FR-072)
                total - distributed
            } else {
                base_amount
            };

            if amount <= Decimal::ZERO {
                return Err(AppError::validation(
                    "Calculated installment amount must be positive"
                ));
            }

            amounts.push(amount);
            distributed += amount;
        }

        Ok(amounts)
    }

    /// Recalculate installment schedules after adjustment (FR-077, FR-078, FR-079, FR-080)
    /// 
    /// Only unpaid installments can be recalculated. Paid installments remain unchanged.
    /// 
    /// # Arguments
    /// * `existing_schedules` - Current installment schedules
    /// * `remaining_total` - Remaining unpaid invoice amount
    /// * `remaining_tax` - Remaining unpaid tax amount
    /// * `remaining_fee` - Remaining unpaid service fee amount
    /// * `currency` - Invoice currency
    /// 
    /// # Returns
    /// Updated schedules for unpaid installments
    pub fn recalculate_unpaid_schedules(
        existing_schedules: Vec<InstallmentSchedule>,
        remaining_total: Decimal,
        remaining_tax: Decimal,
        remaining_fee: Decimal,
        currency: Currency,
    ) -> Result<Vec<InstallmentSchedule>> {
        // Separate paid and unpaid
        let (paid, mut unpaid): (Vec<_>, Vec<_>) = existing_schedules
            .into_iter()
            .partition(|s| !s.can_be_adjusted());

        if unpaid.is_empty() {
            return Ok(paid);
        }

        info!(
            "Recalculating {} unpaid installments (remaining: amount={}, tax={}, fee={})",
            unpaid.len(),
            remaining_total,
            remaining_tax,
            remaining_fee
        );

        let unpaid_count = unpaid.len();

        // Calculate new equal amounts for unpaid installments
        let new_base_amounts = Self::calculate_equal_amounts(
            remaining_total,
            unpaid_count,
            currency,
        )?;

        // Redistribute tax and fees proportionally
        let mut total_distributed_tax = Decimal::ZERO;
        let mut total_distributed_fee = Decimal::ZERO;

        for (i, schedule) in unpaid.iter_mut().enumerate() {
            let base_amount = new_base_amounts[i];
            let proportion = base_amount / remaining_total;

            // Calculate new tax and fee amounts
            let tax_amount = if i == unpaid_count - 1 {
                remaining_tax - total_distributed_tax
            } else {
                let calculated = (remaining_tax * proportion).round_dp(currency.scale());
                calculated
            };

            let service_fee_amount = if i == unpaid_count - 1 {
                remaining_fee - total_distributed_fee
            } else {
                let calculated = (remaining_fee * proportion).round_dp(currency.scale());
                calculated
            };

            // Update schedule
            schedule.amount = base_amount;
            schedule.tax_amount = tax_amount;
            schedule.service_fee_amount = service_fee_amount;
            schedule.updated_at = chrono::Utc::now().naive_utc();

            total_distributed_tax += tax_amount;
            total_distributed_fee += service_fee_amount;
        }

        // Combine paid and updated unpaid schedules
        let mut result = paid;
        result.append(&mut unpaid);
        result.sort_by_key(|s| s.installment_number);

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_calculate_equal_schedules() {
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: None,
        };

        let schedules = InstallmentCalculator::calculate_schedules(
            "inv-123".to_string(),
            dec!(300000), // Total
            dec!(33000),  // Tax (11%)
            dec!(15000),  // Fee (5%)
            &config,
            Currency::IDR,
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(schedules.is_ok());
        let schedules = schedules.unwrap();
        assert_eq!(schedules.len(), 3);

        // Check amounts sum to total
        let total_amount: Decimal = schedules.iter().map(|s| s.amount).sum();
        let total_tax: Decimal = schedules.iter().map(|s| s.tax_amount).sum();
        let total_fee: Decimal = schedules.iter().map(|s| s.service_fee_amount).sum();

        assert_eq!(total_amount, dec!(300000));
        assert_eq!(total_tax, dec!(33000));
        assert_eq!(total_fee, dec!(15000));

        // Check proportional distribution
        for schedule in &schedules {
            let proportion = schedule.amount / dec!(300000);
            let expected_tax = (dec!(33000) * proportion).round_dp(0);
            let expected_fee = (dec!(15000) * proportion).round_dp(0);

            // Allow small difference due to rounding in last installment
            let tax_diff = (schedule.tax_amount - expected_tax).abs();
            let fee_diff = (schedule.service_fee_amount - expected_fee).abs();

            assert!(tax_diff <= dec!(2), "Tax difference too large: {}", tax_diff);
            assert!(fee_diff <= dec!(2), "Fee difference too large: {}", fee_diff);
        }
    }

    #[test]
    fn test_calculate_custom_amounts() {
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: Some(vec![dec!(100000), dec!(120000), dec!(80000)]),
        };

        let schedules = InstallmentCalculator::calculate_schedules(
            "inv-123".to_string(),
            dec!(300000),
            dec!(33000),
            dec!(15000),
            &config,
            Currency::IDR,
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(schedules.is_ok());
        let schedules = schedules.unwrap();

        assert_eq!(schedules[0].amount, dec!(100000));
        assert_eq!(schedules[1].amount, dec!(120000));
        assert_eq!(schedules[2].amount, dec!(80000));

        // Verify totals
        let total_amount: Decimal = schedules.iter().map(|s| s.amount).sum();
        assert_eq!(total_amount, dec!(300000));
    }

    #[test]
    fn test_last_installment_absorbs_rounding() {
        // Use amount that creates rounding with 3 divisions
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: None,
        };

        let schedules = InstallmentCalculator::calculate_schedules(
            "inv-123".to_string(),
            dec!(100), // Will be 33.33 + 33.33 + 33.34
            dec!(11),  // Will have rounding
            dec!(5),   // Will have rounding
            &config,
            Currency::IDR,
            NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        );

        assert!(schedules.is_ok());
        let schedules = schedules.unwrap();

        // Exact totals must match
        let total_amount: Decimal = schedules.iter().map(|s| s.amount).sum();
        let total_tax: Decimal = schedules.iter().map(|s| s.tax_amount).sum();
        let total_fee: Decimal = schedules.iter().map(|s| s.service_fee_amount).sum();

        assert_eq!(total_amount, dec!(100));
        assert_eq!(total_tax, dec!(11));
        assert_eq!(total_fee, dec!(5));
    }

    #[test]
    fn test_recalculate_unpaid_schedules() {
        // Create initial schedules
        let mut schedules = vec![
            InstallmentSchedule::new(
                "inv-123".to_string(),
                1,
                dec!(100000),
                dec!(11000),
                dec!(5000),
                NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
            ).unwrap(),
            InstallmentSchedule::new(
                "inv-123".to_string(),
                2,
                dec!(100000),
                dec!(11000),
                dec!(5000),
                NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            ).unwrap(),
            InstallmentSchedule::new(
                "inv-123".to_string(),
                3,
                dec!(100000),
                dec!(11000),
                dec!(5000),
                NaiveDate::from_ymd_opt(2026, 2, 1).unwrap(),
            ).unwrap(),
        ];

        // Mark first as paid
        schedules[0].mark_as_paid("ref-1".to_string()).unwrap();

        // Recalculate remaining
        let result = InstallmentCalculator::recalculate_unpaid_schedules(
            schedules,
            dec!(200000), // Remaining amount
            dec!(22000),  // Remaining tax
            dec!(10000),  // Remaining fee
            Currency::IDR,
        );

        assert!(result.is_ok());
        let updated = result.unwrap();

        assert_eq!(updated.len(), 3);

        // First should be unchanged (paid)
        assert_eq!(updated[0].amount, dec!(100000));
        assert_eq!(updated[0].tax_amount, dec!(11000));

        // Second and third should be recalculated equally
        assert_eq!(updated[1].amount, dec!(100000));
        assert_eq!(updated[2].amount, dec!(100000));

        // Totals should match
        let total_tax: Decimal = updated[1..].iter().map(|s| s.tax_amount).sum();
        let total_fee: Decimal = updated[1..].iter().map(|s| s.service_fee_amount).sum();

        assert_eq!(total_tax, dec!(22000));
        assert_eq!(total_fee, dec!(10000));
    }

    #[test]
    fn test_due_date_monthly_intervals() {
        let config = InstallmentConfig {
            installment_count: 3,
            custom_amounts: None,
        };

        let start = NaiveDate::from_ymd_opt(2025, 11, 15).unwrap();
        let schedules = InstallmentCalculator::calculate_schedules(
            "inv-123".to_string(),
            dec!(300000),
            dec!(0),
            dec!(0),
            &config,
            Currency::IDR,
            start,
        ).unwrap();

        assert_eq!(schedules[0].due_date, NaiveDate::from_ymd_opt(2025, 11, 15).unwrap());
        assert_eq!(schedules[1].due_date, NaiveDate::from_ymd_opt(2025, 12, 15).unwrap());
        assert_eq!(schedules[2].due_date, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
    }
}
