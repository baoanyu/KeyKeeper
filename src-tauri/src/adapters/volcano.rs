use async_trait::async_trait;
use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::models::{Entitlement, QuotaUnit};
use super::{sanitize_error_body, QuotaFetcher};

type HmacSha256 = Hmac<Sha256>;

const EMPTY_PAYLOAD_SHA256: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

// ── QueryBalanceAcct 目标常量 ──
const BILLING_HOST: &str = "open.volcengineapi.com";
/// R-3：region 按权威签名 Demo 向量口径取 cn-beijing；
/// 官方另一份示例出现 cn-north-1，真实取值待 Q3（API Explorer 签名 curl）终验，
/// 若不符仅改此常量。
const BILLING_REGION: &str = "cn-beijing";
const BILLING_SERVICE: &str = "billing";

pub struct VolcanoFetcher {
    client: Arc<Client>,
}

impl VolcanoFetcher {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

// ═══════════════════════════════════════════════════════════════
// 通用 V4 签名器（R-3a）
//
// 结构严格对齐官方《签名过程 Demo》：签名密钥派生无 "VOLC" 前缀，
// CanonicalURI 为 "/"，动作经 Action/Version 查询参数指定并参与签名。
// ═══════════════════════════════════════════════════════════════

struct SigningRequest<'a> {
    method: &'a str,
    uri: &'a str,
    query: &'a BTreeMap<String, String>,
    host: &'a str,
    region: &'a str,
    service: &'a str,
    access_key: &'a str,
    secret_key: &'a str,
    x_date: &'a str,
}

fn sign(req: &SigningRequest) -> String {
    let date_stamp = &req.x_date[..8];

    let canonical_query = encode_query(req.query);
    let canonical_headers = format!("host:{}\nx-date:{}\n", req.host, req.x_date);
    let signed_headers = "host;x-date";

    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        req.method,
        req.uri,
        canonical_query,
        canonical_headers,
        signed_headers,
        EMPTY_PAYLOAD_SHA256
    );

    let credential_scope = format!("{}/{}/{}/request", date_stamp, req.region, req.service);
    let string_to_sign = format!(
        "HMAC-SHA256\n{}\n{}\n{}",
        req.x_date,
        credential_scope,
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );

    let signing_key = signature_key(req.secret_key, date_stamp, req.region, req.service);
    let mut mac = HmacSha256::new_from_slice(&signing_key).expect("HMAC accepts any key size");
    mac.update(string_to_sign.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());

    format!(
        "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        req.access_key, credential_scope, signed_headers, signature
    )
}

/// 签名密钥派生（无前缀）：
/// kDate = HMAC(SK, date) → kRegion → kService → kSigning = HMAC(.., "request")
fn signature_key(secret: &str, date: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac(secret.as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, service.as_bytes());
    hmac(&k_service, b"request")
}

fn hmac(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}

