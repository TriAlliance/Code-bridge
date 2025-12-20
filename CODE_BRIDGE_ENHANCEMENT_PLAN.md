# Code Bridge Enhancement Plan
## Comprehensive Feature Roadmap for Cross-Platform Developer Tools

**Created:** December 20, 2025
**Project:** Coachly Code Bridge
**Platforms:** macOS (Mac mini) ↔ Ubuntu (Windows/WSL) ↔ QNAP NAS

---

## Executive Summary

Based on extensive research across 11 specialized areas, this document presents a comprehensive plan to enhance Code Bridge with powerful developer productivity tools. The enhancements transform Code Bridge from a simple file sync tool into a complete cross-platform development ecosystem.

### Key Enhancement Categories

| Category | Priority | Impact | Complexity |
|----------|----------|--------|------------|
| 1. Screenshot & Snippet Tool | HIGH | Very High | Medium |
| 2. Universal Clipboard Sync | HIGH | Very High | Medium |
| 3. Notification Bridge | HIGH | High | Medium |
| 4. Terminal Session Sharing | MEDIUM | High | Low |
| 5. File Transformation Engine | MEDIUM | High | Medium |
| 6. Dev Environment Sync | MEDIUM | Medium | Low |
| 7. Real-time Collaboration | LOW | Very High | High |
| 8. Quick Actions & Automation | LOW | Medium | Low |

---

## Phase 2: Screenshot & Snippet Tool

### Overview
A powerful screenshot capture system with automatic OCR, annotation, and cross-platform sync.

### Features

#### 2.1 Smart Screenshot Capture
```rust
// Key features:
- Global hotkeys (Cmd+Shift+4 style)
- Region selection with magnifier
- Window capture with shadow
- Full screen capture
- Scrolling capture for long content
- Delayed capture (timer)
```

#### 2.2 OCR Text Extraction
```toml
# Using leptess (Rust Tesseract bindings)
[dependencies]
leptess = "0.14"
```

**Capabilities:**
- Extract text from screenshots automatically
- Support 100+ languages
- Searchable screenshot archive
- Copy extracted text to clipboard

#### 2.3 Annotation Tools
- Arrows, boxes, circles
- Text labels
- Blur/pixelate sensitive info
- Highlight/underline
- Numbered steps
- Emoji stamps

#### 2.4 Automatic Sync
- Save to QNAP central storage
- P2P sync to all connected devices
- Content-addressed deduplication
- Thumbnail generation

### Implementation

**New Crate:** `bridge-screenshot`

```
crates/bridge-screenshot/
├── src/
│   ├── lib.rs
│   ├── capture/
│   │   ├── mod.rs
│   │   ├── macos.rs      # ScreenCaptureKit
│   │   ├── linux.rs      # X11/Wayland
│   │   └── selection.rs  # Region selection UI
│   ├── ocr/
│   │   ├── mod.rs
│   │   └── tesseract.rs
│   ├── annotation/
│   │   ├── mod.rs
│   │   ├── shapes.rs
│   │   └── renderer.rs
│   └── storage.rs
└── Cargo.toml
```

### Tech Stack
- **macOS:** ScreenCaptureKit (macOS 12.3+)
- **Linux:** XDG Screenshot Portal / grim (Wayland)
- **OCR:** leptess (Tesseract bindings)
- **Image:** image-rs with AVIF/WebP support
- **Storage:** Content-addressed (BLAKE3)

---

## Phase 3: Universal Clipboard Sync

### Overview
Seamless clipboard sharing across all devices with rich content support.

### Features

#### 3.1 Content Types
- Plain text
- Rich text (HTML/RTF)
- Images (auto-compressed)
- Files (via reference)
- Code snippets (with syntax)
- URLs (with preview)

#### 3.2 Clipboard History
```rust
pub struct ClipboardHistory {
    entries: VecDeque<ClipboardEntry>,
    max_entries: usize,       // Default: 1000
    max_age: Duration,        // Default: 30 days
    searchable: bool,
}

pub struct ClipboardEntry {
    id: String,
    content_type: ContentType,
    data: Vec<u8>,
    preview: Option<String>,
    source_device: String,
    timestamp: DateTime<Utc>,
    pinned: bool,
    tags: Vec<String>,
}
```

