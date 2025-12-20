# Clipboard Sync Research: Innovative Solutions for Code Bridge

## Executive Summary

This document provides comprehensive research and implementation recommendations for integrating an advanced, developer-focused clipboard sync system into Code Bridge. The proposed system combines P2P synchronization, intelligent content handling, strong security, and developer-centric features to create a best-in-class clipboard management solution.

**Target Users:** Developers working across multiple devices (macOS, Ubuntu, Windows)
**Core Value:** Seamless, secure, intelligent clipboard sync without cloud dependencies
**Architecture Alignment:** Built on Code Bridge's existing libp2p + Rust + CRDT stack

---

## 1. Universal Clipboard Sync Architecture

### 1.1 Real-Time Sync Strategy

Building on Code Bridge's P2P infrastructure:

```rust
// Core clipboard sync engine
pub struct ClipboardSyncEngine {
    // P2P networking (reuse Code Bridge's libp2p)
    p2p_network: P2pNetwork,

    // CRDT for conflict-free clipboard history
    clipboard_crdt: Automerge,

    // Local clipboard monitoring
    clipboard_monitor: ClipboardMonitor,

    // Clip history database
    history_db: ClipHistoryDB,

    // Content-addressed storage for large clips
    content_store: ContentAddressedStore,
}
```

**Key Features:**
- **Sub-500ms Sync Latency:** Real-time clipboard sync using existing libp2p QUIC transport
- **Zero Cloud Dependency:** Pure P2P sync, no intermediary servers
- **Offline-Capable:** Queue clips locally, sync when peers reconnect
- **Multi-Device Mesh:** Sync across all connected devices simultaneously

### 1.2 Cross-Platform Clipboard Access

**Recommended Rust Crate: `arboard`**
- Maintained by 1Password team
- Supports text, images, HTML, RTF
- Cross-platform: macOS, Linux (X11/Wayland), Windows

```toml
[dependencies]
arboard = "3.4"  # Clipboard access
```

**Platform-Specific Handling:**

```rust
use arboard::{Clipboard, ImageData};

pub struct CrossPlatformClipboard {
    clipboard: Clipboard,
}

impl CrossPlatformClipboard {
    // Monitor clipboard changes (polling on Linux/macOS, events on Windows)
    pub async fn watch_clipboard(&mut self) -> ClipboardStream {
        // Poll every 100ms (macOS/Linux) or use clipboard events (Windows)
        // Detect changes via hash comparison
    }

    // Get clipboard content with type detection
    pub fn get_clip(&mut self) -> Result<ClipContent> {
        // Try multiple formats in order of preference
        if let Ok(image) = self.clipboard.get_image() {
            Ok(ClipContent::Image(image))
        } else if let Ok(text) = self.clipboard.get_text() {
            Ok(ClipContent::Text(text))
        } else {
            Ok(ClipContent::Empty)
        }
    }

    // Set clipboard content
    pub fn set_clip(&mut self, content: ClipContent) -> Result<()> {
        match content {
            ClipContent::Text(text) => self.clipboard.set_text(text),
            ClipContent::Image(data) => self.clipboard.set_image(data),
            ClipContent::Html(html) => self.clipboard.set_html(html, Some(&html)),
        }
    }
}
```

**Linux Wayland Considerations:**
- Use `arboard` with XWayland fallback for compatibility
- Consider `wl-clipboard-rs` for pure Wayland support
- Handle clipboard ownership (clipboard content disappears when app closes)
  - Solution: Keep daemon running in background to maintain ownership

### 1.3 Clipboard History with CRDT

**Storage Architecture:**

```rust
use automerge::{Automerge, transaction::Transactable};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipEntry {
    pub id: ClipId,               // Unique ID (UUID)
    pub content: ClipContent,      // Text, image, file, etc.
    pub content_hash: Hash,        // SHA-256 for deduplication
    pub timestamp: DateTime<Utc>,
    pub device_id: DeviceId,
    pub content_type: ContentType,
    pub metadata: ClipMetadata,
    pub pinned: bool,
    pub tags: Vec<String>,
    pub source_app: Option<String>, // Which app copied this?
}

pub struct ClipHistoryDB {
    // CRDT for distributed clipboard history
    automerge_doc: Automerge,

    // Local SQLite for fast queries
    local_db: SqliteConnection,

    // Content-addressed storage for large items
    content_store: ContentAddressedStore,

    // Full-text search index
    search_index: TantivyIndex,
}

impl ClipHistoryDB {
    // Add clip with automatic deduplication
    pub async fn add_clip(&mut self, entry: ClipEntry) -> Result<ClipId> {
        // Check if content already exists (by hash)
        if let Some(existing) = self.find_by_hash(&entry.content_hash).await? {
            return Ok(existing.id);
        }

        // Store in CRDT for sync
        self.automerge_doc.change(|doc| {
            doc.put(&entry.id.to_string(), entry.clone());
        });

        // Index for search
        self.search_index.add_document(entry.to_document())?;

        // Store large content separately
        if entry.content.size() > 1_000_000 { // 1MB threshold
            let cid = self.content_store.store(&entry.content).await?;
            entry.content = ClipContent::Reference(cid);
        }

        Ok(entry.id)
    }

    // Search clipboard history
    pub async fn search(&self, query: &str) -> Result<Vec<ClipEntry>> {
        self.search_index.search(query, 50)
    }

    // Get recent clips (with pagination)
    pub async fn recent(&self, limit: usize, offset: usize) -> Result<Vec<ClipEntry>> {
        self.local_db.query(
            "SELECT * FROM clips ORDER BY timestamp DESC LIMIT ? OFFSET ?",
            params![limit, offset]
        )
    }

    // Sync with peer using CRDT
    pub async fn sync_with_peer(&mut self, peer_id: &PeerId) -> Result<SyncStats> {
        // Exchange CRDT changes
        let changes = self.automerge_doc.get_changes();
        let peer_changes = self.p2p_network.exchange_changes(peer_id, changes).await?;

        // Merge automatically (conflict-free)
        self.automerge_doc.apply_changes(peer_changes)?;

        Ok(SyncStats { /* ... */ })
    }
}
```

### 1.4 Pin Important Clips

```rust
pub struct PinnedClipsManager {
    db: ClipHistoryDB,
}

impl PinnedClipsManager {
    // Pin a clip (appears at top of history)
    pub async fn pin_clip(&mut self, clip_id: &ClipId) -> Result<()> {
        self.db.update_clip(clip_id, |clip| {
            clip.pinned = true;
            clip.pin_timestamp = Some(Utc::now());
        }).await
    }

    // Get all pinned clips
    pub async fn get_pinned(&self) -> Result<Vec<ClipEntry>> {
        self.db.query("pinned = true ORDER BY pin_timestamp DESC").await
    }

    // Pin with optional expiry
    pub async fn pin_with_expiry(&mut self, clip_id: &ClipId, duration: Duration) -> Result<()> {
        let expiry = Utc::now() + duration;
        self.db.update_clip(clip_id, |clip| {
            clip.pinned = true;
            clip.pin_expiry = Some(expiry);
        }).await
    }
}
```

### 1.5 Content Type Organization

**Automatic Classification:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContentType {
    Text(TextType),
    Code(CodeType),
    Image(ImageFormat),
    File(FileReference),
    Url(UrlMetadata),
    RichText(RichTextFormat),
}

#[derive(Debug, Clone)]
pub enum TextType {
    PlainText,
    Markdown,
    Json,
    Xml,
    Csv,
}

