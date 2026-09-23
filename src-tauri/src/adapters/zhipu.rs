use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::sync::Arc;
use crate::models::{Entitlement, QuotaUnit};
use super::{sanitize_error_body, QuotaFetcher};

pub struct ZhipuFetcher {
    client: Arc<Client>,
}

impl ZhipuFetcher {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl QuotaFetcher for ZhipuFetcher {
    async fn fetch_entitlements(&self, api_key: &str) -> Result<Vec<Entitlement>> {
        let resp = self.client
            .get("https://open.bigmodel.cn/api/paas/v4/balance")
            .bearer_auth(api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let sanitized = sanitize_error_body(&text);
            return Err(anyhow!("HTTP {}: {}", status, sanitized));
        }

        let json: serde_json::Value = resp.json().await?;

        let remaining = json
            .get("data")
            .and_then(|d| d.get("remaining_tokens"))
            .and_then(|t| t.as_f64());

        match remaining {
            // F8: wallet-style API — total unknown
            Some(r) => Ok(vec![Entitlement::new("余额")
                .with_balance(QuotaUnit::Tokens, None, r)]),
            None => Err(anyhow!("响应缺少 data.remaining_tokens 字段")),
        }
    }
}
