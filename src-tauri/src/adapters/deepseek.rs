use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use std::sync::Arc;
use crate::models::{Entitlement, QuotaUnit};
use super::{sanitize_error_body, QuotaFetcher};

pub struct DeepSeekFetcher {
    client: Arc<Client>,
}

impl DeepSeekFetcher {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl QuotaFetcher for DeepSeekFetcher {
    async fn fetch_entitlements(&self, api_key: &str) -> Result<Vec<Entitlement>> {
        let resp = self.client
            .get("https://api.deepseek.com/user/balance")
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

        // F2: accept both numeric and string balance; return error on missing field
        let balance = json
            .get("data")
            .and_then(|d| d.get("balance"))
            .and_then(|b| {
                b.as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| b.as_f64())
            });

        match balance {
            // F8: wallet-style API — total unknown, don't fabricate total == remaining
            // ⚠️ data.balance 字段真实性待验证（见 refactor-plan-v2.md §3.2，Phase 3 处理）
            Some(b) => Ok(vec![Entitlement::new("余额")
                .with_balance(QuotaUnit::CNY, None, b)]),
            None => Err(anyhow!("响应缺少 data.balance 字段")),
        }
    }
}
