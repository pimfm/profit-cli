use super::{Payment, PaymentProvider};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

pub struct MollieProvider {
    api_key: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct MolliePaymentList {
    #[serde(rename = "_embedded")]
    embedded: MollieEmbedded,
}

#[derive(Deserialize)]
struct MollieEmbedded {
    payments: Vec<MolliePayment>,
}

#[derive(Deserialize)]
struct MolliePayment {
    id: String,
    amount: MollieAmount,
    status: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Deserialize)]
struct MollieAmount {
    value: String,
    currency: String,
}

impl MollieProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentProvider for MollieProvider {
    fn name(&self) -> &str {
        "Mollie"
    }

    async fn fetch_recent_payments(&self, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<Payment>> {
        let resp = self.client
            .get("https://api.mollie.com/v2/payments?limit=250&sort=created")
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Mollie API error: {}", resp.status());
        }

        let list: MolliePaymentList = resp.json().await?;
        let mut payments = Vec::new();

        for mp in list.embedded.payments {
            let created = chrono::DateTime::parse_from_rfc3339(&mp.created_at)?
                .with_timezone(&chrono::Utc);

            if created < since {
                continue;
            }

            if mp.status != "paid" {
                continue;
            }

            let amount_f: f64 = mp.amount.value.parse()?;
            let amount_cents = (amount_f * 100.0).round() as i64;

            payments.push(Payment {
                id: mp.id,
                amount_cents,
                currency: mp.amount.currency,
                status: mp.status,
                created_at: created,
                provider: "Mollie".to_string(),
            });
        }

        Ok(payments)
    }
}
