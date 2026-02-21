use super::{Payment, PaymentProvider};
use anyhow::Result;
use async_trait::async_trait;
use rand::Rng;

pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PaymentProvider for MockProvider {
    fn name(&self) -> &str {
        "Mock"
    }

    async fn fetch_recent_payments(&self, _since: chrono::DateTime<chrono::Utc>) -> Result<Vec<Payment>> {
        let mut rng = rand::thread_rng();
        let amount_units: i64 = rng.gen_range(2..=15);
        let amount_cents = amount_units * 100;

        let payment = Payment {
            id: format!("mock_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
            amount_cents,
            currency: "EUR".to_string(),
            status: "paid".to_string(),
            created_at: chrono::Utc::now(),
            provider: "Mock".to_string(),
        };

        Ok(vec![payment])
    }
}
