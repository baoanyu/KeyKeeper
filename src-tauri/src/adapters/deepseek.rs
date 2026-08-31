use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use crate::models::{PlanType, QuotaInfo, QuotaUnit};
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
    async fn fetch_quota(&self, api_key: &str) -> Result<QuotaInfo> {
        let resp = self.client
            .get("https://api.deepseek.com/user/balance")
            .bearer_auth(api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let sanitized = sanitize_error_body(&text);
            return Ok(QuotaInfo::error("DeepSeek", &format!("HTTP {}: {}", status, sanitized)));
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
            Some(b) => Ok(QuotaInfo {
                provider_name: "DeepSeek".to_string(),
                plan_type: PlanType::PayAsYouGo,
                quota_unit: QuotaUnit::CNY,
                // F8: wallet-style API — total unknown, don't fabricate total == remaining
                total: None,
                remaining: b,
                is_success: true,
                error_msg: None,
            }),
            None => Ok(QuotaInfo::error("DeepSeek", "响应缺少 data.balance 字段")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = QuotaInfo::error("DeepSeek", "test error");
        assert_eq!(error.provider_name, "DeepSeek");
        assert!(!error.is_success);
        assert_eq!(error.error_msg, Some("test error".to_string()));
    }
}
