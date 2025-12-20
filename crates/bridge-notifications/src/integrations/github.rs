//! GitHub integration for notifications

use super::NotificationSource;
use crate::{
    BuildNotification, BuildSource, BuildStatus, Notification, NotificationAction,
    NotificationError, PREvent, PullRequestNotification, Result,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// GitHub notification source
pub struct GitHubSource {
    token: String,
    device_id: String,
    poll_interval: Duration,
    running: Arc<AtomicBool>,
    repos: Vec<String>,
    last_check: std::sync::Mutex<HashMap<String, DateTime<Utc>>>,
}

impl GitHubSource {
    pub fn new(token: String, device_id: String) -> Self {
        Self {
            token,
            device_id,
            poll_interval: Duration::from_secs(60),
            running: Arc::new(AtomicBool::new(false)),
            repos: Vec::new(),
            last_check: std::sync::Mutex::new(HashMap::new()),
        }
    }

    pub fn with_repos(mut self, repos: Vec<String>) -> Self {
        self.repos = repos;
        self
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    async fn check_notifications(&self, tx: &mpsc::Sender<Notification>) -> Result<()> {
        let client = reqwest::Client::new();

        // Check GitHub notifications
        let response = client
            .get("https://api.github.com/notifications")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", "code-bridge")
            .header("Accept", "application/vnd.github+json")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(NotificationError::Integration(format!(
                "GitHub API error: {}",
                response.status()
            )));
        }

        let notifications: Vec<GitHubNotification> = response.json().await?;

        for gh_notif in notifications {
            let notification = self.convert_notification(&gh_notif);
            tx.send(notification).await.ok();
        }

        Ok(())
    }

    async fn check_workflow_runs(&self, tx: &mpsc::Sender<Notification>) -> Result<()> {
        let client = reqwest::Client::new();

        for repo in &self.repos {
            let url = format!(
                "https://api.github.com/repos/{}/actions/runs?per_page=5",
                repo
            );

            let response = client
                .get(&url)
                .header("Authorization", format!("Bearer {}", self.token))
                .header("User-Agent", "code-bridge")
                .header("Accept", "application/vnd.github+json")
                .send()
                .await?;

            if !response.status().is_success() {
                continue;
            }

            let runs: WorkflowRunsResponse = response.json().await?;
            let mut last_check = self.last_check.lock().unwrap();
            let last_time = last_check
                .get(repo)
                .cloned()
                .unwrap_or_else(|| Utc::now() - chrono::Duration::hours(1));

            for run in runs.workflow_runs {
                if run.updated_at > last_time {
                    let build = BuildNotification {
                        id: format!("gh-{}-{}", repo, run.id),
                        source: BuildSource::GitHubActions,
                        repo: repo.clone(),
                        branch: run.head_branch,
                        commit: run.head_sha,
                        status: match run.conclusion.as_deref() {
                            Some("success") => BuildStatus::Success,
                            Some("failure") => BuildStatus::Failed,
                            Some("cancelled") => BuildStatus::Cancelled,
                            None if run.status == "in_progress" => BuildStatus::Running,
                            _ => BuildStatus::Pending,
                        },
                        url: run.html_url,
                        timestamp: run.updated_at,
                        duration: None,
                        actions: Vec::new(),
                    };

                    let notification = build.to_notification(&self.device_id);
                    tx.send(notification).await.ok();
                }
            }

            last_check.insert(repo.clone(), Utc::now());
        }

        Ok(())
    }

    fn convert_notification(&self, gh: &GitHubNotification) -> Notification {
        let title = format!("{}: {}", gh.repository.full_name, gh.subject.title);
        let body = format!("Type: {}", gh.subject.type_field);

        let mut notification = Notification::new("GitHub", &title, &body)
            .with_device(self.device_id.clone())
            .with_metadata("repo", &gh.repository.full_name)
            .with_metadata("type", &gh.subject.type_field);

        if let Some(url) = &gh.subject.url {
            // Convert API URL to web URL
            let web_url = url
                .replace("api.github.com/repos", "github.com")
                .replace("/pulls/", "/pull/");

            notification = notification.with_action(NotificationAction::open_url("View", &web_url));
        }

        notification
    }
}

#[async_trait::async_trait]
impl NotificationSource for GitHubSource {
    fn name(&self) -> &str {
        "GitHub"
    }

    async fn start(&self, tx: mpsc::Sender<Notification>) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);

        while self.running.load(Ordering::SeqCst) {
            // Check notifications
            if let Err(e) = self.check_notifications(&tx).await {
                tracing::warn!("GitHub notification check failed: {}", e);
            }

            // Check workflow runs
            if let Err(e) = self.check_workflow_runs(&tx).await {
                tracing::warn!("GitHub workflow check failed: {}", e);
            }

            tokio::time::sleep(self.poll_interval).await;
        }

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}

// GitHub API types
#[derive(Debug, Deserialize)]
struct GitHubNotification {
    id: String,
    repository: Repository,
    subject: Subject,
    reason: String,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct Repository {
    full_name: String,
    html_url: String,
}

#[derive(Debug, Deserialize)]
struct Subject {
    title: String,
    url: Option<String>,
    #[serde(rename = "type")]
    type_field: String,
}

#[derive(Debug, Deserialize)]
struct WorkflowRunsResponse {
    workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Deserialize)]
struct WorkflowRun {
    id: u64,
    name: String,
    head_branch: String,
    head_sha: String,
    status: String,
    conclusion: Option<String>,
    html_url: String,
    updated_at: DateTime<Utc>,
}
