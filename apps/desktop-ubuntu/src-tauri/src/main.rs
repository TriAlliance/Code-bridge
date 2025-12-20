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

#[derive(Debug, Serialize)]
struct ClipboardEntry {
    id: String,
    content_type: String,
    preview: String,
    timestamp: u64,
    source_device: String,
    favorite: bool,
}

#[derive(Debug, Serialize)]
struct NotificationInfo {
    id: String,
    app: String,
    title: String,
    body: String,
    category: String,
    priority: String,
    read: bool,
    timestamp: u64,
}

#[derive(Debug, Serialize)]
struct TerminalRecording {
    id: String,
    title: String,
    shell: String,
    duration: f64,
    timestamp: u64,
    preview: String,
}

#[derive(Debug, Serialize)]
struct CommandHistoryEntry {
    id: String,
    command: String,
    working_dir: String,
    exit_code: i32,
    duration: f64,
    timestamp: u64,
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

/// Get clipboard history
#[tauri::command]
async fn get_clipboard_history(_state: State<'_, Arc<AppState>>) -> Result<Vec<ClipboardEntry>, String> {
    // Return sample data for now - will be connected to bridge-clipboard crate
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(vec![
        ClipboardEntry {
            id: "1".to_string(),
            content_type: "code".to_string(),
            preview: "fn main() { println!(\"Hello\"); }".to_string(),
            timestamp: now - 3600000,
            source_device: "local".to_string(),
            favorite: true,
        },
        ClipboardEntry {
            id: "2".to_string(),
            content_type: "url".to_string(),
            preview: "https://github.com/anthropics/claude-code".to_string(),
            timestamp: now - 7200000,
            source_device: "macOS".to_string(),
            favorite: false,
        },
        ClipboardEntry {
            id: "3".to_string(),
            content_type: "text".to_string(),
            preview: "Meeting notes from standup discussion".to_string(),
            timestamp: now - 86400000,
            source_device: "local".to_string(),
            favorite: false,
        },
    ])
}

/// Get notifications
#[tauri::command]
async fn get_notifications(_state: State<'_, Arc<AppState>>) -> Result<Vec<NotificationInfo>, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(vec![
        NotificationInfo {
            id: "1".to_string(),
            app: "GitHub Actions".to_string(),
            title: "Build Succeeded".to_string(),
            body: "CI/CD pipeline completed successfully".to_string(),
            category: "build".to_string(),
            priority: "normal".to_string(),
            read: false,
            timestamp: now - 1800000,
        },
        NotificationInfo {
            id: "2".to_string(),
            app: "GitHub".to_string(),
            title: "New PR Review".to_string(),
            body: "Alex commented on your pull request #42".to_string(),
            category: "pr".to_string(),
            priority: "high".to_string(),
            read: false,
            timestamp: now - 3600000,
        },
        NotificationInfo {
            id: "3".to_string(),
            app: "Security Scanner".to_string(),
            title: "Vulnerability Found".to_string(),
            body: "Detected outdated dependency with known CVE".to_string(),
            category: "security".to_string(),
            priority: "critical".to_string(),
            read: true,
            timestamp: now - 7200000,
        },
    ])
}

/// Get terminal recordings
#[tauri::command]
async fn get_recordings(_state: State<'_, Arc<AppState>>) -> Result<Vec<TerminalRecording>, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(vec![
        TerminalRecording {
            id: "1".to_string(),
            title: "Project Setup".to_string(),
            shell: "zsh".to_string(),
            duration: 245.0,
            timestamp: now - 86400000,
            preview: "$ cargo new my-project\n     Created binary (application)".to_string(),
        },
        TerminalRecording {
            id: "2".to_string(),
            title: "Debug Session".to_string(),
            shell: "bash".to_string(),
            duration: 180.0,
            timestamp: now - 172800000,
            preview: "$ npm run test\n\nPASS  src/utils.test.ts".to_string(),
        },
    ])
}

/// Get command history
#[tauri::command]
async fn get_command_history(_state: State<'_, Arc<AppState>>) -> Result<Vec<CommandHistoryEntry>, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    Ok(vec![
        CommandHistoryEntry {
            id: "1".to_string(),
            command: "cargo build --release".to_string(),
            working_dir: "~/Projects/code-bridge".to_string(),
            exit_code: 0,
            duration: 45.2,
            timestamp: now - 300000,
        },
        CommandHistoryEntry {
            id: "2".to_string(),
            command: "git push origin main".to_string(),
            working_dir: "~/Projects/code-bridge".to_string(),
            exit_code: 0,
            duration: 2.1,
            timestamp: now - 600000,
        },
        CommandHistoryEntry {
            id: "3".to_string(),
            command: "npm install".to_string(),
            working_dir: "~/Projects/frontend".to_string(),
            exit_code: 0,
            duration: 12.5,
            timestamp: now - 900000,
        },
    ])
}

/// Transform data between formats
#[tauri::command]
async fn transform_data(
    input: String,
    from_format: String,
    to_format: String,
) -> Result<String, String> {
    // Basic JSON to YAML conversion for demo
    if from_format == "json" && to_format == "yaml" {
        match serde_json::from_str::<serde_json::Value>(&input) {
            Ok(value) => {
                serde_yaml::to_string(&value).map_err(|e| e.to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    } else if from_format == "yaml" && to_format == "json" {
        match serde_yaml::from_str::<serde_yaml::Value>(&input) {
            Ok(value) => {
                serde_json::to_string_pretty(&value).map_err(|e| e.to_string())
            }
            Err(e) => Err(e.to_string()),
        }
    } else {
        Ok(input)
    }
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
            get_clipboard_history,
            get_notifications,
            get_recordings,
            get_command_history,
            transform_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