#[derive(Debug, Clone)]
pub struct CodeType {
    pub language: String,      // Detected language (rust, python, js, etc.)
    pub syntax_tree: Option<Tree>, // Parsed AST for advanced features
}

pub struct ContentTypeDetector {
    // Tree-sitter parsers for code detection
    parsers: HashMap<String, TreeSitterParser>,
}

impl ContentTypeDetector {
    pub fn detect(&self, content: &str) -> ContentType {
        // 1. Check for URLs
        if Self::is_url(content) {
            return ContentType::Url(Self::extract_url_metadata(content));
        }

        // 2. Check for structured data
        if let Ok(_) = serde_json::from_str::<Value>(content) {
            return ContentType::Text(TextType::Json);
        }

        // 3. Try to detect code language
        if let Some(lang) = self.detect_code_language(content) {
            return ContentType::Code(CodeType {
                language: lang,
                syntax_tree: self.parse_code(content, &lang),
            });
        }

        // 4. Check for Markdown
        if Self::looks_like_markdown(content) {
            return ContentType::Text(TextType::Markdown);
        }

        // 5. Default to plain text
        ContentType::Text(TextType::PlainText)
    }

    fn detect_code_language(&self, content: &str) -> Option<String> {
        // Try parsing with different tree-sitter grammars
        for (lang, parser) in &self.parsers {
            if let Ok(tree) = parser.parse(content) {
                // Check if parse quality is high (few errors)
                if tree.root_node().has_error() {
                    continue;
                }
                return Some(lang.clone());
            }
        }
        None
    }
}
```

---

## 2. Smart Content Handling

### 2.1 Code Snippet Detection & Syntax Highlighting

**Implementation with Tree-sitter:**

```toml
[dependencies]
tree-sitter = "0.20"
tree-sitter-rust = "0.20"
tree-sitter-python = "0.20"
tree-sitter-javascript = "0.20"
tree-sitter-typescript = "0.20"
# ... add more languages as needed
```

```rust
use tree_sitter::{Parser, Language};

pub struct CodeSnippetHandler {
    parsers: HashMap<String, Parser>,
    highlighter: SyntaxHighlighter,
}

impl CodeSnippetHandler {
    // Detect and parse code snippet
    pub fn process_snippet(&mut self, content: &str) -> ProcessedSnippet {
        let lang = self.detect_language(content);

        ProcessedSnippet {
            original: content.to_string(),
            language: lang.clone(),
            highlighted_html: self.highlighter.highlight(content, &lang),
            formatted: self.format_code(content, &lang),
            ast: self.parse_ast(content, &lang),
        }
    }

    // Auto-format code on paste
    pub fn format_code(&self, content: &str, language: &str) -> String {
        match language {
            "rust" => self.format_with_rustfmt(content),
            "python" => self.format_with_black(content),
            "javascript" | "typescript" => self.format_with_prettier(content),
            _ => content.to_string(),
        }
    }

    // Extract metadata from code
    pub fn extract_metadata(&self, ast: &Tree) -> CodeMetadata {
        CodeMetadata {
            imports: self.extract_imports(ast),
            functions: self.extract_functions(ast),
            classes: self.extract_classes(ast),
            complexity: self.calculate_complexity(ast),
        }
    }
}
```

**Syntax Highlighting:**

```rust
use syntect::parsing::SyntaxSet;
use syntect::highlighting::{ThemeSet, Style};

pub struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl SyntaxHighlighter {
    pub fn highlight(&self, code: &str, language: &str) -> String {
        let syntax = self.syntax_set.find_syntax_by_extension(language)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];

        syntect::html::highlighted_html_for_string(
            code,
            &self.syntax_set,
            syntax,
            theme
        ).unwrap()
    }
}
```

### 2.2 URL Preview & Metadata Extraction

```rust
use reqwest::Client;
use scraper::{Html, Selector};

pub struct UrlPreviewEngine {
    http_client: Client,
    cache: LruCache<Url, UrlMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlMetadata {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub site_name: Option<String>,
    pub favicon: Option<String>,
    pub content_type: Option<String>,
}

impl UrlPreviewEngine {
    // Fetch and extract metadata from URL
    pub async fn fetch_metadata(&mut self, url: &str) -> Result<UrlMetadata> {
        // Check cache first
        if let Some(cached) = self.cache.get(&url.parse()?) {
            return Ok(cached.clone());
        }

        // Fetch HTML
        let response = self.http_client.get(url)
            .timeout(Duration::from_secs(5))
            .send()
            .await?;

        let html = response.text().await?;
        let document = Html::parse_document(&html);

        // Extract Open Graph metadata
        let metadata = UrlMetadata {
            url: url.to_string(),
            title: Self::extract_og_tag(&document, "og:title")
                .or_else(|| Self::extract_title(&document)),
            description: Self::extract_og_tag(&document, "og:description")
                .or_else(|| Self::extract_meta(&document, "description")),
            image: Self::extract_og_tag(&document, "og:image"),
            site_name: Self::extract_og_tag(&document, "og:site_name"),
            favicon: Self::extract_favicon(&document, url),
            content_type: Self::extract_og_tag(&document, "og:type"),
        };

        // Cache for 1 hour
        self.cache.put(url.parse()?, metadata.clone());

        Ok(metadata)
    }

    fn extract_og_tag(document: &Html, property: &str) -> Option<String> {
        let selector = Selector::parse(&format!("meta[property='{}']", property)).ok()?;
        document.select(&selector)
            .next()?
            .value()
            .attr("content")
            .map(String::from)
    }
}
```

### 2.3 Image Compression Before Sync

```rust
use image::{DynamicImage, ImageFormat};

pub struct ImageProcessor {
    max_dimension: u32,  // Max width/height (e.g., 2000px)
    quality: u8,         // JPEG/WebP quality (80)
}

impl ImageProcessor {
    // Compress image before syncing to peers
    pub async fn compress_for_sync(&self, image: &DynamicImage) -> Result<Vec<u8>> {
        let mut img = image.clone();

        // Resize if too large (preserve aspect ratio)
        if img.width() > self.max_dimension || img.height() > self.max_dimension {
            img = img.resize(
                self.max_dimension,
                self.max_dimension,
                image::imageops::FilterType::Lanczos3
            );
        }

        // Encode as WebP for best compression
        let mut buffer = Vec::new();
        let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut buffer);
        img.write_with_encoder(encoder)?;

        // Fallback to JPEG if WebP is larger (rare)
        let jpeg_buffer = self.encode_jpeg(&img)?;
        if jpeg_buffer.len() < buffer.len() {
            Ok(jpeg_buffer)
        } else {
            Ok(buffer)
        }
    }

    // Generate thumbnail for UI previews
    pub fn generate_thumbnail(&self, image: &DynamicImage) -> Vec<u8> {
        let thumb = image.resize(
            200, 200,
            image::imageops::FilterType::Triangle
        );

        let mut buffer = Vec::new();
        thumb.write_to(&mut buffer, ImageFormat::WebP).unwrap();
        buffer
    }
}
```

### 2.4 File Reference vs Full File Sync

```rust
pub enum ClipContent {
    Text(String),
    Image(ImageData),
    File(FileClip),
    FileReference(FileReference),
}

#[derive(Debug, Clone)]
pub struct FileClip {
    pub name: String,
    pub size: u64,
    pub data: Vec<u8>,          // Full file data
    pub mime_type: String,
}

