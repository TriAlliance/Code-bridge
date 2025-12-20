# Cross-Platform File Sharing App: Architecture Recommendations

## Executive Summary

This document provides research-backed recommendations for building a high-performance, native cross-platform file sharing application targeting **macOS**, **Ubuntu/Linux**, and **iOS**.

**Recommended Architecture**: Shared Rust core with platform-specific native UI layers

---

## 1. Core Technology Stack

### 1.1 Shared Business Logic: Rust

**Why Rust?**
- **Performance**: 15-30% faster than Swift for computation-heavy tasks
- **Memory Safety**: No garbage collection, zero-cost abstractions
- **Cross-platform**: Single codebase for core logic across all platforms
- **Excellent Ecosystem**: Strong support for networking, async, and system operations
- **Growing Adoption**: 78% increase in Rust-Swift projects since 2023

**Key Rust Dependencies**:

```toml
[dependencies]
# Async Runtime
tokio = { version = "1.40", features = ["full"] }

# P2P Networking
libp2p = { version = "0.54", features = ["tcp", "mdns", "noise", "yamux"] }

# File System Monitoring
notify = "8.2"  # Cross-platform: FSEvents (macOS), inotify (Linux), kqueue (BSD/iOS)

# Serialization (choose one)
prost = "0.13"  # Protocol Buffers
flatbuffers = "24.3"  # Zero-copy deserialization

# FFI Bridge (platform-specific)
swift-bridge = "0.1"  # For Apple platforms only
# OR
uniffi = "0.28"  # For multi-platform (iOS + Android future)
```

### 1.2 Platform-Specific UI Layers

#### macOS & iOS: SwiftUI + Swift

```swift
// Modern, declarative UI
// Excellent integration with system features
// Native look and feel
// Access to FSEvents, Network framework, etc.
```

**Key Libraries**:
- **SwiftUI**: Modern declarative UI framework
- **Combine**: Reactive programming (integrate with Rust async)
- **Swift Concurrency**: async/await support
- **Network.framework**: Low-level networking (P2P support)

#### Ubuntu/Linux: Choose Based on Requirements

**Option A: Tauri 2.0 (RECOMMENDED for rapid development)**
- Web technologies (HTML/CSS/JS) + Rust backend
- Smallest bundle size (~600KB)
- Cross-platform (can also target macOS/Windows if needed)
- Easy to build attractive UIs with web skills
- Native system integration via Rust

**Option B: GTK4-rs (RECOMMENDED for native GNOME integration)**
- True native widgets
- Best integration with GNOME desktop
- Mature, production-ready
- Used by many Linux applications
- Relm4 framework for Elm-like architecture

**Option C: Qt (via CXX-Qt)**
- Cross-platform if you need Windows support later
- Very mature
- Professional-looking applications
- More complex setup with Rust

---

## 2. Swift-Rust FFI Bridge

### Comparison: swift-bridge vs UniFFI

| Feature | swift-bridge | UniFFI |
|---------|-------------|--------|
| **Focus** | Swift-only, deeper integration | Multi-language (Kotlin, Swift, Python, Ruby) |
| **Performance** | Zero-overhead FFI | Uses serialization layer (slight overhead) |
| **Async Support** | ✅ Yes | ✅ Yes |
| **Cross-platform** | Apple only | iOS + Android + more |
| **Swift Version** | Swift 6.0+ | Broad support |
| **Maturity** | Newer, actively developed | Mozilla-backed, production-ready |
| **Setup Complexity** | Moderate | Higher initial setup |

### Recommendation

**For Apple-only deployment**: Use **swift-bridge**
- Better performance (zero-overhead)
- More ergonomic Swift integration
- Supports async/await bridging
- No serialization overhead

**If Android support is planned**: Use **UniFFI**
- Single FFI layer for multiple platforms
- Well-documented
- Production-proven (used by Firefox, Mozilla VPN)
- Good tooling (cargo-swift plugin)

### Example swift-bridge Setup

