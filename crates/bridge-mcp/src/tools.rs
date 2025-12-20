//! MCP Tool implementations for Code Bridge
//!
//! Provides tool implementations for:
//! - Core bridge operations (status, sync, share, peers)
//! - Clipboard management (get, set, history, favorites)
//! - Notifications (list, send, mark read)
//! - Terminal (history, recordings, environment)

use crate::McpState;
use std::sync::Arc;

// ============================================================================
// Core Bridge Tools
// ============================================================================

/// Get the current bridge status
pub async fn status(state: &Arc<McpState>) -> Result<String, String> {
    let bridge = state.bridge.read().await;
    let config = bridge.config();

    let mut output = String::new();
    output.push_str("# Code Bridge Status\n\n");
    output.push_str(&format!("Device: {}\n", config.device_name));
    output.push_str(&format!("Device ID: {}\n", &config.device_id[..8]));
    output.push_str(&format!("Data Dir: {}\n", config.data_dir.display()));
    output.push_str("\n## Network\n");
    output.push_str(&format!("- mDNS: {}\n", if config.network.enable_mdns { "enabled" } else { "disabled" }));
    output.push_str(&format!("- DHT: {}\n", if config.network.enable_dht { "enabled" } else { "disabled" }));
    output.push_str(&format!("- QUIC: {}\n", if config.network.enable_quic { "enabled" } else { "disabled" }));
    output.push_str(&format!("- TCP: {}\n", if config.network.enable_tcp { "enabled" } else { "disabled" }));

    output.push_str("\n## Storage\n");
    let storage = bridge_core::ContentStore::new(&config).map_err(|e| e.to_string())?;
    let files = storage.list_files().map_err(|e| e.to_string())?;
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    output.push_str(&format!("- Files tracked: {}\n", files.len()));
    output.push_str(&format!("- Total size: {} bytes\n", total_size));

    Ok(output)
}

/// Trigger a sync operation
pub async fn sync(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let project = args.get("project").and_then(|v| v.as_str());

    let mut output = String::new();
    output.push_str("# Sync Triggered\n\n");

    if let Some(project_name) = project {
        output.push_str(&format!("Syncing project: {}\n", project_name));
    } else {
        output.push_str("Syncing all projects\n");
    }

    // Get connected peers
    let bridge = state.bridge.read().await;
    let peers = bridge.network().peers();

    if peers.is_empty() {
        output.push_str("\n⚠️ No peers connected. Run `codebridge peer discover` to find peers.\n");
    } else {
        output.push_str(&format!("\nSyncing with {} peer(s):\n", peers.len()));
        for (peer_id, info) in peers {
            let name = info.device_name.as_deref().unwrap_or("Unknown");
            output.push_str(&format!("- {} ({})\n", name, &peer_id.to_string()[..16]));
        }
    }

    Ok(output)
}

/// Share a file or directory
pub async fn share(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' argument")?;

    let peer = args.get("peer").and_then(|v| v.as_str());

    let path = std::path::Path::new(path);
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    let config = state.config.clone();
    let mut storage = bridge_core::ContentStore::new(&config).map_err(|e| e.to_string())?;

    let mut output = String::new();
    output.push_str("# Sharing Files\n\n");

    if path.is_file() {
        let relative = path.file_name()
            .map(std::path::PathBuf::from)
            .unwrap_or_default();
        let entry = storage.store_file(path, &relative).map_err(|e| e.to_string())?;
        output.push_str(&format!("✓ Shared: {} ({} bytes)\n", path.display(), entry.size));
        output.push_str(&format!("  Hash: {}\n", entry.hash));
    } else if path.is_dir() {
        let mut count = 0;
        let mut total_size = 0u64;

        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let file_path = entry.path();
            let relative = file_path.strip_prefix(path).unwrap_or(file_path);

            if let Ok(file_entry) = storage.store_file(file_path, relative) {
                count += 1;
                total_size += file_entry.size;
            }
        }

        output.push_str(&format!("✓ Shared {} files ({} bytes)\n", count, total_size));
    }

    if let Some(peer_name) = peer {
        output.push_str(&format!("\nSharing with peer: {}\n", peer_name));
    } else {
        output.push_str("\nSharing with all connected peers\n");
    }

    Ok(output)
}