#[derive(Debug, Clone)]
pub struct FileReference {
    pub path: PathBuf,          // Original file path
    pub cid: Cid,               // Content-addressed ID (if synced)
    pub size: u64,
    pub mime_type: String,
    pub available_on: Vec<DeviceId>, // Which devices have this file
}

pub struct FileClipHandler {
    content_store: ContentAddressedStore,
    max_inline_size: u64,  // Files smaller than this are inlined (e.g., 10MB)
}

impl FileClipHandler {
    // Decide whether to sync full file or just reference
    pub async fn handle_file_clip(&self, file_path: &Path) -> Result<ClipContent> {
        let metadata = fs::metadata(file_path).await?;
        let size = metadata.len();

        if size <= self.max_inline_size {
            // Small file: include full data
            let data = fs::read(file_path).await?;
            Ok(ClipContent::File(FileClip {
                name: file_path.file_name().unwrap().to_string_lossy().to_string(),
                size,
                data,
                mime_type: self.detect_mime_type(file_path),
            }))
        } else {
            // Large file: store in content-addressed storage, sync reference only
            let cid = self.content_store.add_file(file_path).await?;
            Ok(ClipContent::FileReference(FileReference {
                path: file_path.to_path_buf(),
                cid,
                size,
                mime_type: self.detect_mime_type(file_path),
                available_on: vec![self.device_id()],
            }))
        }
    }

    // On-demand file fetch from peer
    pub async fn fetch_file_from_peer(
        &self,
        file_ref: &FileReference,
        peer_id: &PeerId
    ) -> Result<PathBuf> {
        // Request file via Code Bridge's existing P2P transfer
        self.p2p_network.request_file(&file_ref.cid, peer_id).await
    }
}
```

### 2.5 Rich Text Preservation

```rust
pub struct RichTextHandler {
    // Preserve formatting across platforms
}

impl RichTextHandler {
    // Convert between different rich text formats
    pub fn normalize_rich_text(&self, content: &str, format: RichTextFormat) -> String {
        match format {
            RichTextFormat::Html => self.html_to_markdown(content),
            RichTextFormat::Rtf => self.rtf_to_markdown(content),
            RichTextFormat::Markdown => content.to_string(),
        }
    }

    // Preserve HTML when pasting from browser
    pub fn handle_html_clip(&self, html: &str, plain_text: &str) -> ClipContent {
        ClipContent::RichText(RichText {
            html: html.to_string(),
            plain: plain_text.to_string(),
            markdown: self.html_to_markdown(html),
        })
    }
}
```

---

## 3. Security Features

### 3.1 End-to-End Encryption

Building on Code Bridge's security model:

```rust
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce
};

pub struct ClipboardEncryption {
    // Per-device keypair (reuse Code Bridge's)
    device_keypair: Ed25519Keypair,

    // Shared secret with each peer (ECDH)
    peer_secrets: HashMap<PeerId, SharedSecret>,
}

impl ClipboardEncryption {
    // Encrypt clipboard content for specific peer
    pub fn encrypt_for_peer(
        &self,
        content: &[u8],
        peer_id: &PeerId
    ) -> Result<EncryptedClip> {
        let shared_secret = self.peer_secrets.get(peer_id)
            .ok_or(Error::NoPeerSecret)?;

        let cipher = ChaCha20Poly1305::new(shared_secret.as_bytes().into());
        let nonce = Nonce::from_slice(&self.generate_nonce());

        let ciphertext = cipher.encrypt(nonce, content)
            .map_err(|_| Error::EncryptionFailed)?;

        Ok(EncryptedClip {
            ciphertext,
            nonce: nonce.to_vec(),
            sender: self.device_keypair.public_key(),
        })
    }

    // Decrypt incoming clipboard content
    pub fn decrypt_from_peer(
        &self,
        encrypted: &EncryptedClip,
        peer_id: &PeerId
    ) -> Result<Vec<u8>> {
        let shared_secret = self.peer_secrets.get(peer_id)
            .ok_or(Error::NoPeerSecret)?;

        let cipher = ChaCha20Poly1305::new(shared_secret.as_bytes().into());
        let nonce = Nonce::from_slice(&encrypted.nonce);

        cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| Error::DecryptionFailed)
    }
}
```

### 3.2 Sensitive Data Detection

**Pattern-Based Detection:**

```rust
use regex::Regex;

pub struct SensitiveDataDetector {
    patterns: Vec<SensitivePattern>,
}

#[derive(Debug, Clone)]
pub struct SensitivePattern {
    pub name: String,
    pub regex: Regex,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy)]
pub enum Severity {
    Low,      // Email addresses, usernames
    Medium,   // SSH keys, tokens with low permissions
    High,     // Passwords, API keys, credit cards
    Critical, // Private keys, AWS keys with admin access
}

impl SensitiveDataDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                // AWS credentials
                SensitivePattern {
                    name: "AWS Access Key".to_string(),
                    regex: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
                    severity: Severity::Critical,
                },

                // API Keys (generic)
                SensitivePattern {
                    name: "API Key".to_string(),
                    regex: Regex::new(r"api[_-]?key['\"]?\s*[:=]\s*['\"]?([a-zA-Z0-9_\-]{32,})").unwrap(),
                    severity: Severity::High,
                },

                // GitHub tokens
                SensitivePattern {
                    name: "GitHub Token".to_string(),
                    regex: Regex::new(r"ghp_[a-zA-Z0-9]{36}").unwrap(),
                    severity: Severity::High,
                },

                // Private SSH keys
                SensitivePattern {
                    name: "SSH Private Key".to_string(),
                    regex: Regex::new(r"-----BEGIN (RSA|OPENSSH|DSA|EC|PGP) PRIVATE KEY-----").unwrap(),
                    severity: Severity::Critical,
                },

                // JWT tokens
                SensitivePattern {
                    name: "JWT Token".to_string(),
                    regex: Regex::new(r"eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}").unwrap(),
                    severity: Severity::Medium,
                },

                // Credit card numbers
                SensitivePattern {
                    name: "Credit Card".to_string(),
                    regex: Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").unwrap(),
                    severity: Severity::Critical,
                },

                // Passwords in code
                SensitivePattern {
                    name: "Password in Code".to_string(),
                    regex: Regex::new(r#"password['\"]?\s*[:=]\s*['\"]([^'"\s]{8,})"#).unwrap(),
                    severity: Severity::High,
                },
            ],
        }
    }

    // Scan content for sensitive data
    pub fn scan(&self, content: &str) -> Vec<SensitiveMatch> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            if let Some(captures) = pattern.regex.captures(content) {
                matches.push(SensitiveMatch {
                    pattern_name: pattern.name.clone(),
                    matched_text: captures.get(0).unwrap().as_str().to_string(),
                    severity: pattern.severity,
                    position: captures.get(0).unwrap().start(),
                });
            }
        }

        matches
    }

    // Check if content should be blocked from syncing
    pub fn should_block_sync(&self, content: &str) -> bool {
        self.scan(content).iter()
            .any(|m| matches!(m.severity, Severity::Critical | Severity::High))
    }
}
```

**AI-Powered Detection (Optional):**

```rust
pub struct AiSensitiveDetector {
    // Use local LLM for more nuanced detection
    model: OnnxModel,
}

impl AiSensitiveDetector {
    // Classify content using ML
    pub async fn classify(&self, content: &str) -> SensitivityScore {
        // Run lightweight classifier (e.g., DistilBERT fine-tuned)
        // Returns probability that content contains sensitive data
    }
}
```

### 3.3 Auto-Clear Sensitive Clips

```rust
pub struct ClipboardSecurityManager {
    detector: SensitiveDataDetector,
    auto_clear_duration: Duration,
}

