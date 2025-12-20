//! Coachly Code Bridge CLI
//!
//! A Git-like command-line interface for P2P file sharing.

use anyhow::{Context, Result};
use bridge_core::{Bridge, Config, FileWatcher};
use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "codebridge")]
#[command(author = "Coachly Team")]
#[command(version)]
#[command(about = "P2P file sharing for developers", long_about = None)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new project for syncing
    Init {
        /// Project name
        name: Option<String>,

        /// Project path (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// Add files or directories to sync
    Add {
        /// Paths to add
        paths: Vec<PathBuf>,
    },

    /// Show sync status
    Status,

    /// Sync files with peers
    Sync {
        /// Watch for changes and sync continuously
        #[arg(short, long)]
        watch: bool,
    },

    /// Manage peers
    Peer {
        #[command(subcommand)]
        command: PeerCommands,
    },

    /// Screenshot management
    Screenshot {
        #[command(subcommand)]
        command: ScreenshotCommands,
    },

    /// Start the daemon
    Daemon {
        /// Run in foreground (don't daemonize)
        #[arg(short, long)]
        foreground: bool,
    },

    /// Show configuration
    Config {
        /// Edit configuration
        #[arg(short, long)]
        edit: bool,
    },

    /// Start MCP server for Claude Code integration
    Mcp {
        /// Transport: stdio (default) or sse
        #[arg(short, long, default_value = "stdio")]
        transport: String,

        /// Port for SSE transport
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
}

#[derive(Subcommand)]
enum PeerCommands {
    /// Discover peers on the network
    Discover,

    /// List connected peers
    List,

    /// Add a peer
    Add {
        /// Peer identifier or address
        peer: String,
    },

    /// Remove a peer
    Remove {
        /// Peer identifier
        peer: String,
    },
}

#[derive(Subcommand)]
enum ScreenshotCommands {
    /// Watch a directory for new screenshots
    Watch {
        /// Directory to watch (defaults to ~/Screenshots)
        #[arg(short, long)]
        path: Option<PathBuf>,
    },

    /// List recent screenshots
    List {
        /// Number of screenshots to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Search screenshots by OCR text
    Search {
        /// Search query
        query: String,
    },

    /// Open screenshot browser
    Browse,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let filter = if cli.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();

    match cli.command {
        Commands::Init { name, path } => cmd_init(name, path).await,
        Commands::Add { paths } => cmd_add(paths).await,
        Commands::Status => cmd_status().await,
        Commands::Sync { watch } => cmd_sync(watch).await,
        Commands::Peer { command } => cmd_peer(command).await,
        Commands::Screenshot { command } => cmd_screenshot(command).await,
        Commands::Daemon { foreground } => cmd_daemon(foreground).await,
        Commands::Config { edit } => cmd_config(edit).await,
        Commands::Mcp { transport, port } => cmd_mcp(transport, port).await,
    }
}

async fn cmd_init(name: Option<String>, path: Option<PathBuf>) -> Result<()> {
    let project_path = path.unwrap_or_else(|| std::env::current_dir().unwrap());
    let project_name = name.unwrap_or_else(|| {
        project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string()
    });

    println!("{} Initializing project: {}", "→".blue(), project_name.bold());

    // Create .codebridge directory
    let bridge_dir = project_path.join(".codebridge");
    std::fs::create_dir_all(&bridge_dir)?;

    // Create project config
    let project_config = serde_json::json!({
        "name": project_name,
        "version": "1.0.0",
        "created_at": chrono::Utc::now().to_rfc3339(),
        "sync": {
            "paths": ["src", "docs"],
            "ignore": [".git", "node_modules", "target", "*.pyc"]
        }
    });

    let config_path = bridge_dir.join("project.json");
    std::fs::write(&config_path, serde_json::to_string_pretty(&project_config)?)?;

    println!("{} Created {}", "✓".green(), config_path.display());
    println!();
    println!("Project initialized! Next steps:");
    println!("  {} Add files to sync", "1.".dimmed());
    println!("     {} add src/ docs/", "codebridge".cyan());
    println!("  {} Discover peers on your network", "2.".dimmed());
    println!("     {} peer discover", "codebridge".cyan());
    println!("  {} Start syncing", "3.".dimmed());
    println!("     {} sync --watch", "codebridge".cyan());

    Ok(())
}

async fn cmd_add(paths: Vec<PathBuf>) -> Result<()> {
    if paths.is_empty() {
        println!("{} No paths specified", "!".yellow());
        return Ok(());
    }

    let config = Config::load_or_create()?;
    let mut storage = bridge_core::ContentStore::new(&config)?;

    let progress = ProgressBar::new(paths.len() as u64);
    progress.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("█▓░"),
    );

    let mut total_files = 0;
    let mut total_size = 0u64;

    for path in &paths {
        progress.set_message(format!("{}", path.display()));

        if path.is_dir() {
            for entry in walkdir::WalkDir::new(path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let file_path = entry.path();
                let relative = file_path.strip_prefix(path).unwrap_or(file_path);

                if let Ok(entry) = storage.store_file(file_path, relative) {
                    total_files += 1;
                    total_size += entry.size;
                }
            }
        } else if path.is_file() {
            let relative = path.file_name().map(PathBuf::from).unwrap_or_default();
            if let Ok(entry) = storage.store_file(path, &relative) {
                total_files += 1;
                total_size += entry.size;
            }
        }

        progress.inc(1);
    }

    progress.finish_and_clear();

    let size_str = humansize::format_size(total_size, humansize::BINARY);
    println!(
        "{} Added {} files ({})",
        "✓".green(),
        total_files.to_string().bold(),
        size_str
    );

    Ok(())
}

async fn cmd_status() -> Result<()> {
    let config = Config::load_or_create()?;
    let storage = bridge_core::ContentStore::new(&config)?;

    println!("{}", "Code Bridge Status".bold());
    println!("{}", "─".repeat(40).dimmed());

    // Show device info
    println!("Device:     {}", config.device_name.cyan());
    println!("Device ID:  {}", &config.device_id[..8].dimmed());
    println!();

    // Show stored files
    let files = storage.list_files()?;
    println!("{} {} files tracked", "Files:".bold(), files.len());

    let total_size: u64 = files.iter().map(|f| f.size).sum();
    println!(
        "{} {}",
        "Size:".bold(),
        humansize::format_size(total_size, humansize::BINARY)
    );

    println!();

    // TODO: Show peer status once network is running
    println!("{} Discovering peers...", "Peers:".bold());
    println!("  {} Run {} to start", "(daemon not running)".dimmed(), "codebridge daemon".cyan());

    Ok(())
}

async fn cmd_sync(watch: bool) -> Result<()> {
    println!("{} Starting sync...", "→".blue());

    let mut bridge = Bridge::new().await?;
    bridge.start().await?;

    if watch {
        println!("{} Watching for changes (Ctrl+C to stop)", "→".blue());

        // Set up file watcher
        let current_dir = std::env::current_dir()?;
        bridge.watcher_mut().watch(&current_dir)?;

        // Poll for events
        loop {
            // Check for file changes
            let changes = bridge.watcher().poll();
            for change in changes {
                match change {
                    bridge_core::watch::FileChange::Modified(path) => {
                        println!("  {} Modified: {}", "~".yellow(), path.display());
                    }
                    bridge_core::watch::FileChange::Created(path) => {
                        println!("  {} Created: {}", "+".green(), path.display());
                    }
                    bridge_core::watch::FileChange::Deleted(path) => {
                        println!("  {} Deleted: {}", "-".red(), path.display());
                    }
                    bridge_core::watch::FileChange::Renamed(from, to) => {
                        println!("  {} Renamed: {} → {}", "→".blue(), from.display(), to.display());
                    }
                }
            }

            // Poll network events
            if let Some(event) = bridge.network_mut().poll().await {
                match event {
                    bridge_core::p2p::NetworkEvent::PeerDiscovered { peer_id, .. } => {
                        println!("  {} Peer discovered: {}", "●".green(), peer_id);
                    }
                    bridge_core::p2p::NetworkEvent::PeerDisconnected { peer_id } => {
                        println!("  {} Peer disconnected: {}", "○".red(), peer_id);
                    }
                    _ => {}
                }
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    } else {
        // One-time sync
        println!("{} Sync complete", "✓".green());
    }

    Ok(())
}

async fn cmd_peer(command: PeerCommands) -> Result<()> {
    match command {
        PeerCommands::Discover => {
            println!("{} Discovering peers on the network...", "→".blue());

            let config = Config::load_or_create()?;
            let mut network = bridge_core::PeerNetwork::new(&config).await?;
            network.start().await?;

            println!("  {} Local peer ID: {}", "●".green(), network.local_peer_id());
            println!();
            println!("Listening for peers (30 seconds)...");

            let timeout = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
            let mut found_peers = 0;

            while tokio::time::Instant::now() < timeout {
                if let Some(event) = network.poll().await {
                    if let bridge_core::p2p::NetworkEvent::PeerDiscovered { peer_id, addresses } = event {
                        found_peers += 1;
                        println!(
                            "  {} Found: {} at {}",
                            "✓".green(),
                            peer_id.to_string()[..16].to_string().cyan(),
                            addresses.first().map(|a| a.to_string()).unwrap_or_default()
                        );
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }

            if found_peers == 0 {
                println!();
                println!("{} No peers found. Make sure:", "!".yellow());
                println!("  • Other devices are running Code Bridge");
                println!("  • Devices are on the same network");
            }
        }

        PeerCommands::List => {
            println!("{}", "Connected Peers".bold());
            println!("{}", "─".repeat(40).dimmed());
            println!("  {} Run {} first", "(start daemon)".dimmed(), "codebridge daemon".cyan());
        }

        PeerCommands::Add { peer } => {
            println!("{} Adding peer: {}", "→".blue(), peer);
            // TODO: Implement peer addition
            println!("{} Peer added", "✓".green());
        }

        PeerCommands::Remove { peer } => {
            println!("{} Removing peer: {}", "→".blue(), peer);
            // TODO: Implement peer removal
            println!("{} Peer removed", "✓".green());
        }
    }

    Ok(())
}

async fn cmd_screenshot(command: ScreenshotCommands) -> Result<()> {
    match command {
        ScreenshotCommands::Watch { path } => {
            let watch_path = path.unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join("Screenshots"))
                    .unwrap_or_else(|| PathBuf::from("~/Screenshots"))
            });

            println!(
                "{} Watching for screenshots in: {}",
                "→".blue(),
                watch_path.display()
            );

            let mut watcher = bridge_core::watch::ScreenshotWatcher::new()?;
            watcher.watch(&watch_path)?;

            println!("Press Ctrl+C to stop...");

            loop {
                let screenshots = watcher.poll();
                for screenshot in screenshots {
                    println!(
                        "  {} New screenshot: {}",
                        "📷".to_string(),
                        screenshot.display()
                    );
                    // TODO: Process screenshot (compress, OCR, sync)
                }

                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        ScreenshotCommands::List { limit } => {
            println!("{}", "Recent Screenshots".bold());
            println!("{}", "─".repeat(40).dimmed());

            let config = Config::load_or_create()?;
            let storage = bridge_core::ContentStore::new(&config)?;

            let files = storage.list_files()?;
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

            if screenshots.is_empty() {
                println!("  {} No screenshots found", "(none)".dimmed());
            } else {
                for s in screenshots {
                    let size = humansize::format_size(s.size, humansize::BINARY);
                    println!(
                        "  {} {} ({})",
                        "•",
                        s.path.display(),
                        size.dimmed()
                    );
                }
            }
        }

        ScreenshotCommands::Search { query } => {
            println!("{} Searching screenshots for: {}", "→".blue(), query.italic());
            // TODO: Implement OCR search
            println!("  {} OCR search not yet implemented", "(coming soon)".dimmed());
        }

        ScreenshotCommands::Browse => {
            println!("{} Opening screenshot browser...", "→".blue());
            // TODO: Open web browser or TUI
            println!("  {} Browser not yet implemented", "(coming soon)".dimmed());
        }
    }

    Ok(())
}

async fn cmd_daemon(foreground: bool) -> Result<()> {
    if foreground {
        println!("{} Starting Code Bridge daemon...", "→".blue());
    } else {
        println!("{} Starting Code Bridge daemon in background...", "→".blue());
        // TODO: Implement daemonization
    }

    let mut bridge = Bridge::new().await?;
    bridge.start().await?;

    println!("{} Daemon started", "✓".green());
    println!("  Peer ID: {}", bridge.network().local_peer_id());

    // Keep running
    loop {
        if let Some(event) = bridge.network_mut().poll().await {
            info!("Network event: {:?}", event);
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

async fn cmd_config(edit: bool) -> Result<()> {
    let config_path = Config::config_path()?;

    if edit {
        // Open in editor
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".to_string());
        std::process::Command::new(&editor)
            .arg(&config_path)
            .status()?;
    } else {
        let config = Config::load_or_create()?;

        println!("{}", "Code Bridge Configuration".bold());
        println!("{}", "─".repeat(40).dimmed());
        println!();
        println!("Config file: {}", config_path.display().to_string().dimmed());
        println!();
        println!("{}", serde_json::to_string_pretty(&config)?);
    }

    Ok(())
}

async fn cmd_mcp(transport: String, port: u16) -> Result<()> {
    println!("{} Starting MCP server...", "→".blue());
    println!("  Transport: {}", transport.cyan());

    if transport == "sse" {
        println!("  Port: {}", port);
    }

    // TODO: Implement MCP server
    println!();
    println!("{} MCP server not yet implemented", "!".yellow());
    println!("This will allow Claude Code to access the bridge via tools:");
    println!("  • {} - Share a file with peers", "bridge_share".cyan());
    println!("  • {} - Sync all files", "bridge_sync".cyan());
    println!("  • {} - List available peers", "bridge_peers".cyan());
    println!("  • {} - List and search screenshots", "bridge_screenshots".cyan());

    Ok(())
}
