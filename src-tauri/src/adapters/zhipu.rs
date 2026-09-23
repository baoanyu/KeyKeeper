use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
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

// R-2：Coding Plan 配额窗口接口 strong-typed 结构。
// 契约来源：可运行第三方实现 dsh-usage-glm-cn（lib/logic.js），【实证】级。
#[derive(Debug, Deserialize)]
struct QuotaLimitResponse {
    code: i64,
    msg: Option<String>,
    data: Option<QuotaLimitData>,
}

#[derive(Debug, Deserialize)]
struct QuotaLimitData {
    limits: Vec<QuotaLimit>,
}

#[derive(Debug, Deserialize)]
struct QuotaLimit {
    #[serde(rename = "type")]
    limit_type: String,
    unit: i64,
    percentage: f64,
    #[serde(rename = "nextResetTime")]
    next_reset_time: String,
}

/// R-2：解析配额窗口响应。
///
/// 注意该端点鉴权失败也返回 HTTP 200（体内 code=401/1001），必须以体 code 判定。
pub(crate) fn parse_quota_limit(json: serde_json::Value) -> Result<Vec<Entitlement>> {
    let resp: QuotaLimitResponse = serde_json::from_value(json)
        .map_err(|e| anyhow!("响应结构解析失败: {e}"))?;

    if resp.code != 200 {
        return Err(anyhow!(
            "接口返回 code {}: {}",
            resp.code,
            resp.msg.unwrap_or_default()
        ));
    }

    let data = resp.data.ok_or_else(|| anyhow!("响应缺少 data"))?;
    if data.limits.is_empty() {
        return Err(anyhow!("响应 limits 为空"));
    }

    data.limits
        .iter()
        .map(|limit| {
            let label = match limit.unit {
                3 => "5小时滚动",
                6 => "周额度",
                5 => "MCP月度",
                other => return Err(anyhow!("未知配额窗口 unit={other}（需扩展映射）")),
            };

            let reset_ts = parse_rfc3339(&limit.next_reset_time)?;
            let unit = if limit.limit_type == "TIME_LIMIT" {
                QuotaUnit::Unknown
            } else {
                QuotaUnit::Tokens
            };

            let mut entitlement = Entitlement::new(label)
                .with_used_percent(limit.percentage)
                .with_expires(reset_ts);
            entitlement.unit = unit;
            Ok(entitlement)
        })
        .collect()
}

/// RFC3339（如 `2026-09-23T18:00:00Z`）→ Unix 秒
fn parse_rfc3339(s: &str) -> Result<i64> {
    let dt = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|e| anyhow!("重置时间解析失败（{s}）: {e}"))?;
    Ok(dt.timestamp())
}

#[async_trait]
impl QuotaFetcher for ZhipuFetcher {
    async fn fetch_entitlements(&self, api_key: &str) -> Result<Vec<Entitlement>> {
        let resp = self
            .client
            .get("https://open.bigmodel.cn/api/monitor/usage/quota/limit")
            .bearer_auth(api_key)
            .send()
            .await?;

        let status = resp.status();
        let json: serde_json::Value = if status.is_success() {
            resp.json().await?
        } else {
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow!("HTTP {status}: {}", sanitize_error_body(&text)));
        };

        parse_quota_limit(json)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_quota_limit;
    use crate::models::QuotaUnit;

    const FIXTURE: &str = include_str!("../../tests/fixtures/zhipu_quota_limit.json");

    #[test]
    fn parses_quota_windows() {
        let json = serde_json::from_str(FIXTURE).unwrap();
        let entitlements = parse_quota_limit(json).expect("实证示例应解析成功");
        assert_eq!(entitlements.len(), 3);

        let rolling = &entitlements[0];
        assert_eq!(rolling.label, "5小时滚动");
        assert_eq!(rolling.used_percent, Some(42.0));
        assert_eq!(rolling.unit, QuotaUnit::Tokens);
        assert!(rolling.expires_at.is_some());

        let weekly = &entitlements[1];
        assert_eq!(weekly.label, "周额度");
        assert_eq!(weekly.used_percent, Some(15.0));

        let mcp = &entitlements[2];
        assert_eq!(mcp.label, "MCP月度");
        assert_eq!(mcp.unit, QuotaUnit::Unknown);
    }

    #[test]
    fn treats_http200_body_error_as_failure() {
        let json = serde_json::json!({"code": 401, "msg": "令牌已过期或验证不正确", "success": false});
        assert!(parse_quota_limit(json).is_err());
    }

    #[test]
    fn rejects_legacy_remaining_tokens_shape() {
        let json = serde_json::json!({"data": {"remaining_tokens": 123}});
        assert!(parse_quota_limit(json).is_err());
    }
}
