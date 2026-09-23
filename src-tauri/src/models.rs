use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════
// 数据模型 v2
//
// 定位变更：本应用从「余额查询器」改为「额度与订阅管理台」。
// 用户真正要知道的是「还剩几天到期」，`remaining: f64` 表达不了。
// ═══════════════════════════════════════════════════════════════

/// 额度单位
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QuotaUnit {
    CNY,
    Tokens,
    Seconds,
    Unknown,
}

/// 数据来源
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// 通过 API 自动查询
    Api,
    /// 用户手动录入到期日
    Manual,
}

/// 一个额度包 —— 对应手写记录里的一行。
///
/// 之所以是 Vec：同一个平台可以挂着多个独立到期日，
/// 例如「超算 DeepSeek」下同时有 `0.1` 和 `10M 体验` 两个包。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entitlement {
    pub label: String,
    /// 到期时间戳（语义为本地时区当天 23:59:59，到期当天仍可用）
    pub expires_at: Option<i64>,
    pub unit: QuotaUnit,
    pub total: Option<f64>,
    pub remaining: Option<f64>,
    /// 备注 / 估算说明
    pub note: Option<String>,
}

/// 一个平台的完整状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStatus {
    /// 稳定标识，对应 PlatformSpec.id
    pub id: String,
    pub display_name: String,
    pub source: Source,
    pub entitlements: Vec<Entitlement>,
    pub console_url: Option<String>,
    /// 平台级错误（查询失败 / Key 失效等）。
    ///
    /// 注意：这是对抗性审查 F-5 精简模型时漏掉的字段 —— 当时把 message
    /// 并入了 Entitlement.note，但 entitlements 为空时（查询失败）错误
    /// 信息就无处安放。实施阶段发现，现补回。
    pub error: Option<String>,
    pub updated_at: i64,
}

impl PlatformStatus {
    /// 构造一个"查询失败"的平台状态
    pub fn failed(spec: &PlatformSpec, error: &str) -> Self {
        Self {
            id: spec.id.to_string(),
            display_name: spec.display_name.to_string(),
            source: match spec.mode {
                PlatformMode::Api => Source::Api,
                PlatformMode::Manual => Source::Manual,
            },
            entitlements: Vec::new(),
            console_url: non_empty(spec.console_url),
            error: Some(error.to_string()),
            updated_at: now_ts(),
        }
    }
}

/// 当前 Unix 时间戳（秒）
pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn non_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

// ═══════════════════════════════════════════════════════════════
// 平台元数据（吸收 backlog P2-8）
//
// 此前新增一个平台要改 5 处：types.ts / commands.rs /
// AddProviderForm.vue / QuotaCard.vue / main.rs。
// 现在只改下面这个数组。
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlatformMode {
    /// 有公开 API，需要 API Key，走 Keychain
    Api,
    /// 无 API，手动录入到期日
    Manual,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformSpec {
    pub id: &'static str,
    pub display_name: &'static str,
    pub mode: PlatformMode,
    pub console_url: &'static str,
    pub key_docs_url: &'static str,
    pub key_hint: &'static str,
    /// 空字符串表示不做格式校验
    pub key_pattern: &'static str,
}

/// 唯一事实来源。新增平台只改这里。
///
/// ⚠️ 各平台的 API 真实性见 doc/refactor-plan-v2.md §3.2 —— 除火山方舟外，
/// 其余 Api 平台的字段/端点均**未经真实响应验证**，Phase 3 才处理。
pub const PLATFORM_SPECS: &[PlatformSpec] = &[
    PlatformSpec {
        id: "deepseek",
        display_name: "DeepSeek",
        mode: PlatformMode::Api,
        console_url: "https://platform.deepseek.com/",
        key_docs_url: "https://platform.deepseek.com/api_keys",
        key_hint: "sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
        key_pattern: r"^sk-[a-f0-9]{32,}$",
    },
    PlatformSpec {
        id: "zhipu",
        display_name: "智谱AI",
        mode: PlatformMode::Api,
        console_url: "https://open.bigmodel.cn/usercenter/apikeys",
        key_docs_url: "https://open.bigmodel.cn/usercenter/apikeys",
        key_hint: "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx.xxxxxxxx",
        key_pattern: r"^[a-f0-9]{32}\.[A-Za-z0-9]+$",
    },
    PlatformSpec {
        id: "volcano",
        display_name: "火山方舟",
        mode: PlatformMode::Api,
        console_url: "https://console.volcengine.com/ark",
        key_docs_url: "https://console.volcengine.com/ark",
        key_hint: "AccessKey:SecretKey",
        key_pattern: "",
    },
    // ── 以下为 Manual 模式：无公开 API，用户手动录入到期日 ──
    PlatformSpec {
        id: "longcat",
        display_name: "LongCat",
        mode: PlatformMode::Manual,
        console_url: "https://longcat.chat/",
        key_docs_url: "",
        key_hint: "",
        key_pattern: "",
    },
    PlatformSpec {
        id: "xiaomi-mimo",
        display_name: "小米 MiMo",
        mode: PlatformMode::Manual,
        console_url: "https://platform.xiaomimimo.com/",
        key_docs_url: "",
        key_hint: "",
        key_pattern: "",
    },
    PlatformSpec {
        id: "chaosuan-deepseek",
        display_name: "超算 DeepSeek",
        mode: PlatformMode::Manual,
        // 控制台 URL 待用户补充（见计划 §6）
        console_url: "",
        key_docs_url: "",
        key_hint: "",
        key_pattern: "",
    },
    PlatformSpec {
        id: "doubao-work",
        display_name: "豆包工作",
        mode: PlatformMode::Manual,
        // 按用户指示采用推荐值，未经验证
        console_url: "https://www.doubao.com/",
        key_docs_url: "",
        key_hint: "",
        key_pattern: "",
    },
    PlatformSpec {
        id: "qoder",
        display_name: "Qoder",
        mode: PlatformMode::Manual,
        console_url: "https://qoder.dev/",
        key_docs_url: "",
        key_hint: "",
        key_pattern: "",
    },
];

pub fn find_spec(id: &str) -> Option<&'static PlatformSpec> {
    PLATFORM_SPECS.iter().find(|s| s.id == id)
}

