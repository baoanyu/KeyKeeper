use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    PayAsYouGo,
    CodingPlan,
    Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaUnit {
    CNY,
    Tokens,
    Seconds,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub provider_name: String,
    pub plan_type: PlanType,
    pub quota_unit: QuotaUnit,
    pub total: f64,
    pub remaining: f64,
    pub is_success: bool,
    pub error_msg: Option<String>,
}

impl QuotaInfo {
    pub fn error(provider_name: &str, error_msg: &str) -> Self {
        Self {
            provider_name: provider_name.to_string(),
            plan_type: PlanType::PayAsYouGo,
            quota_unit: QuotaUnit::Unknown,
            total: 0.0,
            remaining: 0.0,
            is_success: false,
            error_msg: Some(error_msg.to_string()),
        }
    }
}