```rust
// Rust side
#[swift_bridge::bridge]
mod ffi {
    extern "Rust" {
        type FileWatcher;

        #[swift_bridge(init)]
        fn new(path: String) -> FileWatcher;

        async fn start_watching(&self) -> Vec<String>;
    }
}

pub struct FileWatcher {
    path: String,
}

impl FileWatcher {
    fn new(path: String) -> Self {
        Self { path }
    }

    async fn start_watching(&self) -> Vec<String> {
        // Implementation using notify crate
        vec![]
    }
}
```

```swift
// Swift side (auto-generated)
let watcher = FileWatcher(path: "/Users/...")
Task {
    let changes = await watcher.start_watching()
}
```

---

## 3. P2P Networking Architecture

### Recommended: libp2p

**Why libp2p?**
- Industry standard for P2P (used by IPFS, Ethereum, Polkadot)
- Protocol-agnostic
- Built-in NAT traversal
- mDNS for local discovery
- QUIC/TCP transport options
- Strong Rust support with tokio integration

**Architecture**:

```
┌─────────────────────────────────────────┐
│         Platform UI Layer               │
│   (SwiftUI / Tauri / GTK4)             │
└─────────────────┬───────────────────────┘
                  │ FFI Bridge
┌─────────────────▼───────────────────────┐
│         Rust Core Library               │
│  ┌─────────────────────────────────┐   │
│  │  P2P Network Layer (libp2p)     │   │
│  │  - Peer Discovery (mDNS/DHT)    │   │
│  │  - Transport (QUIC/TCP)         │   │
│  │  - Security (Noise protocol)    │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │  File Watching (notify)         │   │
│  │  - FSEvents (macOS/iOS)         │   │
│  │  - inotify (Linux)              │   │
│  └─────────────────────────────────┘   │
│  ┌─────────────────────────────────┐   │
│  │  File Transfer Protocol         │   │
│  │  - Chunking                     │   │
│  │  - Resumption                   │   │
│  │  - Verification (checksums)     │   │
│  └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### Alternative: Custom Tokio-based Solution

For simpler use cases:
- Use `tokio` for async runtime
- `quinn` for QUIC protocol
- `mdns` for local network discovery
- Custom protocol on top

---

## 4. File Watching Implementation

### notify Crate (v8.2.0)

**Platform Support**:
- **macOS**: FSEvents (recommended) or kqueue
- **iOS**: kqueue
- **Linux**: inotify
- **Fallback**: Polling watcher (cross-platform)

**Usage**:

```rust
use notify::{Watcher, RecursiveMode, recommended_watcher};

fn watch_directory(path: &Path) -> notify::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let mut watcher = recommended_watcher(tx)?;

    watcher.watch(path, RecursiveMode::Recursive)?;

    for res in rx {
        match res {
            Ok(event) => println!("Event: {:?}", event),
            Err(e) => println!("Error: {:?}", e),
        }
    }

    Ok(())
}
```

**Important Considerations**:
- FSEvents on macOS has security restrictions for files not owned by user
- Different file editors trigger different event sequences
- Use debouncing for rapid file changes
- notify-debouncer-full crate for advanced debouncing

---

## 5. Data Serialization

### Comparison

| Format | Best For | Pros | Cons |
|--------|----------|------|------|
| **FlatBuffers** | High-performance file sharing | Zero-copy deserialization, fast access | Larger serialized size |
| **Protocol Buffers** | General purpose | Smaller size, wide adoption | Requires parsing step |
| **Cap'n Proto** | RPC systems | Zero-copy, RPC built-in | Complex, less flexible |

### Recommendation: FlatBuffers

**Why FlatBuffers for file sharing?**
- **Zero-copy deserialization**: Access data without parsing
- **Random access**: Read specific fields without reading entire message
- **Native types**: No variable-length encoding overhead
- **Language support**: Excellent Rust and Swift codegen

**Schema Example**:

```flatbuffers
namespace FileShare;

