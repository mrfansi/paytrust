use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tax {
    pub id: Option<i64>,
    pub rate: Decimal,
    pub category: Option<String>,
    pub description: Option<String>,
}

impl Tax {
    pub fn new(rate: Decimal, category: Option<String>) -> Self {
        Self {
            id: None,
            rate,
            category,
            description: None,
        }
    }
}