// ═══════════════════════════════════════════════════════════════
// 旧模型 —— 待 Phase 1 迁移完成后删除
// ═══════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PlanType {
    PayAsYouGo,
    CodingPlan,
    Subscription,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub provider_name: String,
    pub plan_type: PlanType,
    pub quota_unit: QuotaUnit,
    pub total: Option<f64>,
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
            total: None,
            remaining: 0.0,
            is_success: false,
            error_msg: Some(error_msg.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_mode_serializes_to_snake_case() {
        assert_eq!(serde_json::to_string(&PlatformMode::Api).unwrap(), "\"api\"");
        assert_eq!(serde_json::to_string(&PlatformMode::Manual).unwrap(), "\"manual\"");
    }

    #[test]
    fn source_serializes_to_snake_case() {
        assert_eq!(serde_json::to_string(&Source::Api).unwrap(), "\"api\"");
        assert_eq!(serde_json::to_string(&Source::Manual).unwrap(), "\"manual\"");
    }

    /// 前端靠 get_platform_specs 渲染平台选择器，字段名是契约的一部分。
    #[test]
    fn spec_json_shape_matches_frontend_contract() {
        let spec = find_spec("chaosuan-deepseek").expect("超算 DeepSeek 应在清单中");
        let v = serde_json::to_value(spec).unwrap();
        assert_eq!(v["id"], "chaosuan-deepseek");
        assert_eq!(v["display_name"], "超算 DeepSeek");
        assert_eq!(v["mode"], "manual");
        assert!(v.get("console_url").is_some());
        assert!(v.get("key_pattern").is_some());
    }

    /// id 是 PlatformStatus 的稳定标识，重复会导致前端 key 冲突。
    #[test]
    fn spec_ids_are_unique() {
        let mut ids: Vec<&str> = PLATFORM_SPECS.iter().map(|s| s.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "PLATFORM_SPECS 中存在重复 id");
    }

    #[test]
    fn find_spec_returns_none_for_unknown() {
        assert!(find_spec("deepseek").is_some());
        assert!(find_spec("no-such-platform").is_none());
    }

    /// Api 模式必须能拿到 Key 引导信息，否则用户无从下手。
    #[test]
    fn api_specs_have_key_hint_and_docs() {
        for spec in PLATFORM_SPECS.iter().filter(|s| s.mode == PlatformMode::Api) {
            assert!(!spec.key_hint.is_empty(), "{} 缺少 key_hint", spec.id);
            assert!(!spec.key_docs_url.is_empty(), "{} 缺少 key_docs_url", spec.id);
        }
    }

    #[test]
    fn failed_status_carries_error_and_empty_entitlements() {
        let spec = find_spec("deepseek").unwrap();
        let st = PlatformStatus::failed(spec, "HTTP 401");
        assert_eq!(st.id, "deepseek");
        assert_eq!(st.source, Source::Api);
        assert!(st.entitlements.is_empty());
        assert_eq!(st.error.as_deref(), Some("HTTP 401"));
    }
}
