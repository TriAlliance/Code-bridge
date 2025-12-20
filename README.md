# Code Bridge

> A modern, developer-focused P2P file sharing platform that puts you in control of your data.

**Status:** Architecture & Research Phase
**Version:** 0.1.0-alpha
**License:** Apache 2.0 / MIT

---

## What is Code Bridge?

Code Bridge is a next-generation file sharing platform designed specifically for developers. Unlike traditional cloud-based solutions, Code Bridge uses peer-to-peer technology to give you complete control over your files while maintaining the collaborative features you need.

### Core Principles

1. **Local-First**: Your data lives on your devices, not in someone else's cloud
2. **Peer-to-Peer**: Direct connections between devices, no intermediary servers
3. **Developer-Native**: Git-like CLI, IDE integration, scripting support
4. **Privacy-Focused**: End-to-end encryption, no telemetry, open source
5. **Offline-Capable**: Work offline indefinitely, sync when convenient

### Key Features

- **P2P Mesh Networking**: Direct device-to-device file sharing using libp2p
- **CRDT-Based Sync**: Automatic conflict resolution for concurrent edits
- **Git-Like Versioning**: Familiar version control semantics
- **Content-Addressed Storage**: IPFS-style deduplication and integrity
- **Cross-Platform**: macOS, Linux, with iOS/Android planned
- **Real-Time Collaboration**: Google Docs-style editing with CRDTs
- **Modern Tech Stack**: Rust core + Tauri desktop (lightweight, fast)

---

## Quick Start

```bash
# Install Code Bridge
curl -fsSL https://codebridge.dev/install.sh | sh

# Initialize a project
codebridge init my-project

# Discover peers on your network
codebridge peer discover

# Add a peer
codebridge peer add bob@laptop

# Share files
codebridge add src/
codebridge commit -m "Initial commit"
codebridge push bob

# Enable auto-sync
codebridge watch --auto-sync
```

---

## Architecture Overview

Code Bridge is built on a modern technology stack:

```
┌─────────────────────────────────────────────────────────────┐
│                      Code Bridge Stack                       │
├─────────────────────────────────────────────────────────────┤
│ Frontend:    Tauri + React + TypeScript                     │
│ Backend:     Rust (tokio async runtime)                     │
│ Networking:  libp2p + WebRTC + QUIC                         │
│ Sync:        Automerge (CRDT) + Vector Clocks               │
│ Storage:     Content-Addressed (IPFS-style) + SQLite        │
│ Security:    Ed25519 + ChaCha20-Poly1305 + TLS 1.3          │
│ Discovery:   mDNS (local) + Kademlia DHT (global)           │
│ Optional:    Tailscale/WireGuard for mesh VPN               │
└─────────────────────────────────────────────────────────────┘
```

### Why These Technologies?

- **Rust**: Performance, safety, and excellent cross-platform support
- **libp2p**: Battle-tested P2P networking (powers IPFS, Ethereum)
- **Tauri**: Lightweight desktop apps (3-10MB vs 100MB+ Electron)
- **CRDTs**: Conflict-free replication for offline-first sync
- **WebRTC**: Direct browser-to-browser file transfers
- **QUIC**: Fast, encrypted transport protocol

---

## Documentation

- **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Complete architecture design and implementation guide
- **[TECHNOLOGY_COMPARISON.md](./TECHNOLOGY_COMPARISON.md)** - Detailed technology stack analysis
- **[ROADMAP.md](./ROADMAP.md)** - Development roadmap and milestones

---

## Use Cases

### For Individual Developers

```bash
# Sync your dotfiles across devices
codebridge init ~/dotfiles
codebridge add .
codebridge commit -m "My dotfiles"
codebridge sync  # Auto-discovers your other devices
```

### For Teams

```bash
# Share a project with your team
codebridge init my-app
codebridge peer add alice@macbook
codebridge peer add bob@linux
codebridge push --all  # Share with all peers
```

### For Open Source

```bash
# Distribute your project P2P
codebridge init open-source-project
codebridge publish --public  # Announce to DHT
# Anyone can: codebridge clone QmProjectID
```

### For Remote Work

```bash
# Work offline, sync later
codebridge watch --auto-sync
# Edit files while offline...
# When you reconnect: automatic sync!
```

---

## Comparison with Alternatives

