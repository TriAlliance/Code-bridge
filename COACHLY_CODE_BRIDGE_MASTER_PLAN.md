# Coachly Code Bridge - Master Architecture Plan

## Executive Summary

A cross-platform, P2P file sharing bridge designed specifically for the Coachly development ecosystem. This system enables seamless sharing of screenshots, codebases, and development assets between your Windows/Ubuntu and macOS environments without relying on GitHub.

### Your Environment

| Location | Platform | Codebases |
|----------|----------|-----------|
| Windows 11 (WSL/Ubuntu) | Linux | Web, iOS App, Android App |
| Mac mini | macOS | iOS Watch App, Garmin Watch App |
| QNAP NAS | Central Hub | Shared Storage |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         COACHLY CODE BRIDGE                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────┐    P2P (libp2p)    ┌──────────────┐                      │
│  │   UBUNTU     │◄──────────────────►│    macOS     │                      │
│  │  (Windows)   │      QUIC          │  (Mac mini)  │                      │
│  │              │                    │              │                      │
│  │ • Web Code   │                    │ • iOS Watch  │                      │
│  │ • iOS Code   │                    │ • Garmin App │                      │
│  │ • Android    │                    │              │                      │
│  └──────┬───────┘                    └──────┬───────┘                      │
│         │                                   │                              │
│         │           SMB3 / NFS              │                              │
│         └───────────────┬───────────────────┘                              │
│                         │                                                   │
│                ┌────────▼────────┐                                         │
│                │   QNAP NAS      │                                         │
│                │ (Central Hub)   │                                         │
│                │                 │                                         │
│                │ • Screenshots   │                                         │
│                │ • Backups       │                                         │
│                │ • Shared Assets │                                         │
│                │ • Docker Svcs   │                                         │
│                └─────────────────┘                                         │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Recommended Technology Stack

### Core Technologies (Confidence: ⭐⭐⭐⭐⭐)

| Component | Technology | Why |
|-----------|------------|-----|
| **Core Language** | Rust | Performance, memory safety, cross-platform |
| **P2P Network** | libp2p | Industry standard (IPFS, Ethereum), NAT traversal |
| **Desktop App** | Tauri 2.x | 10-15x smaller than Electron (3-10 MB) |
| **macOS Native** | SwiftUI + swift-bridge | Native performance + Rust core |
| **Sync Engine** | CRDTs (Automerge) | Conflict-free, offline-first |
| **Transport** | QUIC | 35% faster than TCP+TLS, 0-RTT reconnection |
| **File Watching** | notify crate | FSEvents (macOS), inotify (Linux) |
| **Serialization** | FlatBuffers | Zero-copy, fastest for file transfer |

### QNAP Integration

| Component | Technology | Why |
|-----------|------------|-----|
| **Protocol** | SMB3 (primary), NFS (Linux) | Best cross-platform support |
| **Central Services** | Docker on QNAP | Container Station support |
| **Security** | Let's Encrypt SSL + 2FA | Free, automatic renewal |
| **Performance** | Jumbo Frames + SSD Cache | 2x throughput improvement |

### Screenshot System

| Component | Technology | Why |
|-----------|------------|-----|
| **macOS Capture** | ScreenCaptureKit | Modern, Apple Silicon optimized |
| **Linux Capture** | Flameshot API | Cross-platform, CLI-friendly |
| **Format** | AVIF (primary), WebP (fallback) | 50% smaller than JPEG |
| **OCR** | Tesseract.js | Searchable screenshots |
| **Organization** | AI categorization | Auto-tag by project/content |

---

## Three-Tier Architecture

### Tier 1: Direct P2P (Primary - Fastest)

```
Ubuntu ◄─────── libp2p + QUIC ──────► macOS
           (LAN: 100 MB/s)
           (WAN: Network limited)
```

**Features:**
- Zero server dependency
- End-to-end encrypted
- Auto-discovery via mDNS (local) + DHT (global)
- NAT traversal (>90% success rate)
- Real-time sync with CRDTs

