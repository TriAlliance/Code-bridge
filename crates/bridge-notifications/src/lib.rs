//! Cross-platform notification bridge for developers
//!
//! This crate provides developer-focused notifications with:
//! - System notification sync across devices
//! - CI/CD build status (GitHub Actions, etc.)
//! - Test failure alerts
//! - Smart filtering and prioritization

pub mod capture;
pub mod display;
pub mod integrations;
pub mod filters;
pub mod actions;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Notification failed: {0}")]
    Failed(String),

    #[error("Integration error: {0}")]
    Integration(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NotificationError>;

/// Notification priority levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Notification category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Category {
    System,
    Build,
    Test,
    Deploy,
    PullRequest,
    Issue,
    Alert,
    Security,
    Chat,
    Email,
    Custom(String),
}

/// A notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub body: String,
    pub timestamp: DateTime<Utc>,
    pub icon: Option<Vec<u8>>,
    pub priority: Priority,
    pub category: Category,
    pub source_device: String,
    pub actions: Vec<NotificationAction>,
    pub read: bool,
    pub dismissed: bool,
    pub metadata: HashMap<String, String>,
}

impl Notification {
    pub fn new(app_name: &str, title: &str, body: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            app_name: app_name.to_string(),
            title: title.to_string(),
            subtitle: None,
            body: body.to_string(),
            timestamp: Utc::now(),
            icon: None,
            priority: Priority::Normal,
            category: Category::System,
            source_device: String::new(),
            actions: Vec::new(),
            read: false,
            dismissed: false,
            metadata: HashMap::new(),
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_category(mut self, category: Category) -> Self {
        self.category = category;
        self
    }

    pub fn with_subtitle(mut self, subtitle: &str) -> Self {
        self.subtitle = Some(subtitle.to_string());
        self
    }

    pub fn with_action(mut self, action: NotificationAction) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_device(mut self, device: String) -> Self {
        self.source_device = device;
        self
    }
}

/// Notification action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub action_type: ActionType,
}

impl NotificationAction {
    pub fn new(id: &str, label: &str, action_type: ActionType) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            action_type,
        }
    }

    pub fn open_url(label: &str, url: &str) -> Self {
        Self::new("open_url", label, ActionType::OpenUrl(url.to_string()))
    }

    pub fn run_command(label: &str, command: &str) -> Self {
        Self::new(
            "run_command",
            label,
            ActionType::RunCommand {
                command: command.to_string(),
                cwd: None,
            },
        )
    }
}

/// Types of actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    OpenUrl(String),
    OpenFile(String),
    RunCommand {
        command: String,
        cwd: Option<String>,
    },
    ApiCall {
        method: String,
        url: String,
        body: Option<String>,
        headers: HashMap<String, String>,
    },
    Dismiss,
    Custom(String),
}

