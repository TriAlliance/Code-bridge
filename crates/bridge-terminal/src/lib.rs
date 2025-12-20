//! Terminal session recording and command history sync
//!
//! This crate provides:
//! - Terminal session recording (asciinema-compatible)
//! - Command history sync across devices
//! - Shell environment sharing

pub mod recording;
pub mod history;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TerminalError {
    #[error("Recording failed: {0}")]
    Recording(String),

    #[error("History error: {0}")]
    History(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, TerminalError>;

/// Terminal session recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalRecording {
    pub id: String,
    pub title: String,
    pub shell: String,
    pub working_dir: PathBuf,
    pub width: u16,
    pub height: u16,
    pub events: Vec<TerminalEvent>,
    pub duration: Duration,
    pub recorded_at: DateTime<Utc>,
    pub source_device: String,
    pub tags: Vec<String>,
}

impl TerminalRecording {
    pub fn new(title: &str, shell: &str, width: u16, height: u16) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: title.to_string(),
            shell: shell.to_string(),
            working_dir: std::env::current_dir().unwrap_or_default(),
            width,
            height,
            events: Vec::new(),
            duration: Duration::ZERO,
            recorded_at: Utc::now(),
            source_device: String::new(),
            tags: Vec::new(),
        }
    }

    pub fn add_event(&mut self, event: TerminalEvent) {
        if let TerminalEvent::Output { time, .. } = &event {
            self.duration = *time;
        }
        self.events.push(event);
    }

    /// Convert to asciinema v2 format
    pub fn to_asciinema(&self) -> String {
        let mut lines = Vec::new();

        // Header
        let header = serde_json::json!({
            "version": 2,
            "width": self.width,
            "height": self.height,
            "timestamp": self.recorded_at.timestamp(),
            "title": self.title,
            "env": {
                "SHELL": self.shell,
                "TERM": "xterm-256color"
            }
        });
        lines.push(header.to_string());

        // Events
        for event in &self.events {
            match event {
                TerminalEvent::Output { time, data } => {
                    let event = serde_json::json!([
                        time.as_secs_f64(),
                        "o",
                        data
                    ]);
                    lines.push(event.to_string());
                }
                TerminalEvent::Input { time, data } => {
                    let event = serde_json::json!([
                        time.as_secs_f64(),
                        "i",
                        data
                    ]);
                    lines.push(event.to_string());
                }
                _ => {}
            }
        }

        lines.join("\n")
    }

    /// Save recording to file
    pub fn save(&self, path: &PathBuf) -> Result<()> {
        let content = self.to_asciinema();
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Terminal event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TerminalEvent {
    /// Terminal output
    Output {
        time: Duration,
        data: String,
    },
    /// User input
    Input {
        time: Duration,
        data: String,
    },
    /// Terminal resize
    Resize {
        time: Duration,
        width: u16,
        height: u16,
    },
    /// Marker/annotation
    Marker {
        time: Duration,
        label: String,
    },
}

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub command: String,
    pub working_dir: PathBuf,
    pub exit_code: Option<i32>,
    pub duration: Option<Duration>,
    pub timestamp: DateTime<Utc>,
    pub source_device: String,
    pub shell: String,
    pub session_id: Option<String>,
}

impl HistoryEntry {
    pub fn new(command: String, working_dir: PathBuf, shell: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            command,
            working_dir,
            exit_code: None,
            duration: None,
            timestamp: Utc::now(),
            source_device: String::new(),
            shell: shell.to_string(),
            session_id: None,
        }
    }

    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    pub fn with_device(mut self, device: String) -> Self {
        self.source_device = device;
        self
    }
}