### Tier 2: QNAP Hub (Secondary - Reliable)

```
Ubuntu ◄─────── SMB3/NFS ──────► QNAP ◄─────── SMB3 ──────► macOS
```

**Features:**
- Always-available central storage
- Backup and versioning
- Docker services for processing
- Screenshot OCR pipeline
- Web interface for browsing

### Tier 3: Syncthing Fallback (Tertiary - Battle-tested)

```
Ubuntu ◄─────── Syncthing ──────► macOS
           (Block Protocol)
```

**Features:**
- Proven reliability (v2.0 released 2025)
- Excellent REST API for integration
- Handles all edge cases
- Can run as a service

---

## Application Architecture

### macOS App (Native Swift + Rust)

```
┌───────────────────────────────────────────────────┐
│           macOS App (SwiftUI)                     │
│  ┌─────────────────────────────────────────────┐  │
│  │  Menu Bar Interface                         │  │
│  │  • Sync status indicator                    │  │
│  │  • Recent screenshots                       │  │
│  │  • Quick share                              │  │
│  └─────────────────────────────────────────────┘  │
│  ┌─────────────────────────────────────────────┐  │
│  │  Main Window (SwiftUI)                      │  │
│  │  • Project browser                          │  │
│  │  • Screenshot gallery                       │  │
│  │  • Sync settings                            │  │
│  └─────────────────────────────────────────────┘  │
│                       │                           │
│                swift-bridge FFI                   │
│                       │                           │
│  ┌─────────────────────────────────────────────┐  │
│  │  Rust Core Library                          │  │
│  │  • libp2p networking                        │  │
│  │  • File watching (notify)                   │  │
│  │  • CRDT sync engine                         │  │
│  │  • Content-addressed storage                │  │
│  └─────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

### Ubuntu App (Tauri + Rust)

```
┌───────────────────────────────────────────────────┐
│           Ubuntu App (Tauri 2.x)                  │
│  ┌─────────────────────────────────────────────┐  │
│  │  Web UI (HTML/CSS/JS or Svelte)             │  │
│  │  • System tray integration                  │  │
│  │  • Notification support                     │  │
│  │  • Native file dialogs                      │  │
│  └─────────────────────────────────────────────┘  │
│                       │                           │
│                  Tauri IPC                        │
│                       │                           │
│  ┌─────────────────────────────────────────────┐  │
│  │  Rust Core Library (SHARED)                 │  │
│  │  • Same libp2p networking                   │  │
│  │  • Same file watching                       │  │
│  │  • Same CRDT sync engine                    │  │
│  └─────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────┘
```

### CLI Tool (Both Platforms)

```bash
# Git-like developer experience
codebridge init coachly-web
codebridge add src/
codebridge status
codebridge sync
codebridge peer add mac-mini
codebridge screenshot watch ~/Screenshots
codebridge share screenshot-001.png
```

---

## Screenshot Sharing System

### Capture Pipeline

```
┌─────────────────────────────────────────────────────────────────────┐
│                    SCREENSHOT PIPELINE                              │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│   1. CAPTURE                                                        │
│   ┌─────────────────┐  ┌─────────────────┐                         │
│   │ ScreenCaptureKit│  │ Flameshot       │                         │
│   │ (macOS)         │  │ (Ubuntu)        │                         │
│   └────────┬────────┘  └────────┬────────┘                         │
│            │                    │                                   │
│            └─────────┬──────────┘                                   │
│                      ▼                                              │
│   2. PROCESS                                                        │
│   ┌─────────────────────────────────────────────────────────┐      │
│   │ • OCR extraction (Tesseract)                            │      │
│   │ • AVIF compression (50% smaller)                        │      │
│   │ • Thumbnail generation                                  │      │
│   │ • AI categorization (project, type)                     │      │
│   │ • Content hash (deduplication)                          │      │
│   └────────────────────────────────────────────────────────────┘      │
│                      │                                              │
│                      ▼                                              │
│   3. SYNC                                                           │
│   ┌─────────────────────────────────────────────────────────┐      │
│   │ • P2P broadcast to connected peers                      │      │
│   │ • Upload to QNAP (background)                           │      │
│   │ • Index in local database                               │      │
│   └─────────────────────────────────────────────────────────┘      │
│                      │                                              │
│                      ▼                                              │
│   4. ACCESS                                                         │
│   ┌─────────────────────────────────────────────────────────┐      │
│   │ • Full-text search (OCR text)                           │      │
│   │ • Project-based filtering                               │      │
│   │ • Quick share (copy URL/file)                           │      │
│   │ • Annotation tools                                      │      │
│   └─────────────────────────────────────────────────────────┘      │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### Automatic Project Detection

