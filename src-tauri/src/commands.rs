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
    let store = match app.store("keykeeper-store.json") {
        Ok(s) => s,
        Err(e) => {
            // P3-4: log instead of silently returning empty
            log::warn!("Failed to open store: {}", e);
            return Vec::new();
        }
    };
    let value = store.get(STORE_KEY);
    if let Some(value) = value {
        match serde_json::from_value::<Vec<String>>(value.clone()) {
            Ok(providers) => return providers,
            Err(e) => {
                // P3-4: log corrupted store data
                log::warn!("Failed to parse providers from store: {}", e);
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

    // F5: only seed Qoder first_launch if Qoder is actually configured
    let qoder_configured = providers.contains(&"Qoder".to_string());
    let qoder_first_launch = if qoder_configured {
        let stored = get_qoder_first_launch(&app).await;
        match stored {
            Some(t) => Some(t),
            None => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64();
                // P2-3: log persistence failure instead of silent `let _`
                if let Err(e) = set_qoder_first_launch(&app, now).await {
                    log::error!("Failed to persist Qoder first launch time: {}", e);
                }
                Some(now)
            }
        }
    } else {
        None
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
                    Box::new(QoderFetcher::new(qoder_first_launch))
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

// F1 (P0 fix): expose key retrieval so frontend can snapshot before reconfigure
#[tauri::command]
pub async fn get_provider_key(provider: String) -> Result<String, String> {
    keystore::get_key(&provider).map_err(|e| e.to_string())
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