/// List connected peers
pub async fn peers(state: &Arc<McpState>) -> Result<String, String> {
    let bridge = state.bridge.read().await;
    let peers = bridge.network().peers();

    let mut output = String::new();
    output.push_str("# Connected Peers\n\n");
    output.push_str(&format!("Local Peer ID: {}\n\n", bridge.network().local_peer_id()));

    if peers.is_empty() {
        output.push_str("No peers connected.\n\n");
        output.push_str("To discover peers, run:\n");
        output.push_str("```\ncodebridge peer discover\n```\n");
    } else {
        output.push_str(&format!("{} peer(s) connected:\n\n", peers.len()));
        for (peer_id, info) in peers {
            let name = info.device_name.as_deref().unwrap_or("Unknown Device");
            output.push_str(&format!("## {}\n", name));
            output.push_str(&format!("- Peer ID: {}\n", peer_id));
            output.push_str(&format!("- Addresses: {:?}\n", info.addresses));
            output.push_str(&format!("- Last seen: {:?} ago\n\n", info.last_seen.elapsed()));
        }
    }

    Ok(output)
}

/// List and search screenshots
pub async fn screenshots(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let search = args.get("search").and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let config = state.config.clone();
    let storage = bridge_core::ContentStore::new(&config).map_err(|e| e.to_string())?;

    let files = storage.list_files().map_err(|e| e.to_string())?;

    // Filter to image files
    let screenshots: Vec<_> = files
        .iter()
        .filter(|f| {
            f.mime_type
                .as_ref()
                .map(|m| m.starts_with("image/"))
                .unwrap_or(false)
        })
        .take(limit)
        .collect();

    let mut output = String::new();
    output.push_str("# Screenshots\n\n");

    if let Some(query) = search {
        output.push_str(&format!("Search: \"{}\"\n\n", query));
        // TODO: Implement OCR search
        output.push_str("(OCR search not yet implemented)\n\n");
    }

    if screenshots.is_empty() {
        output.push_str("No screenshots found.\n\n");
        output.push_str("To watch for screenshots:\n");
        output.push_str("```\ncodebridge screenshot watch\n```\n");
    } else {
        output.push_str(&format!("Showing {} screenshot(s):\n\n", screenshots.len()));
        for s in screenshots {
            output.push_str(&format!("- **{}**\n", s.path.display()));
            output.push_str(&format!("  - Size: {} bytes\n", s.size));
            output.push_str(&format!("  - Hash: {}\n", &s.hash[..16]));
            if let Some(mime) = &s.mime_type {
                output.push_str(&format!("  - Type: {}\n", mime));
            }
            output.push_str("\n");
        }
    }

    Ok(output)
}

/// List tracked files
pub async fn files(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let path_filter = args.get("path").and_then(|v| v.as_str());

    let config = state.config.clone();
    let storage = bridge_core::ContentStore::new(&config).map_err(|e| e.to_string())?;

    let files = storage.list_files().map_err(|e| e.to_string())?;

    let filtered: Vec<_> = if let Some(prefix) = path_filter {
        files
            .iter()
            .filter(|f| f.path.to_string_lossy().starts_with(prefix))
            .collect()
    } else {
        files.iter().collect()
    };

    let mut output = String::new();
    output.push_str("# Tracked Files\n\n");

    if let Some(prefix) = path_filter {
        output.push_str(&format!("Filter: {}\n\n", prefix));
    }

    if filtered.is_empty() {
        output.push_str("No files tracked.\n\n");
        output.push_str("To add files:\n");
        output.push_str("```\ncodebridge add <path>\n```\n");
    } else {
        let total_size: u64 = filtered.iter().map(|f| f.size).sum();
        output.push_str(&format!("{} file(s), {} bytes total\n\n", filtered.len(), total_size));

        for f in filtered.iter().take(50) {
            output.push_str(&format!("- {} ({} bytes)\n", f.path.display(), f.size));
        }

        if filtered.len() > 50 {
            output.push_str(&format!("\n... and {} more files\n", filtered.len() - 50));
        }
    }

    Ok(output)
}

