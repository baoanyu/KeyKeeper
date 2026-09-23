pub mod models;
pub mod keystore;
pub mod adapters;
pub mod scheduler;
pub mod commands;

use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton};
use tauri::menu::{Menu, MenuItem};
use tauri::{Manager, WebviewWindow, WindowEvent, Emitter};
use tokio::sync::Mutex as TokioMutex;
use std::sync::Arc;
use std::time::Duration;
use reqwest::Client;

use commands::{
    get_all_quotas, save_provider_key, delete_provider, get_saved_providers,
    add_provider, get_provider_key, check_low_balance, get_platform_specs, AppState,
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
        .plugin(tauri_plugin_notification::init())
        .manage(AppState {
            providers: TokioMutex::new(Vec::new()),
            http_client,
        })
        .setup(|app| {
            // P3-2: safe unwrap — exit early if main window is unavailable
            let window = match app.get_webview_window("main") {
                Some(w) => w,
                None => return Err("main window not found".into()),
            };

            // Create tray menu with quit option
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit_item])?;

            // P3-3: safe unwrap — propagate error if no window icon configured
            let icon = match app.default_window_icon() {
                Some(icon) => icon.clone(),
                None => return Err(Box::from("default window icon not configured")),
            };

            // Tray icon setup with click handler and menu
            // U-24: retain handle for dynamic tooltip updates (currently unused)
            let _tray = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("KeyKeeper - API 配额管理")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event({
                    let window = window.clone();
                    move |_tray, event| {
                        if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                            toggle_window(&window);
                        }
                    }
                })
                .build(app)?;

            // Hide window on focus loss
            window.on_window_event({
                let window = window.clone();
                move |event| {
                    if let WindowEvent::Focused(false) = event {
                        let _ = window.hide();
                    }
                }
            });

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
            get_all_quotas,
            save_provider_key,
            delete_provider,
            get_saved_providers,
            add_provider,
            get_provider_key,
            check_low_balance,
            get_platform_specs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running KeyKeeper");
}

fn toggle_window(window: &WebviewWindow) {
    if window.is_visible().unwrap_or(false) {
        let _ = window.hide();
    } else {
        position_window_below_tray(window);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn position_window_below_tray(window: &WebviewWindow) {
    // Improved positioning: place window at top-right where menu bar icons are
    if let Ok(Some(monitor)) = window.current_monitor() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let logical_width = size.width as f64 / scale;
        let logical_height = size.height as f64 / scale;

        // Position at top-right area (where menu bar icons typically are)
        let x = (logical_width - 420.0).max(10.0); // 20px margin from right
        let y: f64 = (30.0f64).min(logical_height - 100.0); // Just below menu bar

        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition {
            x,
            y,
        }));
    }
}