table FileMetadata {
  name: string;
  size: ulong;
  checksum: string;
  chunk_size: uint;
  total_chunks: uint;
  created_at: ulong;
}

table FileChunk {
  index: uint;
  data: [ubyte];
  checksum: string;
}

table TransferRequest {
  file_id: string;
  metadata: FileMetadata;
}

root_type TransferRequest;
```

---

## 6. Monorepo Structure

```
code-bridge/
├── Cargo.toml                 # Workspace definition
├── rust-toolchain.toml        # Rust version pinning
├── .github/
│   └── workflows/
│       ├── ci.yml            # Cross-platform CI
│       ├── release-macos.yml
│       ├── release-linux.yml
│       └── release-ios.yml
│
├── crates/
│   ├── core/                 # Shared Rust business logic
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── networking/
│   │       ├── file_watcher/
│   │       └── transfer/
│   │
│   ├── ffi-swift/           # Swift FFI bridge
│   │   ├── Cargo.toml
│   │   ├── build.rs         # swift-bridge codegen
│   │   └── src/lib.rs
│   │
│   └── cli/                 # Optional CLI tool
│       ├── Cargo.toml
│       └── src/main.rs
│
├── platforms/
│   ├── macos/               # macOS app
│   │   ├── CodeBridge.xcodeproj
│   │   ├── Sources/
│   │   └── Package.swift    # SPM for Rust lib
│   │
│   ├── ios/                 # iOS app
│   │   ├── CodeBridge.xcodeproj
│   │   └── Sources/
│   │
│   └── linux/
│       ├── tauri/           # Option A: Tauri
│       │   ├── Cargo.toml
│       │   ├── src-tauri/
│       │   └── ui/
│       │
│       └── gtk4/            # Option B: GTK4
│           ├── Cargo.toml
│           └── src/
│
├── schemas/                 # FlatBuffers schemas
│   └── protocol.fbs
│
├── scripts/
│   ├── build-macos.sh
│   ├── build-linux.sh
│   └── notarize-macos.sh
│
└── docs/
    ├── ARCHITECTURE.md
    └── API.md
```

---

## 7. CI/CD Strategy

### GitHub Actions Matrix Build

```yaml
name: CI

on: [push, pull_request]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        rust: [stable, nightly]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - run: cargo test --all-features

  build-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build Rust core
        run: cargo build --release -p core
      - name: Build macOS app
        run: xcodebuild -project platforms/macos/CodeBridge.xcodeproj
      - name: Code sign
        env:
          MACOS_CERTIFICATE: ${{ secrets.MACOS_CERTIFICATE }}
          MACOS_CERTIFICATE_PWD: ${{ secrets.MACOS_CERTIFICATE_PWD }}
        run: ./scripts/codesign-macos.sh

  build-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install GTK4 dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-4-dev
      - name: Build Linux app
        run: cargo build --release -p linux-gtk4
      - name: Create AppImage
        run: ./scripts/create-appimage.sh
```

### Cross-Compilation

Use `cargo-cross` or GitHub Actions matrix:

```bash
# Install cross
cargo install cross

# Build for different targets
cross build --target x86_64-unknown-linux-gnu
cross build --target aarch64-unknown-linux-gnu
cross build --target x86_64-apple-darwin
cross build --target aarch64-apple-darwin
```

---

## 8. Distribution Strategy

### macOS

**Primary: DMG with Notarization**

Advantages:
- Offline notarization stapling (works in air-gapped environments)
- No quarantine dialog (due to user copy action)
- Professional appearance
- Code signing + notarization required for macOS 10.15+

**Process**:
```bash
# 1. Build and sign
xcodebuild archive -scheme CodeBridge -archivePath CodeBridge.xcarchive

# 2. Export
xcodebuild -exportArchive -archivePath CodeBridge.xcarchive \
  -exportPath export -exportOptionsPlist ExportOptions.plist

# 3. Create DMG
hdiutil create -volname "CodeBridge" -srcfolder export/CodeBridge.app \
  -ov -format UDZO CodeBridge.dmg

