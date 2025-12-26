//! Local build monitoring

use async_trait::async_trait;
use super::NotificationSource;
use crate::{
    BuildNotification, BuildSource, BuildStatus, Notification, NotificationError, Result,
    TestFailure, TestNotification,
};
use chrono::Utc;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

/// Local build monitor
pub struct LocalBuildMonitor {
    device_id: String,
    project_path: PathBuf,
    running: Arc<AtomicBool>,
}

impl LocalBuildMonitor {
    pub fn new(device_id: String, project_path: PathBuf) -> Self {
        Self {
            device_id,
            project_path,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run cargo build and monitor output
    pub async fn run_cargo_build(&self, tx: &mpsc::Sender<Notification>) -> Result<BuildNotification> {
        let start = Instant::now();

        let mut child = Command::new("cargo")
            .arg("build")
            .current_dir(&self.project_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;

        let stderr = child.stderr.take().unwrap();
        let mut reader = BufReader::new(stderr).lines();

        let mut errors = Vec::new();

        while let Some(line) = reader.next_line().await.unwrap_or(None) {
            if line.contains("error[") {
                errors.push(line);
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| NotificationError::Failed(e.to_string()))?;

        let duration = start.elapsed();

        let build = BuildNotification {
            id: format!("local-{}", uuid::Uuid::new_v4()),
            source: BuildSource::LocalBuild,
            repo: self.project_path.to_string_lossy().to_string(),
            branch: self.get_git_branch().unwrap_or_else(|| "unknown".to_string()),
            commit: self.get_git_commit().unwrap_or_else(|| "unknown".to_string()),
            status: if status.success() {
                BuildStatus::Success
            } else {
                BuildStatus::Failed
            },
            url: format!("file://{}", self.project_path.display()),
            timestamp: Utc::now(),
            duration: Some(duration),
            actions: Vec::new(),
        };

        let notification = build.to_notification(&self.device_id);
        tx.send(notification).await.ok();

        Ok(build)
    }

    /// Run cargo test and monitor output
    pub async fn run_cargo_test(&self, tx: &mpsc::Sender<Notification>) -> Result<TestNotification> {
        let start = Instant::now();

        let output = Command::new("cargo")
            .args(["test", "--", "--format=json", "-Z", "unstable-options"])
            .current_dir(&self.project_path)
            .output()
            .await
            .map_err(|e| NotificationError::Failed(e.to_string()))?;

        let duration = start.elapsed();

        // Parse test output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let (passed, failed, skipped, failures) = self.parse_test_output(&stdout);

        let test = TestNotification {
            id: format!("test-{}", uuid::Uuid::new_v4()),
            framework: "cargo-test".to_string(),
            total: passed + failed + skipped,
            passed,
            failed,
            skipped,
            duration,
            failures,
            project_path: self.project_path.to_string_lossy().to_string(),
        };

        let notification = test.to_notification(&self.device_id);
        tx.send(notification).await.ok();

        Ok(test)
    }

    fn parse_test_output(&self, output: &str) -> (usize, usize, usize, Vec<TestFailure>) {
        let mut passed = 0;
        let mut failed = 0;
        let mut skipped = 0;
        let mut failures = Vec::new();

        for line in output.lines() {
            if let Ok(event) = serde_json::from_str::<serde_json::Value>(line) {
                if let Some(event_type) = event.get("event").and_then(|e| e.as_str()) {
                    match event_type {
                        "ok" => passed += 1,
                        "failed" => {
                            failed += 1;
                            if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                                let message = event
                                    .get("stdout")
                                    .and_then(|s| s.as_str())
                                    .unwrap_or("Test failed")
                                    .to_string();

                                failures.push(TestFailure {
                                    test_name: name.to_string(),
                                    error_message: message,
                                    file: None,
                                    line: None,
                                });
                            }
                        }
                        "ignored" => skipped += 1,
                        _ => {}
                    }
                }
            }
        }

        // Fallback: parse simple format
        if passed + failed + skipped == 0 {
            for line in output.lines() {
                if line.contains("test result: ok") {
                    if let Some(caps) = regex::Regex::new(r"(\d+) passed")
                        .ok()
                        .and_then(|r| r.captures(line))
                    {
                        passed = caps[1].parse().unwrap_or(0);
                    }
                } else if line.contains("test result: FAILED") {
                    if let Some(caps) = regex::Regex::new(r"(\d+) failed")
                        .ok()
                        .and_then(|r| r.captures(line))
                    {
                        failed = caps[1].parse().unwrap_or(0);
                    }
                }
            }
        }

        (passed, failed, skipped, failures)
    }

    fn get_git_branch(&self) -> Option<String> {
        std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.project_path)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }

    fn get_git_commit(&self) -> Option<String> {
        std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(&self.project_path)
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
    }
}

#[async_trait]
impl NotificationSource for LocalBuildMonitor {
    fn name(&self) -> &str {
        "Local Build"
    }

    async fn start(&self, tx: mpsc::Sender<Notification>) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);

        // Watch for file changes and trigger builds
        // For now, just return - the actual file watching would be done elsewhere

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

/// Watch a project for changes and trigger builds
pub struct BuildWatcher {
    device_id: String,
    projects: Vec<PathBuf>,
}

impl BuildWatcher {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            projects: Vec::new(),
        }
    }

    pub fn add_project(&mut self, path: PathBuf) {
        self.projects.push(path);
    }

    /// Watch for file changes and trigger builds
    pub async fn watch(&self, tx: mpsc::Sender<Notification>) -> Result<()> {
        use notify::{Event, RecursiveMode, Watcher};

        let (file_tx, mut file_rx) = mpsc::channel(100);

        let mut watcher = notify::recommended_watcher(move |res: std::result::Result<Event, _>| {
            if let Ok(event) = res {
                file_tx.blocking_send(event).ok();
            }
        })
        .map_err(|e| NotificationError::Failed(e.to_string()))?;

        for project in &self.projects {
            watcher
                .watch(project, RecursiveMode::Recursive)
                .map_err(|e| NotificationError::Failed(e.to_string()))?;
        }

        let mut debounce = tokio::time::Instant::now();

        while let Some(event) = file_rx.recv().await {
            // Debounce file changes
            if debounce.elapsed() < std::time::Duration::from_secs(2) {
                continue;
            }
            debounce = tokio::time::Instant::now();

            // Check if it's a source file
            let is_source = event.paths.iter().any(|p| {
                p.extension()
                    .map(|e| ["rs", "ts", "js", "py", "go"].contains(&e.to_str().unwrap_or("")))
                    .unwrap_or(false)
            });

            if is_source {
                // Find the project this file belongs to
                for project in &self.projects {
                    if event.paths.iter().any(|p| p.starts_with(project)) {
                        let monitor = LocalBuildMonitor::new(self.device_id.clone(), project.clone());
                        monitor.run_cargo_build(&tx).await.ok();
                        break;
                    }
                }
            }
        }

        Ok(())
    }
}
