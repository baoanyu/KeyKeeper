use anyhow::Result;
use tauri::State;
use tauri_plugin_store::StoreExt;
use std::sync::Arc;
use reqwest::Client;
use crate::adapters::deepseek::DeepSeekFetcher;
use crate::adapters::zhipu::ZhipuFetcher;
use crate::adapters::qoder::QoderFetcher;
use crate::adapters::volcano::VolcanoFetcher;
use crate::adapters::QuotaFetcher;
use crate::keystore;
use crate::models::QuotaInfo;
use crate::scheduler::fetch_all_quotas;

pub struct AppState {
    pub providers: tokio::sync::Mutex<Vec<String>>,
    pub http_client: Arc<Client>,
}

const STORE_KEY: &str = "providers_list";
const QODER_FIRST_LAUNCH_KEY: &str = "qoder_first_launch";

async fn load_providers(app: &tauri::AppHandle) -> Vec<String> {
    let store = app.store("keykeeper-store.json").ok();
    if let Some(store) = store {
        let value = store.get(STORE_KEY);
        if let Some(value) = value {
            if let Ok(providers) = serde_json::from_value::<Vec<String>>(value.clone()) {
                return providers;
            }
        }
    }
    Vec::new()
}

async fn save_providers(app: &tauri::AppHandle, providers: &[String]) -> Result<()> {
    let store = app.store("keykeeper-store.json")?;
    store.set(STORE_KEY, serde_json::to_value(providers)?);
    store.save()?;
    Ok(())
}

async fn get_qoder_first_launch(app: &tauri::AppHandle) -> Option<f64> {
    let store = app.store("keykeeper-store.json").ok()?;
    let value = store.get(QODER_FIRST_LAUNCH_KEY)?;
    value.as_f64()
}

async fn set_qoder_first_launch(app: &tauri::AppHandle, timestamp: f64) -> Result<()> {
    let store = app.store("keykeeper-store.json")?;
    store.set(QODER_FIRST_LAUNCH_KEY, serde_json::to_value(timestamp)?);
    store.save()?;
    Ok(())
}

async fn ensure_providers_loaded(state: &AppState, app: &tauri::AppHandle) -> Vec<String> {
    let guard = state.providers.lock().await;
    if guard.is_empty() {
        drop(guard);
        let stored = load_providers(app).await;
        let mut guard = state.providers.lock().await;
        *guard = stored;
        guard.clone()
    } else {
        guard.clone()
    }
}

#[tauri::command]
pub async fn get_all_quotas(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<QuotaInfo>, String> {
    let providers = ensure_providers_loaded(&state, &app).await;
    
    // Get or set Qoder first launch time
    let qoder_first_launch = get_qoder_first_launch(&app).await;
    let qoder_first_launch = match qoder_first_launch {
        Some(t) => t,
        None => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let _ = set_qoder_first_launch(&app, now).await;
            now
        }
    };
    
    let client = state.http_client.clone();
    let mut tasks: Vec<(String, String, Box<dyn QuotaFetcher>)> = Vec::new();

    for provider in &providers {
        match keystore::get_key(provider) {
            Ok(api_key) => {
                let fetcher: Box<dyn QuotaFetcher> = if provider == "DeepSeek" {
                    Box::new(DeepSeekFetcher::new(client.clone()))
                } else if provider == "ZhipuAI" {
                    Box::new(ZhipuFetcher::new(client.clone()))
                } else if provider == "Qoder" {
                    Box::new(QoderFetcher::new(Some(qoder_first_launch)))
                } else if provider == "Volcano" {
                    Box::new(VolcanoFetcher::new(client.clone()))
                } else {
                    log::warn!("Unknown provider: {}", provider);
                    continue;
                };
                tasks.push((provider.clone(), api_key, fetcher));
            }
            Err(e) => {
                log::warn!("Failed to get key for {}: {}", provider, e);
            }
        }
    }

    let results = fetch_all_quotas(tasks).await;
    Ok(results)
}

#[tauri::command]
pub async fn save_provider_key(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    provider: String,
    key: String,
) -> Result<(), String> {
    keystore::save_key(&provider, &key).map_err(|e| e.to_string())?;
    
    let mut providers = state.providers.lock().await;
    if !providers.contains(&provider) {
        providers.push(provider);
        let _ = save_providers(&app, &providers).await;
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_provider(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    provider: String,
) -> Result<(), String> {
    keystore::delete_key(&provider).map_err(|e| e.to_string())?;
    
    let mut providers = state.providers.lock().await;
    providers.retain(|p| p != &provider);
    let _ = save_providers(&app, &providers).await;
    Ok(())
}

#[tauri::command]
pub async fn get_saved_providers(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    Ok(ensure_providers_loaded(&state, &app).await)
}

#[tauri::command]
pub async fn add_provider(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    provider: String,
) -> Result<(), String> {
    let mut providers = state.providers.lock().await;
    if !providers.contains(&provider) {
        providers.push(provider);
        let _ = save_providers(&app, &providers).await;
    }
    Ok(())
}
