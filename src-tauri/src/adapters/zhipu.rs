use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use crate::models::{PlanType, QuotaInfo, QuotaUnit};
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
    async fn fetch_quota(&self, api_key: &str) -> Result<QuotaInfo> {
        let resp = self.client
            .get("https://open.bigmodel.cn/api/paas/v4/balance")
            .bearer_auth(api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            let sanitized = sanitize_error_body(&text);
            return Ok(QuotaInfo::error("ZhipuAI", &format!("HTTP {}: {}", status, sanitized)));
        }

        let json: serde_json::Value = resp.json().await?;

        let remaining = json
            .get("data")
            .and_then(|d| d.get("remaining_tokens"))
            .and_then(|t| t.as_f64());

        match remaining {
            Some(r) => Ok(QuotaInfo {
                provider_name: "ZhipuAI".to_string(),
                plan_type: PlanType::PayAsYouGo,
                quota_unit: QuotaUnit::Tokens,
                // F8: wallet-style API — total unknown
                total: None,
                remaining: r,
                is_success: true,
                error_msg: None,
            }),
            None => Ok(QuotaInfo::error("ZhipuAI", "响应缺少 data.remaining_tokens 字段")),
        }
    }
}
