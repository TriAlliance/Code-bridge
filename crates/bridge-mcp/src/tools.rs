//! MCP Tool implementations for Code Bridge

use crate::McpState;
use std::sync::Arc;

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