#### 3.3 Smart Features
- **Sensitive content detection** - Don't sync passwords
- **Large content handling** - Stream files instead of clipboard
- **Conflict resolution** - Last-write-wins with history
- **Search** - Full-text search across history
- **Favorites** - Pin frequently used items

### Implementation

**New Crate:** `bridge-clipboard`

```
crates/bridge-clipboard/
├── src/
│   ├── lib.rs
│   ├── platform/
│   │   ├── mod.rs
│   │   ├── macos.rs    # NSPasteboard
│   │   └── linux.rs    # wl-clipboard / xclip
│   ├── sync.rs
│   ├── history.rs
│   └── detection.rs    # Content type detection
└── Cargo.toml
```

### Platform APIs
- **macOS:** `NSPasteboard` via objc crate
- **Linux:** `wl-copy`/`wl-paste` (Wayland) or `xclip` (X11)
- **Sync:** P2P gossipsub with content deduplication

---

## Phase 4: Developer Notification Bridge

### Overview
Cross-platform notification sync with developer-specific integrations.

### Features

#### 4.1 System Notification Sync
- Capture notifications from any app
- Display on all connected devices
- Sync read/dismiss status
- Smart filtering (work hours, focus mode)

#### 4.2 Developer Integrations

| Source | Notifications |
|--------|--------------|
| GitHub | PR reviews, CI status, mentions |
| GitLab | Merge requests, pipelines |
| Local Build | cargo/npm build success/failure |
| Tests | Test failures with details |
| Logs | Error pattern detection |
| Monitoring | Datadog, Prometheus alerts |
| Security | Vulnerability alerts (cargo-audit) |

#### 4.3 Action Buttons
```rust
pub enum NotificationAction {
    OpenUrl(String),
    OpenFile(PathBuf),
    RunCommand { command: String, cwd: PathBuf },
    ApiCall { method: String, url: String, body: Option<String> },
    Custom(String), // App-specific actions
}

// Examples:
// - "Approve PR" → POST to GitHub API
// - "Re-run Build" → cargo build
// - "View Failures" → Open test report
// - "Acknowledge Alert" → Mark resolved
```

#### 4.4 Smart Filtering
- Priority-based display
- Time-of-day rules
- Focus mode integration
- Per-app rules
- AI-powered importance scoring

### Implementation

**New Crate:** `bridge-notifications`

```
crates/bridge-notifications/
├── src/
│   ├── lib.rs
│   ├── capture/
│   │   ├── mod.rs
│   │   ├── macos.rs    # UNUserNotificationCenter
│   │   └── linux.rs    # D-Bus org.freedesktop.Notifications
│   ├── display/
│   │   ├── mod.rs
│   │   ├── macos.rs
│   │   └── linux.rs
│   ├── integrations/
│   │   ├── mod.rs
│   │   ├── github.rs
│   │   ├── gitlab.rs
│   │   ├── build.rs
│   │   ├── test.rs
│   │   └── monitoring.rs
│   ├── filters.rs
│   └── actions.rs
└── Cargo.toml
```

---

## Phase 5: Terminal Session Sharing

### Overview
Share terminal sessions, command history, and shell environments across devices.

### Features

#### 5.1 Session Recording
```rust
pub struct TerminalRecording {
    id: String,
    title: String,
    shell: String,
    working_dir: PathBuf,
    events: Vec<TerminalEvent>,
    duration: Duration,
    recorded_at: DateTime<Utc>,
}

pub enum TerminalEvent {
    Input(String),
    Output(String),
    Resize { cols: u16, rows: u16 },
    Timestamp(Duration),
}
```

- Record terminal sessions (asciinema-compatible)
- Replay with variable speed
- Search within recordings
- Share via P2P

#### 5.2 Command History Sync
- Unified history across all devices
- Shell-agnostic (bash, zsh, fish)
- Context-aware (project, directory)
- Search and fuzzy find

#### 5.3 Environment Sync
- Export/import shell aliases
- Sync environment variables (non-sensitive)
- Share shell functions
- Tool version management

### Implementation

**New Crate:** `bridge-terminal`

```
crates/bridge-terminal/
├── src/
│   ├── lib.rs
│   ├── recording/
│   │   ├── mod.rs
│   │   ├── capture.rs
│   │   └── replay.rs
│   ├── history/
│   │   ├── mod.rs
│   │   ├── bash.rs
│   │   ├── zsh.rs
│   │   └── fish.rs
│   └── environment.rs
└── Cargo.toml
```