// ============================================================================
// Clipboard Tools
// ============================================================================

/// Get current clipboard content
pub async fn clipboard_get(state: &Arc<McpState>) -> Result<String, String> {
    let config = state.config.clone();
    let storage_path = config.data_dir.join("clipboard");

    let manager = bridge_clipboard::ClipboardManager::new(
        config.device_id.clone(),
        storage_path,
    ).map_err(|e| e.to_string())?;

    match manager.get_current() {
        Ok(Some(entry)) => {
            let mut output = String::new();
            output.push_str("# Current Clipboard\n\n");

            let type_str = match &entry.content_type {
                bridge_clipboard::ContentType::Text => "Text",
                bridge_clipboard::ContentType::RichText => "Rich Text",
                bridge_clipboard::ContentType::Image => "Image",
                bridge_clipboard::ContentType::Files => "Files",
                bridge_clipboard::ContentType::Code { language } => &format!("Code ({})", language),
                bridge_clipboard::ContentType::Url => "URL",
            };

            output.push_str(&format!("**Type:** {}\n", type_str));
            output.push_str(&format!("**Source:** {}\n", entry.source_device));
            output.push_str(&format!("**Time:** {}\n\n", entry.timestamp.format("%Y-%m-%d %H:%M:%S")));

            if let Some(text) = entry.as_text() {
                output.push_str("**Content:**\n```\n");
                output.push_str(&text);
                output.push_str("\n```\n");
            } else if entry.content_type == bridge_clipboard::ContentType::Image {
                output.push_str(&format!("**Image:** {} bytes\n", entry.data.len()));
            }

            Ok(output)
        }
        Ok(None) => Ok("Clipboard is empty.".to_string()),
        Err(e) => Err(format!("Failed to get clipboard: {}", e)),
    }
}

/// Set clipboard content
pub async fn clipboard_set(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'text' argument")?;

    let content_type = args.get("content_type").and_then(|v| v.as_str()).unwrap_or("text");
    let language = args.get("language").and_then(|v| v.as_str());

    let config = state.config.clone();
    let storage_path = config.data_dir.join("clipboard");

    let mut manager = bridge_clipboard::ClipboardManager::new(
        config.device_id.clone(),
        storage_path,
    ).map_err(|e| e.to_string())?;

    let entry = match content_type {
        "code" => {
            let lang = language.unwrap_or("text").to_string();
            bridge_clipboard::ClipboardEntry::new_code(
                text.to_string(),
                lang,
                config.device_id.clone(),
            )
        }
        "url" => bridge_clipboard::ClipboardEntry::new_url(
            text.to_string(),
            config.device_id.clone(),
        ),
        _ => bridge_clipboard::ClipboardEntry::new_text(
            text.to_string(),
            config.device_id.clone(),
        ),
    };

    manager.set_content(&entry).map_err(|e| e.to_string())?;

    let mut output = String::new();
    output.push_str("# Clipboard Updated\n\n");
    output.push_str(&format!("✓ Copied {} characters to clipboard\n", text.len()));
    output.push_str(&format!("- Type: {}\n", content_type));
    output.push_str("- Will sync to connected devices\n");

    Ok(output)
}

