pub mod adyen;
pub mod mock;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub amount_cents: i64,
    pub currency: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub provider: String,
}

#[async_trait]
pub trait PaymentProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn fetch_recent_payments(&self, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<Payment>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PspConfig {
    pub provider: String,
    pub api_key: String,
}