# 4. Sign DMG
codesign --sign "Developer ID Application: YourName" CodeBridge.dmg

# 5. Notarize
xcrun notarytool submit CodeBridge.dmg --keychain-profile "notarization" --wait

# 6. Staple ticket
xcrun stapler staple CodeBridge.dmg
```

**Secondary: Homebrew Cask**

```ruby
cask "code-bridge" do
  version "1.0.0"
  sha256 "..."

  url "https://github.com/you/code-bridge/releases/download/v#{version}/CodeBridge.dmg"
  name "CodeBridge"
  desc "P2P file sharing application"
  homepage "https://github.com/you/code-bridge"

  app "CodeBridge.app"
end
```

**Tertiary: Mac App Store**
- Requires Apple Developer Program ($99/year)
- Strictest sandboxing requirements
- May limit P2P functionality

### Linux (Ubuntu/Debian)

**Priority Order**:

1. **Flatpak (RECOMMENDED)**
   - Best for end-user desktop environments
   - Default on many distros (Fedora, Mint, Pop!_OS)
   - Good sandboxing with portal system
   - Flathub distribution

2. **AppImage (RECOMMENDED for portability)**
   - No installation required
   - Portable (USB stick, testing)
   - No auto-updates (manual)
   - Fastest startup time

3. **Snap**
   - Good for Ubuntu-centric deployment
   - Enterprise/server focus
   - Centralized distribution
   - Slower startup

4. **DEB Package**
   - Traditional Debian/Ubuntu package
   - Native package manager integration
   - Requires dependency management

**Flatpak Setup**:

```yaml
# org.codebridge.CodeBridge.yml
app-id: org.codebridge.CodeBridge
runtime: org.gnome.Platform
runtime-version: '45'
sdk: org.gnome.Sdk
sdk-extensions:
  - org.freedesktop.Sdk.Extension.rust-stable
command: code-bridge
finish-args:
  - --share=network
  - --share=ipc
  - --socket=wayland
  - --socket=fallback-x11
  - --filesystem=home
modules:
  - name: code-bridge
    buildsystem: simple
    build-commands:
      - cargo --offline fetch --manifest-path Cargo.toml
      - cargo --offline build --release
      - install -Dm755 target/release/code-bridge /app/bin/code-bridge
```

### iOS

**TestFlight (Development/Beta)**
- Invite testers
- Automatic updates
- Crash reporting

**App Store (Production)**
- Requires Apple Developer Program
- App Review process
- Wider distribution

---

## 9. Recommended Tech Stack Summary

### ✅ Recommended Stack for Code-Bridge

```
┌──────────────────────────────────────────────────────────┐
│                    PLATFORM LAYER                        │
├──────────────────┬──────────────────┬────────────────────┤
│      macOS       │       iOS        │       Linux        │
│                  │                  │                    │
│   SwiftUI +      │   SwiftUI +      │   Tauri 2.0       │
│   Combine        │   Combine        │   (Web UI)        │
│                  │                  │                    │
│   OR: AppKit     │                  │   OR: GTK4-rs     │
│   for advanced   │                  │   (Native)        │
└────────┬─────────┴────────┬─────────┴──────────┬─────────┘
         │                  │                    │
         │     swift-bridge │     (if UniFFI:    │
         │     FFI Layer    │      uniffi-rs)    │
         │                  │                    │
