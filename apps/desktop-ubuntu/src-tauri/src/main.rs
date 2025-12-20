//! Coachly Code Bridge Desktop App (Tauri)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bridge_core::{Bridge, Config};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::RwLock;

/// Application state
struct AppState {
    bridge: RwLock<Option<Bridge>>,
    config: RwLock<Config>,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    device_name: String,
    device_id: String,
    peer_count: usize,
    file_count: usize,
    is_syncing: bool,
}

#[derive(Debug, Serialize)]
struct PeerInfo {
    id: String,
    name: String,
    addresses: Vec<String>,
    connected: bool,
}

#[derive(Debug, Serialize)]
struct FileInfo {
    path: String,
    hash: String,
    size: u64,
    mime_type: Option<String>,
}

/// Get the current bridge status
#[tauri::command]
async fn get_status(state: State<'_, Arc<AppState>>) -> Result<StatusResponse, String> {
    let config = state.config.read().await;
    let bridge = state.bridge.read().await;

    let (peer_count, file_count) = if let Some(ref b) = *bridge {
        let peers = b.network().peers().len();
        let files = b.storage().list_files().unwrap_or_default().len();
        (peers, files)
    } else {
        (0, 0)
    };

    Ok(StatusResponse {
        device_name: config.device_name.clone(),
        device_id: config.device_id[..8].to_string(),
        peer_count,
        file_count,
        is_syncing: false,
    })
}

/// List connected peers
#[tauri::command]
async fn list_peers(state: State<'_, Arc<AppState>>) -> Result<Vec<PeerInfo>, String> {
    let bridge = state.bridge.read().await;

    let peers = if let Some(ref b) = *bridge {
        b.network()
            .peers()
            .iter()
            .map(|(id, info)| PeerInfo {
                id: id.to_string(),
                name: info.device_name.clone().unwrap_or_else(|| "Unknown".to_string()),
                addresses: info.addresses.iter().map(|a| a.to_string()).collect(),
                connected: true,
            })
            .collect()
    } else {
        vec![]
    };

    Ok(peers)
}

/// List tracked files
#[tauri::command]
async fn list_files(state: State<'_, Arc<AppState>>) -> Result<Vec<FileInfo>, String> {
    let config = state.config.read().await;
    let storage = bridge_core::ContentStore::new(&config).map_err(|e| e.to_string())?;

    let files = storage
        .list_files()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|f| FileInfo {
            path: f.path.to_string_lossy().to_string(),
            hash: f.hash,
            size: f.size,
            mime_type: f.mime_type,
        })
        .collect();

    Ok(files)
}

/// Add files to sync
#[tauri::command]
async fn add_files(paths: Vec<String>, state: State<'_, Arc<AppState>>) -> Result<usize, String> {
    let config = state.config.read().await;
    let mut storage = bridge_core::ContentStore::new(&config).map_err(|e| e.to_string())?;

    let mut count = 0;
    for path_str in paths {
        let path = std::path::Path::new(&path_str);
        if path.is_file() {
            let relative = path.file_name()
                .map(std::path::PathBuf::from)
                .unwrap_or_default();
            if storage.store_file(path, &relative).is_ok() {
                count += 1;
            }
        } else if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();
                let relative = file_path.strip_prefix(path).unwrap_or(file_path);
                if storage.store_file(file_path, relative).is_ok() {
                    count += 1;
                }
            }
        }
    }

    Ok(count)
}

/// Start syncing
#[tauri::command]
async fn start_sync(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut bridge = state.bridge.write().await;

    if let Some(ref mut b) = *bridge {
        b.start().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Stop syncing
#[tauri::command]
async fn stop_sync(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut bridge = state.bridge.write().await;

    if let Some(ref mut b) = *bridge {
        b.stop().await.map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Discover peers
#[tauri::command]
async fn discover_peers(state: State<'_, Arc<AppState>>) -> Result<Vec<PeerInfo>, String> {
    // Trigger peer discovery
    let bridge = state.bridge.read().await;

    if bridge.is_none() {
        return Err("Bridge not initialized".to_string());
    }

    // Return current peers for now
    list_peers(state).await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Initialize state
            let config = Config::load_or_create().expect("Failed to load config");

            let state = Arc::new(AppState {
                bridge: RwLock::new(None),
                config: RwLock::new(config),
            });

            app.manage(state.clone());

            // Initialize bridge in background
            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                match Bridge::new().await {
                    Ok(bridge) => {
                        let mut b = state_clone.bridge.write().await;
                        *b = Some(bridge);
                        println!("Bridge initialized successfully");
                    }
                    Err(e) => {
                        eprintln!("Failed to initialize bridge: {}", e);
                    }
                }
            });

            // Create system tray
            #[cfg(desktop)]
            {
                use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState};
                use tauri::menu::{Menu, MenuItem};

                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show, &quit])?;

                let _tray = TrayIconBuilder::new()
                    .menu(&menu)
                    .tooltip("Code Bridge")
                    .on_menu_event(|app, event| {
                        match event.id.as_ref() {
                            "quit" => {
                                app.exit(0);
                            }
                            "show" => {
                                if let Some(window) = app.get_webview_window("main") {
                                    window.show().unwrap();
                                    window.set_focus().unwrap();
                                }
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let tauri::tray::TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                            if let Some(window) = tray.app_handle().get_webview_window("main") {
                                window.show().unwrap();
                                window.set_focus().unwrap();
                            }
                        }
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            list_peers,
            list_files,
            add_files,
            start_sync,
            stop_sync,
            discover_peers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
