use super::{Payment, PaymentProvider};
use anyhow::Result;
use async_trait::async_trait;
use serde::Deserialize;

pub struct AdyenProvider {
    api_key: String,
    merchant_account: String,
    client: reqwest::Client,
}

#[derive(Deserialize)]
struct AdyenPaymentList {
    #[serde(default)]
    data: Vec<AdyenPayment>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdyenPayment {
    psp_reference: String,
    amount: AdyenAmount,
    status: String,
    creation_date: String,
}

#[derive(Deserialize)]
struct AdyenAmount {
    value: i64,
    currency: String,
}

impl AdyenProvider {
    pub fn new(api_key: String, merchant_account: String) -> Self {
        Self {
            api_key,
            merchant_account,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl PaymentProvider for AdyenProvider {
    fn name(&self) -> &str {
        "Adyen"
    }

    async fn fetch_recent_payments(&self, since: chrono::DateTime<chrono::Utc>) -> Result<Vec<Payment>> {
        let body = serde_json::json!({
            "merchantAccountCode": self.merchant_account,
            "createdSince": since.to_rfc3339(),
            "createdUntil": chrono::Utc::now().to_rfc3339(),
            "status": "Authorised",
            "limit": 100,
        });

        let resp = self.client
            .post("https://management-test.adyen.com/v3/payments")
            .header("X-API-Key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("Adyen API error: {}", resp.status());
        }

        let list: AdyenPaymentList = resp.json().await?;
        let mut payments = Vec::new();

        for ap in list.data {
            let created = chrono::DateTime::parse_from_rfc3339(&ap.creation_date)
                .unwrap_or_else(|_| chrono::Utc::now().into())
                .with_timezone(&chrono::Utc);

            if created < since {
                continue;
            }

            payments.push(Payment {
                id: ap.psp_reference,
                amount_cents: ap.amount.value,
                currency: ap.amount.currency,
                status: ap.status,
                created_at: created,
                provider: "Adyen".to_string(),
            });
        }

        Ok(payments)
    }
}