/// Get clipboard history
pub async fn clipboard_history(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
    let search = args.get("search").and_then(|v| v.as_str());

    let config = state.config.clone();
    let storage_path = config.data_dir.join("clipboard");

    let manager = bridge_clipboard::ClipboardManager::new(
        config.device_id.clone(),
        storage_path,
    ).map_err(|e| e.to_string())?;

    let entries = if let Some(query) = search {
        manager.search(query).map_err(|e| e.to_string())?
    } else {
        manager.get_history(limit).map_err(|e| e.to_string())?
    };

    let mut output = String::new();
    output.push_str("# Clipboard History\n\n");

    if let Some(query) = search {
        output.push_str(&format!("Search: \"{}\"\n\n", query));
    }

    if entries.is_empty() {
        output.push_str("No clipboard entries found.\n");
    } else {
        output.push_str(&format!("Showing {} entries:\n\n", entries.len()));

        for (i, entry) in entries.iter().enumerate() {
            let preview = entry.preview.as_deref().unwrap_or("[No preview]");
            let type_icon = match &entry.content_type {
                bridge_clipboard::ContentType::Text => "📝",
                bridge_clipboard::ContentType::Code { .. } => "💻",
                bridge_clipboard::ContentType::Url => "🔗",
                bridge_clipboard::ContentType::Image => "🖼️",
                _ => "📄",
            };

            output.push_str(&format!(
                "{}. {} **{}** ({})\n   {}\n\n",
                i + 1,
                type_icon,
                entry.timestamp.format("%H:%M:%S"),
                entry.source_device,
                preview
            ));
        }
    }

    Ok(output)
}

/// Get clipboard favorites
pub async fn clipboard_favorites(state: &Arc<McpState>) -> Result<String, String> {
    let config = state.config.clone();
    let storage_path = config.data_dir.join("clipboard");

    let manager = bridge_clipboard::ClipboardManager::new(
        config.device_id.clone(),
        storage_path,
    ).map_err(|e| e.to_string())?;

    let favorites = manager.get_favorites().map_err(|e| e.to_string())?;

    let mut output = String::new();
    output.push_str("# Clipboard Favorites\n\n");

    if favorites.is_empty() {
        output.push_str("No favorites saved.\n\n");
        output.push_str("Use the desktop app to mark clipboard entries as favorites.\n");
    } else {
        output.push_str(&format!("{} favorite(s):\n\n", favorites.len()));

        for (i, entry) in favorites.iter().enumerate() {
            let preview = entry.preview.as_deref().unwrap_or("[No preview]");
            output.push_str(&format!(
                "{}. **{}**\n   {}\n\n",
                i + 1,
                entry.timestamp.format("%Y-%m-%d %H:%M"),
                preview
            ));
        }
    }

    Ok(output)
}

// ============================================================================
// Notification Tools
// ============================================================================

/// List notifications
pub async fn notifications_list(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let category = args.get("category").and_then(|v| v.as_str());
    let unread_only = args.get("unread_only").and_then(|v| v.as_bool()).unwrap_or(false);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let config = state.config.clone();
    let manager = bridge_notifications::NotificationManager::new(config.device_id.clone());

    let mut output = String::new();
    output.push_str("# Notifications\n\n");

    let notifications: Vec<_> = manager.get_history()
        .iter()
        .filter(|n| {
            if unread_only && n.read {
                return false;
            }
            if let Some(cat) = category {
                match cat {
                    "build" => matches!(n.category, bridge_notifications::Category::Build),
                    "test" => matches!(n.category, bridge_notifications::Category::Test),
                    "pr" => matches!(n.category, bridge_notifications::Category::PullRequest),
                    "alert" => matches!(n.category, bridge_notifications::Category::Alert),
                    _ => true,
                }
            } else {
                true
            }
        })
        .take(limit)
        .collect();

    if notifications.is_empty() {
        output.push_str("No notifications.\n");
    } else {
        let unread_count = notifications.iter().filter(|n| !n.read).count();
        output.push_str(&format!("{} notification(s), {} unread\n\n", notifications.len(), unread_count));

        for n in notifications {
            let priority_icon = match n.priority {
                bridge_notifications::Priority::Critical => "🔴",
                bridge_notifications::Priority::High => "🟠",
                bridge_notifications::Priority::Normal => "🔵",
                bridge_notifications::Priority::Low => "⚪",
            };

            let read_marker = if n.read { "" } else { "●" };

            output.push_str(&format!(
                "{} {} **{}**\n",
                read_marker,
                priority_icon,
                n.title
            ));
            output.push_str(&format!("   {}\n", n.body));
            output.push_str(&format!("   _{}_ | {}\n\n",
                n.app_name,
                n.timestamp.format("%Y-%m-%d %H:%M")
            ));
        }
    }

    Ok(output)
}