impl ClipboardSecurityManager {
    // Monitor clipboard and auto-clear sensitive content
    pub async fn monitor_and_protect(&mut self) {
        let mut last_clip_hash = None;

        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;

            let clip = self.clipboard.get_text().ok();
            if let Some(content) = clip {
                let hash = Self::hash_content(&content);

                // Skip if unchanged
                if Some(hash) == last_clip_hash {
                    continue;
                }
                last_clip_hash = Some(hash);

                // Scan for sensitive data
                let matches = self.detector.scan(&content);

                if !matches.is_empty() {
                    // Notify user
                    self.notify_sensitive_data_detected(&matches).await;

                    // Auto-clear after timeout (configurable)
                    let duration = self.auto_clear_duration;
                    tokio::spawn(async move {
                        tokio::time::sleep(duration).await;
                        // Clear clipboard
                        let _ = Clipboard::new().unwrap().clear();
                    });

                    // Optionally: Block sync entirely
                    if self.config.block_sensitive_sync {
                        self.block_clip_from_sync(&content).await;
                    }
                }
            }
        }
    }

    // User notification
    async fn notify_sensitive_data_detected(&self, matches: &[SensitiveMatch]) {
        let message = format!(
            "Sensitive data detected in clipboard: {}\nWill auto-clear in {} seconds",
            matches.iter().map(|m| &m.pattern_name).collect::<Vec<_>>().join(", "),
            self.auto_clear_duration.as_secs()
        );

        // Show OS notification
        #[cfg(target_os = "macos")]
        self.show_macos_notification(&message).await;

        #[cfg(target_os = "linux")]
        self.show_linux_notification(&message).await;

        #[cfg(target_os = "windows")]
        self.show_windows_notification(&message).await;
    }
}
```

### 3.4 Selective Sync (Exclude Certain Apps)

```rust
pub struct AppSourceFilter {
    // Blacklist: Never sync from these apps
    blacklisted_apps: HashSet<String>,

    // Whitelist mode: Only sync from these apps
    whitelisted_apps: Option<HashSet<String>>,
}

impl AppSourceFilter {
    // Detect which app copied to clipboard
    pub fn get_source_app(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        {
            // Use NSWorkspace to get active app
            use cocoa::appkit::NSWorkspace;
            // ... get frontmost app bundle ID
        }

        #[cfg(target_os = "linux")]
        {
            // Use X11 _NET_ACTIVE_WINDOW or sway/i3 IPC
            // ... get active window class
        }

        #[cfg(target_os = "windows")]
        {
            // Use GetForegroundWindow + GetWindowThreadProcessId
            // ... get process name
        }
    }

    // Check if clip should be synced based on source app
    pub fn should_sync_from_app(&self, app: &str) -> bool {
        // Check blacklist
        if self.blacklisted_apps.contains(app) {
            return false;
        }

        // Check whitelist (if enabled)
        if let Some(whitelist) = &self.whitelisted_apps {
            return whitelist.contains(app);
        }

        true
    }
}

// Example configuration
// ~/.config/codebridge/clipboard.toml
/*
[clipboard.filter]
# Never sync from password managers
blacklist = [
    "1Password",
    "Bitwarden",
    "KeePassXC",
    "com.agilebits.onepassword7",
]

# Or use whitelist mode
# whitelist = ["Code", "Terminal", "iTerm", "Slack"]
*/
```

### 3.5 Audit Logging

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardAuditLog {
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub device_id: DeviceId,
    pub content_hash: Hash,        // SHA-256 of content
    pub content_type: ContentType,
    pub source_app: Option<String>,
    pub destination_peers: Vec<PeerId>,
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AuditEventType {
    ClipboardCopied,
    ClipboardSynced,
    ClipboardReceived,
    SensitiveDataDetected,
    SyncBlocked,
    ClipboardCleared,
}

pub struct AuditLogger {
    log_file: PathBuf,
}

impl AuditLogger {
    pub async fn log_event(&mut self, event: ClipboardAuditLog) -> Result<()> {
        // Append to JSON lines file
        let json = serde_json::to_string(&event)?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
            .await?;

        file.write_all(format!("{}\n", json).as_bytes()).await?;

        Ok(())
    }

    // Query audit logs
    pub async fn query_logs(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        event_type: Option<AuditEventType>,
    ) -> Result<Vec<ClipboardAuditLog>> {
        // Read and filter log file
        let contents = fs::read_to_string(&self.log_file).await?;

        let logs: Vec<ClipboardAuditLog> = contents.lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .filter(|log| {
                log.timestamp >= start && log.timestamp <= end &&
                event_type.as_ref().map_or(true, |t| &log.event_type == t)
            })
            .collect();

        Ok(logs)
    }
}
```

---

## 4. Developer-Focused Features

### 4.1 Code Formatting on Paste

```rust
pub struct PasteFormatter {
    formatters: HashMap<String, Box<dyn CodeFormatter>>,
}

pub trait CodeFormatter: Send + Sync {
    fn format(&self, code: &str) -> Result<String>;
}

// Rust formatter using rustfmt
pub struct RustFormatter;

impl CodeFormatter for RustFormatter {
    fn format(&self, code: &str) -> Result<String> {
        // Call rustfmt API or CLI
        let formatted = rustfmt::format_input(
            rustfmt::Input::Text(code.to_string()),
            &rustfmt::Config::default(),
            None
        )?;

        Ok(formatted.to_string())
    }
}

// Auto-indent based on context
pub struct SmartIndenter {
    tab_width: usize,
    use_spaces: bool,
}

impl SmartIndenter {
    // Detect current indentation and adjust pasted code
    pub fn adjust_indentation(&self, code: &str, cursor_context: &str) -> String {
        // 1. Detect indent level at cursor position
        let current_indent = self.detect_indent_level(cursor_context);

        // 2. Normalize pasted code (remove common leading whitespace)
        let normalized = self.normalize_indentation(code);

        // 3. Re-indent to match cursor context
        self.apply_indentation(&normalized, current_indent)
    }

    fn detect_indent_level(&self, context: &str) -> usize {
        // Count leading whitespace of current line
        context.chars()
            .take_while(|c| c.is_whitespace())
            .count()
    }
}
```

### 4.2 Convert Between Formats (JSON ↔ YAML)

```rust
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;

pub struct FormatConverter {
    // Supported conversions
}

impl FormatConverter {
    // JSON to YAML
    pub fn json_to_yaml(&self, json: &str) -> Result<String> {
        let value: JsonValue = serde_json::from_str(json)?;
        let yaml = serde_yaml::to_string(&value)?;
        Ok(yaml)
    }

    // YAML to JSON
    pub fn yaml_to_json(&self, yaml: &str) -> Result<String> {
        let value: YamlValue = serde_yaml::from_str(yaml)?;
        let json = serde_json::to_string_pretty(&value)?;
        Ok(json)
    }

    // JSON to TOML
    pub fn json_to_toml(&self, json: &str) -> Result<String> {
        let value: JsonValue = serde_json::from_str(json)?;
        let toml = toml::to_string_pretty(&value)?;
        Ok(toml)
    }

    // Auto-detect and convert
    pub fn auto_convert(&self, content: &str, target_format: Format) -> Result<String> {
        let detected = self.detect_format(content)?;

        match (detected, target_format) {
            (Format::Json, Format::Yaml) => self.json_to_yaml(content),
            (Format::Yaml, Format::Json) => self.yaml_to_json(content),
            (Format::Json, Format::Toml) => self.json_to_toml(content),
            // ... handle all combinations
            _ => Ok(content.to_string()),
        }
    }
}

// Quick actions in UI
pub enum QuickAction {
    FormatCode,
    MinifyJson,
    PrettifyJson,
    ConvertToYaml,
    ConvertToJson,
    UrlEncode,
    UrlDecode,
    Base64Encode,
    Base64Decode,
    HashSha256,
    ToUpperCase,
    ToLowerCase,
    RemoveWhitespace,
}
```