/// Shell environment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellEnvironment {
    pub shell: String,
    pub aliases: Vec<Alias>,
    pub functions: Vec<ShellFunction>,
    pub environment: Vec<EnvVar>,
    pub path_additions: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alias {
    pub name: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellFunction {
    pub name: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub sensitive: bool,
}

impl ShellEnvironment {
    pub fn new(shell: &str) -> Self {
        Self {
            shell: shell.to_string(),
            aliases: Vec::new(),
            functions: Vec::new(),
            environment: Vec::new(),
            path_additions: Vec::new(),
        }
    }

    /// Load environment from current shell
    pub fn from_current() -> Result<Self> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let mut env = Self::new(&shell);

        // Load aliases
        env.aliases = Self::load_aliases(&shell)?;

        // Load safe environment variables
        for (key, value) in std::env::vars() {
            if !Self::is_sensitive_env(&key) {
                env.environment.push(EnvVar {
                    name: key,
                    value,
                    sensitive: false,
                });
            }
        }

        Ok(env)
    }

    fn load_aliases(shell: &str) -> Result<Vec<Alias>> {
        use std::process::Command;

        let output = if shell.contains("zsh") {
            Command::new("zsh").args(["-ic", "alias"]).output()
        } else {
            Command::new("bash").args(["-ic", "alias"]).output()
        }
        .map_err(|e| TerminalError::Recording(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut aliases = Vec::new();

        for line in stdout.lines() {
            // Parse alias format: alias name='command'
            if let Some((name, cmd)) = line.strip_prefix("alias ").and_then(|l| l.split_once('=')) {
                let command = cmd.trim_matches('\'').trim_matches('"').to_string();
                aliases.push(Alias {
                    name: name.to_string(),
                    command,
                });
            }
        }

        Ok(aliases)
    }

    fn is_sensitive_env(key: &str) -> bool {
        let sensitive_patterns = [
            "PASSWORD", "SECRET", "TOKEN", "KEY", "CREDENTIALS",
            "AWS_", "API_", "AUTH", "PRIVATE", "SSH_",
        ];

        sensitive_patterns.iter().any(|p| key.contains(p))
    }

    /// Generate shell script to apply this environment
    pub fn to_shell_script(&self) -> String {
        let mut script = String::new();

        // Aliases
        for alias in &self.aliases {
            script.push_str(&format!("alias {}='{}'\n", alias.name, alias.command));
        }

        // Functions
        for func in &self.functions {
            script.push_str(&format!("{} () {{\n{}\n}}\n", func.name, func.body));
        }

        // Environment (non-sensitive only)
        for env in &self.environment {
            if !env.sensitive {
                script.push_str(&format!("export {}=\"{}\"\n", env.name, env.value));
            }
        }

        // PATH additions
        if !self.path_additions.is_empty() {
            let paths: Vec<_> = self.path_additions.iter()
                .map(|p| p.to_string_lossy())
                .collect();
            script.push_str(&format!("export PATH=\"{}:$PATH\"\n", paths.join(":")));
        }

        script
    }
}

/// Terminal manager
pub struct TerminalManager {
    device_id: String,
    history: history::CommandHistory,
    recordings_path: PathBuf,
}

impl TerminalManager {
    pub fn new(device_id: String, storage_path: PathBuf) -> Result<Self> {
        let recordings_path = storage_path.join("recordings");
        std::fs::create_dir_all(&recordings_path)?;

        let history = history::CommandHistory::new(storage_path)?;

        Ok(Self {
            device_id,
            history,
            recordings_path,
        })
    }

    /// Start a new recording
    pub fn start_recording(&self, title: &str) -> Result<recording::Recorder> {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        recording::Recorder::new(title, &shell, &self.recordings_path)
    }

    /// Get command history
    pub fn get_history(&self, limit: usize) -> Result<Vec<HistoryEntry>> {
        self.history.get_recent(limit)
    }

    /// Search command history
    pub fn search_history(&self, query: &str) -> Result<Vec<HistoryEntry>> {
        self.history.search(query)
    }

    /// Add command to history
    pub fn add_to_history(&mut self, entry: HistoryEntry) -> Result<()> {
        self.history.add(entry)
    }

    /// Get shell environment
    pub fn get_environment(&self) -> Result<ShellEnvironment> {
        ShellEnvironment::from_current()
    }

    /// List recordings
    pub fn list_recordings(&self) -> Result<Vec<TerminalRecording>> {
        let mut recordings = Vec::new();

        for entry in std::fs::read_dir(&self.recordings_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "cast").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(recording) = self.parse_asciinema(&content) {
                        recordings.push(recording);
                    }
                }
            }
        }

        recordings.sort_by(|a, b| b.recorded_at.cmp(&a.recorded_at));
        Ok(recordings)
    }

    fn parse_asciinema(&self, content: &str) -> Result<TerminalRecording> {
        let mut lines = content.lines();

        // Parse header
        let header: serde_json::Value = lines
            .next()
            .and_then(|l| serde_json::from_str(l).ok())
            .ok_or_else(|| TerminalError::Recording("Invalid header".to_string()))?;

        let mut recording = TerminalRecording::new(
            header["title"].as_str().unwrap_or("Untitled"),
            header["env"]["SHELL"].as_str().unwrap_or("/bin/bash"),
            header["width"].as_u64().unwrap_or(80) as u16,
            header["height"].as_u64().unwrap_or(24) as u16,
        );

        // Parse events
        for line in lines {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                if let (Some(time), Some(event_type), Some(data)) = (
                    event.get(0).and_then(|t| t.as_f64()),
                    event.get(1).and_then(|t| t.as_str()),
                    event.get(2).and_then(|d| d.as_str()),
                ) {
                    let duration = Duration::from_secs_f64(time);
                    match event_type {
                        "o" => recording.add_event(TerminalEvent::Output {
                            time: duration,
                            data: data.to_string(),
                        }),
                        "i" => recording.add_event(TerminalEvent::Input {
                            time: duration,
                            data: data.to_string(),
                        }),
                        _ => {}
                    }
                }
            }
        }

        Ok(recording)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recording_creation() {
        let mut recording = TerminalRecording::new("Test", "/bin/bash", 80, 24);
        recording.add_event(TerminalEvent::Output {
            time: Duration::from_millis(100),
            data: "Hello".to_string(),
        });

        assert_eq!(recording.events.len(), 1);
    }

    #[test]
    fn test_history_entry() {
        let entry = HistoryEntry::new(
            "ls -la".to_string(),
            PathBuf::from("/home/user"),
            "bash",
        )
        .with_exit_code(0);

        assert_eq!(entry.exit_code, Some(0));
    }

    #[test]
    fn test_asciinema_format() {
        let mut recording = TerminalRecording::new("Test", "/bin/bash", 80, 24);
        recording.add_event(TerminalEvent::Output {
            time: Duration::from_millis(100),
            data: "test".to_string(),
        });

        let asciinema = recording.to_asciinema();
        assert!(asciinema.contains("\"version\": 2"));
    }
}