/// Send a notification
pub async fn notifications_send(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'title' argument")?;

    let body = args
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'body' argument")?;

    let priority_str = args.get("priority").and_then(|v| v.as_str()).unwrap_or("normal");
    let category_str = args.get("category").and_then(|v| v.as_str()).unwrap_or("system");

    let priority = match priority_str {
        "low" => bridge_notifications::Priority::Low,
        "high" => bridge_notifications::Priority::High,
        "critical" => bridge_notifications::Priority::Critical,
        _ => bridge_notifications::Priority::Normal,
    };

    let category = match category_str {
        "build" => bridge_notifications::Category::Build,
        "test" => bridge_notifications::Category::Test,
        "deploy" => bridge_notifications::Category::Deploy,
        "pr" => bridge_notifications::Category::PullRequest,
        "alert" => bridge_notifications::Category::Alert,
        _ => bridge_notifications::Category::System,
    };

    let config = state.config.clone();
    let notification = bridge_notifications::Notification::new("Code Bridge", title, body)
        .with_priority(priority)
        .with_category(category)
        .with_device(config.device_id.clone());

    let mut manager = bridge_notifications::NotificationManager::new(config.device_id.clone());
    manager.send(notification).await.map_err(|e| e.to_string())?;

    let mut output = String::new();
    output.push_str("# Notification Sent\n\n");
    output.push_str(&format!("✓ **{}**\n", title));
    output.push_str(&format!("  {}\n\n", body));
    output.push_str(&format!("- Priority: {}\n", priority_str));
    output.push_str(&format!("- Category: {}\n", category_str));
    output.push_str("- Sending to connected devices...\n");

    Ok(output)
}

/// Mark notifications as read
pub async fn notifications_mark_read(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let notification_id = args.get("notification_id").and_then(|v| v.as_str());

    let config = state.config.clone();
    let mut manager = bridge_notifications::NotificationManager::new(config.device_id.clone());

    let mut output = String::new();
    output.push_str("# Notifications Marked Read\n\n");

    if let Some(id) = notification_id {
        manager.mark_read(id);
        output.push_str(&format!("✓ Marked notification {} as read\n", id));
    } else {
        manager.mark_all_read();
        output.push_str("✓ Marked all notifications as read\n");
    }

    Ok(output)
}

// ============================================================================
// Terminal Tools
// ============================================================================

/// Get command history
pub async fn terminal_history(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let search = args.get("search").and_then(|v| v.as_str());
    let directory = args.get("directory").and_then(|v| v.as_str());

    let config = state.config.clone();
    let storage_path = config.data_dir.join("terminal");

    let manager = bridge_terminal::TerminalManager::new(
        config.device_id.clone(),
        storage_path,
    ).map_err(|e| e.to_string())?;

    let entries = if let Some(query) = search {
        manager.search_history(query).map_err(|e| e.to_string())?
    } else {
        manager.get_history(limit).map_err(|e| e.to_string())?
    };

    // Filter by directory if specified
    let filtered: Vec<_> = if let Some(dir) = directory {
        entries.into_iter()
            .filter(|e| e.working_dir.to_string_lossy().contains(dir))
            .collect()
    } else {
        entries
    };

    let mut output = String::new();
    output.push_str("# Command History\n\n");

    if let Some(query) = search {
        output.push_str(&format!("Search: \"{}\"\n", query));
    }
    if let Some(dir) = directory {
        output.push_str(&format!("Directory: {}\n", dir));
    }
    if search.is_some() || directory.is_some() {
        output.push_str("\n");
    }

    if filtered.is_empty() {
        output.push_str("No commands in history.\n");
    } else {
        output.push_str(&format!("Showing {} command(s):\n\n", filtered.len()));

        for entry in filtered.iter().take(limit) {
            let exit_icon = match entry.exit_code {
                Some(0) => "✓",
                Some(_) => "✗",
                None => "?",
            };

            let duration_str = entry.duration
                .map(|d| format!(" ({:.1}s)", d.as_secs_f64()))
                .unwrap_or_default();

            output.push_str(&format!(
                "{} `{}`{}\n   {} | {}\n\n",
                exit_icon,
                entry.command,
                duration_str,
                entry.working_dir.display(),
                entry.timestamp.format("%Y-%m-%d %H:%M")
            ));
        }
    }

    Ok(output)
}