### Tech Stack
- **Recording:** PTY multiplexing with portable-pty
- **Format:** Asciinema v2 compatible JSON
- **History:** SQLite with FTS5 full-text search
- **Sync:** P2P with CRDT for conflict-free merge

---

## Phase 6: File Transformation Engine

### Overview
On-the-fly file format conversion during P2P transfers.

### Features

#### 6.1 Image Transformations
```rust
pub enum ImageTransform {
    // Format conversion
    ToAvif { quality: u8 },   // 50% smaller than JPEG
    ToWebP { quality: u8 },   // 34% smaller than JPEG
    ToPng,
    ToJpeg { quality: u8 },

    // Modifications
    Resize { width: u32, height: u32 },
    Thumbnail { size: u32 },
    Compress { max_size_kb: u32 },

    // OCR
    ExtractText { language: String },
}
```

#### 6.2 Data Format Conversion
```rust
// Using Serde for zero-overhead conversion
let json: serde_json::Value = serde_json::from_str(&input)?;
let yaml = serde_yaml::to_string(&json)?;
let toml = toml::to_string_pretty(&json)?;
```

**Supported Formats:**
- JSON ↔ YAML ↔ TOML ↔ XML
- Protobuf ↔ JSON
- CSV ↔ JSON
- MessagePack ↔ JSON

#### 6.3 Document Conversion
- Markdown → HTML/PDF
- Jupyter Notebook → HTML/Markdown/Python
- HTML → Markdown
- Code → Syntax-highlighted HTML

#### 6.4 Audio/Video
- Audio transcription (Whisper)
- Video thumbnail generation
- Audio format conversion
- Waveform visualization

### Implementation

**New Crate:** `bridge-transform`

```
crates/bridge-transform/
├── src/
│   ├── lib.rs
│   ├── registry.rs       # Transformer registry
│   ├── pipeline.rs       # Async processing pipeline
│   ├── transformers/
│   │   ├── mod.rs
│   │   ├── image.rs
│   │   ├── data.rs
│   │   ├── document.rs
│   │   ├── media.rs
│   │   └── code.rs
│   └── cache.rs          # Content-addressed cache
└── Cargo.toml
```

### Tech Stack
```toml
[dependencies]
# Image
image = "0.24"
ravif = "0.11"
leptess = "0.14"

# Data formats
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"
quick-xml = "0.31"

# Documents
pulldown-cmark = "0.9"
comrak = "0.18"

# Audio
whisper-rs = "0.15"
symphonia = "0.5"
```

---

## Phase 7: Development Environment Sync

### Overview
Keep development environments consistent across all machines.

### Features

#### 7.1 Dotfiles Management
```rust
pub struct DotfilesSync {
    // Files to sync
    paths: Vec<DotfilePath>,

    // Templating for machine-specific values
    variables: HashMap<String, String>,

    // Encryption for sensitive files
    encrypted_patterns: Vec<String>,
}

pub struct DotfilePath {
    source: PathBuf,      // ~/.zshrc
    target: PathBuf,      // In Code Bridge storage
    machine_specific: bool,
}
```

**Synced Files:**
- Shell configs (.zshrc, .bashrc)
- Git config (.gitconfig)
- SSH config (structure only, not keys)
- Editor settings (VSCode settings.json, vim configs)
- Tool configs (starship.toml, tmux.conf)

#### 7.2 IDE Settings Sync
- VSCode extensions list
- VSCode settings.json
- Keyboard shortcuts
- Snippets
- Themes

#### 7.3 Tool Version Management
```rust
// Track installed tool versions
pub struct ToolVersions {
    node: Option<String>,    // via nvm/fnm
    python: Option<String>,  // via pyenv
    rust: Option<String>,    // via rustup
    go: Option<String>,
    tools: HashMap<String, String>,
}

// Sync and alert on version mismatches
```

### Implementation

**New Crate:** `bridge-environment`

```
crates/bridge-environment/
├── src/
│   ├── lib.rs
│   ├── dotfiles/
│   │   ├── mod.rs
│   │   ├── sync.rs
│   │   └── templates.rs
│   ├── ide/
│   │   ├── mod.rs
│   │   └── vscode.rs
│   └── versions.rs
└── Cargo.toml
```

