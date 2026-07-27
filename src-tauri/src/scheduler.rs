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

    let provider_names: Vec<String> = tasks
        .iter()
        .map(|(name, _, _)| name.clone())
        .collect();

    for (_idx, (_provider_name, api_key, fetcher)) in tasks.into_iter().enumerate() {
        let sem = semaphore.clone();
        let provider_name = provider_names[_idx].clone();
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
        .enumerate()
        .map(|(i, r)| match r {
            Ok(quota) => quota,
            Err(join_err) => QuotaInfo::error(
                &provider_names[i],
                &format!("Internal panic: {}", join_err),
            ),
        })
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