/// query 按 RFC3986 编码；BTreeMap 保证按 key ASCII 排序
fn encode_query(query: &BTreeMap<String, String>) -> String {
    query
        .iter()
        .map(|(k, v)| format!("{}={}", rfc3986_encode(k), rfc3986_encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn rfc3986_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

// ═══════════════════════════════════════════════════════════════
// QueryBalanceAcct 响应解析（R-3b）
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct BalanceApiResponse {
    #[serde(rename = "ResponseMetadata")]
    response_metadata: Option<ResponseMetadata>,
    #[serde(rename = "Result")]
    result: Option<BalanceResult>,
}

#[derive(Debug, Deserialize)]
struct ResponseMetadata {
    error: Option<ApiError>,
}

#[derive(Debug, Deserialize)]
struct ApiError {
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BalanceResult {
    #[serde(rename = "AvailableBalance")]
    available_balance: String,
    #[serde(rename = "CashBalance")]
    cash_balance: String,
    #[serde(rename = "FreezeAmount")]
    freeze_amount: String,
    #[serde(rename = "ArrearsBalance")]
    arrears_balance: String,
}

pub(crate) fn parse_balance(json: serde_json::Value) -> Result<Vec<Entitlement>> {
    let resp: BalanceApiResponse = serde_json::from_value(json)
        .map_err(|e| anyhow!("响应结构解析失败: {e}"))?;

    if let Some(err) = resp.response_metadata.and_then(|m| m.error) {
        return Err(anyhow!(
            "{}: {}",
            err.code.unwrap_or_default(),
            err.message.unwrap_or_default()
        ));
    }

    let result = resp.result.ok_or_else(|| anyhow!("响应缺少 Result"))?;

    let available = result
        .available_balance
        .parse::<f64>()
        .map_err(|e| anyhow!("AvailableBalance 解析失败（{}）: {e}", result.available_balance))?;

    let note = format!(
        "现金 {} / 冻结 {} / 欠费 {}",
        result.cash_balance, result.freeze_amount, result.arrears_balance
    );

    Ok(vec![Entitlement::new("账户余额")
        .with_balance(QuotaUnit::CNY, None, available)
        .with_note(&note)])
}

#[async_trait]
impl QuotaFetcher for VolcanoFetcher {
    async fn fetch_entitlements(&self, api_key: &str) -> Result<Vec<Entitlement>> {
        // Key 形态：AccessKey:SecretKey（splitn(2)，SecretKey 中允许出现 ':'）
        let mut parts = api_key.splitn(2, ':');
        let access_key = parts.next().unwrap_or("");
        let secret_key = parts.next().unwrap_or("");
        if access_key.is_empty() || secret_key.is_empty() {
            return Err(anyhow!("API Key 格式错误，应为 AccessKey:SecretKey"));
        }

        let x_date = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let mut query = BTreeMap::new();
        query.insert("Action".to_string(), "QueryBalanceAcct".to_string());
        query.insert("Version".to_string(), "2022-01-01".to_string());

        let signing = SigningRequest {
            method: "GET",
            uri: "/",
            query: &query,
            host: BILLING_HOST,
            region: BILLING_REGION,
            service: BILLING_SERVICE,
            access_key,
            secret_key,
            x_date: &x_date,
        };
        let authorization = sign(&signing);

        // URL 用签名器同一套编码构造，避免传输层与签名串不一致
        let url = format!("https://{}/?{}", BILLING_HOST, encode_query(&query));
        let resp = self
            .client
            .get(url)
            .header("X-Date", &x_date)
            .header("Authorization", authorization)
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
    use super::*;

    // 官方《签名过程 Demo》向量（67270）
    const DEMO_AK: &str = "AKLTZWU0MDFkMjA4Yzk3ZGI3N2Q0NjM1MWMxM2IyOTU0OWM0ZGU";
    const DEMO_SK: &str = "WkRZeE1EQmxPVGhsWWpWak5HVmtNbUUxTXpZeU9UVXlOMlE1TmpZeVlqTQ==";
    const DEMO_X_DATE: &str = "20240619T071306Z";

    /// 签名必须逐字节命中官方 Demo 期望签名；此测试无需真实凭证。
    #[test]
    fn sign_matches_official_demo_vector() {
        let mut query = BTreeMap::new();
        query.insert("Action".to_string(), "ListUsers".to_string());
        query.insert("Limit".to_string(), "10".to_string());
        query.insert("Offset".to_string(), "0".to_string());
        query.insert("Version".to_string(), "2018-01-01".to_string());

        let req = SigningRequest {
            method: "GET",
            uri: "/",
            query: &query,
            host: "iam.volcengineapi.com",
            region: "cn-beijing",
            service: "iam",
            access_key: DEMO_AK,
            secret_key: DEMO_SK,
            x_date: DEMO_X_DATE,
        };
        let auth = sign(&req);
        assert!(
            auth.contains(
                "Signature=e31c4558bcfe08a286001f59cedbf0791ffd0b2362f10e55ee2627467bcdde93"
            ),
            "签名与官方 Demo 期望不符: {auth}"
        );
    }

    #[test]
    fn parses_query_balance_acct() {
        let fixture = include_str!("../../tests/fixtures/volcano_balance.json");
        let json = serde_json::from_str(fixture).unwrap();
        let entitlements = parse_balance(json).unwrap();
        assert_eq!(entitlements.len(), 1);
        assert_eq!(entitlements[0].label, "账户余额");
        assert_eq!(entitlements[0].remaining, Some(77.01));
    }

    #[test]
    fn rfc3986_encodes_special_chars() {
        assert_eq!(rfc3986_encode("a b"), "a%20b");
        assert_eq!(rfc3986_encode("a-b_c.d~e"), "a-b_c.d~e");
    }
}
