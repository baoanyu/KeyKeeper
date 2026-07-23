use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use futures::future::join_all;
use crate::adapters::QuotaFetcher;
use crate::models::QuotaInfo;

const MAX_CONCURRENCY: usize = 4; // Match number of adapters
const REQUEST_TIMEOUT_SECS: u64 = 10;

pub async fn fetch_all_quotas(
    tasks: Vec<(String, String, Box<dyn QuotaFetcher>)>,
) -> Vec<QuotaInfo> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut handles = Vec::new();

    for (provider_name, api_key, fetcher) in tasks {
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = timeout(
                Duration::from_secs(REQUEST_TIMEOUT_SECS),
                fetcher.fetch_quota(&api_key),
            )
            .await;

            match result {
                Ok(Ok(quota)) => quota,
                Ok(Err(e)) => QuotaInfo::error(&provider_name, &e.to_string()),
                Err(_) => QuotaInfo::error(&provider_name, "Request timed out"),
            }
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;
    results
        .into_iter()
        .filter_map(|r| r.ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_all_quotas_empty() {
        let tasks: Vec<(String, String, Box<dyn QuotaFetcher>)> = Vec::new();
        let results = fetch_all_quotas(tasks).await;
        assert!(results.is_empty());
    }
}
