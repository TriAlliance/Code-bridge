# Quick Start Guide: Code-Bridge

This guide will help you get started building the cross-platform file sharing application based on the recommended architecture.

## Prerequisites

### All Platforms
- **Rust**: 1.77+ (install via [rustup](https://rustup.rs/))
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

### macOS/iOS Development
- **Xcode**: 15.0+ (from App Store)
- **Command Line Tools**:
  ```bash
  xcode-select --install
  ```
- **Homebrew** (optional, for dependencies):
  ```bash
  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
  ```

### Linux Development
- **Ubuntu/Debian**:
  ```bash
  sudo apt update
  sudo apt install -y build-essential pkg-config libssl-dev \
    libgtk-4-dev libadwaita-1-dev # For GTK4 option
  ```

- **Fedora**:
  ```bash
  sudo dnf install -y gcc pkg-config openssl-devel \
    gtk4-devel libadwaita-devel # For GTK4 option
  ```

## Project Setup

### 1. Initialize the Monorepo

```bash
# Create project structure
mkdir -p code-bridge/{crates,platforms,schemas,scripts,docs}
cd code-bridge

# Initialize git
git init
echo "target/\n.DS_Store\n*.xcuserstate\nnode_modules/" > .gitignore

# Create Cargo workspace
cat > Cargo.toml << 'EOF'
[workspace]
resolver = "2"
members = [
    "crates/core",
    "crates/ffi-swift",
    "crates/cli",
    "platforms/linux/gtk4",  # or platforms/linux/tauri
]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT OR Apache-2.0"
repository = "https://github.com/yourusername/code-bridge"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.40", features = ["full"] }

# Networking
libp2p = { version = "0.54", features = ["tcp", "mdns", "noise", "yamux", "dns", "tokio"] }
quinn = "0.11"

# File watching
notify = { version = "8.2", features = ["macos_fsevent"] }
notify-debouncer-full = "0.4"

# Serialization
flatbuffers = "24.3"
# OR: prost = "0.13"

# FFI
swift-bridge = "0.1"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Async utilities
futures = "0.3"
async-trait = "0.1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
EOF

# Pin Rust version
cat > rust-toolchain.toml << 'EOF'
[toolchain]
channel = "1.82.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = [
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "aarch64-apple-ios",
    "x86_64-apple-ios",
]
EOF
```

### 2. Create Core Rust Library

```bash
# Create core library
cargo new --lib crates/core
cd crates/core

cat > Cargo.toml << 'EOF'
[package]
name = "code-bridge-core"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
libp2p.workspace = true
notify.workspace = true
notify-debouncer-full.workspace = true
anyhow.workspace = true
thiserror.workspace = true
tracing.workspace = true
futures.workspace = true
async-trait.workspace = true
flatbuffers.workspace = true
EOF
```

Create the core library structure:

```bash
mkdir -p src/{networking,file_watcher,transfer}

cat > src/lib.rs << 'EOF'
//! Code-Bridge Core Library
//!
//! Cross-platform file sharing core functionality

pub mod networking;
pub mod file_watcher;
pub mod transfer;

pub use networking::P2PNetwork;
pub use file_watcher::FileWatcher;
pub use transfer::FileTransfer;

/// Core error types
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Network error: {0}")]
    Network(String),

    #[error("File watcher error: {0}")]
    FileWatcher(String),

    #[error("Transfer error: {0}")]
    Transfer(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
EOF

cat > src/file_watcher.rs << 'EOF'
//! Cross-platform file watching using notify

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, FileIdMap};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;
use crate::{Error, Result};

pub struct FileWatcher {
    path: PathBuf,
    tx: Option<mpsc::UnboundedSender<Vec<PathBuf>>>,
}

impl FileWatcher {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            tx: None,
        }
    }

    /// Start watching the directory for changes
    pub async fn start(&mut self) -> Result<mpsc::UnboundedReceiver<Vec<PathBuf>>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let tx_clone = tx.clone();

        // Create debouncer
        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        let paths: Vec<PathBuf> = events
                            .iter()
                            .flat_map(|e| e.paths.clone())
                            .collect();

                        if !paths.is_empty() {
                            let _ = tx_clone.send(paths);
                        }
                    }
                    Err(e) => tracing::error!("Watch error: {:?}", e),
                }
            },
        ).map_err(|e| Error::FileWatcher(e.to_string()))?;

        // Watch directory
        debouncer.watcher()
            .watch(&self.path, RecursiveMode::Recursive)
            .map_err(|e| Error::FileWatcher(e.to_string()))?;

        self.tx = Some(tx);

        // Keep debouncer alive (in production, store in struct)
        tokio::spawn(async move {
            // Debouncer must stay alive
            let _debouncer = debouncer;
            tokio::signal::ctrl_c().await.ok();
        });

        Ok(rx)
    }
}
EOF

cat > src/networking.rs << 'EOF'
//! P2P networking using libp2p

use libp2p::{
    core::upgrade,
    mdns, noise,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp, yamux, PeerId, Swarm, Transport,
};
use std::error::Error;
use tokio::select;
use crate::Result;

pub struct P2PNetwork {
    swarm: Option<Swarm<mdns::tokio::Behaviour>>,
    peer_id: PeerId,
}

impl P2PNetwork {
    pub fn new() -> Result<Self> {
        let peer_id = PeerId::random();

        Ok(Self {
            swarm: None,
            peer_id,
        })
    }

    pub async fn start(&mut self, port: u16) -> Result<()> {
        // This is a simplified example
        // In production, implement proper behaviour with protocols

        tracing::info!("Starting P2P network on port {}", port);
        tracing::info!("Peer ID: {}", self.peer_id);

        // TODO: Full libp2p implementation
        // - Custom protocol for file transfer
        // - DHT for peer discovery
        // - Request/response protocol

        Ok(())
    }

    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
}
EOF

cat > src/transfer.rs << 'EOF'
//! File transfer protocol

use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::{Error, Result};

const CHUNK_SIZE: usize = 64 * 1024; // 64 KB chunks

pub struct FileTransfer {
    file_path: PathBuf,
}

impl FileTransfer {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            file_path: path.into(),
        }
    }

    /// Read file in chunks
    pub async fn read_chunks(&self) -> Result<Vec<Vec<u8>>> {
        let mut file = File::open(&self.file_path).await?;
        let mut chunks = Vec::new();
        let mut buffer = vec![0u8; CHUNK_SIZE];

        loop {
            let n = file.read(&mut buffer).await?;
            if n == 0 {
                break;
            }
            chunks.push(buffer[..n].to_vec());
        }

        Ok(chunks)
    }

    /// Write chunks to file
    pub async fn write_chunks(&self, chunks: &[Vec<u8>]) -> Result<()> {
        let mut file = File::create(&self.file_path).await?;

        for chunk in chunks {
            file.write_all(chunk).await?;
        }

        file.flush().await?;
        Ok(())
    }

    /// Calculate SHA256 checksum
    pub async fn checksum(&self) -> Result<String> {
        // TODO: Implement SHA256 hashing
        // use sha2::{Sha256, Digest};
        Ok(String::from("placeholder_checksum"))
    }
}
EOF
```

### 3. Create Swift FFI Bridge (for macOS/iOS)

```bash
cd /path/to/code-bridge
cargo new --lib crates/ffi-swift

cat > crates/ffi-swift/Cargo.toml << 'EOF'
[package]
name = "code-bridge-ffi-swift"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["staticlib", "cdylib"]

[dependencies]
code-bridge-core = { path = "../core" }
swift-bridge.workspace = true
tokio.workspace = true

[build-dependencies]
swift-bridge-build = "0.1"
EOF

cat > crates/ffi-swift/build.rs << 'EOF'
fn main() {
    let out_dir = "generated";

    swift_bridge_build::parse_bridges(vec!["src/lib.rs"])
        .write_all_concatenated(out_dir, "code-bridge");
}
EOF

cat > crates/ffi-swift/src/lib.rs << 'EOF'
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type FileWatcher;

        #[swift_bridge(init)]
        fn new(path: String) -> FileWatcher;

        async fn start_watching(&mut self) -> Vec<String>;
    }

    extern "Rust" {
        type P2PNetwork;

        #[swift_bridge(init)]
        fn new() -> P2PNetwork;

        async fn start(&mut self, port: u16) -> bool;

        fn peer_id(&self) -> String;
    }
}

pub struct FileWatcher {
    inner: code_bridge_core::FileWatcher,
    runtime: tokio::runtime::Runtime,
}

impl FileWatcher {
    fn new(path: String) -> Self {
        Self {
            inner: code_bridge_core::FileWatcher::new(path),
            runtime: tokio::runtime::Runtime::new().unwrap(),
        }
    }

    async fn start_watching(&mut self) -> Vec<String> {
        // This is simplified - in production, handle the channel properly
        vec![]
    }
}

pub struct P2PNetwork {
    inner: Option<code_bridge_core::P2PNetwork>,
    runtime: tokio::runtime::Runtime,
}

impl P2PNetwork {
    fn new() -> Self {
        let inner = code_bridge_core::P2PNetwork::new().ok();
        Self {
            inner,
            runtime: tokio::runtime::Runtime::new().unwrap(),
        }
    }

    async fn start(&mut self, port: u16) -> bool {
        if let Some(ref mut network) = self.inner {
            network.start(port).await.is_ok()
        } else {
            false
        }
    }

    fn peer_id(&self) -> String {
        self.inner
            .as_ref()
            .map(|n| n.peer_id().to_string())
            .unwrap_or_default()
    }
}
EOF
```

### 4. Create macOS App (SwiftUI)

```bash
cd platforms
mkdir -p macos
# Use Xcode to create a new macOS app, or use this script:

cat > macos/create_xcode_project.sh << 'EOF'
#!/bin/bash
# This is a placeholder - use Xcode GUI to create project
# File > New > Project > macOS > App
# Name: CodeBridge
# Interface: SwiftUI
# Language: Swift

echo "Please create macOS project in Xcode"
echo "Then add the Rust library as a dependency"
EOF

chmod +x macos/create_xcode_project.sh
```

### 5. Build and Test

```bash
# Build Rust core
cargo build --package code-bridge-core

# Run tests
cargo test --all

# Build FFI library for Swift
cargo build --package code-bridge-ffi-swift --target aarch64-apple-darwin
cargo build --package code-bridge-ffi-swift --target x86_64-apple-darwin

# Create universal binary (macOS)
lipo -create \
    target/aarch64-apple-darwin/debug/libcode_bridge_ffi_swift.a \
    target/x86_64-apple-darwin/debug/libcode_bridge_ffi_swift.a \
    -output target/universal-macos/libcode_bridge_ffi_swift.a
```

## Next Steps

1. **Complete the Core Implementation**
   - Finish libp2p integration
   - Implement file transfer protocol
   - Add encryption support

2. **Build Platform UIs**
   - macOS: SwiftUI with native file picker
   - iOS: SwiftUI with iOS-specific features
   - Linux: Choose Tauri or GTK4 and implement

3. **Setup CI/CD**
   - GitHub Actions for automated testing
   - Cross-platform builds
   - Release automation

4. **Testing**
   - Unit tests for Rust core
   - Integration tests for P2P
   - UI tests for each platform

5. **Distribution**
   - Code signing setup
   - Notarization for macOS
   - Flatpak/AppImage for Linux
   - TestFlight for iOS

## Useful Commands

```bash
# Format code
cargo fmt --all

# Lint
cargo clippy --all-targets --all-features

# Check for security vulnerabilities
cargo audit

# Update dependencies
cargo update

# Build release
cargo build --release

# Generate documentation
cargo doc --no-deps --open
```

## Troubleshooting

### macOS/iOS

**Problem**: Xcode can't find Rust library
**Solution**: Check `Library Search Paths` in Xcode build settings

**Problem**: Code signing errors
**Solution**: Configure signing certificate in Xcode

### Linux

**Problem**: GTK4 headers not found
**Solution**: Install dev packages: `sudo apt install libgtk-4-dev`

**Problem**: Flatpak build fails
**Solution**: Check runtime version matches in manifest

## Resources

- **Rust Book**: https://doc.rust-lang.org/book/
- **Tokio Tutorial**: https://tokio.rs/tokio/tutorial
- **libp2p Docs**: https://docs.rs/libp2p/
- **swift-bridge**: https://github.com/chinedufn/swift-bridge
- **SwiftUI**: https://developer.apple.com/xcode/swiftui/
- **Tauri**: https://v2.tauri.app/
- **GTK4**: https://gtk-rs.org/

## License

This project template is dual-licensed under MIT OR Apache-2.0.