---

## Phase 8: Real-time Collaboration (Future)

### Overview
Live collaboration features for pair programming and code review.

### Features

#### 8.1 Collaborative Editing
```rust
// Using Yjs via y-crdt Rust port
use y_crdt::Doc;

pub struct CollaborativeDocument {
    doc: Doc,
    awareness: Awareness,  // Cursors, selections
    file_path: PathBuf,
}
```

- Real-time text sync (OT/CRDT)
- Multiple cursors with user colors
- Selection highlighting
- Presence indicators

#### 8.2 Screen Sharing
- Share entire screen or window
- Low-latency P2P streaming
- Remote cursor visibility
- Annotation overlay

#### 8.3 Voice/Video (Optional)
- WebRTC-based P2P
- Low-bandwidth audio
- Optional video
- Push-to-talk

### Tech Stack
- **CRDT:** y-crdt (Yjs Rust port)
- **Screen Capture:** scrap crate
- **Encoding:** libvpx/AV1
- **Voice:** opus codec
- **Transport:** WebRTC or QUIC

---

## Phase 9: Quick Actions & Automation

### Overview
Raycast/Alfred-style command palette with automation.

### Features

#### 9.1 Command Palette
```rust
pub struct QuickAction {
    id: String,
    name: String,
    description: String,
    keywords: Vec<String>,
    icon: Option<String>,
    action: ActionType,
    shortcut: Option<Shortcut>,
}

pub enum ActionType {
    BuiltIn(BuiltInAction),
    Script { path: PathBuf, args: Vec<String> },
    Url(String),
    Workflow(Vec<ActionStep>),
}
```

**Built-in Actions:**
- Sync now
- Share file/folder
- Take screenshot
- Search clipboard history
- Open recent files
- Connect to peer
- Show notifications
- Run transformation

#### 9.2 Workflows
```yaml
# Example: Share annotated screenshot
name: "Quick Bug Report"
steps:
  - action: screenshot
    params:
      region: true
  - action: annotate
    params:
      add_timestamp: true
  - action: upload
    params:
      destination: qnap
  - action: copy_link
  - action: notify
    params:
      message: "Screenshot shared!"
```

#### 9.3 Keyboard Shortcuts
- Global hotkeys
- Customizable bindings
- Context-aware shortcuts
- Chord support (vim-style)

### Implementation

**New Crate:** `bridge-actions`

```
crates/bridge-actions/
├── src/
│   ├── lib.rs
│   ├── palette.rs
│   ├── actions/
│   │   ├── mod.rs
│   │   └── builtin.rs
│   ├── workflows/
│   │   ├── mod.rs
│   │   ├── parser.rs
│   │   └── executor.rs
│   └── shortcuts.rs
└── Cargo.toml
```

---

## Updated Dependency Tree

```toml
# Cargo.toml (workspace)
[workspace]
members = [
    # Existing
    "crates/bridge-core",
    "crates/bridge-cli",
    "crates/bridge-ffi",
    "crates/bridge-mcp",

    # New - Phase 2-9
    "crates/bridge-screenshot",
    "crates/bridge-clipboard",
    "crates/bridge-notifications",
    "crates/bridge-terminal",
    "crates/bridge-transform",
    "crates/bridge-environment",
    "crates/bridge-collab",
    "crates/bridge-actions",
]

[workspace.dependencies]
# Existing...

# Screenshot & OCR
leptess = "0.14"
scrap = "0.5"

# Clipboard
arboard = "3.2"

# Notifications
zbus = "4.0"          # Linux D-Bus
objc2 = "0.5"         # macOS

# Terminal
portable-pty = "0.8"

# Transformation
image = "0.24"
ravif = "0.11"
whisper-rs = "0.15"
pulldown-cmark = "0.9"
comrak = "0.18"

# Collaboration
y-crdt = "0.18"

# Actions
rdev = "0.5"          # Global hotkeys
```

---

## Implementation Roadmap

### Immediate (Week 1-2)
1. **Screenshot Tool Core**
   - Region capture on macOS/Linux
   - QNAP storage integration
   - Basic OCR

2. **Clipboard Sync Core**
   - Text/image sync
   - P2P broadcast
   - History storage

