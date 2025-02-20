use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PriceFeedData {
    pub symbol: String,
    pub timestamp: String,
    pub price: f64,
}