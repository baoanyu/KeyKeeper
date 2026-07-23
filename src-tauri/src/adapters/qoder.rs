use async_trait::async_trait;
use anyhow::Result;
use crate::models::{PlanType, QuotaInfo, QuotaUnit};
use super::QuotaFetcher;

const CODING_PLAN_DURATION_SECS: f64 = 5.0 * 3600.0; // 5 hours

pub struct QoderFetcher {
    pub first_launch_time: Option<f64>,
}

impl QoderFetcher {
    pub fn new(first_launch_time: Option<f64>) -> Self {
        Self { first_launch_time }
    }
}

#[async_trait]
impl QuotaFetcher for QoderFetcher {
    async fn fetch_quota(&self, _api_key: &str) -> Result<QuotaInfo> {
        // Use persisted first launch time for consistent countdown
        let first_launch = self.first_launch_time.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64()
        });
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        
        let elapsed = now - first_launch;
        let remaining = (CODING_PLAN_DURATION_SECS - elapsed).max(0.0);

        Ok(QuotaInfo {
            provider_name: "Qoder".to_string(),
            plan_type: PlanType::CodingPlan,
            quota_unit: QuotaUnit::Seconds,
            total: CODING_PLAN_DURATION_SECS,
            remaining,
            is_success: true,
            error_msg: Some("本地估算（无公开余额接口）".to_string()),
        })
    }
}
