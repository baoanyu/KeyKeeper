use anyhow::Result;
use tauri::State;
use tauri_plugin_store::StoreExt;
use std::collections::HashMap;
use std::sync::Arc;
use reqwest::Client;
use crate::adapters::deepseek::DeepSeekFetcher;
use crate::adapters::zhipu::ZhipuFetcher;
use crate::adapters::volcano::VolcanoFetcher;
use crate::keystore;
use crate::models::{
    find_spec, Entitlement, PlatformMode, PlatformSpec, PlatformStatus, Source, PLATFORM_SPECS,
};
use crate::scheduler::{fetch_all_platforms, FetchTask};

pub struct AppState {
    pub http_client: Arc<Client>,
    /// R-6：手动录入读-改-写串行化，防止并发保存互相覆盖丢条目
    pub manual_write_lock: tokio::sync::Mutex<()>,
}

const STORE_FILE: &str = "keykeeper-store.json";
/// 手动录入数据：platform_id → 该平台的额度包列表 + 录入时间
const MANUAL_PLATFORMS_KEY: &str = "manual_platforms";

/// store 中手动录入条目的持久化结构
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ManualEntry {
    entitlements: Vec<Entitlement>,
    updated_at: i64,
}

type ManualMap = HashMap<String, ManualEntry>;

fn load_manual_map(app: &tauri::AppHandle) -> ManualMap {
    let store = match app.store(STORE_FILE) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("Failed to open store: {}", e);
            return ManualMap::new();
        }
    };
    match store.get(MANUAL_PLATFORMS_KEY) {
        Some(value) => match serde_json::from_value::<ManualMap>(value.clone()) {
            Ok(map) => map,
            Err(e) => {
                log::warn!("Failed to parse manual_platforms from store: {}", e);
                ManualMap::new()
            }
        },
        None => ManualMap::new(),
    }
}

fn save_manual_map(app: &tauri::AppHandle, map: &ManualMap) -> Result<()> {
    let store = app.store(STORE_FILE)?;
    // R-6：保存前留存旧值，落盘失败时回滚内存态，
    // 避免"界面显示已保存、重启后回退"的二次困惑。
    let previous = store.get(MANUAL_PLATFORMS_KEY);
    store.set(MANUAL_PLATFORMS_KEY, serde_json::to_value(map)?);
    if let Err(e) = store.save() {
        if let Some(prev) = previous {
            store.set(MANUAL_PLATFORMS_KEY, prev);
        }
        return Err(anyhow::anyhow!(e));
    }
    Ok(())
}

/// R-7：判断 anyhow 错误链中是否为 Keychain「条目不存在」
fn is_no_entry(e: &anyhow::Error) -> bool {
    matches!(
        e.downcast_ref::<keyring::Error>(),
        Some(keyring::Error::NoEntry)
    )
}

/// 校验 id 是 Manual 模式的已知平台，返回其 spec
fn manual_spec(id: &str) -> Result<&'static PlatformSpec, String> {
    match find_spec(id) {
        Some(spec) if spec.mode == PlatformMode::Manual => Ok(spec),
        Some(spec) => Err(format!("{} 不是手动录入平台", spec.display_name)),
        None => Err(format!("未知平台: {}", id)),
    }
}

/// 校验 id 是 Api 模式的已知平台，返回其 spec
fn api_spec(id: &str) -> Result<&'static PlatformSpec, String> {
    match find_spec(id) {
        Some(spec) if spec.mode == PlatformMode::Api => Ok(spec),
        Some(spec) => Err(format!("{} 不是 API 平台", spec.display_name)),
        None => Err(format!("未知平台: {}", id)),
    }
}