┌────────▼──────────────────▼────────────────────▼─────────┐
│              RUST CORE LIBRARY (Workspace)               │
│                                                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Networking: libp2p + tokio                     │    │
│  │  - Transport: QUIC (quinn) / TCP                │    │
│  │  - Discovery: mDNS + Kademlia DHT               │    │
│  │  - Security: Noise protocol                     │    │
│  └─────────────────────────────────────────────────┘    │
│                                                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  File Watching: notify v8.2                     │    │
│  │  - macOS: FSEvents                              │    │
│  │  - iOS: kqueue                                  │    │
│  │  - Linux: inotify                               │    │
│  └─────────────────────────────────────────────────┘    │
│                                                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  Serialization: FlatBuffers                     │    │
│  │  - Zero-copy deserialization                    │    │
│  │  - Swift + Rust codegen                         │    │
│  └─────────────────────────────────────────────────┘    │
│                                                           │
│  ┌─────────────────────────────────────────────────┐    │
│  │  File Transfer Protocol                         │    │
│  │  - Chunking, resumption, verification           │    │
│  └─────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────┘
```

### Distribution Channels

- **macOS**: DMG (notarized) → Homebrew Cask → App Store
- **iOS**: TestFlight → App Store
- **Linux**: Flatpak (Flathub) + AppImage

### Development Tools

```toml
# .cargo/config.toml
[build]
jobs = 4

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.aarch64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]
```

```toml
# rust-toolchain.toml
[toolchain]
channel = "1.82.0"  # or "stable"
components = ["rustfmt", "clippy", "rust-src"]
targets = [
  "x86_64-apple-darwin",
  "aarch64-apple-darwin",
  "x86_64-unknown-linux-gnu",
  "aarch64-unknown-linux-gnu",
  "aarch64-apple-ios",
  "x86_64-apple-ios",
]
```

---

## 10. Implementation Phases

### Phase 1: Core Foundation (Weeks 1-4)
- [ ] Setup monorepo structure
- [ ] Implement Rust core library
  - [ ] File watching with notify
  - [ ] Basic P2P with libp2p
  - [ ] FlatBuffers protocol definitions
- [ ] Setup CI/CD for Rust testing

### Phase 2: macOS Native (Weeks 5-8)
- [ ] SwiftUI interface
- [ ] swift-bridge FFI integration
- [ ] File picker and permissions
- [ ] Local network testing
- [ ] DMG creation and code signing

### Phase 3: Linux Support (Weeks 9-12)
- [ ] Choose UI framework (Tauri vs GTK4)
- [ ] Implement Linux UI
- [ ] Cross-platform testing
- [ ] Flatpak packaging

### Phase 4: iOS Support (Weeks 13-16)
- [ ] iOS UI (shared Swift code with macOS)
- [ ] iOS permissions (Background, Network)
- [ ] TestFlight distribution
- [ ] App Store preparation

### Phase 5: Polish & Distribution (Weeks 17-20)
- [ ] Performance optimization
- [ ] Security audit
- [ ] Documentation
- [ ] App Store submissions
- [ ] Marketing website

---

## 11. Key Technical Decisions

### Decision Matrix

| Requirement | Choice | Reasoning |
|-------------|--------|-----------|
| **FFI Bridge** | swift-bridge | Apple-only, better performance |
| **Linux UI** | Tauri 2.0 | Faster development, smaller size |
| **Networking** | libp2p | Industry standard, NAT traversal |
| **File Watching** | notify | Cross-platform, mature |
| **Serialization** | FlatBuffers | Zero-copy, fast |
| **macOS Distribution** | DMG | Offline support, professional |
| **Linux Distribution** | Flatpak + AppImage | Flatpak for repos, AppImage for portable |
| **Async Runtime** | tokio | De facto standard, excellent ecosystem |

---

## 12. Security Considerations

### Code Signing & Notarization
- **macOS**: Required for Gatekeeper
- **iOS**: Required for App Store
- **Certificates**: Developer ID Application, Apple Development

### Sandboxing
- **macOS/iOS**: App Sandbox entitlements
- **Linux Flatpak**: Portal system for file access
- **Linux Snap**: AppArmor confinement

### P2P Security
- **libp2p Noise protocol**: Encrypted connections
- **mTLS**: Mutual authentication
- **Identity verification**: Public key infrastructure

### File Transfer Security
- **Checksum verification**: SHA256 for chunks
- **Encryption**: Optional end-to-end encryption
- **Access control**: User permissions

---

## 13. Performance Targets

| Metric | Target |
|--------|--------|
| App startup time | < 500ms (macOS/Linux), < 300ms (iOS) |
| P2P peer discovery | < 2s on local network |
| File transfer speed | 90% of network bandwidth |
| Memory usage | < 100MB at idle |
| Binary size | < 10MB (macOS/iOS), < 5MB (Linux) |
| UI responsiveness | 60 FPS, no frame drops |

---

## 14. Testing Strategy

### Unit Tests
```bash
cargo test --all-features
```

### Integration Tests
```bash
cargo test --test integration_tests
```

### Platform-Specific Tests
- **macOS**: XCTest for Swift code
- **Linux**: cargo test + manual UI testing
- **iOS**: XCTest + Simulator testing

### E2E Tests
- Automated P2P connection testing
- File transfer verification
- Cross-platform compatibility

---

## Conclusion

This architecture provides:
- ✅ **Native performance** on all platforms
- ✅ **Code reuse** through Rust core
- ✅ **Native UI** with platform-specific frameworks
- ✅ **Modern tooling** and development workflow
- ✅ **Professional distribution** channels
- ✅ **Security** through code signing and sandboxing
- ✅ **Scalability** for future platforms (Android, Windows)

The combination of Rust for the core business logic with native Swift (macOS/iOS) and modern Linux frameworks (Tauri or GTK4) provides the best balance of performance, developer experience, and user experience.

---

## References & Further Reading

### Swift-Rust FFI
- [swift-bridge GitHub](https://github.com/chinedufn/swift-bridge)
- [UniFFI Documentation](https://mozilla.github.io/uniffi-rs/)
- [Rust and Swift Interoperability 2025](https://markaicode.com/rust-swift-interoperability-cross-platform-apple-applications-2025/)
- [Calling Rust from Swift](https://www.strathweb.com/2023/07/calling-rust-code-from-swift/)

### Cross-Platform UI
- [Tauri 2.0 Documentation](https://v2.tauri.app/)
- [GTK4-rs](https://gtk-rs.org/)
- [2025 Survey of Rust GUI Libraries](https://www.boringcactus.com/2025/04/13/2025-survey-of-rust-gui-libraries.html)
- [Tauri vs Other Frameworks](https://medium.com/rustaceans/how-tauri-and-rust-are-revolutionizing-cross-platform-app-development-in-2025-65515c0793f1)

### P2P Networking
- [libp2p Tutorial](https://blog.logrocket.com/libp2p-tutorial-build-a-peer-to-peer-app-in-rust/)
- [libp2p Documentation](https://docs.rs/libp2p/)
- [Tokio Documentation](https://tokio.rs/)

### File Watching
- [notify crate](https://docs.rs/notify/)
- [Notify Rust Guide](https://generalistprogrammer.com/tutorials/notify-rust-crate-guide)

### Serialization
- [FlatBuffers Documentation](https://flatbuffers.dev/)
- [Protocol Buffers Comparison](https://github.com/kcchu/buffer-benchmarks)
- [Cap'n Proto vs FlatBuffers](https://capnproto.org/news/2014-06-17-capnproto-flatbuffers-sbe.html)

### Distribution
- [macOS Code Signing & Notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution)
- [DMG Distribution Guide](https://gist.github.com/rsms/929c9c2fec231f0cf843a1a746a416f5)
- [Flatpak vs Snap vs AppImage 2025](https://dev.to/rosgluk/snap-vs-flatpak-ultimate-guide-for-2025-545m)
- [Linux Packaging Comparison](https://www.baeldung.com/linux/snaps-flatpak-appimage)

### Monorepo & CI/CD
- [Moon - Rust Monorepo Tool](https://github.com/moonrepo/moon)
- [Cross-Compilation in Rust](https://fpira.com/blog/2025/01/cross-compilation-in-rust)
- [Monorepo Tools 2025](https://www.aviator.co/blog/monorepo-tools/)

---

**Document Version**: 1.0
**Last Updated**: 2025-12-20
**Author**: Architecture Research Team