### 4.3 Strip Formatting Option

```rust
pub struct ClipboardFormatter {
    // Strip all rich text formatting
    pub fn strip_formatting(&self, clip: &ClipContent) -> String {
        match clip {
            ClipContent::RichText(rt) => rt.plain.clone(),
            ClipContent::Html(html) => self.html_to_plain_text(html),
            ClipContent::Text(text) => text.clone(),
            _ => String::new(),
        }
    }

    // HTML to plain text (preserve structure)
    fn html_to_plain_text(&self, html: &str) -> String {
        use html2text::from_read;

        from_read(html.as_bytes(), 80)
    }

    // Paste as plain text (keyboard shortcut: Cmd+Shift+V)
    pub async fn paste_as_plain_text(&mut self) -> Result<()> {
        let clip = self.clipboard.get_clip()?;
        let plain = self.strip_formatting(&clip);

        // Set clipboard to plain text only
        self.clipboard.set_text(plain)?;

        // Simulate paste (platform-specific)
        self.simulate_paste_keystroke().await?;

        Ok(())
    }
}
```

### 4.4 Template/Variable Substitution

```rust
use tera::{Tera, Context};

pub struct TemplateEngine {
    tera: Tera,
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    // Expand template in clipboard
    pub fn expand_template(&self, template: &str) -> Result<String> {
        let mut context = Context::new();

        // Add user-defined variables
        for (key, value) in &self.variables {
            context.insert(key, value);
        }

        // Add system variables
        context.insert("date", &Utc::now().format("%Y-%m-%d").to_string());
        context.insert("time", &Utc::now().format("%H:%M:%S").to_string());
        context.insert("username", &whoami::username());
        context.insert("hostname", &whoami::hostname());

        self.tera.render_str(template, &context)
            .map_err(Into::into)
    }
}

// Example templates:
/*
// ~/.config/codebridge/templates/git-commit.txt
fix({{ module }}): {{ description }}

- {{ detail1 }}
- {{ detail2 }}

Closes #{{ issue_number }}

// Expansion:
fix(auth): resolve login timeout issue

- Increase timeout from 5s to 30s
- Add retry logic for network failures

Closes #1234
*/
```

### 4.5 Regex-Based Transformations

```rust
use regex::Regex;

pub struct RegexTransformer {
    // Saved regex transformations
    saved_transforms: Vec<RegexTransform>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegexTransform {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    pub description: Option<String>,
}

impl RegexTransformer {
    // Apply regex transformation to clipboard
    pub fn transform(&self, content: &str, transform: &RegexTransform) -> Result<String> {
        let re = Regex::new(&transform.pattern)?;
        Ok(re.replace_all(content, &transform.replacement).to_string())
    }

    // Predefined useful transforms
    pub fn default_transforms() -> Vec<RegexTransform> {
        vec![
            RegexTransform {
                name: "Extract URLs".to_string(),
                pattern: r"https?://[^\s]+".to_string(),
                replacement: "$0\n".to_string(),
                description: Some("Extract all URLs to separate lines".to_string()),
            },

            RegexTransform {
                name: "CamelCase to snake_case".to_string(),
                pattern: r"([a-z])([A-Z])".to_string(),
                replacement: r"$1_$2".to_string(),
                description: Some("Convert CamelCase to snake_case".to_string()),
            },

            RegexTransform {
                name: "Remove trailing whitespace".to_string(),
                pattern: r"\s+$".to_string(),
                replacement: "".to_string(),
                description: Some("Remove trailing whitespace from lines".to_string()),
            },

            RegexTransform {
                name: "Extract email addresses".to_string(),
                pattern: r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b".to_string(),
                replacement: "$0\n".to_string(),
                description: Some("Extract all email addresses".to_string()),
            },
        ]
    }
}
```

---

## 5. Integration Ideas

### 5.1 Paste to Specific Device

```rust
pub struct TargetedClipboardSync {
    p2p_network: P2pNetwork,
    connected_peers: HashMap<PeerId, PeerInfo>,
}

impl TargetedClipboardSync {
    // Send clipboard content to specific device only
    pub async fn send_to_device(&self, device_id: &PeerId, content: ClipContent) -> Result<()> {
        // Create one-time clipboard sync message
        let message = ClipboardSyncMessage {
            id: Uuid::new_v4(),
            content,
            sender: self.device_id(),
            timestamp: Utc::now(),
            mode: SyncMode::Targeted,
        };

        // Send directly to target device
        self.p2p_network.send_to_peer(device_id, message).await?;

        Ok(())
    }

    // UI: Quick share menu
    pub fn show_device_picker(&self) -> Vec<DeviceOption> {
        self.connected_peers.iter()
            .map(|(id, info)| DeviceOption {
                id: id.clone(),
                name: info.name.clone(),
                platform: info.platform,
                online: info.online,
            })
            .collect()
    }
}

// Example CLI:
// codebridge clip send mac-mini
// codebridge clip send --device ubuntu-workstation
```

### 5.2 Clipboard Queue/Stack Mode

```rust
#[derive(Debug, Clone)]
pub enum ClipboardMode {
    Normal,        // Last-in, current clipboard
    Queue(VecDeque<ClipEntry>),  // FIFO queue
    Stack(Vec<ClipEntry>),       // LIFO stack
}

pub struct ClipboardModeManager {
    current_mode: ClipboardMode,
}

impl ClipboardModeManager {
    // Push to stack (Cmd+C multiple times)
    pub fn push_to_stack(&mut self, entry: ClipEntry) {
        match &mut self.current_mode {
            ClipboardMode::Stack(stack) => {
                stack.push(entry);
            }
            _ => {
                // Switch to stack mode
                self.current_mode = ClipboardMode::Stack(vec![entry]);
            }
        }
    }

    // Pop from stack (Cmd+V cycles through)
    pub fn pop_from_stack(&mut self) -> Option<ClipEntry> {
        match &mut self.current_mode {
            ClipboardMode::Stack(stack) => stack.pop(),
            _ => None,
        }
    }

    // Enqueue (for multi-paste workflows)
    pub fn enqueue(&mut self, entry: ClipEntry) {
        match &mut self.current_mode {
            ClipboardMode::Queue(queue) => {
                queue.push_back(entry);
            }
            _ => {
                let mut queue = VecDeque::new();
                queue.push_back(entry);
                self.current_mode = ClipboardMode::Queue(queue);
            }
        }
    }

    // Dequeue
    pub fn dequeue(&mut self) -> Option<ClipEntry> {
        match &mut self.current_mode {
            ClipboardMode::Queue(queue) => queue.pop_front(),
            _ => None,
        }
    }
}

// Use case: Copy multiple snippets, paste in order
/*
1. Copy snippet A (added to stack)
2. Copy snippet B (added to stack)
3. Copy snippet C (added to stack)
4. Paste: C appears
5. Paste: B appears
6. Paste: A appears
*/
```

### 5.3 Quick Actions on Clipboard Content

