use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use crate::models::{PlanType, QuotaInfo, QuotaUnit};
use super::QuotaFetcher;

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
            return Ok(QuotaInfo::error("ZhipuAI", &format!("HTTP {}: {}", status, text)));
        }

        let json: serde_json::Value = resp.json().await?;
        
        let remaining = json
            .get("data")
            .and_then(|d| d.get("remaining_tokens"))
            .and_then(|t| t.as_f64())
            .unwrap_or(0.0);

        Ok(QuotaInfo {
            provider_name: "ZhipuAI".to_string(),
            plan_type: PlanType::PayAsYouGo,
            quota_unit: QuotaUnit::Tokens,
            total: remaining,
            remaining,
            is_success: true,
            error_msg: None,
        })
    }
}