/// Build notification (CI/CD)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildNotification {
    pub id: String,
    pub source: BuildSource,
    pub repo: String,
    pub branch: String,
    pub commit: String,
    pub status: BuildStatus,
    pub url: String,
    pub timestamp: DateTime<Utc>,
    pub duration: Option<std::time::Duration>,
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildSource {
    GitHubActions,
    GitLabCI,
    CircleCI,
    Jenkins,
    LocalBuild,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

impl BuildNotification {
    pub fn to_notification(&self, device_id: &str) -> Notification {
        let (priority, icon_emoji) = match self.status {
            BuildStatus::Failed => (Priority::High, ""),
            BuildStatus::Success => (Priority::Normal, ""),
            BuildStatus::Running => (Priority::Low, ""),
            BuildStatus::Cancelled => (Priority::Low, ""),
            BuildStatus::Pending => (Priority::Low, ""),
        };

        let title = format!(
            "{} Build {} - {}",
            icon_emoji,
            match self.status {
                BuildStatus::Success => "Passed",
                BuildStatus::Failed => "Failed",
                BuildStatus::Running => "Running",
                BuildStatus::Cancelled => "Cancelled",
                BuildStatus::Pending => "Pending",
            },
            self.repo
        );

        let body = format!(
            "Branch: {}\nCommit: {}",
            self.branch,
            &self.commit[..7.min(self.commit.len())]
        );

        Notification::new("CI/CD", &title, &body)
            .with_priority(priority)
            .with_category(Category::Build)
            .with_device(device_id.to_string())
            .with_action(NotificationAction::open_url("View Build", &self.url))
            .with_metadata("repo", &self.repo)
            .with_metadata("branch", &self.branch)
            .with_metadata("commit", &self.commit)
    }
}

/// Test result notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestNotification {
    pub id: String,
    pub framework: String,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: std::time::Duration,
    pub failures: Vec<TestFailure>,
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

impl TestNotification {
    pub fn to_notification(&self, device_id: &str) -> Notification {
        let (priority, title) = if self.failed > 0 {
            (
                Priority::High,
                format!(" {} test(s) failed", self.failed),
            )
        } else {
            (
                Priority::Normal,
                format!(" All {} tests passed", self.total),
            )
        };

        let body = format!(
            "Passed: {} | Failed: {} | Skipped: {}\nDuration: {:.2}s",
            self.passed,
            self.failed,
            self.skipped,
            self.duration.as_secs_f64()
        );

        let mut notification = Notification::new("Tests", &title, &body)
            .with_priority(priority)
            .with_category(Category::Test)
            .with_device(device_id.to_string());

        if self.failed > 0 {
            notification = notification.with_action(NotificationAction::new(
                "rerun_failed",
                "Re-run Failed",
                ActionType::Custom("rerun_failed_tests".to_string()),
            ));
        }

        notification
    }
}

/// Pull request notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestNotification {
    pub id: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub author: String,
    pub url: String,
    pub event: PREvent,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PREvent {
    Opened,
    ReviewRequested,
    Approved,
    ChangesRequested,
    Merged,
    Closed,
    Commented,
}

impl PullRequestNotification {
    pub fn to_notification(&self, device_id: &str) -> Notification {
        let (priority, event_text) = match &self.event {
            PREvent::ReviewRequested => (Priority::High, "Review requested"),
            PREvent::ChangesRequested => (Priority::High, "Changes requested"),
            PREvent::Approved => (Priority::Normal, "Approved"),
            PREvent::Merged => (Priority::Normal, "Merged"),
            PREvent::Opened => (Priority::Normal, "Opened"),
            PREvent::Closed => (Priority::Low, "Closed"),
            PREvent::Commented => (Priority::Low, "New comment"),
        };

        let title = format!("PR #{}: {}", self.number, self.title);
        let body = format!("{} by @{}", event_text, self.author);

        let mut notification = Notification::new("GitHub", &title, &body)
            .with_priority(priority)
            .with_category(Category::PullRequest)
            .with_device(device_id.to_string())
            .with_action(NotificationAction::open_url("View PR", &self.url));

        if matches!(self.event, PREvent::ReviewRequested) {
            notification = notification
                .with_action(NotificationAction::new(
                    "approve",
                    "Approve",
                    ActionType::Custom("approve_pr".to_string()),
                ))
                .with_action(NotificationAction::new(
                    "request_changes",
                    "Request Changes",
                    ActionType::Custom("request_changes_pr".to_string()),
                ));
        }

        notification
    }
}

/// Notification manager
pub struct NotificationManager {
    device_id: String,
    filters: filters::FilterEngine,
    history: Vec<Notification>,
    max_history: usize,
}

impl NotificationManager {
    pub fn new(device_id: String) -> Self {
        Self {
            device_id,
            filters: filters::FilterEngine::new(),
            history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Send a notification
    pub async fn send(&mut self, notification: Notification) -> Result<()> {
        // Apply filters
        if !self.filters.should_display(&notification) {
            tracing::debug!("Notification filtered: {}", notification.title);
            return Ok(());
        }

        // Display notification
        display::show_notification(&notification).await?;

        // Add to history
        self.history.push(notification);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        Ok(())
    }

    /// Get notification history
    pub fn get_history(&self) -> &[Notification] {
        &self.history
    }

    /// Get unread notifications
    pub fn get_unread(&self) -> Vec<&Notification> {
        self.history.iter().filter(|n| !n.read).collect()
    }

    /// Mark notification as read
    pub fn mark_read(&mut self, notification_id: &str) {
        if let Some(n) = self.history.iter_mut().find(|n| n.id == notification_id) {
            n.read = true;
        }
    }

    /// Mark all as read
    pub fn mark_all_read(&mut self) {
        for n in &mut self.history {
            n.read = true;
        }
    }

    /// Dismiss notification
    pub fn dismiss(&mut self, notification_id: &str) {
        if let Some(n) = self.history.iter_mut().find(|n| n.id == notification_id) {
            n.dismissed = true;
        }
    }

    /// Execute notification action
    pub async fn execute_action(&self, action: &NotificationAction) -> Result<()> {
        actions::execute_action(&action.action_type).await
    }

    /// Add filter rule
    pub fn add_filter(&mut self, rule: filters::FilterRule) {
        self.filters.add_rule(rule);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_creation() {
        let notification = Notification::new("Test App", "Test Title", "Test Body")
            .with_priority(Priority::High)
            .with_category(Category::Build);

        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.priority, Priority::High);
    }

    #[test]
    fn test_build_notification() {
        let build = BuildNotification {
            id: "123".to_string(),
            source: BuildSource::GitHubActions,
            repo: "user/repo".to_string(),
            branch: "main".to_string(),
            commit: "abc1234567890".to_string(),
            status: BuildStatus::Failed,
            url: "https://github.com".to_string(),
            timestamp: Utc::now(),
            duration: Some(std::time::Duration::from_secs(120)),
            actions: Vec::new(),
        };

        let notification = build.to_notification("device1");
        assert_eq!(notification.priority, Priority::High);
        assert!(notification.title.contains("Failed"));
    }
}