Screenshots are automatically categorized by:
1. **Active window** - Which IDE/app was focused
2. **Git repository** - Associated with active project
3. **Hotkey** - Manual project assignment
4. **AI analysis** - Code vs UI vs diagram detection

---

## QNAP Central Hub Setup

### Docker Services on QNAP

```yaml
# docker-compose.yml for QNAP Container Station
version: '3.8'

services:
  # Bridge Service - REST API for all clients
  bridge-api:
    image: coachly/code-bridge-api
    ports:
      - "8080:8080"
    volumes:
      - /share/Screenshots:/data/screenshots
      - /share/Projects:/data/projects
    environment:
      - RUST_LOG=info
    restart: always

  # Screenshot OCR Processor
  ocr-worker:
    image: coachly/ocr-worker
    volumes:
      - /share/Screenshots:/data/screenshots
    environment:
      - TESSERACT_LANG=eng

  # Metadata Database
  postgres:
    image: postgres:16
    volumes:
      - /share/Database/postgres:/var/lib/postgresql/data
    environment:
      - POSTGRES_PASSWORD=${DB_PASSWORD}

  # Real-time notifications
  redis:
    image: redis:7-alpine
    volumes:
      - /share/Database/redis:/data
```

### Shared Folder Structure

```
/share/
├── Screenshots/
│   ├── 2025/
│   │   ├── 12/
│   │   │   ├── 20/
│   │   │   │   ├── screenshot-001.avif
│   │   │   │   ├── screenshot-001.json  (metadata)
│   │   │   │   └── screenshot-001.txt   (OCR text)
├── Projects/
│   ├── coachly-web/
│   ├── coachly-ios/
│   ├── coachly-android/
│   ├── coachly-watch-ios/
│   └── coachly-watch-garmin/
├── Shared/
│   └── assets/
└── Backups/
    └── daily/
```

### Network Configuration

```
# Recommended QNAP Settings

1. Enable Jumbo Frames (MTU 9000)
   - Settings > Network > TCP/IP > Advanced

2. SMB3 Configuration
   - Enable SMB3 signing (disable for max performance)
   - Set minimum protocol to SMB3

3. SSD Cache (if available)
   - Read-Write cache for Screenshots folder
   - Skip cache for large media files

4. Security
   - Enable Let's Encrypt SSL
   - Enable 2FA for admin
   - Change default ports (443 -> 8443)
```

---

## Implementation Phases

### Phase 1: Foundation (Weeks 1-4)

**Deliverables:**
- [ ] Rust core library with libp2p
- [ ] Local peer discovery (mDNS)
- [ ] Basic file transfer (same LAN)
- [ ] CLI tool (init, add, sync)
- [ ] Content-addressed storage

**Milestones:**
- Week 1: Project setup, Rust workspace, libp2p POC
- Week 2: File watching with notify crate
- Week 3: Content hashing and deduplication
- Week 4: CLI implementation

### Phase 2: P2P Networking (Weeks 5-8)

**Deliverables:**
- [ ] NAT traversal (STUN/TURN)
- [ ] Global discovery (Kademlia DHT)
- [ ] QUIC transport
- [ ] Encrypted connections
- [ ] Multi-peer sync

