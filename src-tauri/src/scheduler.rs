use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};
use futures::future::join_all;
use crate::adapters::QuotaFetcher;
use crate::models::{non_empty, now_ts, PlatformSpec, PlatformStatus, Source};

const MAX_CONCURRENCY: usize = 4; // Match number of adapters
const REQUEST_TIMEOUT_SECS: u64 = 10;

/// 一个并发抓取任务：平台元数据 + API Key + 对应适配器
pub struct FetchTask {
    pub spec: &'static PlatformSpec,
    pub api_key: String,
    pub fetcher: Box<dyn QuotaFetcher>,
}

pub async fn fetch_all_platforms(tasks: Vec<FetchTask>) -> Vec<PlatformStatus> {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENCY));
    let mut handles = Vec::new();
    let mut specs: Vec<&'static PlatformSpec> = Vec::new();

    for task in tasks {
        let FetchTask { spec, api_key, fetcher } = task;
        specs.push(spec);
        let sem = semaphore.clone();
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            let result = timeout(
                Duration::from_secs(REQUEST_TIMEOUT_SECS),
                fetcher.fetch_entitlements(&api_key),
            )
            .await;

            match result {
                Ok(Ok(entitlements)) => PlatformStatus {
                    id: spec.id.to_string(),
                    display_name: spec.display_name.to_string(),
                    source: Source::Api,
                    entitlements,
                    console_url: non_empty(spec.console_url),
                    error: None,
                    updated_at: now_ts(),
                },
                Ok(Err(e)) => PlatformStatus::failed(spec, &e.to_string()),
                Err(_) => PlatformStatus::failed(spec, "Request timed out"),
            }
        });
        handles.push(handle);
    }

    let results = join_all(handles).await;
    results
        .into_iter()
        .enumerate()
        .map(|(i, r)| match r {
            Ok(status) => status,
            Err(join_err) => PlatformStatus::failed(
                specs[i],
                &format!("Internal panic: {}", join_err),
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_all_platforms_empty() {
        let results = fetch_all_platforms(Vec::new()).await;
        assert!(results.is_empty());
    }
}
