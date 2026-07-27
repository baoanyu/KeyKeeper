use async_trait::async_trait;
use anyhow::Result;
use reqwest::Client;
use std::sync::Arc;
use crate::models::{PlanType, QuotaInfo, QuotaUnit};
use super::{sanitize_error_body, QuotaFetcher};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};

type HmacSha256 = Hmac<Sha256>;

pub struct VolcanoFetcher {
    client: Arc<Client>,
}

impl VolcanoFetcher {
    pub fn new(client: Arc<Client>) -> Self {
        Self { client }
    }
}

const VOLCANO_HOST: &str = "open.volcengineapi.com";
const VOLCANO_REGION: &str = "cn-north-1";
const VOLCANO_SERVICE: &str = "ark";

#[async_trait]
impl QuotaFetcher for VolcanoFetcher {
    async fn fetch_quota(&self, api_key: &str) -> Result<QuotaInfo> {
        let now = chrono::Utc::now();
        let x_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = now.format("%Y%m%d").to_string();
        
        // Parse API key: format is "AccessKey:SecretKey"
        let parts: Vec<&str> = api_key.split(':').collect();
        if parts.len() != 2 {
            return Ok(QuotaInfo::error("Volcano", "API Key 格式错误，应为 AccessKey:SecretKey"));
        }
        let access_key = parts[0];
        let secret_key = parts[1];
        
        // Build canonical request
        let canonical_uri = "/api/v3/quota/balance";
        let canonical_querystring = "";
        let canonical_headers = format!("host:{}\nx-date:{}\n", VOLCANO_HOST, x_date);
        let signed_headers = "host;x-date";
        let payload_hash = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        
        let canonical_request = format!(
            "GET\n{}\n{}\n{}\n{}\n{}",
            canonical_uri, canonical_querystring, canonical_headers, signed_headers, payload_hash
        );
        
        // Build string to sign
        let credential_scope = format!("{}/{}/{}/request", date_stamp, VOLCANO_REGION, VOLCANO_SERVICE);
        let string_to_sign = format!(
            "HMAC-SHA256\n{}\n{}\n{}",
            x_date,
            credential_scope,
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );
        
        // Calculate signature
        let signing_key = get_signature_key(secret_key, &date_stamp, VOLCANO_REGION, VOLCANO_SERVICE);
        let mut mac = HmacSha256::new_from_slice(&signing_key)
            .map_err(|e| anyhow::anyhow!("HMAC error: {}", e))?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());
        
        let authorization = format!(
            "HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            access_key, credential_scope, signed_headers, signature
        );
        
        let resp = self.client
            .get(format!("https://{}{}", VOLCANO_HOST, canonical_uri))
            .header("Host", VOLCANO_HOST)
            .header("X-Date", &x_date)
            .header("Authorization", &authorization)
            .send()
            .await;

        match resp {
            Ok(resp) if resp.status().is_success() => {
                let json: serde_json::Value = resp.json().await?;
                let remaining = json
                    .get("data")
                    .and_then(|d| d.get("remaining"))
                    .and_then(|r| r.as_f64())
                    .unwrap_or(0.0);

                Ok(QuotaInfo {
                    provider_name: "Volcano".to_string(),
                    plan_type: PlanType::PayAsYouGo,
                    quota_unit: QuotaUnit::CNY,
                    total: remaining,
                    remaining,
                    is_success: true,
                    error_msg: None,
                })
            }
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.unwrap_or_default();
                let sanitized = sanitize_error_body(&text);
                Ok(QuotaInfo::error("Volcano", &format!("HTTP {}: {}", status, sanitized)))
            }
            Err(e) => Ok(QuotaInfo::error("Volcano", &e.to_string())),
        }
    }
}

fn get_signature_key(key: &str, date_stamp: &str, region: &str, service: &str) -> Vec<u8> {
    let k_date = hmac_sha256(format!("VOLC{}", key).as_bytes(), date_stamp.as_bytes());
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    hmac_sha256(&k_service, b"request")
}

fn hmac_sha256(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(msg);
    mac.finalize().into_bytes().to_vec()
}
