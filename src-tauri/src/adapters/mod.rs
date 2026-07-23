use async_trait::async_trait;
use anyhow::Result;
use crate::models::QuotaInfo;

#[async_trait]
pub trait QuotaFetcher: Send + Sync {
    async fn fetch_quota(&self, api_key: &str) -> Result<QuotaInfo>;
}

pub mod deepseek;
pub mod zhipu;
pub mod qoder;
pub mod volcano;
