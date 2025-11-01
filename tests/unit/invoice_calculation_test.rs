// T032: Property-based test for invoice total calculation
// Using proptest to verify invoice total calculation across all valid inputs
//
// Properties tested:
// 1. invoice_total = sum(line_item_subtotals) (basic aggregation)
// 2. invoice total is non-negative when all line items are non-negative
// 3. invoice total precision matches currency scale
// 4. invoice total with single line item equals that line item's subtotal
// 5. invoice total with empty line items is zero
// 6. order of line items doesn't affect total (commutative property)

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

// Mock Currency enum (will be replaced with actual import)
#[derive(Debug, Clone, Copy, PartialEq)]
enum Currency {
    IDR,
    MYR,
    USD,
}

impl Currency {
    fn scale(&self) -> u32 {
        match self {
            Currency::IDR => 0,
            Currency::MYR | Currency::USD => 2,
        }
    }
}

// Mock LineItem struct (will be replaced with actual import)
#[derive(Debug, Clone)]
struct LineItem {
    quantity: i32,
    unit_price: Decimal,
    currency: Currency,
}

impl LineItem {
    fn calculate_subtotal(&self) -> Decimal {
        let subtotal = Decimal::from(self.quantity) * self.unit_price;
        self.round_to_currency(subtotal)
    }

    fn round_to_currency(&self, amount: Decimal) -> Decimal {
        amount.round_dp(self.currency.scale())
    }
}

// Mock Invoice struct (will be replaced with actual import)
#[derive(Debug, Clone)]
struct Invoice {
    line_items: Vec<LineItem>,
    currency: Currency,
}

impl Invoice {
    fn calculate_total(&self) -> Decimal {
        let total = self.line_items
            .iter()
            .map(|item| item.calculate_subtotal())
            .sum();
        
        self.round_to_currency(total)
    }
    
    fn round_to_currency(&self, amount: Decimal) -> Decimal {
        amount.round_dp(self.currency.scale())
    }
}

// Strategy to generate a line item with valid properties
fn line_item_strategy(currency: Currency) -> impl Strategy<Value = LineItem> {
    let scale = currency.scale();
    (1i32..=100, 1u64..=100000)
        .prop_map(move |(quantity, price_cents)| {
            let unit_price = Decimal::from(price_cents) / Decimal::from(10u64.pow(scale));
            LineItem {
                quantity,
                unit_price,
                currency,
            }
        })
}

// Strategy to generate an invoice with multiple line items
fn invoice_strategy() -> impl Strategy<Value = Invoice> {
    let currency_strategy = prop::sample::select(vec![Currency::IDR, Currency::MYR, Currency::USD]);
    
    currency_strategy.prop_flat_map(|currency| {
        let line_items_strategy = prop::collection::vec(
            line_item_strategy(currency),
            0..=10, // 0 to 10 line items
        );
        
        line_items_strategy.prop_map(move |line_items| Invoice {
            line_items,
            currency,
        })
    })
}

proptest! {
    /// Property: Invoice total equals sum of all line item subtotals
    #[test]
    fn test_invoice_total_equals_sum_of_subtotals(
        invoice in invoice_strategy()
    ) {
        let calculated_total = invoice.calculate_total();
        let expected_total = invoice.line_items
            .iter()
            .map(|item| item.calculate_subtotal())
            .sum::<Decimal>();
        let expected_total = invoice.round_to_currency(expected_total);
        
        prop_assert_eq!(calculated_total, expected_total,
            "Invoice total should equal sum of line item subtotals");
    }
    
    /// Property: Invoice total is non-negative when all line items are non-negative
    #[test]
    fn test_invoice_total_non_negative(
        invoice in invoice_strategy()
    ) {
        let total = invoice.calculate_total();
        
        prop_assert!(total >= Decimal::ZERO,
            "Invoice total should be non-negative, got: {}", total);
    }
    
    /// Property: Invoice total precision matches currency scale
    #[test]
    fn test_invoice_total_precision(
        invoice in invoice_strategy()
    ) {
        let total = invoice.calculate_total();
        let scale = invoice.currency.scale();
        
        // Check that the result has at most 'scale' decimal places
        let total_str = total.to_string();
        if let Some(dot_pos) = total_str.find('.') {
            let decimal_places = total_str.len() - dot_pos - 1;
            prop_assert!(decimal_places <= scale as usize,
                "Invoice total has {} decimal places, expected at most {} for {:?}",
                decimal_places, scale, invoice.currency);
        }
    }
    
    /// Property: Empty invoice has zero total
    #[test]
    fn test_empty_invoice_zero_total(
        currency_idx in 0usize..3
    ) {
        let currencies = [Currency::IDR, Currency::MYR, Currency::USD];
        let currency = currencies[currency_idx];
        
        let invoice = Invoice {
            line_items: vec![],
            currency,
        };
        
        let total = invoice.calculate_total();
        
        prop_assert_eq!(total, Decimal::ZERO,
            "Empty invoice should have zero total");
    }
    
    /// Property: Single line item invoice total equals that item's subtotal
    #[test]
    fn test_single_line_item_total(
        quantity in 1i32..=1000,
        price_cents in 1u64..=100000,
        currency_idx in 0usize..3
    ) {
        let currencies = [Currency::IDR, Currency::MYR, Currency::USD];
        let currency = currencies[currency_idx];
        let scale = currency.scale();
        let unit_price = Decimal::from(price_cents) / Decimal::from(10u64.pow(scale));
        
        let line_item = LineItem {
            quantity,
            unit_price,
            currency,
        };
        
        let invoice = Invoice {
            line_items: vec![line_item.clone()],
            currency,
        };
        
        let invoice_total = invoice.calculate_total();
        let line_item_subtotal = line_item.calculate_subtotal();
        
        prop_assert_eq!(invoice_total, line_item_subtotal,
            "Single line item invoice total should equal line item subtotal");
    }
}

