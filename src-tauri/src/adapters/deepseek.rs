use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;
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

// R-1：官方 GET /user/balance 响应 strong-typed 结构。
// 契约来源：DeepSeek 官方 API 文档《Get User Balance》。
#[derive(Debug, Deserialize)]
struct BalanceResponse {
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

/// R-1：解析官方 /user/balance 响应。
///
/// 真实结构为顶层 `balance_infos[]`（无 `data` 包裹层），金额为字符串。
/// 字段缺失 / 金额非法 / 账户不可用均返回 Err，不静默取 0。
pub(crate) fn parse_balance(json: serde_json::Value) -> Result<Vec<Entitlement>> {
    let resp: BalanceResponse = serde_json::from_value(json)
        .map_err(|e| anyhow!("响应结构解析失败: {e}"))?;

    if !resp.is_available {
        return Err(anyhow!("账户不可用（is_available=false）"));
    }
    if resp.balance_infos.is_empty() {
        return Err(anyhow!("响应 balance_infos 为空"));
    }

    resp.balance_infos
        .iter()
        .map(|info| {
            let amount = info
                .total_balance
                .parse::<f64>()
                .map_err(|e| anyhow!("total_balance 解析失败（{}）: {e}", info.total_balance))?;

            let unit = match info.currency.to_ascii_uppercase().as_str() {
                "CNY" => QuotaUnit::CNY,
                other => return Err(anyhow!("未知币种: {other}（需扩展 QuotaUnit）")),
            };

            let note = format!(
                "{} 赠金 {} / 充值 {}",
                info.currency, info.granted_balance, info.topped_up_balance
            );

            Ok(Entitlement::new("余额")
                .with_balance(unit, None, amount)
                .with_note(&note))
        })
        .collect()
}

#[async_trait]
impl QuotaFetcher for DeepSeekFetcher {
    async fn fetch_entitlements(&self, api_key: &str) -> Result<Vec<Entitlement>> {
        let resp = self
            .client
            .get("https://api.deepseek.com/user/balance")
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

        parse_balance(json)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_balance;
    use crate::models::QuotaUnit;

    const FIXTURE: &str = include_str!("../../tests/fixtures/deepseek_balance.json");

    #[test]
    fn parses_official_balance_shape() {
        let json = serde_json::from_str(FIXTURE).unwrap();
        let entitlements = parse_balance(json).expect("官方示例应解析成功");
        assert_eq!(entitlements.len(), 1);
        let e = &entitlements[0];
        assert_eq!(e.label, "余额");
        assert_eq!(e.unit, QuotaUnit::CNY);
        assert_eq!(e.remaining, Some(110.00));
        assert!(e.note.as_deref().unwrap().contains("赠金 10.00"));
        assert!(e.note.as_deref().unwrap().contains("充值 100.00"));
    }

    #[test]
    fn rejects_unavailable_account() {
        let json = serde_json::json!({"is_available": false, "balance_infos": []});
        assert!(parse_balance(json).is_err());
    }

    #[test]
    fn rejects_legacy_data_wrapper_shape() {
        // 旧实现假设的 data.balance 结构必须解析失败，防止回退到错误契约
        let json = serde_json::json!({"data": {"balance": "10.00"}});
        assert!(parse_balance(json).is_err());
    }
}