| Feature | Code Bridge | GitHub | Dropbox | Syncthing |
|---------|-------------|--------|---------|-----------|
| Architecture | P2P + Local-first | Centralized | Centralized | P2P |
| Offline Support | Full | Limited | Basic | Full |
| Data Ownership | User | Company | Company | User |
| Vendor Lock-in | None | High | High | None |
| Git Integration | Built-in | Native | None | None |
| Real-time Collab | CRDT-based | Codespaces | None | None |
| Developer Tools | CLI + IDE | CLI + IDE | Limited | Limited |
| Privacy | E2E encrypted | On platform | On platform | E2E encrypted |

**Code Bridge combines the best of all worlds:**
- Git's versioning + developer UX
- Syncthing's P2P architecture + privacy
- Dropbox's ease of use + auto-sync
- Modern tech stack (Rust, libp2p, CRDTs)

---

## Technology Deep Dives

### P2P Networking with libp2p

libp2p is a modular networking stack that powers IPFS, Ethereum, and Filecoin. It provides:

- **Transport Agnostic**: QUIC, TCP, WebSockets, WebRTC
- **NAT Traversal**: Hole punching, relay servers
- **Peer Discovery**: mDNS (local), DHT (global)
- **Security**: Noise protocol, TLS 1.3
- **Multiplexing**: Multiple streams over one connection

### CRDT-Based Sync

Conflict-free Replicated Data Types (CRDTs) enable offline-first, eventually consistent sync:

```rust
// Example: Real-time text editing with CRDTs
use automerge::Automerge;

let mut doc1 = Automerge::new();
let mut doc2 = doc1.fork();

// Alice edits offline
doc1.change(|doc| {
    doc.put("README", "Hello from Alice");
});

// Bob edits offline
doc2.change(|doc| {
    doc.put("README", "Hello from Bob");
});

// Merge automatically resolves conflicts!
doc1.merge(&mut doc2);
```

### Content-Addressed Storage

IPFS-style content addressing ensures data integrity and enables deduplication:

```
Traditional:  /path/to/file.txt
                ↓
              Server stores at path

Content-Addressed:  Qmabcd123... (SHA-256 hash)
                      ↓
                    Content is the address
                    • Verifiable integrity
                    • Automatic deduplication
                    • Immutable references
```

---

## Development Status

**Current Phase:** Architecture & Research ✅
**Next Phase:** Phase 1 - Foundation (Months 1-3)

### Completed

- ✅ Technology research and evaluation
- ✅ Architecture design
- ✅ Stack selection (Rust + Tauri + libp2p)
- ✅ P2P networking strategy
- ✅ CRDT sync design
- ✅ Security architecture

### In Progress

- 🔄 Repository structure setup
- 🔄 Initial Rust workspace configuration
- 🔄 libp2p integration POC

### Planned (Phase 1)

- ⏳ Core infrastructure (Rust + libp2p)
- ⏳ Content-addressed storage implementation
- ⏳ Local peer discovery (mDNS)
- ⏳ Basic CLI (init, add, commit)
- ⏳ Tauri desktop app skeleton

See [ROADMAP.md](./ROADMAP.md) for detailed timeline.

---

## Contributing

Code Bridge is in early development. We welcome:

- **Feedback** on architecture and design decisions
- **Research** into P2P networking and sync protocols
- **Code contributions** (coming soon)
- **Documentation** improvements

### Areas of Interest

If you have expertise in:
- P2P networking (libp2p, WebRTC)
- CRDTs and distributed systems
- Rust async programming
- Cross-platform desktop development (Tauri)
- Security and cryptography

We'd love to hear from you!

---

## License

Dual-licensed under:
- Apache License 2.0
- MIT License

Choose whichever works best for your use case.

---

## Contact

- **Project Lead:** TBD
- **Repository:** github.com/codebridge/codebridge (coming soon)
- **Discord:** discord.gg/codebridge (coming soon)
- **Email:** hello@codebridge.dev

---

## Acknowledgments

Code Bridge stands on the shoulders of giants:

- **libp2p** - Modular P2P networking stack
- **IPFS** - Content-addressed storage inspiration
- **Syncthing** - P2P sync protocol design
- **Automerge** - CRDT implementation
- **Tauri** - Lightweight desktop framework
- **Rust Community** - Amazing ecosystem and tools

Special thanks to the open source communities building the future of decentralized technology.

---

**Built with ❤️ by developers, for developers.**