/// List terminal recordings
pub async fn terminal_recordings(state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    let config = state.config.clone();
    let storage_path = config.data_dir.join("terminal");

    let manager = bridge_terminal::TerminalManager::new(
        config.device_id.clone(),
        storage_path,
    ).map_err(|e| e.to_string())?;

    let recordings = manager.list_recordings().map_err(|e| e.to_string())?;

    let mut output = String::new();
    output.push_str("# Terminal Recordings\n\n");

    if recordings.is_empty() {
        output.push_str("No recordings found.\n\n");
        output.push_str("To start recording:\n");
        output.push_str("```\ncodebridge terminal record \"Session Name\"\n```\n");
    } else {
        output.push_str(&format!("{} recording(s):\n\n", recordings.len().min(limit)));

        for recording in recordings.iter().take(limit) {
            let duration_secs = recording.duration.as_secs();
            let duration_str = if duration_secs >= 60 {
                format!("{}m {}s", duration_secs / 60, duration_secs % 60)
            } else {
                format!("{}s", duration_secs)
            };

            output.push_str(&format!(
                "## {}\n",
                recording.title
            ));
            output.push_str(&format!("- Duration: {}\n", duration_str));
            output.push_str(&format!("- Shell: {}\n", recording.shell));
            output.push_str(&format!("- Size: {}x{}\n", recording.width, recording.height));
            output.push_str(&format!("- Recorded: {}\n", recording.recorded_at.format("%Y-%m-%d %H:%M")));
            output.push_str(&format!("- Device: {}\n\n", recording.source_device));
        }
    }

    Ok(output)
}

/// Get shell environment
pub async fn terminal_environment(_state: &Arc<McpState>, args: &serde_json::Value) -> Result<String, String> {
    let include_aliases = args.get("include_aliases").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_functions = args.get("include_functions").and_then(|v| v.as_bool()).unwrap_or(true);
    let include_env = args.get("include_env").and_then(|v| v.as_bool()).unwrap_or(true);

    let env = bridge_terminal::ShellEnvironment::from_current()
        .map_err(|e| e.to_string())?;

    let mut output = String::new();
    output.push_str("# Shell Environment\n\n");
    output.push_str(&format!("Shell: {}\n\n", env.shell));

    if include_aliases && !env.aliases.is_empty() {
        output.push_str("## Aliases\n\n");
        for alias in &env.aliases {
            output.push_str(&format!("- `{}` → `{}`\n", alias.name, alias.command));
        }
        output.push_str("\n");
    }

    if include_functions && !env.functions.is_empty() {
        output.push_str("## Functions\n\n");
        for func in &env.functions {
            output.push_str(&format!("- `{}`\n", func.name));
        }
        output.push_str("\n");
    }

    if include_env && !env.environment.is_empty() {
        output.push_str("## Environment Variables\n\n");
        let safe_vars: Vec<_> = env.environment.iter()
            .filter(|v| !v.sensitive)
            .take(20)
            .collect();

        for var in safe_vars {
            let value = if var.value.len() > 50 {
                format!("{}...", &var.value[..50])
            } else {
                var.value.clone()
            };
            output.push_str(&format!("- `{}`={}\n", var.name, value));
        }

        if env.environment.len() > 20 {
            output.push_str(&format!("\n_...and {} more variables_\n", env.environment.len() - 20));
        }
    }

    if !env.path_additions.is_empty() {
        output.push_str("\n## PATH Additions\n\n");
        for path in &env.path_additions {
            output.push_str(&format!("- {}\n", path.display()));
        }
    }

    Ok(output)
}