**Milestones:**
- Week 5: NAT traversal implementation
- Week 6: DHT integration
- Week 7: QUIC + encryption
- Week 8: Multi-peer testing

### Phase 3: Desktop Apps (Weeks 9-12)

**Deliverables:**
- [ ] Tauri app for Ubuntu
- [ ] SwiftUI app for macOS
- [ ] System tray integration
- [ ] Screenshot capture integration
- [ ] Notification system

**Milestones:**
- Week 9: Tauri setup and basic UI
- Week 10: macOS SwiftUI + swift-bridge
- Week 11: Screenshot pipeline
- Week 12: Polish and testing

### Phase 4: QNAP Integration (Weeks 13-16)

**Deliverables:**
- [ ] Docker services deployment
- [ ] SMB3/NFS mounting
- [ ] OCR pipeline
- [ ] Web interface
- [ ] Backup automation

**Milestones:**
- Week 13: Docker compose setup
- Week 14: API service implementation
- Week 15: OCR worker
- Week 16: Web interface

### Phase 5: Advanced Features (Weeks 17-20)

**Deliverables:**
- [ ] CRDT sync engine (Automerge)
- [ ] Conflict resolution
- [ ] Version history
- [ ] Real-time collaboration
- [ ] AI categorization

**Milestones:**
- Week 17: CRDT integration
- Week 18: Version history
- Week 19: AI features
- Week 20: Production readiness

---

## Monorepo Structure

```
code-bridge/
├── Cargo.toml                    # Workspace root
├── README.md
├── .github/
│   └── workflows/
│       ├── ci.yml               # Rust tests + linting
│       ├── release-macos.yml    # macOS app build
│       └── release-linux.yml    # Linux app build
│
├── crates/
│   ├── bridge-core/             # Shared Rust library
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── p2p/             # libp2p networking
│   │       ├── sync/            # CRDT sync engine
│   │       ├── storage/         # Content-addressed storage
│   │       └── watch/           # File system watching
│   │
│   ├── bridge-cli/              # Command-line tool
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs
│   │
│   └── bridge-ffi/              # FFI bindings for Swift
│       ├── Cargo.toml
│       ├── src/lib.rs
│       └── bridge.swift         # Generated by swift-bridge
│
├── apps/
│   ├── desktop-ubuntu/          # Tauri app
│   │   ├── src-tauri/
│   │   │   ├── Cargo.toml
│   │   │   └── src/
│   │   └── src/                 # Web UI (Svelte/Vue)
│   │
│   └── desktop-macos/           # Native Swift app
│       ├── Package.swift
│       ├── Sources/
│       │   └── CodeBridge/
│       │       ├── App.swift
│       │       ├── Views/
│       │       └── ViewModels/
│       └── CodeBridge.xcodeproj
│
├── services/
│   └── qnap/
│       ├── docker-compose.yml
│       ├── api/                 # Bridge API service
│       └── ocr-worker/          # Screenshot OCR
│
└── docs/
    ├── ARCHITECTURE.md
    ├── SETUP.md
    └── API.md
```

---

## Security Architecture

### End-to-End Encryption

```
┌─────────────────────────────────────────────────────────────────┐
│                    SECURITY LAYERS                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Layer 1: Transport Security                                    │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ • TLS 1.3 for all connections                             │  │
│  │ • QUIC with built-in encryption                           │  │
│  │ • Certificate pinning for QNAP                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Layer 2: Peer Authentication                                   │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ • Ed25519 keypairs per device                             │  │
│  │ • Peer ID derived from public key                         │  │
│  │ • Manual peer approval (trust on first use)               │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
│  Layer 3: Content Encryption (Optional)                         │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ • AES-256-GCM for sensitive files                         │  │
│  │ • Per-file encryption keys                                │  │
│  │ • Key exchange via X25519                                 │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Access Control

```yaml
# Device permissions example
devices:
  ubuntu-workstation:
    id: "12D3KooW..."
    name: "Windows/Ubuntu Dev"
    permissions:
      - read: all
      - write: all
      - admin: true

  mac-mini:
    id: "12D3KooX..."
    name: "Mac mini"
    permissions:
      - read: all
      - write: all
      - admin: false

