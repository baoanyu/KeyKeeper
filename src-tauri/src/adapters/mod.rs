use async_trait::async_trait;
use anyhow::Result;
use crate::models::Entitlement;

#[async_trait]
pub trait QuotaFetcher: Send + Sync {
    /// 查询该平台的额度包列表（Api 平台通常只有一个）。
    /// 查询失败返回 `Err`，由调度层转为 `PlatformStatus::failed`。
    async fn fetch_entitlements(&self, api_key: &str) -> Result<Vec<Entitlement>>;
}

/// Sanitize an error response body to avoid leaking credentials that some
/// API gateways echo back in error payloads (e.g. Authorization headers).
pub fn sanitize_error_body(body: &str) -> String {
    let truncated: String = body.chars().take(200).collect();
    truncated
        .split_whitespace()
        .map(|w| {
            if w.contains("=sk-")
                || w.contains("sk-")
                || w.starts_with("Bearer")
                || w.contains("Bearer%")
            {
                "***"
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub mod deepseek;
pub mod zhipu;
pub mod volcano;
