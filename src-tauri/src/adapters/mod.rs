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
///
/// R-13 修订：旧实现按空白分词后只替换 "Bearer" 一词，**其后的凭证原样保留**，
/// 且不识别智谱格式 Key（32hex.secret）。此处重写为：
/// 1. "Bearer/Authorization" 之后的那个词按凭证抹掉；
/// 2. 智谱 Key 形态、`sk-`、`Bearer%20` 等形态整体抹掉。
pub fn sanitize_error_body(body: &str) -> String {
    let mut words: Vec<String> = Vec::new();
    let mut expect_credential = false;

    for w in body.split_whitespace() {
        // 上一个词是 Bearer/Authorization，本词应为凭证
        if expect_credential {
            if bare_word(w) == "Bearer" {
                // 形如 "Authorization: Bearer xxx"，继续等待真正的凭证
                words.push("***".to_string());
                continue;
            }
            words.push("***".to_string());
            expect_credential = false;
            continue;
        }

        // 与凭证词粘连的形态："Bearer=xxx" / "Bearer:xxx" / URL 编码的 Bearer%20
        if w.starts_with("Bearer=")
            || w.starts_with("Bearer:")
            || w.contains("Bearer%")
        {
            words.push("***".to_string());
            continue;
        }

        if bare_word(w) == "Bearer" || bare_word(w).eq_ignore_ascii_case("Authorization") {
            words.push("***".to_string());
            expect_credential = true;
            continue;
        }

        if w.contains("sk-") || is_zhipu_key(w) {
            words.push("***".to_string());
        } else {
            words.push(w.to_string());
        }
    }

    words.join(" ").chars().take(200).collect()
}

/// 去掉词尾常见标点，便于匹配 "Bearer:" / "Authorization:" 等
fn bare_word(w: &str) -> &str {
    w.trim_end_matches([':', ',', ';'])
}

/// 智谱 Key 形态：32 位 hex + '.' + 非空 base62 段
fn is_zhipu_key(w: &str) -> bool {
    let Some((id, secret)) = w.split_once('.') else {
        return false;
    };
    id.len() == 32
        && id.bytes().all(|b| b.is_ascii_hexdigit())
        && !secret.is_empty()
        && secret.bytes().all(|b| b.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::sanitize_error_body;

    #[test]
    fn redacts_token_after_bearer() {
        let out = sanitize_error_body("Unauthorized: Bearer sk-deadbeef1234567890abcdef request id 1");
        assert!(!out.contains("sk-deadbeef"));
        assert!(out.contains("***"));
    }

    #[test]
    fn redacts_authorization_header_form() {
        let out = sanitize_error_body("Authorization: Bearer abcdef0123456789abcdef0123456789.abcdEFG");
        assert!(!out.contains("abcdef0123456789abcdef0123456789"));
        assert!(!out.contains("abcdEFG"));
    }

    #[test]
    fn keeps_plain_text() {
        let out = sanitize_error_body("rate limit exceeded, retry later");
        assert_eq!(out, "rate limit exceeded, retry later");
    }

    #[test]
    fn redacts_url_encoded_bearer() {
        let out = sanitize_error_body("got Authorization%3A%20Bearer%20sk-leaked token");
        assert!(!out.contains("sk-leaked"));
    }
}

pub mod deepseek;
pub mod zhipu;
pub mod volcano;