projects:
  coachly-web:
    allowed_devices: [ubuntu-workstation, mac-mini]

  coachly-watch-ios:
    allowed_devices: [mac-mini]  # macOS only for iOS dev
```

---

## Performance Targets

| Metric | Target | Technology |
|--------|--------|------------|
| **LAN Transfer Speed** | >100 MB/s | QUIC + direct P2P |
| **Screenshot Sync** | <500ms | Real-time + mDNS |
| **App Bundle Size** | <15 MB | Tauri (not Electron) |
| **Memory Usage** | <100 MB | Rust efficiency |
| **NAT Traversal** | >90% success | libp2p + STUN/TURN |
| **Startup Time** | <1s | Native apps |
| **OCR Processing** | <2s per image | Tesseract optimized |

---

## Why This Architecture?

### vs. GitHub/GitLab

| Feature | Code Bridge | GitHub |
|---------|------------|--------|
| **Cost** | Free (self-hosted) | Paid for private |
| **Privacy** | 100% local control | Cloud-dependent |
| **Speed** | LAN speed (100+ MB/s) | Internet limited |
| **Screenshots** | Native support | Manual upload |
| **Offline** | Full functionality | Limited |
| **Large files** | No limits | LFS limits |

### vs. Dropbox/Google Drive

| Feature | Code Bridge | Dropbox |
|---------|------------|---------|
| **P2P Sync** | Direct device-to-device | Cloud relay |
| **Developer Focus** | Git-like UX, CLI | Consumer UI |
| **Conflict Resolution** | CRDT (automatic) | Last-write-wins |
| **Privacy** | Self-hosted | Cloud storage |
| **Cost** | Free | Subscription |

### vs. Syncthing Alone

| Feature | Code Bridge | Syncthing |
|---------|------------|-----------|
| **Screenshots** | Capture + OCR + organize | Just sync |
| **Developer UX** | Git-like CLI | Config-based |
| **QNAP Integration** | Native | Manual setup |
| **AI Features** | Categorization, search | None |
| **Native Apps** | SwiftUI, Tauri | Web UI |

---

## Quick Start (After Implementation)

### Installation

```bash
# Ubuntu
flatpak install flathub io.coachly.codebridge

# macOS
brew install --cask codebridge

# or download from releases
```

### First Run

```bash
# Initialize a project
codebridge init coachly-web
cd coachly-web

# Add files to sync
codebridge add src/ docs/ screenshots/

# Discover peers on network
codebridge peer discover
# Found: mac-mini (12D3KooW...)

# Add peer
codebridge peer add mac-mini

# Start syncing
codebridge sync --watch
```

### Screenshot Workflow

```bash
# Watch screenshot folder
codebridge screenshot watch ~/Screenshots

# Take screenshot (system hotkey)
# -> Automatically captured, compressed, OCR'd, synced

# Search screenshots
codebridge screenshot search "error message"

# Open in browser
codebridge screenshot browse
```

---

## Next Steps

1. **Review this plan** and provide feedback
2. **Set up the monorepo** structure
3. **Start with Phase 1** - Rust core library
4. **Configure QNAP** - Enable Docker, create folders
5. **Iterate** based on real-world usage

---

## Sources

- [libp2p Official Site](https://libp2p.io/)
- [Tauri 2.0 Documentation](https://v2.tauri.app/)
- [Syncthing REST API](https://docs.syncthing.net/dev/rest.html)
- [QNAP Container Station](https://qnap-dev.github.io/container-station-api/)
- [ScreenCaptureKit](https://developer.apple.com/documentation/screencapturekit)
- [swift-bridge](https://github.com/chinedufn/swift-bridge)
- [Automerge CRDT](https://automerge.org/)

---

*Generated by Claude Code Expert Sub-Agents - December 2025*