```rust
pub struct QuickActionsEngine {
    actions: Vec<QuickAction>,
}

#[derive(Debug, Clone)]
pub struct QuickAction {
    pub name: String,
    pub icon: String,
    pub action: ActionType,
    pub applicable_to: Vec<ContentType>,
}

#[derive(Debug, Clone)]
pub enum ActionType {
    // Code actions
    FormatCode,
    RunCode,
    ExplainCode,

    // URL actions
    OpenInBrowser,
    FetchMetadata,
    ShortenUrl,

    // Image actions
    CompressImage,
    ConvertFormat,
    ExtractText,  // OCR

    // Text actions
    Translate(String),  // target language
    SummarizeText,
    CountWords,

    // File actions
    UploadToGist,
    ShareViaPaste,

    // Custom shell command
    CustomCommand(String),
}

impl QuickActionsEngine {
    // Get applicable actions for current clipboard content
    pub fn get_actions_for_content(&self, content: &ClipContent) -> Vec<QuickAction> {
        let content_type = content.content_type();

        self.actions.iter()
            .filter(|action| action.applicable_to.contains(&content_type))
            .cloned()
            .collect()
    }

    // Execute action
    pub async fn execute_action(&self, action: &QuickAction, content: &ClipContent) -> Result<ActionResult> {
        match &action.action {
            ActionType::FormatCode => {
                let formatted = self.format_code(content)?;
                Ok(ActionResult::Clipboard(formatted))
            }

            ActionType::OpenInBrowser => {
                if let ClipContent::Url(url) = content {
                    webbrowser::open(&url.url)?;
                    Ok(ActionResult::Success)
                } else {
                    Err(Error::InvalidContentType)
                }
            }

            ActionType::ExtractText => {
                if let ClipContent::Image(img) = content {
                    let text = self.ocr_extract(img).await?;
                    Ok(ActionResult::Text(text))
                } else {
                    Err(Error::InvalidContentType)
                }
            }

            ActionType::CustomCommand(cmd) => {
                // Run custom shell command with clipboard content as stdin
                let output = self.run_command(cmd, content).await?;
                Ok(ActionResult::Text(output))
            }

            _ => Err(Error::UnsupportedAction),
        }
    }
}
```

### 5.4 Share to Specific Peer

Already covered in 5.1, but with UI enhancement:

```rust
// Menu bar quick share
pub struct QuickShareMenu {
    peers: Vec<PeerInfo>,
}

impl QuickShareMenu {
    // Show system menu with connected devices
    pub fn show_menu(&self) -> Result<Option<PeerId>> {
        #[cfg(target_os = "macos")]
        {
            // Use NSMenu
            // Show: "Share clipboard with: [Mac mini] [Ubuntu] [iPhone]"
        }

        #[cfg(target_os = "linux")]
        {
            // Use libappindicator or similar
        }

        #[cfg(target_os = "windows")]
        {
            // Use Windows notification with action buttons
        }
    }
}
```

### 5.5 Integration with Snippet Manager

```rust
// Integrate with Code Bridge's existing storage
pub struct SnippetManager {
    storage: ContentAddressedStore,
    db: ClipHistoryDB,
}

impl SnippetManager {
    // Save clipboard as permanent snippet
    pub async fn save_as_snippet(
        &mut self,
        clip: &ClipEntry,
        snippet: SnippetMetadata
    ) -> Result<SnippetId> {
        let snippet_entry = Snippet {
            id: Uuid::new_v4(),
            content: clip.content.clone(),
            title: snippet.title,
            description: snippet.description,
            tags: snippet.tags,
            language: snippet.language,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            usage_count: 0,
        };

        // Store in content-addressed storage
        let cid = self.storage.store(&snippet_entry).await?;

        // Index for search
        self.db.add_snippet(snippet_entry).await?;

        Ok(snippet_entry.id)
    }

    // Browse and insert snippets
    pub async fn browse_snippets(&self, query: Option<&str>) -> Vec<Snippet> {
        if let Some(q) = query {
            self.db.search_snippets(q).await.unwrap_or_default()
        } else {
            self.db.all_snippets().await.unwrap_or_default()
        }
    }
}
```

---

## 6. Innovative Features

### 6.1 Voice-to-Clipboard Transcription

```rust
use whisper_rs::{WhisperContext, FullParams};

pub struct VoiceToClipboard {
    whisper: WhisperContext,
    audio_input: AudioRecorder,
}

impl VoiceToClipboard {
    // Record audio and transcribe to clipboard
    pub async fn record_and_transcribe(&mut self) -> Result<String> {
        // Start recording
        println!("Recording... (press Enter to stop)");
        let audio_data = self.audio_input.record().await?;

        // Transcribe using Whisper (runs locally)
        let mut params = FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        self.whisper.full(params, &audio_data)?;

        // Extract transcribed text
        let num_segments = self.whisper.full_n_segments()?;
        let mut transcript = String::new();

        for i in 0..num_segments {
            let segment_text = self.whisper.full_get_segment_text(i)?;
            transcript.push_str(&segment_text);
            transcript.push(' ');
        }

        // Copy to clipboard
        let mut clipboard = Clipboard::new()?;
        clipboard.set_text(&transcript)?;

        Ok(transcript)
    }
}

// CLI usage:
// codebridge clip voice-record
// codebridge clip transcribe audio.wav
```

### 6.2 Screenshot Region to Clipboard

Integration with Code Bridge's screenshot system:

```rust
pub struct ScreenshotToClipboard {
    #[cfg(target_os = "macos")]
    screen_capture: ScreenCaptureKit,

    #[cfg(target_os = "linux")]
    flameshot: FlameshotCapture,

    #[cfg(target_os = "windows")]
    windows_capture: WindowsScreenCapture,
}

impl ScreenshotToClipboard {
    // Capture screenshot region and copy to clipboard
    pub async fn capture_region(&self) -> Result<ClipContent> {
        #[cfg(target_os = "macos")]
        {
            // Use ScreenCaptureKit for region selection
            let region = self.select_region_interactive().await?;
            let screenshot = self.screen_capture.capture_region(region).await?;

            // Process image
            let processed = self.process_screenshot(screenshot).await?;

            Ok(ClipContent::Image(processed))
        }

        #[cfg(target_os = "linux")]
        {
            // Use Flameshot in region mode
            let screenshot = self.flameshot.capture_region().await?;
            Ok(ClipContent::Image(screenshot))
        }

        #[cfg(target_os = "windows")]
        {
            // Use Windows.Graphics.Capture API
            let screenshot = self.windows_capture.capture_region().await?;
            Ok(ClipContent::Image(screenshot))
        }
    }

    // With OCR
    pub async fn capture_and_ocr(&self) -> Result<String> {
        let screenshot = self.capture_region().await?;

        if let ClipContent::Image(img) = screenshot {
            let text = self.ocr_engine.extract_text(&img).await?;

            // Copy text to clipboard
            let mut clipboard = Clipboard::new()?;
            clipboard.set_text(&text)?;

            Ok(text)
        } else {
            Err(Error::InvalidContentType)
        }
    }
}

// Keyboard shortcuts:
// Cmd+Shift+4 -> Screenshot to clipboard (macOS default)
// Cmd+Shift+5 -> Screenshot with OCR to clipboard (custom)
```

### 6.3 AI-Powered Clipboard Enhancement