### Short-term (Week 3-4)
3. **Notification Bridge**
   - System notification capture
   - GitHub CI integration
   - Local build monitoring

4. **File Transformation**
   - Image format conversion
   - Data format conversion (JSON/YAML/TOML)

### Medium-term (Week 5-8)
5. **Terminal Features**
   - Session recording
   - Command history sync

6. **Dev Environment**
   - Dotfiles sync
   - VSCode settings sync

### Long-term (Month 2+)
7. **Quick Actions**
   - Command palette
   - Workflow engine

8. **Real-time Collaboration**
   - CRDT text sync
   - Screen sharing

---

## MCP Server Extensions

Update `bridge-mcp` with new tools:

```rust
// New MCP tools for each feature
let tools = vec![
    // Screenshot
    Tool::new("bridge_screenshot", "Capture and share screenshot"),
    Tool::new("bridge_ocr", "Extract text from image"),

    // Clipboard
    Tool::new("bridge_clipboard_history", "Search clipboard history"),
    Tool::new("bridge_clipboard_sync", "Sync clipboard content"),

    // Notifications
    Tool::new("bridge_notify", "Send notification to devices"),
    Tool::new("bridge_build_status", "Get build/CI status"),

    // Transform
    Tool::new("bridge_convert", "Convert file format"),
    Tool::new("bridge_transcribe", "Transcribe audio to text"),

    // Environment
    Tool::new("bridge_env_sync", "Sync development environment"),

    // Actions
    Tool::new("bridge_action", "Run quick action"),
];
```

---

## CLI Command Extensions

```bash
# Screenshot
codebridge screenshot            # Interactive capture
codebridge screenshot --region   # Region capture
codebridge screenshot --window   # Window capture
codebridge screenshot --ocr      # Capture with OCR

# Clipboard
codebridge clipboard sync        # Sync clipboard now
codebridge clipboard history     # Show history
codebridge clipboard search      # Search history

# Notifications
codebridge notify send "message" # Send notification
codebridge notify list           # List recent
codebridge notify rules          # Manage filters

# Transform
codebridge convert image.png --to webp
codebridge convert data.json --to yaml
codebridge transcribe audio.mp3

# Terminal
codebridge terminal record       # Start recording
codebridge terminal replay       # Replay session
codebridge terminal history sync # Sync command history

# Environment
codebridge env sync              # Sync dotfiles
codebridge env diff              # Show differences
codebridge env export            # Export settings

# Actions
codebridge action run "sync-now"
codebridge action list
codebridge action create
```

---

## Desktop App Updates

### macOS SwiftUI App

Add new views:
```
Views/
├── ContentView.swift      # Dashboard (existing)
├── ScreenshotsView.swift  # Enhanced with capture
├── ClipboardView.swift    # NEW: Clipboard history
├── NotificationsView.swift # NEW: Notification center
├── TerminalView.swift     # NEW: Session recordings
├── TransformView.swift    # NEW: File conversion
├── EnvironmentView.swift  # NEW: Dev environment
├── ActionsView.swift      # NEW: Quick actions
└── SettingsView.swift     # Updated settings
```

### Ubuntu Tauri App

Mirror the macOS features with Tauri + SvelteKit:
```
src/
├── routes/
│   ├── +page.svelte       # Dashboard
│   ├── screenshots/
│   ├── clipboard/
│   ├── notifications/
│   ├── terminal/
│   ├── transform/
│   ├── environment/
│   └── actions/
└── lib/
    ├── bridge.ts          # Rust FFI bindings
    └── components/
```

---

## Summary

This enhancement plan transforms Code Bridge into a comprehensive cross-platform development ecosystem with:

1. **Screenshot & Snippet Tool** - Capture, annotate, OCR, sync
2. **Universal Clipboard** - Rich content sync with history
3. **Notification Bridge** - Developer-focused alerts with actions
4. **Terminal Sharing** - Session recording and history sync
5. **File Transformation** - Format conversion on-the-fly
6. **Environment Sync** - Consistent dev setup across machines
7. **Real-time Collaboration** - Pair programming support
8. **Quick Actions** - Automation and command palette

All features leverage the existing P2P infrastructure and QNAP storage, creating a unified developer experience across macOS and Ubuntu.