#[cfg(test)]
mod deterministic_tests {
    use super::*;
    
    #[test]
    fn test_invoice_total_multiple_items_idr() {
        let line_items = vec![
            LineItem {
                quantity: 2,
                unit_price: Decimal::from(15000),
                currency: Currency::IDR,
            },
            LineItem {
                quantity: 3,
                unit_price: Decimal::from(25000),
                currency: Currency::IDR,
            },
            LineItem {
                quantity: 1,
                unit_price: Decimal::from(10000),
                currency: Currency::IDR,
            },
        ];
        
        let invoice = Invoice {
            line_items,
            currency: Currency::IDR,
        };
        
        // (2 * 15000) + (3 * 25000) + (1 * 10000) = 30000 + 75000 + 10000 = 115000
        let total = invoice.calculate_total();
        assert_eq!(total, Decimal::from(115000));
    }
    
    #[test]
    fn test_invoice_total_with_rounding_myr() {
        let line_items = vec![
            LineItem {
                quantity: 3,
                unit_price: Decimal::from_str("12.345").unwrap(), // 37.035 -> 37.04
                currency: Currency::MYR,
            },
            LineItem {
                quantity: 2,
                unit_price: Decimal::from_str("25.678").unwrap(), // 51.356 -> 51.36
                currency: Currency::MYR,
            },
        ];
        
        let invoice = Invoice {
            line_items,
            currency: Currency::MYR,
        };
        
        // Line 1: 37.04, Line 2: 51.36, Total: 88.40
        let total = invoice.calculate_total();
        assert_eq!(total, Decimal::from_str("88.40").unwrap());
    }
    
    #[test]
    fn test_invoice_total_with_many_items() {
        // Test with many small items to check accumulation accuracy
        let line_items: Vec<LineItem> = (1..=100)
            .map(|i| LineItem {
                quantity: 1,
                unit_price: Decimal::from_str("0.99").unwrap(),
                currency: Currency::USD,
            })
            .collect();
        
        let invoice = Invoice {
            line_items,
            currency: Currency::USD,
        };
        
        // 100 * 0.99 = 99.00
        let total = invoice.calculate_total();
        assert_eq!(total, Decimal::from_str("99.00").unwrap());
    }
    
    #[test]
    fn test_invoice_commutative_property() {
        // Create invoice with items in one order
        let items1 = vec![
            LineItem {
                quantity: 5,
                unit_price: Decimal::from_str("10.50").unwrap(),
                currency: Currency::USD,
            },
            LineItem {
                quantity: 2,
                unit_price: Decimal::from_str("25.75").unwrap(),
                currency: Currency::USD,
            },
        ];
        
        // Same items in different order
        let items2 = vec![
            LineItem {
                quantity: 2,
                unit_price: Decimal::from_str("25.75").unwrap(),
                currency: Currency::USD,
            },
            LineItem {
                quantity: 5,
                unit_price: Decimal::from_str("10.50").unwrap(),
                currency: Currency::USD,
            },
        ];
        
        let invoice1 = Invoice {
            line_items: items1,
            currency: Currency::USD,
        };
        
        let invoice2 = Invoice {
            line_items: items2,
            currency: Currency::USD,
        };
        
        assert_eq!(invoice1.calculate_total(), invoice2.calculate_total(),
            "Invoice total should be the same regardless of line item order");
    }
}