/// 汇总所有平台状态：Api 平台并发查询，Manual 平台从 store 读取。
/// 未配置的平台（Api 无 Key / Manual 无录入）不出现在结果中。
#[tauri::command]
pub async fn get_all_platforms(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<PlatformStatus>, String> {
    let mut statuses: Vec<PlatformStatus> = Vec::new();
    let mut tasks = Vec::new();

    // R-10：手动录入 map 只加载一次（旧实现在循环内对每个 Manual 平台重复反序列化）
    let manual_map = load_manual_map(&app);

    for spec in PLATFORM_SPECS {
        match spec.mode {
            PlatformMode::Api => {
                let api_key = match keystore::get_key(spec.id) {
                    Ok(key) => key,
                    // R-7：NoEntry = 未配置，跳过；其他 Keychain 错误要显式兜底
                    Err(e) if is_no_entry(&e) => continue,
                    Err(e) => {
                        log::warn!("Keychain read failed for {}: {}", spec.id, e);
                        statuses.push(PlatformStatus::failed(
                            spec,
                            &format!("Keychain 读取失败: {e}"),
                        ));
                        continue;
                    }
                };
                let fetcher: Box<dyn crate::adapters::QuotaFetcher> = match spec.id {
                    "deepseek" => Box::new(DeepSeekFetcher::new(state.http_client.clone())),
                    "zhipu" => Box::new(ZhipuFetcher::new(state.http_client.clone())),
                    "volcano" => Box::new(VolcanoFetcher::new(state.http_client.clone())),
                    _ => {
                        log::warn!("No fetcher registered for api platform: {}", spec.id);
                        continue;
                    }
                };
                tasks.push(FetchTask { spec, api_key, fetcher });
            }
            PlatformMode::Manual => {
                if let Some(entry) = manual_map.get(spec.id) {
                    statuses.push(PlatformStatus {
                        id: spec.id.to_string(),
                        display_name: spec.display_name.to_string(),
                        source: Source::Manual,
                        entitlements: entry.entitlements.clone(),
                        console_url: crate::models::non_empty(spec.console_url),
                        error: None,
                        updated_at: entry.updated_at,
                    });
                }
            }
        }
    }

    let api_statuses = fetch_all_platforms(tasks).await;
    statuses.extend(api_statuses);
    Ok(statuses)
}

/// 保存 Api 平台的 Key（Keychain 以平台 id 为条目名）
#[tauri::command]
pub async fn save_api_key(id: String, key: String) -> Result<(), String> {
    api_spec(&id)?;
    if key.trim().is_empty() {
        return Err("Key 不能为空".to_string());
    }
    keystore::save_key(&id, &key).map_err(|e| e.to_string())
}

/// 读取 Api 平台的 Key（用于重配置时快照旧 Key）
#[tauri::command]
pub async fn get_api_key(id: String) -> Result<String, String> {
    api_spec(&id)?;
    keystore::get_key(&id).map_err(|e| e.to_string())
}

/// 删除平台：Api 删 Keychain Key，Manual 删 store 录入条目。
/// Keychain 条目缺失时视为删除成功（P0-1b）。
#[tauri::command]
pub async fn delete_platform(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
) -> Result<(), String> {
    let spec = find_spec(&id).ok_or_else(|| format!("未知平台: {}", id))?;
    match spec.mode {
        PlatformMode::Api => keystore::delete_key(&id).map_err(|e| e.to_string()),
        PlatformMode::Manual => {
            // R-6：与其他写入互斥
            let _guard = state.manual_write_lock.lock().await;
            let mut map = load_manual_map(&app);
            map.remove(&id);
            save_manual_map(&app, &map).map_err(|e| e.to_string())
        }
    }
}

/// 覆盖式保存手动录入的额度包列表（新增 / 编辑 / 续费统一入口）。
/// 保存空列表等价于删除该平台的录入数据。
/// 错误必须传播到前端 —— 静默吞掉会导致内存与磁盘不一致（见 CLAUDE.md 错误传播模式）。
#[tauri::command]
pub async fn save_manual_platform(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    id: String,
    entitlements: Vec<Entitlement>,
) -> Result<(), String> {
    manual_spec(&id)?;
    // R-6：读-改-写全程持锁，防止并发保存不同平台时后写覆盖先写
    let _guard = state.manual_write_lock.lock().await;
    let mut map = load_manual_map(&app);
    if entitlements.is_empty() {
        map.remove(&id);
    } else {
        map.insert(
            id,
            ManualEntry {
                entitlements,
                updated_at: crate::models::now_ts(),
            },
        );
    }
    save_manual_map(&app, &map).map_err(|e| e.to_string())
}

/// 读取指定平台的手动录入额度包（编辑表单初始化用）
#[tauri::command]
pub async fn get_manual_platform(
    app: tauri::AppHandle,
    id: String,
) -> Result<Vec<Entitlement>, String> {
    manual_spec(&id)?;
    let map = load_manual_map(&app);
    Ok(map.get(&id).map(|e| e.entitlements.clone()).unwrap_or_default())
}

/// 返回全部平台元数据（吸收 backlog P2-8）。
///
/// 前端据此渲染平台选择器 / Key 提示 / 控制台链接，
/// **新增平台只需改 `models.rs` 的 `PLATFORM_SPECS` 一处**。
#[tauri::command]
pub fn get_platform_specs() -> Vec<PlatformSpec> {
    PLATFORM_SPECS.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 手动录入的持久化契约：serde 往返不丢字段
    /// （1.3 验证标准「重启后录入的到期日仍在」依赖此往返）
    #[test]
    fn manual_map_serde_round_trip() {
        let mut map = ManualMap::new();
        map.insert(
            "chaosuan-deepseek".to_string(),
            ManualEntry {
                entitlements: vec![
                    Entitlement::new("0.1").with_expires(1761148799),
                    Entitlement::new("10M 体验").with_expires(1761148799),
                ],
                updated_at: 1758556800,
            },
        );
        let json = serde_json::to_string(&map).unwrap();
        let back: ManualMap = serde_json::from_str(&json).unwrap();
        let entry = back.get("chaosuan-deepseek").unwrap();
        assert_eq!(entry.entitlements.len(), 2);
        assert_eq!(entry.entitlements[0].label, "0.1");
        assert_eq!(entry.entitlements[0].expires_at, Some(1761148799));
        assert_eq!(entry.updated_at, 1758556800);
    }

    /// 空 store 键应回退为空 map，而不是解析失败
    #[test]
    fn manual_map_deserializes_from_empty_object() {
        let map: ManualMap = serde_json::from_str("{}").unwrap();
        assert!(map.is_empty());
    }
}
