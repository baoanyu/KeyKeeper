pub mod models;
pub mod keystore;
pub mod adapters;
pub mod scheduler;
pub mod commands;

use tauri::{Emitter, Manager, WindowEvent};
use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;

use commands::{
    get_all_platforms, save_api_key, get_api_key, delete_platform,
    save_manual_platform, get_manual_platform, get_platform_specs, AppState,
};

const AUTO_REFRESH_INTERVAL_SECS: u64 = 300; // 5 minutes

pub fn run() {
    env_logger::init();

    // Create a shared HTTP client with connection pooling
    let http_client = Arc::new(
        Client::builder()
            .timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .expect("Failed to build HTTP client")
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .manage(AppState {
            http_client,
        })
        // §1.4 决策：关窗即退出应用（无后台进程，系统通知已随之移除）
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event {
                window.app_handle().exit(0);
            }
        })
        .setup(|app| {
            // Setup auto-refresh timer
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(AUTO_REFRESH_INTERVAL_SECS));
                // Consume the first immediate tick so the initial refresh
                // is not duplicated with App.vue's onMounted refresh().
                interval.tick().await;
                loop {
                    interval.tick().await;
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.emit("auto-refresh", ());
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_all_platforms,
            save_api_key,
            get_api_key,
            delete_platform,
            save_manual_platform,
            get_manual_platform,
            get_platform_specs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KeyKeeper");
}