```rust
use llm::{Model, InferenceSession};

pub struct AiClipboardEnhancer {
    // Local LLM (e.g., Llama 3, Mistral)
    model: Box<dyn Model>,
}

impl AiClipboardEnhancer {
    // Enhance clipboard content with AI
    pub async fn enhance(&self, content: &str, task: EnhancementTask) -> Result<String> {
        let prompt = match task {
            EnhancementTask::ImproveWriting => {
                format!("Improve the following text while preserving its meaning:\n\n{}", content)
            }

            EnhancementTask::FixGrammar => {
                format!("Fix grammar and spelling errors:\n\n{}", content)
            }

            EnhancementTask::Summarize => {
                format!("Provide a concise summary:\n\n{}", content)
            }

            EnhancementTask::ExplainCode => {
                format!("Explain what this code does:\n\n```\n{}\n```", content)
            }

            EnhancementTask::GenerateDocstring => {
                format!("Generate a docstring for this function:\n\n{}", content)
            }

            EnhancementTask::TranslateToLanguage(lang) => {
                format!("Translate to {}:\n\n{}", lang, content)
            }
        };

        // Run inference
        let mut session = self.model.start_session(Default::default());
        let enhanced = self.model.infer(&mut session, &prompt)?;

        Ok(enhanced)
    }

    // Contextual suggestions based on clipboard content
    pub fn suggest_actions(&self, content: &ClipContent) -> Vec<SuggestedAction> {
        match content {
            ClipContent::Code(code) => vec![
                SuggestedAction::new("Explain code", EnhancementTask::ExplainCode),
                SuggestedAction::new("Add comments", EnhancementTask::GenerateDocstring),
                SuggestedAction::new("Optimize", EnhancementTask::OptimizeCode),
            ],

            ClipContent::Text(text) if text.len() > 1000 => vec![
                SuggestedAction::new("Summarize", EnhancementTask::Summarize),
                SuggestedAction::new("Extract key points", EnhancementTask::ExtractKeyPoints),
            ],

            ClipContent::Text(_) => vec![
                SuggestedAction::new("Improve writing", EnhancementTask::ImproveWriting),
                SuggestedAction::new("Fix grammar", EnhancementTask::FixGrammar),
            ],

            _ => vec![],
        }
    }
}
```

### 6.4 Cross-Device Drag-and-Drop Simulation

```rust
pub struct VirtualDragDrop {
    p2p_network: P2pNetwork,
}

impl VirtualDragDrop {
    // Initiate virtual drag-and-drop to remote device
    pub async fn drag_to_device(
        &self,
        content: ClipContent,
        target_device: &PeerId,
        destination: VirtualLocation
    ) -> Result<()> {
        // Send drag-drop message
        let message = DragDropMessage {
            content,
            destination,
            action: DropAction::Move,  // or Copy
        };

        self.p2p_network.send_to_peer(target_device, message).await?;

        Ok(())
    }

    // Receive and handle drag-drop from remote device
    pub async fn handle_incoming_drop(&self, message: DragDropMessage) -> Result<()> {
        match message.destination {
            VirtualLocation::Desktop => {
                // Save file to desktop
                let desktop = dirs::desktop_dir().unwrap();
                self.save_file_to(&message.content, &desktop).await?;
            }

            VirtualLocation::Downloads => {
                let downloads = dirs::download_dir().unwrap();
                self.save_file_to(&message.content, &downloads).await?;
            }

            VirtualLocation::CurrentFolder => {
                // Drop into currently active folder
                let current_folder = self.get_active_folder()?;
                self.save_file_to(&message.content, &current_folder).await?;
            }

            VirtualLocation::ApplicationWindow(app) => {
                // Simulate paste into target application
                self.paste_into_app(&app, &message.content).await?;
            }
        }

        Ok(())
    }
}

// Use case: Drag image from macOS to Ubuntu desktop
// 1. On macOS: Right-click image -> "Send to Ubuntu Desktop"
// 2. On Ubuntu: Image appears on desktop
```

### 6.5 Smart Clipboard Predictions

```rust
pub struct ClipboardPredictor {
    // ML model for predicting next clipboard action
    prediction_model: PredictionModel,

    // Usage history
    clipboard_history: Vec<ClipEntry>,
}

impl ClipboardPredictor {
    // Predict what user might want to copy next
    pub fn predict_next_clips(&self, context: &Context) -> Vec<PredictedClip> {
        // Analyze:
        // - Recent clipboard history
        // - Current app
        // - Time of day
        // - Previous patterns

        let features = self.extract_features(context);
        let predictions = self.prediction_model.predict(&features);

        predictions
    }

    // Suggest based on repetitive patterns
    pub fn suggest_snippet(&self) -> Option<ClipEntry> {
        // Example: User copies same email signature every day
        // Suggest pinning it or creating a snippet
    }
}
```

---

## 7. Implementation Recommendations

### 7.1 Phased Rollout

**Phase 1: Core Clipboard Sync (Weeks 1-4)**
- Basic P2P clipboard sync
- Cross-platform clipboard access (arboard)
- Real-time sync using existing libp2p infrastructure
- Clipboard history (local storage)
- Simple UI (system tray menu)

**Phase 2: Security & Filtering (Weeks 5-6)**
- Sensitive data detection
- Auto-clear timers
- App source filtering
- End-to-end encryption (reuse Code Bridge's)
- Audit logging

**Phase 3: Developer Features (Weeks 7-9)**
- Code snippet detection
- Syntax highlighting
- Format conversions (JSON/YAML/TOML)
- Regex transformations
- Quick actions

**Phase 4: Advanced Features (Weeks 10-12)**
- Pin/favorite clips
- Searchable history (full-text search)
- Screenshot to clipboard
- Voice transcription (Whisper)
- AI enhancements (optional)

### 7.2 Technology Stack

```toml
[dependencies]
# Clipboard access
arboard = "3.4"

# P2P networking (reuse Code Bridge's)
libp2p = "0.54"

# CRDT for clipboard history sync
automerge = "0.6"

# Database
rusqlite = "0.32"

# Full-text search
tantivy = "0.22"

# Code parsing
tree-sitter = "0.20"
tree-sitter-rust = "0.20"

# Syntax highlighting
syntect = "5.2"

# Image processing
image = "0.25"

# OCR (optional)
tesseract = "0.14"

# Audio transcription (optional)
whisper-rs = "0.10"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
toml = "0.8"

# Encryption (reuse Code Bridge's)
chacha20poly1305 = "0.10"
ed25519-dalek = "2.1"

# Utilities
regex = "1.10"
chrono = "0.4"
uuid = { version = "1.6", features = ["v4"] }
```

### 7.3 User Interface Options

**Option 1: Menu Bar App (Recommended)**
- System tray icon on all platforms
- Quick access to recent clips
- Search bar for history
- Settings panel

**Option 2: Keyboard-First (Power Users)**
- Global hotkeys for clipboard actions
- Fuzzy search popup (Cmd+Shift+V)
- Minimal UI, maximum efficiency

**Option 3: Integration with Existing Code Bridge UI**
- Add "Clipboard" tab to Code Bridge desktop app
- Unified experience with file sharing

### 7.4 Configuration Example

```toml
# ~/.config/codebridge/clipboard.toml

[clipboard]
# Enable clipboard sync
enabled = true

# Max history size
max_history = 1000

# Auto-clear after (seconds, 0 = never)
auto_clear_duration = 0

# Sync mode
sync_mode = "auto"  # auto, manual, targeted

[clipboard.sync]
# Sync images
sync_images = true

# Max image size to sync (bytes)
max_image_size = 10_000_000  # 10MB

# Sync files
sync_files = true

# Max file size to sync
max_file_size = 100_000_000  # 100MB

[clipboard.security]
# Enable sensitive data detection
detect_sensitive = true

# Auto-clear sensitive clips after (seconds)
sensitive_auto_clear = 30

# Block sync of sensitive data
block_sensitive_sync = true

# Patterns to detect (regex)
sensitive_patterns = [
    "AKIA[0-9A-Z]{16}",  # AWS keys
    "ghp_[a-zA-Z0-9]{36}",  # GitHub tokens
]

[clipboard.filter]
# Never sync from these apps
blacklist = [
    "1Password",
    "Bitwarden",
    "KeePassXC",
]

# Only sync from these apps (if set, blacklist ignored)
# whitelist = ["Code", "Terminal", "Slack"]

[clipboard.features]
# Enable code detection
code_detection = true

# Enable syntax highlighting
syntax_highlighting = true

# Enable URL preview
url_preview = true

# Enable OCR on images
ocr_enabled = false

# Enable voice transcription
voice_transcription = false

# Enable AI enhancements
ai_enhancements = false

[clipboard.ui]
# Show notifications
show_notifications = true

# Menu bar icon
show_menu_bar = true

# Global hotkey for clipboard history
hotkey = "Cmd+Shift+V"  # macOS
# hotkey = "Ctrl+Shift+V"  # Linux/Windows
```

### 7.5 CLI Interface

```bash
# Clipboard history
codebridge clip list                    # Show recent clips
codebridge clip search "error"          # Search clipboard history
codebridge clip get 5                   # Get clip by index
codebridge clip delete 5                # Delete clip
codebridge clip clear                   # Clear all history

# Pinning
codebridge clip pin 3                   # Pin clip
codebridge clip unpin 3                 # Unpin clip
codebridge clip pins                    # List pinned clips

# Syncing
codebridge clip sync                    # Manual sync
codebridge clip send mac-mini           # Send to specific device
codebridge clip watch                   # Watch and auto-sync

# Transformations
codebridge clip format                  # Format code in clipboard
codebridge clip convert yaml            # Convert JSON to YAML
codebridge clip transform camel_to_snake # Apply regex transform

# Advanced
codebridge clip ocr                     # OCR on image in clipboard
codebridge clip voice-record            # Voice to text
codebridge clip enhance summarize       # AI enhancement
```

---

## 8. Competitive Analysis

| Feature | Code Bridge Clipboard | Clipboard Manager A | Clipboard Manager B |
|---------|---------------------|-------------------|-------------------|
| **P2P Sync** | ✅ libp2p, no cloud | ❌ Cloud-based | ❌ No sync |
| **Cross-Platform** | ✅ macOS, Linux, Windows | ⚠️ macOS/iOS only | ✅ All platforms |
| **Code Highlighting** | ✅ Tree-sitter | ❌ None | ⚠️ Basic |
| **Sensitive Detection** | ✅ Regex + AI | ❌ None | ❌ None |
| **Offline-First** | ✅ Full offline support | ❌ Requires cloud | ✅ Local only |
| **Developer Features** | ✅ Format conversion, regex | ❌ Consumer-focused | ❌ Basic only |
| **E2E Encryption** | ✅ ChaCha20-Poly1305 | ⚠️ TLS only | ❌ None |
| **Open Source** | ✅ Apache 2.0 / MIT | ❌ Proprietary | ✅ GPL |
| **Voice Transcription** | ✅ Local (Whisper) | ❌ None | ❌ None |
| **AI Enhancements** | ✅ Local LLM | ❌ None | ❌ None |

**Unique Selling Points:**
1. **True P2P** - No cloud dependency, works on LAN
2. **Developer-Native** - Code formatting, regex, conversions
3. **Privacy-First** - Sensitive data detection, E2E encryption
4. **Integrated** - Part of Code Bridge ecosystem
5. **Local AI** - No data sent to third parties

---

## 9. Privacy & Compliance

### Data Handling
- **Local-First:** All clipboard data stored locally by default
- **Encrypted in Transit:** ChaCha20-Poly1305 for P2P sync
- **No Telemetry:** Zero data collection
- **User Control:** Users can disable sync, auto-clear, filter apps

### Security Best Practices
- Never sync password manager apps (default blacklist)
- Auto-detect and warn about sensitive data
- Audit logging for compliance
- Option to disable cloud features entirely

---

## 10. Future Enhancements

### Year 1+
- **Mobile Support:** iOS/Android clipboard sync
- **Browser Extension:** Sync browser clipboard
- **Team Features:** Shared clipboard for teams
- **Smart Suggestions:** ML-powered clipboard predictions
- **Plugin System:** Custom clipboard processors

### Research Areas
- **Federated Learning:** Privacy-preserving clipboard predictions
- **Blockchain Integration:** Immutable clipboard history (optional)
- **AR/VR Support:** Clipboard in spatial computing environments

---

## Sources & References

### P2P Clipboard Sync
- [p2p-clipboard - GitHub](https://github.com/gnattu/p2p-clipboard)
- [Cross-Clipboard - Open Source P2P Clipboard Sharing](https://ntsd.dev/projects/cross-clipboard/)
- [ClipCascade - Self-hosted Clipboard Sync](https://www.xda-developers.com/open-source-self-hosted-app-syncs-clipboard-across-devices/)
- [Uniclip - Cross-platform Shared Clipboard](https://github.com/quackduck/uniclip)

### Rust Clipboard Libraries
- [Arboard - Cross-platform Clipboard Library](https://github.com/1Password/arboard)
- [cli-clipboard - Terminal Clipboard Support](https://github.com/allie-wake-up/cli-clipboard)

### Code Snippet Managers
- [massCode - Open Source Code Snippets Manager](https://masscode.io/)
- [Syntax-Clip - Syntax Highlighting for Clipboard](https://github.com/jhuckaby/syntax-clip)
- [ClipClip for Programmers](https://clipclip.com/clipclip-for-programmers/)

### Clipboard Security
- [Secure Clipboard Handling - Android Developers](https://developer.android.com/privacy-and-security/risks/secure-clipboard-handling)
- [Clipboard Data Security - MITRE ATT&CK](https://attack.mitre.org/techniques/T1414/)
- [Clipboard Hijacking Threats](https://securityboulevard.com/2023/03/clipboard-hijacking-can-turn-your-copied-text-into-a-threat/)
- [API Security Risks in 2025](https://cybelangel.com/blog/the-api-threat-report-2025/)

### Clipboard Managers
- [Best Clipboard Managers in 2025 - Zapier](https://zapier.com/blog/best-clipboard-managers/)
- [Clipboard History Pro](https://clipboardextension.com/)
- [Smart Clipboard Pro](https://chrome-stats.com/d/mpckpnadcdgigelebgicgnpkcfdmlfpf)

---

## Conclusion

This clipboard sync system would be a killer feature for Code Bridge, providing developers with:

1. **Seamless cross-device workflow** - Copy on macOS, paste on Ubuntu
2. **Zero cloud dependency** - True P2P with libp2p
3. **Developer superpowers** - Code formatting, regex transforms, AI enhancement
4. **Privacy-first** - Sensitive data detection, E2E encryption
5. **Innovative features** - Voice transcription, screenshot OCR, AI suggestions

**Recommendation:** Start with Phase 1 (core sync) and Phase 2 (security), then iterate based on user feedback. The foundation (libp2p, Rust, CRDT) is already in place from Code Bridge, making this a natural extension.

**Estimated Effort:** 12-16 weeks for full implementation with one developer.

**Risk Level:** Low - Building on proven technologies (arboard, libp2p, Automerge).
