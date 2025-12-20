# Cross-Platform Notification Bridge Research for Developers
**Research Date:** December 20, 2025
**Project:** Code Bridge - P2P Developer Platform
**Focus:** Innovative notification sync and bridging features for multi-device developer workflows

---

## Executive Summary

This document provides comprehensive research on building a cross-platform notification bridge designed specifically for developers. The system enables seamless notification sync, developer-focused alerts, build/CI notifications, and smart filtering across all devices in a P2P mesh network.

### Key Findings

1. **System notification APIs** are mature on all major platforms (macOS, Linux, Windows)
2. **P2P notification relay** is feasible using existing libp2p infrastructure
3. **Developer-specific notifications** (builds, deploys, monitoring) are highly valuable but poorly integrated
4. **Smart filtering with AI** can reduce notification fatigue by 70-80%
5. **Action buttons** significantly improve developer productivity (approve PR without context switching)
6. **Mobile companion apps** are essential for on-call developers

### Unique Value Proposition

Unlike consumer notification sync tools (KDE Connect, Pushbullet), a developer-focused notification bridge offers:
- **Build/CI integration** - GitHub Actions, local builds, test failures
- **Code review alerts** - PR reviews, code comments, mentions
- **Infrastructure monitoring** - Server alerts, log errors, performance warnings
- **Smart filtering** - AI-powered priority sorting, focus mode integration
- **P2P architecture** - No cloud dependency, works on corporate networks
- **Action buttons** - Restart build, approve PR, acknowledge alert from any device

---

## 1. Notification Sync Architecture

### 1.1 System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│              NOTIFICATION BRIDGE ARCHITECTURE                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌──────────────┐    P2P (libp2p)    ┌──────────────┐          │
│  │   macOS      │◄──────────────────►│   Linux      │          │
│  │   Device     │    QUIC/Pubsub     │   Device     │          │
│  │              │                    │              │          │
│  │ • Capture    │                    │ • Capture    │          │
│  │ • Display    │                    │ • Display    │          │
│  │ • Act        │                    │ • Act        │          │
│  └──────┬───────┘                    └──────┬───────┘          │
│         │                                   │                  │
│         └───────────────┬───────────────────┘                  │
│                         │                                       │
│                ┌────────▼────────┐                             │
│                │  Mobile App     │                             │
│                │  (iOS/Android)  │                             │
│                │                 │                             │
│                │ • Push Notifs   │                             │
│                │ • Quick Actions │                             │
│                └─────────────────┘                             │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │           Notification Sources                          │  │
│  │  • System Apps (Mail, Messages, Calendar)              │  │
│  │  • Developer Tools (GitHub, CI/CD, Monitoring)         │  │
│  │  • Communication (Slack, Discord, Email)               │  │
│  │  • Build Systems (Local, Remote)                       │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Notification Capture Layer

#### macOS Implementation

```rust
// Using macOS NSUserNotificationCenter (legacy) and UNUserNotificationCenter (modern)

use cocoa::base::{id, nil};
use cocoa::foundation::{NSString, NSArray};
use objc::{class, msg_send, sel, sel_impl};

pub struct MacOSNotificationCapture {
    // Monitor all app notifications
    notification_center: id,
}

impl MacOSNotificationCapture {
    pub fn new() -> Self {
        unsafe {
            let center: id = msg_send![
                class!(NSUserNotificationCenter),
                defaultUserNotificationCenter
            ];

            Self {
                notification_center: center,
            }
        }
    }

    // Capture delivered notifications
    pub fn get_delivered_notifications(&self) -> Vec<Notification> {
        unsafe {
            let delivered: id = msg_send![
                self.notification_center,
                deliveredNotifications
            ];

            self.parse_ns_notifications(delivered)
        }
    }

    // Listen for new notifications
    pub fn observe_notifications<F>(&self, callback: F)
    where
        F: Fn(Notification) + 'static,
    {
        // Register as observer for NSUserNotificationCenter
        // Call callback when new notification arrives
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub app_name: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub body: String,
    pub timestamp: DateTime<Utc>,
    pub icon: Option<Vec<u8>>,
    pub actions: Vec<NotificationAction>,
    pub priority: Priority,
    pub category: Category,
}
```

**Modern macOS (10.14+):**
- Use `UNUserNotificationCenter` for delivered notifications
- Private APIs for system-wide notification monitoring (requires entitlements)
- Alternative: Monitor SQLite database at `~/Library/Application Support/NotificationCenter/`

#### Linux Implementation

```rust
// Using D-Bus to monitor org.freedesktop.Notifications

use zbus::{Connection, dbus_proxy};

#[dbus_proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;

    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, &zbus::zvariant::Value>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;

    #[dbus_proxy(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;

    #[dbus_proxy(signal)]
    fn action_invoked(&self, id: u32, action_key: &str) -> zbus::Result<()>;
}

pub struct LinuxNotificationCapture {
    connection: Connection,
}

impl LinuxNotificationCapture {
    pub async fn new() -> Result<Self> {
        let connection = Connection::session().await?;
        Ok(Self { connection })
    }

    // Monitor notification signals
    pub async fn observe_notifications<F>(&self, callback: F)
    where
        F: Fn(Notification) + Send + 'static,
    {
        let proxy = NotificationsProxy::new(&self.connection).await?;

        // Subscribe to notification signals
        let mut stream = proxy.receive_notification_closed().await?;

        while let Some(signal) = stream.next().await {
            let args = signal.args()?;
            // Parse and forward to callback
        }
    }
}
```

**Linux Options:**
1. **D-Bus Monitoring** (Recommended) - Works with all desktop environments
2. **Dunst/Mako hooks** - Lightweight, notification daemon-specific
3. **Desktop-specific APIs** - GNOME Shell, KDE Plasma have their own APIs

#### Windows Implementation

```rust
// Using Windows.UI.Notifications

use windows::UI::Notifications::{
    ToastNotificationManager,
    ToastNotification,
    ToastNotificationHistory,
};

pub struct WindowsNotificationCapture {
    history: ToastNotificationHistory,
}

impl WindowsNotificationCapture {
    pub fn new() -> Result<Self> {
        let history = ToastNotificationManager::History()?;
        Ok(Self { history })
    }

    // Get notification history
    pub fn get_notifications(&self) -> Vec<Notification> {
        let history = self.history.GetHistory()?;

        history.into_iter()
            .map(|toast| self.parse_toast_notification(toast))
            .collect()
    }

    // Listen for new notifications
    pub fn observe_notifications<F>(&self, callback: F)
    where
        F: Fn(Notification) + 'static,
    {
        // Use Windows Runtime events to monitor new notifications
    }
}
```

### 1.3 P2P Notification Relay

```rust
use libp2p::{gossipsub, PeerId, Swarm};

pub struct NotificationRelay {
    // P2P networking (reuse Code Bridge infrastructure)
    swarm: Swarm<NotificationBehavior>,

    // Local notification database
    db: NotificationDatabase,

    // Filters and rules
    filters: FilterEngine,
}

impl NotificationRelay {
    // Broadcast notification to all connected peers
    pub async fn broadcast_notification(&mut self, notif: Notification) -> Result<()> {
        // Apply filters first
        if !self.filters.should_sync(&notif) {
            return Ok(());
        }

        // Serialize notification
        let message = NotificationMessage {
            id: notif.id.clone(),
            source_device: self.local_peer_id(),
            notification: notif,
            timestamp: Utc::now(),
        };

        let serialized = bincode::serialize(&message)?;

        // Broadcast via gossipsub
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(Topic::new("notifications"), serialized)?;

        Ok(())
    }

    // Receive notification from peer
    pub async fn handle_incoming_notification(&mut self, message: NotificationMessage) {
        // Check if already seen (deduplication)
        if self.db.has_notification(&message.id).await {
            return;
        }

        // Store in local database
        self.db.store_notification(&message.notification).await?;

        // Apply display rules
        if self.filters.should_display(&message.notification) {
            self.display_notification(&message.notification).await?;
        }
    }
}
```

### 1.4 Notification Storage

```rust
use rusqlite::{Connection, params};

pub struct NotificationDatabase {
    conn: Connection,
}

impl NotificationDatabase {
    pub async fn store_notification(&self, notif: &Notification) -> Result<()> {
        self.conn.execute(
            "INSERT INTO notifications (
                id, app_name, title, subtitle, body,
                timestamp, priority, category, source_device,
                read, dismissed
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                notif.id,
                notif.app_name,
                notif.title,
                notif.subtitle,
                notif.body,
                notif.timestamp,
                notif.priority as i32,
                notif.category as i32,
                notif.source_device,
                false,
                false,
            ],
        )?;

        Ok(())
    }

    // Query notification history
    pub async fn search_notifications(
        &self,
        query: &str,
        filters: NotificationFilters,
    ) -> Result<Vec<Notification>> {
        // Full-text search implementation
        let mut stmt = self.conn.prepare(
            "SELECT * FROM notifications
             WHERE (title LIKE ?1 OR body LIKE ?1)
             AND timestamp >= ?2
             ORDER BY timestamp DESC
             LIMIT ?3"
        )?;

        let rows = stmt.query_map(
            params![
                format!("%{}%", query),
                filters.start_time,
                filters.limit,
            ],
            |row| self.row_to_notification(row)
        )?;

        Ok(rows.collect::<Result<Vec<_>>>()?)
    }

    // Notification sync history
    pub async fn get_sync_log(&self, since: DateTime<Utc>) -> Vec<NotificationEvent> {
        // Return all notification events since timestamp
        // Used for syncing with newly connected peers
    }
}
```

---

## 2. Build and CI Notifications

### 2.1 GitHub Actions Integration

```rust
use octocrab::{Octocrab, models::{workflows, Repository}};

pub struct GitHubActionsMonitor {
    client: Octocrab,
    repositories: Vec<Repository>,
    last_check: HashMap<String, DateTime<Utc>>,
}

impl GitHubActionsMonitor {
    pub async fn new(token: String, repos: Vec<String>) -> Result<Self> {
        let client = Octocrab::builder()
            .personal_token(token)
            .build()?;

        Ok(Self {
            client,
            repositories: Vec::new(),
            last_check: HashMap::new(),
        })
    }

    // Poll for workflow run updates
    pub async fn check_workflow_runs(&mut self) -> Result<Vec<BuildNotification>> {
        let mut notifications = Vec::new();

        for repo in &self.repositories {
            let runs = self.client
                .workflows(&repo.owner.login, &repo.name)
                .list_all_runs()
                .send()
                .await?;

            for run in runs {
                // Check if this is a new event since last check
                if let Some(last) = self.last_check.get(&repo.full_name) {
                    if run.updated_at <= *last {
                        continue;
                    }
                }

                // Create notification based on status
                let notification = BuildNotification {
                    id: format!("gh-{}-{}", repo.full_name, run.id),
                    source: BuildSource::GitHubActions,
                    repo: repo.full_name.clone(),
                    branch: run.head_branch.clone(),
                    commit: run.head_sha.clone(),
                    status: match run.conclusion.as_deref() {
                        Some("success") => BuildStatus::Success,
                        Some("failure") => BuildStatus::Failed,
                        Some("cancelled") => BuildStatus::Cancelled,
                        _ => BuildStatus::Running,
                    },
                    url: run.html_url.clone(),
                    timestamp: run.updated_at,
                    actions: vec![
                        NotificationAction {
                            id: "view".to_string(),
                            label: "View Run".to_string(),
                            action: ActionType::OpenUrl(run.html_url.clone()),
                        },
                        NotificationAction {
                            id: "rerun".to_string(),
                            label: "Re-run".to_string(),
                            action: ActionType::ApiCall {
                                method: "POST".to_string(),
                                url: format!(
                                    "https://api.github.com/repos/{}/actions/runs/{}/rerun",
                                    repo.full_name, run.id
                                ),
                            },
                        },
                    ],
                };

                notifications.push(notification);
            }

            self.last_check.insert(repo.full_name.clone(), Utc::now());
        }

        Ok(notifications)
    }

    // GitHub Webhooks alternative (real-time)
    pub async fn setup_webhook_listener(&self) -> Result<()> {
        // Listen for GitHub webhook events
        // Requires public endpoint or ngrok-style tunnel
        // More real-time than polling
    }
}

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
    pub actions: Vec<NotificationAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildSource {
    GitHubActions,
    GitLabCI,
    CircleCI,
    Jenkins,
    LocalBuild,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BuildStatus {
    Running,
    Success,
    Failed,
    Cancelled,
    Pending,
}
```

### 2.2 Local Build Monitoring

```rust
use notify::{Watcher, RecursiveMode, Event};

pub struct LocalBuildMonitor {
    // Watch for build output files
    watcher: RecommendedWatcher,

    // Track active builds
    active_builds: HashMap<PathBuf, BuildProcess>,
}

impl LocalBuildMonitor {
    // Monitor cargo build
    pub async fn monitor_cargo_build(&mut self, project_path: PathBuf) -> Result<()> {
        let target_dir = project_path.join("target");

        self.watcher.watch(&target_dir, RecursiveMode::Recursive)?;

        // Also monitor build process
        let build_process = Command::new("cargo")
            .arg("build")
            .current_dir(&project_path)
            .spawn()?;

        let build_info = BuildProcess {
            pid: build_process.id(),
            start_time: Utc::now(),
            project: project_path.clone(),
        };

        self.active_builds.insert(project_path, build_info);

        Ok(())
    }

    // Monitor npm/yarn build
    pub async fn monitor_npm_build(&mut self, project_path: PathBuf) -> Result<()> {
        // Similar to cargo, monitor package.json scripts
        // Watch for dist/ or build/ directory changes
    }

    // Process exit handler
    pub async fn on_build_complete(&mut self, project: PathBuf, exit_code: i32) {
        let notification = BuildNotification {
            id: format!("local-{}", Uuid::new_v4()),
            source: BuildSource::LocalBuild,
            repo: project.to_string_lossy().to_string(),
            branch: self.get_current_branch(&project).unwrap_or_default(),
            commit: self.get_current_commit(&project).unwrap_or_default(),
            status: if exit_code == 0 {
                BuildStatus::Success
            } else {
                BuildStatus::Failed
            },
            url: format!("file://{}", project.display()),
            timestamp: Utc::now(),
            actions: vec![
                NotificationAction {
                    id: "open_terminal".to_string(),
                    label: "Open Terminal".to_string(),
                    action: ActionType::OpenTerminal(project.clone()),
                },
                NotificationAction {
                    id: "rebuild".to_string(),
                    label: "Rebuild".to_string(),
                    action: ActionType::RunCommand {
                        command: "cargo build".to_string(),
                        directory: project.clone(),
                    },
                },
            ],
        };

        self.send_notification(notification).await;
    }
}
```

### 2.3 Test Failure Alerts

```rust
pub struct TestMonitor {
    // Parser for test output
    parsers: HashMap<TestFramework, Box<dyn TestOutputParser>>,
}

#[derive(Debug, Clone)]
pub enum TestFramework {
    CargoTest,
    Jest,
    Pytest,
    JUnit,
    XCTest,
}

pub trait TestOutputParser: Send + Sync {
    fn parse_output(&self, output: &str) -> TestResult;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub framework: TestFramework,
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<TestFailure>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub error_message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
    pub stack_trace: Option<String>,
}

impl TestMonitor {
    pub async fn on_test_complete(&self, result: TestResult) {
        if result.failed == 0 {
            // All tests passed - low priority notification
            return self.send_success_notification(result).await;
        }

        // Test failures - high priority with action buttons
        let notification = Notification {
            id: format!("test-{}", Uuid::new_v4()),
            app_name: "Test Suite".to_string(),
            title: format!("{} test(s) failed", result.failed),
            subtitle: Some(format!(
                "{} passed, {} failed",
                result.passed, result.failed
            )),
            body: self.format_failure_summary(&result),
            timestamp: Utc::now(),
            icon: None,
            priority: Priority::High,
            category: Category::Test,
            actions: vec![
                NotificationAction {
                    id: "view_failures".to_string(),
                    label: "View Failures".to_string(),
                    action: ActionType::Custom("show_test_failures".to_string()),
                },
                NotificationAction {
                    id: "rerun_failed".to_string(),
                    label: "Re-run Failed Tests".to_string(),
                    action: ActionType::RunCommand {
                        command: self.generate_rerun_command(&result),
                        directory: std::env::current_dir().unwrap(),
                    },
                },
            ],
        };

        self.send_notification(notification).await;
    }

    fn format_failure_summary(&self, result: &TestResult) -> String {
        let mut summary = String::new();

        for (i, failure) in result.failures.iter().take(3).enumerate() {
            summary.push_str(&format!("{}. {}\n", i + 1, failure.test_name));
            summary.push_str(&format!("   {}\n", failure.error_message));
        }

        if result.failures.len() > 3 {
            summary.push_str(&format!("\n...and {} more", result.failures.len() - 3));
        }

        summary
    }
}
```

### 2.4 Deployment Notifications

```rust
pub struct DeploymentMonitor {
    // Monitor deployment status
    providers: Vec<Box<dyn DeploymentProvider>>,
}

pub trait DeploymentProvider: Send + Sync {
    async fn get_deployments(&self) -> Result<Vec<Deployment>>;
    async fn get_deployment_status(&self, id: &str) -> Result<DeploymentStatus>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub environment: String, // production, staging, development
    pub service: String,
    pub version: String,
    pub status: DeploymentStatus,
    pub url: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Pending,
    InProgress,
    Success,
    Failed,
    RolledBack,
}

// Vercel deployment provider
pub struct VercelProvider {
    api_token: String,
    team_id: Option<String>,
}

impl DeploymentProvider for VercelProvider {
    async fn get_deployments(&self) -> Result<Vec<Deployment>> {
        let client = reqwest::Client::new();

        let response = client
            .get("https://api.vercel.com/v6/deployments")
            .bearer_auth(&self.api_token)
            .send()
            .await?;

        let deployments: VercelDeploymentsResponse = response.json().await?;

        Ok(deployments.deployments.into_iter()
            .map(|d| Deployment {
                id: d.uid,
                environment: d.target.unwrap_or_else(|| "production".to_string()),
                service: d.name,
                version: d.meta.git_commit_sha.unwrap_or_default(),
                status: match d.ready_state.as_str() {
                    "READY" => DeploymentStatus::Success,
                    "ERROR" => DeploymentStatus::Failed,
                    "BUILDING" => DeploymentStatus::InProgress,
                    _ => DeploymentStatus::Pending,
                },
                url: Some(format!("https://{}", d.url)),
                started_at: d.created_at,
                completed_at: d.ready_at,
            })
            .collect())
    }
}

// Similar implementations for:
// - Heroku
// - AWS (CloudFormation, ECS, Lambda)
// - Google Cloud (Cloud Run, App Engine)
// - Azure (App Service, Functions)
// - Netlify
// - Railway
```

### 2.5 PR Review Request Notifications

```rust
pub struct PRMonitor {
    github: Octocrab,
    gitlab: Option<gitlab::AsyncGitlab>,
}

impl PRMonitor {
    pub async fn check_review_requests(&self) -> Result<Vec<PRNotification>> {
        let mut notifications = Vec::new();

        // GitHub pull requests
        let search_query = "type:pr is:open review-requested:@me";
        let prs = self.github
            .search()
            .issues_and_pull_requests(search_query)
            .send()
            .await?;

        for pr in prs.items {
            let notification = PRNotification {
                id: format!("pr-{}", pr.number),
                platform: "GitHub".to_string(),
                repo: pr.repository_url.clone(),
                number: pr.number,
                title: pr.title.clone(),
                author: pr.user.login.clone(),
                url: pr.html_url.clone(),
                created_at: pr.created_at,
                actions: vec![
                    NotificationAction {
                        id: "view_pr".to_string(),
                        label: "View PR".to_string(),
                        action: ActionType::OpenUrl(pr.html_url.clone()),
                    },
                    NotificationAction {
                        id: "approve".to_string(),
                        label: "Approve".to_string(),
                        action: ActionType::ApiCall {
                            method: "POST".to_string(),
                            url: format!(
                                "https://api.github.com/repos/{}/pulls/{}/reviews",
                                pr.repository_url, pr.number
                            ),
                        },
                    },
                    NotificationAction {
                        id: "request_changes".to_string(),
                        label: "Request Changes".to_string(),
                        action: ActionType::Custom("pr_review_dialog".to_string()),
                    },
                ],
            };

            notifications.push(notification);
        }

        Ok(notifications)
    }
}
```

---

## 3. Developer Alerts and Monitoring

### 3.1 Server Monitoring Integration

```rust
// Integration with popular monitoring tools

pub struct MonitoringIntegration {
    providers: Vec<Box<dyn MonitoringProvider>>,
}

pub trait MonitoringProvider: Send + Sync {
    async fn get_alerts(&self) -> Result<Vec<Alert>>;
    async fn acknowledge_alert(&self, alert_id: &str) -> Result<()>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub source: String,
    pub severity: AlertSeverity,
    pub title: String,
    pub description: String,
    pub service: String,
    pub metric: Option<String>,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
    pub timestamp: DateTime<Utc>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialOrd, Ord, PartialEq, Eq)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

// Datadog integration
pub struct DatadogProvider {
    api_key: String,
    app_key: String,
}

impl MonitoringProvider for DatadogProvider {
    async fn get_alerts(&self) -> Result<Vec<Alert>> {
        let client = reqwest::Client::new();

        let response = client
            .get("https://api.datadoghq.com/api/v1/monitor")
            .query(&[("group_states", "alert,warn,no data")])
            .header("DD-API-KEY", &self.api_key)
            .header("DD-APPLICATION-KEY", &self.app_key)
            .send()
            .await?;

        let monitors: Vec<DatadogMonitor> = response.json().await?;

        Ok(monitors.into_iter()
            .filter(|m| m.overall_state != "OK")
            .map(|m| Alert {
                id: m.id.to_string(),
                source: "Datadog".to_string(),
                severity: match m.overall_state.as_str() {
                    "Alert" => AlertSeverity::Critical,
                    "Warn" => AlertSeverity::Warning,
                    _ => AlertSeverity::Info,
                },
                title: m.name,
                description: m.message,
                service: m.tags.iter()
                    .find(|t| t.starts_with("service:"))
                    .map(|t| t.strip_prefix("service:").unwrap().to_string())
                    .unwrap_or_default(),
                metric: Some(m.query),
                value: None,
                threshold: None,
                timestamp: Utc::now(),
                url: Some(format!("https://app.datadoghq.com/monitors/{}", m.id)),
            })
            .collect())
    }
}

// Prometheus/Grafana integration
pub struct PrometheusProvider {
    base_url: String,
}

impl MonitoringProvider for PrometheusProvider {
    async fn get_alerts(&self) -> Result<Vec<Alert>> {
        let client = reqwest::Client::new();

        let response = client
            .get(&format!("{}/api/v1/alerts", self.base_url))
            .send()
            .await?;

        let alerts_response: PrometheusAlertsResponse = response.json().await?;

        Ok(alerts_response.data.alerts.into_iter()
            .map(|a| Alert {
                id: format!("{:?}", a.labels), // Use labels as ID
                source: "Prometheus".to_string(),
                severity: match a.labels.get("severity") {
                    Some(s) if s == "critical" => AlertSeverity::Critical,
                    Some(s) if s == "warning" => AlertSeverity::Warning,
                    _ => AlertSeverity::Info,
                },
                title: a.labels.get("alertname")
                    .cloned()
                    .unwrap_or_else(|| "Unknown Alert".to_string()),
                description: a.annotations.get("description")
                    .or(a.annotations.get("summary"))
                    .cloned()
                    .unwrap_or_default(),
                service: a.labels.get("job")
                    .cloned()
                    .unwrap_or_default(),
                metric: None,
                value: None,
                threshold: None,
                timestamp: a.active_at,
                url: None,
            })
            .collect())
    }
}

// PagerDuty integration
pub struct PagerDutyProvider {
    api_token: String,
}

// Similar implementations for:
// - New Relic
// - Sentry
// - AppDynamics
// - CloudWatch
// - Azure Monitor
```

### 3.2 Log Error Detection

```rust
pub struct LogMonitor {
    // Tail log files and detect errors
    watchers: HashMap<PathBuf, LogWatcher>,

    // Error patterns
    patterns: Vec<ErrorPattern>,
}

#[derive(Debug, Clone)]
pub struct ErrorPattern {
    pub regex: Regex,
    pub severity: AlertSeverity,
    pub description: String,
}

impl LogMonitor {
    pub fn new() -> Self {
        let patterns = vec![
            ErrorPattern {
                regex: Regex::new(r"(?i)fatal|panic|exception").unwrap(),
                severity: AlertSeverity::Critical,
                description: "Critical error detected".to_string(),
            },
            ErrorPattern {
                regex: Regex::new(r"(?i)error|failed").unwrap(),
                severity: AlertSeverity::Error,
                description: "Error detected".to_string(),
            },
            ErrorPattern {
                regex: Regex::new(r"(?i)warn|warning").unwrap(),
                severity: AlertSeverity::Warning,
                description: "Warning detected".to_string(),
            },
        ];

        Self {
            watchers: HashMap::new(),
            patterns,
        }
    }

    // Watch a log file
    pub async fn watch_log(&mut self, path: PathBuf) -> Result<()> {
        let watcher = LogWatcher::new(path.clone()).await?;

        let mut lines = watcher.tail();

        while let Some(line) = lines.next().await {
            self.process_log_line(&path, &line).await;
        }

        Ok(())
    }

    async fn process_log_line(&self, log_file: &Path, line: &str) {
        for pattern in &self.patterns {
            if pattern.regex.is_match(line) {
                let notification = Notification {
                    id: format!("log-{}", Uuid::new_v4()),
                    app_name: "Log Monitor".to_string(),
                    title: format!("Log {} in {}", pattern.description, log_file.display()),
                    subtitle: None,
                    body: line.to_string(),
                    timestamp: Utc::now(),
                    icon: None,
                    priority: match pattern.severity {
                        AlertSeverity::Critical => Priority::Critical,
                        AlertSeverity::Error => Priority::High,
                        AlertSeverity::Warning => Priority::Normal,
                        AlertSeverity::Info => Priority::Low,
                    },
                    category: Category::SystemAlert,
                    actions: vec![
                        NotificationAction {
                            id: "view_log".to_string(),
                            label: "View Log".to_string(),
                            action: ActionType::OpenFile(log_file.to_path_buf()),
                        },
                        NotificationAction {
                            id: "search_context".to_string(),
                            label: "Search Context".to_string(),
                            action: ActionType::Custom("search_log_context".to_string()),
                        },
                    ],
                };

                self.send_notification(notification).await;
                break; // Only notify once per line
            }
        }
    }
}

pub struct LogWatcher {
    file: File,
    path: PathBuf,
}

impl LogWatcher {
    pub async fn new(path: PathBuf) -> Result<Self> {
        let file = File::open(&path).await?;

        // Seek to end of file (tail mode)
        file.seek(SeekFrom::End(0)).await?;

        Ok(Self { file, path })
    }

    pub fn tail(&mut self) -> impl Stream<Item = String> {
        // Return stream of new log lines
        // Use notify crate to watch file for changes
    }
}
```

### 3.3 Performance Threshold Warnings

```rust
pub struct PerformanceMonitor {
    // System metrics
    system: sysinfo::System,

    // Thresholds
    thresholds: PerformanceThresholds,

    // History for trend detection
    history: VecDeque<PerformanceSnapshot>,
}

#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub cpu_usage: f32,          // Percentage
    pub memory_usage: f32,       // Percentage
    pub disk_usage: f32,         // Percentage
    pub network_latency: u64,    // Milliseconds
    pub response_time: u64,      // Milliseconds
}

#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_latency: Option<u64>,
    pub process_metrics: HashMap<String, ProcessMetrics>,
}

impl PerformanceMonitor {
    pub async fn check_thresholds(&mut self) -> Vec<Alert> {
        let mut alerts = Vec::new();

        self.system.refresh_all();

        let snapshot = PerformanceSnapshot {
            timestamp: Utc::now(),
            cpu_usage: self.system.global_cpu_info().cpu_usage(),
            memory_usage: (self.system.used_memory() as f32 /
                          self.system.total_memory() as f32) * 100.0,
            disk_usage: self.calculate_disk_usage(),
            network_latency: None,
            process_metrics: HashMap::new(),
        };

        // CPU threshold
        if snapshot.cpu_usage > self.thresholds.cpu_usage {
            alerts.push(Alert {
                id: format!("perf-cpu-{}", Utc::now().timestamp()),
                source: "Performance Monitor".to_string(),
                severity: AlertSeverity::Warning,
                title: "High CPU Usage".to_string(),
                description: format!(
                    "CPU usage is {:.1}% (threshold: {:.1}%)",
                    snapshot.cpu_usage, self.thresholds.cpu_usage
                ),
                service: "System".to_string(),
                metric: Some("cpu_usage".to_string()),
                value: Some(snapshot.cpu_usage as f64),
                threshold: Some(self.thresholds.cpu_usage as f64),
                timestamp: Utc::now(),
                url: None,
            });
        }

        // Memory threshold
        if snapshot.memory_usage > self.thresholds.memory_usage {
            alerts.push(Alert {
                id: format!("perf-mem-{}", Utc::now().timestamp()),
                source: "Performance Monitor".to_string(),
                severity: AlertSeverity::Warning,
                title: "High Memory Usage".to_string(),
                description: format!(
                    "Memory usage is {:.1}% (threshold: {:.1}%)",
                    snapshot.memory_usage, self.thresholds.memory_usage
                ),
                service: "System".to_string(),
                metric: Some("memory_usage".to_string()),
                value: Some(snapshot.memory_usage as f64),
                threshold: Some(self.thresholds.memory_usage as f64),
                timestamp: Utc::now(),
                url: None,
            });
        }

        // Store snapshot for trend analysis
        self.history.push_back(snapshot);
        if self.history.len() > 100 {
            self.history.pop_front();
        }

        alerts
    }

    // Detect trends (e.g., gradual memory leak)
    pub fn detect_trends(&self) -> Vec<Alert> {
        let mut alerts = Vec::new();

        if self.history.len() < 10 {
            return alerts; // Not enough data
        }

        // Check for sustained high usage
        let recent = self.history.iter().rev().take(10);
        let avg_cpu: f32 = recent.clone().map(|s| s.cpu_usage).sum::<f32>() / 10.0;
        let avg_mem: f32 = recent.map(|s| s.memory_usage).sum::<f32>() / 10.0;

        if avg_cpu > self.thresholds.cpu_usage * 0.8 {
            alerts.push(Alert {
                id: format!("trend-cpu-{}", Utc::now().timestamp()),
                source: "Performance Monitor".to_string(),
                severity: AlertSeverity::Info,
                title: "Sustained High CPU Usage".to_string(),
                description: format!(
                    "Average CPU usage over last 10 checks: {:.1}%",
                    avg_cpu
                ),
                service: "System".to_string(),
                metric: Some("cpu_usage_avg".to_string()),
                value: Some(avg_cpu as f64),
                threshold: None,
                timestamp: Utc::now(),
                url: None,
            });
        }

        alerts
    }
}
```

### 3.4 Security Vulnerability Alerts

```rust
pub struct SecurityMonitor {
    // Dependency vulnerability scanning
    scanners: Vec<Box<dyn VulnerabilityScanner>>,
}

pub trait VulnerabilityScanner: Send + Sync {
    async fn scan_dependencies(&self, project_path: &Path) -> Result<Vec<Vulnerability>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vulnerability {
    pub id: String,
    pub package: String,
    pub version: String,
    pub severity: VulnerabilitySeverity,
    pub title: String,
    pub description: String,
    pub cve: Option<String>,
    pub cvss_score: Option<f32>,
    pub fixed_in: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Low,
    Moderate,
    High,
    Critical,
}

// cargo-audit for Rust projects
pub struct CargoAuditScanner;

impl VulnerabilityScanner for CargoAuditScanner {
    async fn scan_dependencies(&self, project_path: &Path) -> Result<Vec<Vulnerability>> {
        let output = Command::new("cargo")
            .arg("audit")
            .arg("--json")
            .current_dir(project_path)
            .output()
            .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let audit_report: CargoAuditReport = serde_json::from_slice(&output.stdout)?;

        Ok(audit_report.vulnerabilities.values()
            .flatten()
            .map(|v| Vulnerability {
                id: v.advisory.id.clone(),
                package: v.advisory.package.clone(),
                version: v.versions.patched.first()
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                severity: match v.advisory.cvss.as_ref() {
                    Some(cvss) if cvss.score >= 9.0 => VulnerabilitySeverity::Critical,
                    Some(cvss) if cvss.score >= 7.0 => VulnerabilitySeverity::High,
                    Some(cvss) if cvss.score >= 4.0 => VulnerabilitySeverity::Moderate,
                    _ => VulnerabilitySeverity::Low,
                },
                title: v.advisory.title.clone(),
                description: v.advisory.description.clone(),
                cve: v.advisory.cve.clone(),
                cvss_score: v.advisory.cvss.as_ref().map(|c| c.score),
                fixed_in: v.versions.patched.first().map(|v| v.to_string()),
                url: v.advisory.url.clone(),
            })
            .collect())
    }
}

// npm audit for JavaScript projects
pub struct NpmAuditScanner;

// snyk for multi-language support
pub struct SnykScanner {
    api_token: String,
}

impl SecurityMonitor {
    pub async fn scan_project(&self, project_path: &Path) -> Result<Vec<Vulnerability>> {
        let mut vulnerabilities = Vec::new();

        for scanner in &self.scanners {
            let vulns = scanner.scan_dependencies(project_path).await?;
            vulnerabilities.extend(vulns);
        }

        // Deduplicate by CVE
        vulnerabilities.sort_by(|a, b| a.cve.cmp(&b.cve));
        vulnerabilities.dedup_by(|a, b| a.cve == b.cve);

        Ok(vulnerabilities)
    }

    pub async fn notify_vulnerabilities(&self, vulns: Vec<Vulnerability>) {
        for vuln in vulns {
            // Only notify for high/critical
            if matches!(vuln.severity, VulnerabilitySeverity::High | VulnerabilitySeverity::Critical) {
                let notification = Notification {
                    id: format!("vuln-{}", vuln.id),
                    app_name: "Security Scanner".to_string(),
                    title: format!("{} vulnerability in {}",
                        match vuln.severity {
                            VulnerabilitySeverity::Critical => "Critical",
                            VulnerabilitySeverity::High => "High",
                            _ => "Security",
                        },
                        vuln.package
                    ),
                    subtitle: vuln.cve.clone(),
                    body: vuln.title.clone(),
                    timestamp: Utc::now(),
                    icon: None,
                    priority: match vuln.severity {
                        VulnerabilitySeverity::Critical => Priority::Critical,
                        VulnerabilitySeverity::High => Priority::High,
                        _ => Priority::Normal,
                    },
                    category: Category::SecurityAlert,
                    actions: vec![
                        NotificationAction {
                            id: "view_details".to_string(),
                            label: "View Details".to_string(),
                            action: ActionType::OpenUrl(vuln.url.clone()),
                        },
                        NotificationAction {
                            id: "update_package".to_string(),
                            label: format!("Update to {}",
                                vuln.fixed_in.unwrap_or_else(|| "latest".to_string())
                            ),
                            action: ActionType::Custom("update_dependency".to_string()),
                        },
                    ],
                };

                self.send_notification(notification).await;
            }
        }
    }
}
```

### 3.5 Dependency Update Notifications

```rust
pub struct DependencyMonitor {
    // Track outdated dependencies
    checkers: HashMap<PackageManager, Box<dyn DependencyChecker>>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum PackageManager {
    Cargo,
    Npm,
    Yarn,
    Pip,
    Go,
}

pub trait DependencyChecker: Send + Sync {
    async fn check_updates(&self, project_path: &Path) -> Result<Vec<DependencyUpdate>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyUpdate {
    pub package: String,
    pub current_version: String,
    pub latest_version: String,
    pub update_type: UpdateType,
    pub breaking_changes: bool,
    pub release_notes_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpdateType {
    Major,   // 1.x.x -> 2.x.x
    Minor,   // 1.1.x -> 1.2.x
    Patch,   // 1.1.1 -> 1.1.2
}

// cargo-outdated for Rust
pub struct CargoOutdatedChecker;

impl DependencyChecker for CargoOutdatedChecker {
    async fn check_updates(&self, project_path: &Path) -> Result<Vec<DependencyUpdate>> {
        let output = Command::new("cargo")
            .arg("outdated")
            .arg("--format=json")
            .current_dir(project_path)
            .output()
            .await?;

        let outdated: CargoOutdatedReport = serde_json::from_slice(&output.stdout)?;

        Ok(outdated.dependencies.into_iter()
            .map(|dep| DependencyUpdate {
                package: dep.name,
                current_version: dep.project,
                latest_version: dep.latest,
                update_type: self.determine_update_type(&dep.project, &dep.latest),
                breaking_changes: dep.breaking,
                release_notes_url: Some(format!(
                    "https://crates.io/crates/{}/{}",
                    dep.name, dep.latest
                )),
            })
            .collect())
    }
}

impl DependencyMonitor {
    // Check for updates periodically
    pub async fn check_all_projects(&self, projects: Vec<PathBuf>) -> Result<()> {
        for project in projects {
            let package_manager = self.detect_package_manager(&project)?;

            if let Some(checker) = self.checkers.get(&package_manager) {
                let updates = checker.check_updates(&project).await?;

                // Only notify for major/minor updates
                let significant_updates: Vec<_> = updates.into_iter()
                    .filter(|u| matches!(u.update_type, UpdateType::Major | UpdateType::Minor))
                    .collect();

                if !significant_updates.is_empty() {
                    self.notify_updates(project, significant_updates).await;
                }
            }
        }

        Ok(())
    }

    async fn notify_updates(&self, project: PathBuf, updates: Vec<DependencyUpdate>) {
        let notification = Notification {
            id: format!("deps-{}", project.display()),
            app_name: "Dependency Monitor".to_string(),
            title: format!("{} package updates available", updates.len()),
            subtitle: Some(project.to_string_lossy().to_string()),
            body: updates.iter()
                .take(5)
                .map(|u| format!(
                    "{}: {} -> {}",
                    u.package, u.current_version, u.latest_version
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            timestamp: Utc::now(),
            icon: None,
            priority: Priority::Low,
            category: Category::DependencyUpdate,
            actions: vec![
                NotificationAction {
                    id: "view_updates".to_string(),
                    label: "View All Updates".to_string(),
                    action: ActionType::Custom("show_dependency_updates".to_string()),
                },
                NotificationAction {
                    id: "update_all".to_string(),
                    label: "Update All".to_string(),
                    action: ActionType::RunCommand {
                        command: "cargo update".to_string(), // or npm update, etc.
                        directory: project,
                    },
                },
            ],
        };

        self.send_notification(notification).await;
    }
}
```

---

## 4. Communication Bridging

### 4.1 Slack Integration

```rust
use slack_api::{Client as SlackClient, MessageEvent};

pub struct SlackBridge {
    client: SlackClient,
    token: String,
    channels: Vec<String>,
}

impl SlackBridge {
    pub async fn new(token: String, channels: Vec<String>) -> Result<Self> {
        let client = SlackClient::new()?;

        Ok(Self {
            client,
            token,
            channels,
        })
    }

    // Listen for mentions and direct messages
    pub async fn listen_for_messages(&self) -> Result<()> {
        let rtm = self.client.rtm_start(&self.token).await?;

        let mut events = rtm.events();

        while let Some(event) = events.next().await {
            match event {
                Event::Message(msg) => {
                    if self.should_notify(&msg) {
                        self.create_notification(msg).await;
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn should_notify(&self, msg: &MessageEvent) -> bool {
        // Notify if:
        // 1. Direct message
        // 2. Mentioned in message
        // 3. Thread reply
        // 4. Watched channels

        msg.channel.starts_with('D') // DM
            || msg.text.contains("@username") // Mention
            || msg.thread_ts.is_some() // Thread reply
            || self.channels.contains(&msg.channel) // Watched channel
    }

    async fn create_notification(&self, msg: MessageEvent) {
        let notification = Notification {
            id: format!("slack-{}", msg.ts),
            app_name: "Slack".to_string(),
            title: format!("Message from {}", msg.user),
            subtitle: Some(self.get_channel_name(&msg.channel).await),
            body: msg.text,
            timestamp: Utc::now(),
            icon: None,
            priority: if msg.channel.starts_with('D') {
                Priority::High // DMs are high priority
            } else {
                Priority::Normal
            },
            category: Category::Communication,
            actions: vec![
                NotificationAction {
                    id: "view".to_string(),
                    label: "Open in Slack".to_string(),
                    action: ActionType::OpenUrl(format!(
                        "slack://channel?team={}&id={}",
                        msg.team, msg.channel
                    )),
                },
                NotificationAction {
                    id: "reply".to_string(),
                    label: "Reply".to_string(),
                    action: ActionType::Custom("slack_reply_dialog".to_string()),
                },
            ],
        };

        self.send_notification(notification).await;
    }
}
```

### 4.2 Discord Integration

```rust
use serenity::{
    async_trait,
    model::{channel::Message, gateway::Ready},
    prelude::*,
};

pub struct DiscordBridge {
    client: Client,
}

impl DiscordBridge {
    pub async fn new(token: String) -> Result<Self> {
        let intents = GatewayIntents::GUILD_MESSAGES
            | GatewayIntents::DIRECT_MESSAGES
            | GatewayIntents::MESSAGE_CONTENT;

        let client = Client::builder(&token, intents)
            .event_handler(DiscordHandler)
            .await?;

        Ok(Self { client })
    }
}

struct DiscordHandler;

#[async_trait]
impl EventHandler for DiscordHandler {
    async fn message(&self, ctx: Context, msg: Message) {
        // Skip bot messages
        if msg.author.bot {
            return;
        }

        // Check if we should notify
        let should_notify = msg.mentions_me(&ctx).await.unwrap_or(false)
            || msg.is_private()
            || self.is_watched_channel(msg.channel_id);

        if should_notify {
            let notification = Notification {
                id: format!("discord-{}", msg.id),
                app_name: "Discord".to_string(),
                title: format!("Message from {}", msg.author.name),
                subtitle: Some(self.get_channel_name(&ctx, msg.channel_id).await),
                body: msg.content,
                timestamp: msg.timestamp.into(),
                icon: msg.author.avatar_url(),
                priority: if msg.is_private() {
                    Priority::High
                } else {
                    Priority::Normal
                },
                category: Category::Communication,
                actions: vec![
                    NotificationAction {
                        id: "view".to_string(),
                        label: "Open in Discord".to_string(),
                        action: ActionType::OpenUrl(msg.link()),
                    },
                    NotificationAction {
                        id: "reply".to_string(),
                        label: "Reply".to_string(),
                        action: ActionType::Custom("discord_reply_dialog".to_string()),
                    },
                ],
            };

            self.send_notification(notification).await;
        }
    }
}
```

### 4.3 Email Notification Summaries

```rust
use imap::Session;
use native_tls::TlsStream;
use std::net::TcpStream;

pub struct EmailMonitor {
    session: Session<TlsStream<TcpStream>>,
    folders: Vec<String>,
}

impl EmailMonitor {
    pub async fn new(
        server: &str,
        username: &str,
        password: &str,
    ) -> Result<Self> {
        let tls = native_tls::TlsConnector::new()?;
        let client = imap::connect((server, 993), server, &tls)?;

        let session = client.login(username, password)
            .map_err(|e| e.0)?;

        Ok(Self {
            session,
            folders: vec!["INBOX".to_string()],
        })
    }

    // Check for new unread emails
    pub async fn check_new_emails(&mut self) -> Result<Vec<EmailSummary>> {
        let mut summaries = Vec::new();

        for folder in &self.folders {
            self.session.select(folder)?;

            // Search for unread emails
            let uids = self.session.search("UNSEEN")?;

            for uid in uids {
                let messages = self.session.fetch(uid.to_string(), "RFC822.HEADER")?;

                for message in messages.iter() {
                    if let Some(header) = message.header() {
                        let parsed = mailparse::parse_mail(header)?;

                        summaries.push(EmailSummary {
                            uid,
                            from: self.parse_header(&parsed, "From"),
                            subject: self.parse_header(&parsed, "Subject"),
                            date: self.parse_header(&parsed, "Date"),
                            folder: folder.clone(),
                        });
                    }
                }
            }
        }

        Ok(summaries)
    }

    // Create digest notification
    pub async fn create_email_digest(&self, summaries: Vec<EmailSummary>) {
        if summaries.is_empty() {
            return;
        }

        let notification = Notification {
            id: format!("email-digest-{}", Utc::now().timestamp()),
            app_name: "Email".to_string(),
            title: format!("{} new email(s)", summaries.len()),
            subtitle: None,
            body: summaries.iter()
                .take(5)
                .map(|s| format!("{}: {}", s.from, s.subject))
                .collect::<Vec<_>>()
                .join("\n"),
            timestamp: Utc::now(),
            icon: None,
            priority: Priority::Low,
            category: Category::Communication,
            actions: vec![
                NotificationAction {
                    id: "open_inbox".to_string(),
                    label: "Open Inbox".to_string(),
                    action: ActionType::OpenUrl("mailto:".to_string()),
                },
            ],
        };

        self.send_notification(notification).await;
    }
}

#[derive(Debug, Clone)]
pub struct EmailSummary {
    pub uid: u32,
    pub from: String,
    pub subject: String,
    pub date: String,
    pub folder: String,
}
```

### 4.4 Calendar Reminders

```rust
use icalendar::{Calendar, Component};

pub struct CalendarMonitor {
    // Calendar sources (CalDAV, Google Calendar API, etc.)
    sources: Vec<Box<dyn CalendarSource>>,
}

pub trait CalendarSource: Send + Sync {
    async fn get_events(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>>;
}

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub location: Option<String>,
    pub meeting_url: Option<String>,
    pub attendees: Vec<String>,
}

impl CalendarMonitor {
    // Check for upcoming events
    pub async fn check_upcoming_events(&self) -> Result<Vec<CalendarEvent>> {
        let now = Utc::now();
        let next_hour = now + chrono::Duration::hours(1);

        let mut all_events = Vec::new();

        for source in &self.sources {
            let events = source.get_events(now, next_hour).await?;
            all_events.extend(events);
        }

        // Sort by start time
        all_events.sort_by(|a, b| a.start.cmp(&b.start));

        Ok(all_events)
    }

    // Create reminder notifications
    pub async fn create_reminders(&self, events: Vec<CalendarEvent>) {
        for event in events {
            let time_until = event.start - Utc::now();
            let minutes_until = time_until.num_minutes();

            // Remind 15 minutes before
            if minutes_until == 15 {
                let notification = Notification {
                    id: format!("calendar-{}", event.id),
                    app_name: "Calendar".to_string(),
                    title: format!("Meeting in 15 minutes: {}", event.title),
                    subtitle: event.location.clone(),
                    body: event.description.unwrap_or_default(),
                    timestamp: Utc::now(),
                    icon: None,
                    priority: Priority::High,
                    category: Category::Calendar,
                    actions: self.create_calendar_actions(&event),
                };

                self.send_notification(notification).await;
            }
        }
    }

    fn create_calendar_actions(&self, event: &CalendarEvent) -> Vec<NotificationAction> {
        let mut actions = vec![];

        // Join meeting action (if URL available)
        if let Some(url) = &event.meeting_url {
            actions.push(NotificationAction {
                id: "join_meeting".to_string(),
                label: "Join Meeting".to_string(),
                action: ActionType::OpenUrl(url.clone()),
            });
        }

        // Snooze action
        actions.push(NotificationAction {
            id: "snooze".to_string(),
            label: "Snooze 5 min".to_string(),
            action: ActionType::Custom("snooze_notification".to_string()),
        });

        actions
    }
}

// Google Calendar API implementation
pub struct GoogleCalendarSource {
    api_key: String,
    calendar_id: String,
}

#[async_trait]
impl CalendarSource for GoogleCalendarSource {
    async fn get_events(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<CalendarEvent>> {
        let client = reqwest::Client::new();

        let response = client
            .get(&format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events",
                self.calendar_id
            ))
            .query(&[
                ("timeMin", start.to_rfc3339()),
                ("timeMax", end.to_rfc3339()),
                ("singleEvents", "true".to_string()),
                ("orderBy", "startTime".to_string()),
            ])
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        let events_response: GoogleCalendarEventsResponse = response.json().await?;

        Ok(events_response.items.into_iter()
            .map(|item| CalendarEvent {
                id: item.id,
                title: item.summary,
                description: item.description,
                start: item.start.date_time,
                end: item.end.date_time,
                location: item.location,
                meeting_url: item.hangout_link.or_else(|| {
                    // Extract Zoom/Meet links from description
                    self.extract_meeting_url(&item.description.unwrap_or_default())
                }),
                attendees: item.attendees.into_iter()
                    .map(|a| a.email)
                    .collect(),
            })
            .collect())
    }
}
```

### 4.5 Meeting Join Shortcuts

```rust
pub struct MeetingLinkDetector {
    // Patterns for common meeting platforms
    patterns: HashMap<MeetingPlatform, Regex>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum MeetingPlatform {
    Zoom,
    GoogleMeet,
    MicrosoftTeams,
    Webex,
    Slack,
    Discord,
}

impl MeetingLinkDetector {
    pub fn new() -> Self {
        let patterns = hashmap! {
            MeetingPlatform::Zoom => Regex::new(
                r"https://[\w-]+\.zoom\.us/j/\d+"
            ).unwrap(),
            MeetingPlatform::GoogleMeet => Regex::new(
                r"https://meet\.google\.com/[\w-]+"
            ).unwrap(),
            MeetingPlatform::MicrosoftTeams => Regex::new(
                r"https://teams\.microsoft\.com/l/meetup-join/"
            ).unwrap(),
            MeetingPlatform::Webex => Regex::new(
                r"https://[\w-]+\.webex\.com/meet/"
            ).unwrap(),
        };

        Self { patterns }
    }

    pub fn extract_meeting_links(&self, text: &str) -> Vec<MeetingLink> {
        let mut links = Vec::new();

        for (platform, pattern) in &self.patterns {
            if let Some(captures) = pattern.captures(text) {
                if let Some(url) = captures.get(0) {
                    links.push(MeetingLink {
                        platform: platform.clone(),
                        url: url.as_str().to_string(),
                    });
                }
            }
        }

        links
    }

    // Deep link support for quick join
    pub fn create_deep_link(&self, link: &MeetingLink) -> String {
        match link.platform {
            MeetingPlatform::Zoom => {
                // zoommtg://zoom.us/join?confno=123456789
                link.url.replace("https://", "zoommtg://")
            }
            MeetingPlatform::GoogleMeet => {
                // Use browser by default
                link.url.clone()
            }
            MeetingPlatform::MicrosoftTeams => {
                // msteams://teams.microsoft.com/l/...
                link.url.replace("https://", "msteams://")
            }
            MeetingPlatform::Webex => {
                // webexteams://meet?...
                link.url.replace("https://", "webexteams://")
            }
            _ => link.url.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeetingLink {
    pub platform: MeetingPlatform,
    pub url: String,
}
```

---

## 5. Smart Notification Features

### 5.1 AI-Powered Priority Sorting

```rust
use onnxruntime::{environment::Environment, GraphOptimizationLevel, LoggingLevel};

pub struct NotificationPriorityModel {
    // ML model for priority prediction
    env: Environment,
    session: Session,
}

impl NotificationPriorityModel {
    pub fn new(model_path: &Path) -> Result<Self> {
        let env = Environment::builder()
            .with_name("notification_priority")
            .with_log_level(LoggingLevel::Warning)
            .build()?;

        let session = env
            .new_session_builder()?
            .with_optimization_level(GraphOptimizationLevel::All)?
            .with_model_from_file(model_path)?;

        Ok(Self { env, session })
    }

    // Predict notification priority
    pub async fn predict_priority(&self, notif: &Notification) -> PriorityScore {
        // Extract features
        let features = self.extract_features(notif);

        // Run model inference
        let input_tensor = Array::from_shape_vec(
            (1, features.len()),
            features,
        )?;

        let outputs = self.session.run(vec![input_tensor.into()])?;

        // Parse output
        let priority_scores: Vec<f32> = outputs[0].try_extract()?.view().to_slice()?.to_vec();

        PriorityScore {
            critical: priority_scores[0],
            high: priority_scores[1],
            normal: priority_scores[2],
            low: priority_scores[3],
        }
    }

    fn extract_features(&self, notif: &Notification) -> Vec<f32> {
        vec![
            // Temporal features
            self.time_of_day_score(notif.timestamp),
            self.day_of_week_score(notif.timestamp),

            // Content features
            self.urgency_words_score(&notif.title, &notif.body),
            self.sender_importance_score(&notif.app_name),

            // User behavior features (learned)
            self.user_engagement_score(notif),
            self.historical_priority_score(notif),

            // Category features
            match notif.category {
                Category::Build => 0.8,
                Category::Test => 0.7,
                Category::SecurityAlert => 0.9,
                Category::Communication => 0.5,
                _ => 0.5,
            },
        ]
    }

    fn urgency_words_score(&self, title: &str, body: &str) -> f32 {
        let urgent_keywords = [
            "urgent", "critical", "failed", "error", "security",
            "breach", "down", "outage", "alert", "immediate"
        ];

        let text = format!("{} {}", title.to_lowercase(), body.to_lowercase());

        let count = urgent_keywords.iter()
            .filter(|&keyword| text.contains(keyword))
            .count();

        (count as f32 / urgent_keywords.len() as f32).min(1.0)
    }

    fn user_engagement_score(&self, notif: &Notification) -> f32 {
        // Historical engagement rate for this type of notification
        // Tracked over time: did user click/dismiss/ignore?
        self.get_historical_engagement(&notif.app_name, &notif.category)
    }
}

#[derive(Debug, Clone)]
pub struct PriorityScore {
    pub critical: f32,
    pub high: f32,
    pub normal: f32,
    pub low: f32,
}

impl PriorityScore {
    pub fn predicted_priority(&self) -> Priority {
        let max_score = self.critical.max(self.high).max(self.normal).max(self.low);

        if max_score == self.critical {
            Priority::Critical
        } else if max_score == self.high {
            Priority::High
        } else if max_score == self.normal {
            Priority::Normal
        } else {
            Priority::Low
        }
    }
}
```

### 5.2 Notification Batching

```rust
pub struct NotificationBatcher {
    // Pending notifications grouped by category
    pending: HashMap<Category, Vec<Notification>>,

    // Batching rules
    rules: Vec<BatchRule>,

    // Timers for batch delivery
    timers: HashMap<Category, Instant>,
}

#[derive(Debug, Clone)]
pub struct BatchRule {
    pub category: Category,
    pub batch_window: Duration,  // How long to wait before sending batch
    pub max_batch_size: usize,   // Max notifications in one batch
    pub min_batch_size: usize,   // Don't batch if fewer than this
}

impl NotificationBatcher {
    pub fn new() -> Self {
        let rules = vec![
            BatchRule {
                category: Category::Communication,
                batch_window: Duration::from_secs(300), // 5 minutes
                max_batch_size: 10,
                min_batch_size: 3,
            },
            BatchRule {
                category: Category::DependencyUpdate,
                batch_window: Duration::from_secs(3600), // 1 hour
                max_batch_size: 20,
                min_batch_size: 5,
            },
            // Don't batch critical notifications
            BatchRule {
                category: Category::SecurityAlert,
                batch_window: Duration::from_secs(0), // Immediate
                max_batch_size: 1,
                min_batch_size: 1,
            },
        ];

        Self {
            pending: HashMap::new(),
            rules,
            timers: HashMap::new(),
        }
    }

    pub async fn add_notification(&mut self, notif: Notification) {
        let category = notif.category.clone();
        let rule = self.get_rule(&category);

        // Check if should batch
        if rule.batch_window.as_secs() == 0 {
            // Send immediately (no batching)
            self.send_notification(notif).await;
            return;
        }

        // Add to pending
        self.pending.entry(category.clone())
            .or_insert_with(Vec::new)
            .push(notif);

        // Start timer if not already started
        if !self.timers.contains_key(&category) {
            self.timers.insert(category.clone(), Instant::now());
        }

        // Check if should flush
        let pending_count = self.pending.get(&category).map_or(0, |v| v.len());
        let elapsed = self.timers.get(&category)
            .map_or(Duration::ZERO, |t| t.elapsed());

        if pending_count >= rule.max_batch_size || elapsed >= rule.batch_window {
            self.flush_category(&category).await;
        }
    }

    async fn flush_category(&mut self, category: &Category) {
        if let Some(notifications) = self.pending.remove(category) {
            if notifications.len() < self.get_rule(category).min_batch_size {
                // Not enough for a batch, send individually
                for notif in notifications {
                    self.send_notification(notif).await;
                }
            } else {
                // Create batch notification
                let batch = self.create_batch_notification(category, notifications);
                self.send_notification(batch).await;
            }
        }

        self.timers.remove(category);
    }

    fn create_batch_notification(
        &self,
        category: &Category,
        notifications: Vec<Notification>,
    ) -> Notification {
        let count = notifications.len();

        Notification {
            id: format!("batch-{}-{}", category.as_str(), Utc::now().timestamp()),
            app_name: "Notification Center".to_string(),
            title: format!("{} {} notifications", count, category.as_str()),
            subtitle: None,
            body: notifications.iter()
                .take(5)
                .map(|n| format!("• {}", n.title))
                .collect::<Vec<_>>()
                .join("\n")
                + if count > 5 {
                    &format!("\n...and {} more", count - 5)
                } else {
                    ""
                },
            timestamp: Utc::now(),
            icon: None,
            priority: notifications.iter()
                .map(|n| n.priority.clone())
                .max()
                .unwrap_or(Priority::Normal),
            category: category.clone(),
            actions: vec![
                NotificationAction {
                    id: "view_all".to_string(),
                    label: "View All".to_string(),
                    action: ActionType::Custom("show_notification_list".to_string()),
                },
            ],
        }
    }
}
```

### 5.3 Focus Mode Integration

```rust
pub struct FocusMode {
    // Current focus state
    current_mode: Option<FocusModeType>,

    // Schedule
    schedules: Vec<FocusSchedule>,

    // Allow list (what can interrupt focus mode)
    allow_list: FocusAllowList,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusModeType {
    DeepWork,
    Meeting,
    Personal,
    Sleep,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct FocusSchedule {
    pub mode: FocusModeType,
    pub days: Vec<chrono::Weekday>,
    pub start_time: chrono::NaiveTime,
    pub end_time: chrono::NaiveTime,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct FocusAllowList {
    pub categories: Vec<Category>,
    pub apps: Vec<String>,
    pub priorities: Vec<Priority>,
    pub people: Vec<String>, // Specific senders/users
}

impl FocusMode {
    pub fn should_display_notification(&self, notif: &Notification) -> bool {
        // If not in focus mode, show all
        if self.current_mode.is_none() {
            return true;
        }

        // Check allow list
        if self.allow_list.priorities.contains(&notif.priority) {
            return true;
        }

        if self.allow_list.categories.contains(&notif.category) {
            return true;
        }

        if self.allow_list.apps.contains(&notif.app_name) {
            return true;
        }

        // Critical always gets through
        if matches!(notif.priority, Priority::Critical) {
            return true;
        }

        // Otherwise, suppress
        false
    }

    pub fn auto_activate_scheduled(&mut self) {
        let now = chrono::Local::now();
        let current_time = now.time();
        let current_day = now.weekday();

        for schedule in &self.schedules {
            if !schedule.enabled {
                continue;
            }

            if schedule.days.contains(&current_day)
                && current_time >= schedule.start_time
                && current_time < schedule.end_time
            {
                self.activate_mode(schedule.mode.clone());
                return;
            }
        }

        // No scheduled mode active
        self.deactivate_mode();
    }

    pub fn activate_mode(&mut self, mode: FocusModeType) {
        self.current_mode = Some(mode.clone());

        // Notify user
        let notification = Notification {
            id: format!("focus-{}", Utc::now().timestamp()),
            app_name: "Focus Mode".to_string(),
            title: format!("{} mode activated", self.mode_name(&mode)),
            subtitle: Some("Notifications will be filtered".to_string()),
            body: String::new(),
            timestamp: Utc::now(),
            icon: None,
            priority: Priority::Low,
            category: Category::System,
            actions: vec![
                NotificationAction {
                    id: "configure".to_string(),
                    label: "Configure".to_string(),
                    action: ActionType::Custom("focus_mode_settings".to_string()),
                },
            ],
        };

        self.send_notification(notification).await;
    }

    fn mode_name(&self, mode: &FocusModeType) -> String {
        match mode {
            FocusModeType::DeepWork => "Deep Work".to_string(),
            FocusModeType::Meeting => "Meeting".to_string(),
            FocusModeType::Personal => "Personal Time".to_string(),
            FocusModeType::Sleep => "Sleep".to_string(),
            FocusModeType::Custom(name) => name.clone(),
        }
    }
}

// Integration with OS focus modes
#[cfg(target_os = "macos")]
pub struct MacOSFocusIntegration {
    // Monitor macOS Focus status
}

#[cfg(target_os = "macos")]
impl MacOSFocusIntegration {
    pub fn get_current_focus(&self) -> Option<String> {
        // Query macOS Focus status via private APIs or notification center
        // Returns "Do Not Disturb", "Sleep", "Work", etc.
    }

    pub fn sync_focus_state(&self, our_mode: &FocusModeType) {
        // Set macOS Focus to match our focus mode
    }
}
```

### 5.4 Custom Notification Sounds

```rust
use rodio::{Decoder, OutputStream, Sink};

pub struct NotificationSounds {
    // Sound library
    sounds: HashMap<NotificationSoundType, PathBuf>,

    // Per-category/priority mapping
    mappings: HashMap<(Category, Priority), NotificationSoundType>,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum NotificationSoundType {
    Default,
    Success,
    Error,
    Warning,
    Message,
    Build,
    Deploy,
    Alert,
    Custom(String),
}

impl NotificationSounds {
    pub fn play_for_notification(&self, notif: &Notification) {
        let sound_type = self.get_sound_type(notif);

        if let Some(sound_path) = self.sounds.get(&sound_type) {
            self.play_sound(sound_path);
        }
    }

    fn get_sound_type(&self, notif: &Notification) -> NotificationSoundType {
        // Check custom mapping
        if let Some(sound) = self.mappings.get(&(notif.category.clone(), notif.priority.clone())) {
            return sound.clone();
        }

        // Default based on category
        match notif.category {
            Category::Build => NotificationSoundType::Build,
            Category::Test if notif.title.contains("failed") => NotificationSoundType::Error,
            Category::Test => NotificationSoundType::Success,
            Category::SecurityAlert => NotificationSoundType::Alert,
            Category::Communication => NotificationSoundType::Message,
            _ => NotificationSoundType::Default,
        }
    }

    fn play_sound(&self, path: &Path) {
        tokio::spawn(async move {
            let (_stream, stream_handle) = OutputStream::try_default().unwrap();
            let sink = Sink::try_new(&stream_handle).unwrap();

            let file = BufReader::new(File::open(path).unwrap());
            let source = Decoder::new(file).unwrap();

            sink.append(source);
            sink.sleep_until_end();
        });
    }

    // Load sounds from directory
    pub fn load_sounds(&mut self, sound_dir: &Path) -> Result<()> {
        for entry in std::fs::read_dir(sound_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension() {
                if ext == "wav" || ext == "mp3" || ext == "ogg" {
                    let name = path.file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    let sound_type = match name.as_str() {
                        "success" => NotificationSoundType::Success,
                        "error" => NotificationSoundType::Error,
                        "warning" => NotificationSoundType::Warning,
                        "build" => NotificationSoundType::Build,
                        _ => NotificationSoundType::Custom(name),
                    };

                    self.sounds.insert(sound_type, path);
                }
            }
        }

        Ok(())
    }
}
```

### 5.5 Notification Action Buttons

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationAction {
    pub id: String,
    pub label: String,
    pub action: ActionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    // Open URL (browser or deep link)
    OpenUrl(String),

    // Open file in default app
    OpenFile(PathBuf),

    // Open terminal at directory
    OpenTerminal(PathBuf),

    // Run command
    RunCommand {
        command: String,
        directory: PathBuf,
    },

    // API call (for approve PR, acknowledge alert, etc.)
    ApiCall {
        method: String,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
    },

    // Custom action (handled by app)
    Custom(String),
}

pub struct ActionExecutor {
    // Execute notification actions
}

impl ActionExecutor {
    pub async fn execute_action(&self, action: &NotificationAction) -> Result<()> {
        match &action.action {
            ActionType::OpenUrl(url) => {
                open::that(url)?;
            }

            ActionType::OpenFile(path) => {
                open::that(path)?;
            }

            ActionType::OpenTerminal(dir) => {
                #[cfg(target_os = "macos")]
                {
                    Command::new("open")
                        .arg("-a")
                        .arg("Terminal")
                        .arg(dir)
                        .spawn()?;
                }

                #[cfg(target_os = "linux")]
                {
                    // Try common terminal emulators
                    for terminal in &["gnome-terminal", "konsole", "xfce4-terminal", "alacritty"] {
                        if Command::new(terminal)
                            .arg("--working-directory")
                            .arg(dir)
                            .spawn()
                            .is_ok()
                        {
                            break;
                        }
                    }
                }
            }

            ActionType::RunCommand { command, directory } => {
                Command::new("sh")
                    .arg("-c")
                    .arg(command)
                    .current_dir(directory)
                    .spawn()?;
            }

            ActionType::ApiCall { method, url, headers, body } => {
                let client = reqwest::Client::new();

                let mut request = match method.as_str() {
                    "GET" => client.get(url),
                    "POST" => client.post(url),
                    "PUT" => client.put(url),
                    "DELETE" => client.delete(url),
                    _ => return Err(anyhow!("Unsupported HTTP method")),
                };

                if let Some(headers) = headers {
                    for (key, value) in headers {
                        request = request.header(key, value);
                    }
                }

                if let Some(body) = body {
                    request = request.body(body.clone());
                }

                let response = request.send().await?;

                if !response.status().is_success() {
                    return Err(anyhow!("API call failed: {}", response.status()));
                }
            }

            ActionType::Custom(action_id) => {
                // Emit event for app to handle
                self.emit_custom_action(action_id).await?;
            }
        }

        Ok(())
    }
}

// Example: Approve GitHub PR from notification
pub fn create_approve_pr_action(repo: &str, pr_number: u64, token: &str) -> NotificationAction {
    NotificationAction {
        id: "approve_pr".to_string(),
        label: "Approve PR".to_string(),
        action: ActionType::ApiCall {
            method: "POST".to_string(),
            url: format!(
                "https://api.github.com/repos/{}/pulls/{}/reviews",
                repo, pr_number
            ),
            headers: Some(hashmap! {
                "Authorization".to_string() => format!("token {}", token),
                "Accept".to_string() => "application/vnd.github.v3+json".to_string(),
            }),
            body: Some(r#"{"event": "APPROVE"}"#.to_string()),
        },
    }
}

// Example: Restart failed build from notification
pub fn create_restart_build_action(project: &Path) -> NotificationAction {
    NotificationAction {
        id: "restart_build".to_string(),
        label: "Restart Build".to_string(),
        action: ActionType::RunCommand {
            command: "cargo build".to_string(),
            directory: project.to_path_buf(),
        },
    }
}
```

---

## 6. Implementation Architecture

### 6.1 System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│          NOTIFICATION BRIDGE - IMPLEMENTATION LAYERS            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Layer 5: UI / Presentation                             │  │
│  │  • Native notifications (macOS, Linux, Windows)         │  │
│  │  • System tray menu                                     │  │
│  │  │  • Notification list                                     │  │
│  │  • Settings interface                                   │  │
│  └─────────────────────────────────────────────────────────┘  │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Layer 4: Smart Features                               │  │
│  │  • AI priority sorting                                  │  │
│  │  • Notification batching                                │  │
│  │  • Focus mode integration                               │  │
│  │  • Sound management                                     │  │
│  └─────────────────────────────────────────────────────────┘  │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Layer 3: Notification Sources                          │  │
│  │  • Build monitors (GitHub, local)                       │  │
│  │  • Communication bridges (Slack, Discord)               │  │
│  │  • Monitoring integrations (Datadog, Prometheus)        │  │
│  │  • Email, Calendar                                      │  │
│  └─────────────────────────────────────────────────────────┘  │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Layer 2: P2P Sync (libp2p)                            │  │
│  │  • Notification relay                                   │  │
│  │  • History sync (CRDT)                                  │  │
│  │  • Action forwarding                                    │  │
│  │  • DND status sync                                      │  │
│  └─────────────────────────────────────────────────────────┘  │
│                          │                                      │
│  ┌─────────────────────────────────────────────────────────┐  │
│  │  Layer 1: Platform APIs                                 │  │
│  │  • macOS: NSUserNotificationCenter / UNNotificationCenter │  │
│  │  • Linux: D-Bus org.freedesktop.Notifications          │  │
│  │  • Windows: Windows.UI.Notifications                    │  │
│  └─────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 Data Flow

```
1. NOTIFICATION CAPTURE
   System App → Platform API → Capture Layer → Notification Database

2. PROCESSING
   Database → Filter Engine → Priority Model → Smart Features

3. P2P SYNC
   Local Device → libp2p Gossipsub → Remote Peers

4. DISPLAY
   Processed Notification → Platform Display API → User

5. ACTION
   User Click → Action Executor → API/Command/URL
```

### 6.3 Database Schema

```sql
-- Notifications table
CREATE TABLE notifications (
    id TEXT PRIMARY KEY,
    app_name TEXT NOT NULL,
    title TEXT NOT NULL,
    subtitle TEXT,
    body TEXT,
    timestamp DATETIME NOT NULL,
    priority INTEGER NOT NULL,
    category TEXT NOT NULL,
    source_device TEXT,
    read BOOLEAN DEFAULT FALSE,
    dismissed BOOLEAN DEFAULT FALSE,
    acted_on BOOLEAN DEFAULT FALSE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_timestamp (timestamp DESC),
    INDEX idx_category (category),
    INDEX idx_read (read)
);

-- Notification actions
CREATE TABLE notification_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notification_id TEXT NOT NULL,
    action_id TEXT NOT NULL,
    label TEXT NOT NULL,
    action_type TEXT NOT NULL,
    action_data TEXT, -- JSON
    FOREIGN KEY (notification_id) REFERENCES notifications(id)
);

-- Focus mode schedules
CREATE TABLE focus_schedules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mode_type TEXT NOT NULL,
    days TEXT NOT NULL, -- JSON array
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    enabled BOOLEAN DEFAULT TRUE
);

-- User engagement tracking (for ML)
CREATE TABLE notification_engagement (
    notification_id TEXT NOT NULL,
    user_action TEXT NOT NULL, -- clicked, dismissed, ignored, acted
    timestamp DATETIME NOT NULL,
    time_to_action INTEGER, -- milliseconds
    FOREIGN KEY (notification_id) REFERENCES notifications(id)
);

-- Sync log
CREATE TABLE sync_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notification_id TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    action TEXT NOT NULL, -- sent, received, acknowledged
    timestamp DATETIME NOT NULL,
    FOREIGN KEY (notification_id) REFERENCES notifications(id)
);
```

### 6.4 Configuration File

```toml
# ~/.config/codebridge/notifications.toml

[notifications]
enabled = true
sync_across_devices = true

# Capture settings
[capture]
system_notifications = true
app_blacklist = ["Music", "Photos"] # Don't capture from these apps

# P2P sync
[sync]
enabled = true
broadcast_notifications = true
receive_from_peers = true
sync_history = true
sync_dnd_status = true

# Smart features
[smart]
ai_priority_sorting = true
notification_batching = true
batch_window_minutes = 5

# Focus mode
[focus]
enabled = true
auto_schedule = true
sync_with_os = true # Sync with macOS/Linux focus modes

[[focus.schedules]]
mode = "DeepWork"
days = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"]
start_time = "09:00"
end_time = "12:00"
enabled = true

[[focus.allow_list]]
categories = ["SecurityAlert", "Build"]
priorities = ["Critical"]

# Build notifications
[builds]
enabled = true
github_actions = true
github_token = "${GITHUB_TOKEN}"
local_builds = true
watch_directories = [
    "~/projects/codebridge",
    "~/projects/coachly"
]

# Monitoring integrations
[monitoring]
datadog_enabled = false
datadog_api_key = "${DATADOG_API_KEY}"
datadog_app_key = "${DATADOG_APP_KEY}"

prometheus_enabled = false
prometheus_url = "http://localhost:9090"

# Communication integrations
[communication]
slack_enabled = false
slack_token = "${SLACK_TOKEN}"
slack_channels = ["#general", "#dev"]

discord_enabled = false
discord_token = "${DISCORD_TOKEN}"

email_enabled = false
email_server = "imap.gmail.com"
email_username = "${EMAIL_USERNAME}"
email_password = "${EMAIL_PASSWORD}"

# Sounds
[sounds]
enabled = true
sound_directory = "~/.config/codebridge/sounds"

[sounds.mappings]
"Build.Success" = "success.wav"
"Build.Failed" = "error.wav"
"SecurityAlert.Critical" = "alert.wav"
```

---

## 7. Platform-Specific Implementation

### 7.1 macOS Native App (SwiftUI + Rust)

```swift
// macOS notification display
import UserNotifications
import SwiftUI

@main
struct NotificationBridgeApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate

    var body: some Scene {
        MenuBarExtra("Notifications", systemImage: "bell.fill") {
            NotificationMenuView()
        }
        .menuBarExtraStyle(.window)

        Settings {
            SettingsView()
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate, UNUserNotificationCenterDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // Request notification permissions
        UNUserNotificationCenter.current().requestAuthorization(options: [.alert, .sound, .badge]) { granted, error in
            if granted {
                print("Notification permission granted")
            }
        }

        UNUserNotificationCenter.current().delegate = self

        // Start Rust backend
        start_notification_bridge()
    }

    // Handle notification actions
    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        let actionId = response.actionIdentifier
        let notificationId = response.notification.request.identifier

        // Forward to Rust backend
        handle_notification_action(notificationId, actionId)

        completionHandler()
    }
}

// Swift-Rust FFI (generated by swift-bridge)
func start_notification_bridge()
func handle_notification_action(_ notificationId: String, _ actionId: String)
func display_notification(_ notification: Notification)
```

### 7.2 Linux (Tauri + D-Bus)

```rust
// Linux implementation using D-Bus
use zbus::Connection;
use tauri::SystemTrayEvent;

#[tauri::command]
async fn display_notification(notif: Notification) -> Result<(), String> {
    let connection = Connection::session().await
        .map_err(|e| e.to_string())?;

    let proxy = NotificationsProxy::new(&connection).await
        .map_err(|e| e.to_string())?;

    // Convert actions to D-Bus format
    let actions: Vec<&str> = notif.actions.iter()
        .flat_map(|action| vec![action.id.as_str(), action.label.as_str()])
        .collect();

    let notification_id = proxy.notify(
        &notif.app_name,
        0, // replaces_id
        "", // app_icon
        &notif.title,
        &notif.body,
        &actions,
        HashMap::new(), // hints
        5000, // timeout (ms)
    ).await.map_err(|e| e.to_string())?;

    Ok(())
}

// Tauri system tray
fn create_system_tray() -> SystemTray {
    let tray_menu = SystemTrayMenu::new()
        .add_item(CustomMenuItem::new("notifications", "Notifications"))
        .add_item(CustomMenuItem::new("focus_mode", "Focus Mode"))
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(CustomMenuItem::new("settings", "Settings"))
        .add_item(CustomMenuItem::new("quit", "Quit"));

    SystemTray::new().with_menu(tray_menu)
}

fn handle_system_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => {
            match id.as_str() {
                "notifications" => {
                    // Show notifications window
                    let window = app.get_window("main").unwrap();
                    window.show().unwrap();
                }
                "focus_mode" => {
                    // Toggle focus mode
                    toggle_focus_mode();
                }
                "quit" => {
                    std::process::exit(0);
                }
                _ => {}
            }
        }
        _ => {}
    }
}
```

### 7.3 Mobile Companion Apps

```kotlin
// Android companion app (Kotlin)
package dev.codebridge.notifications

class NotificationService : FirebaseMessagingService() {
    override fun onMessageReceived(message: RemoteMessage) {
        val notification = parseNotification(message.data)

        displayNotification(notification)
    }

    private fun displayNotification(notif: Notification) {
        val builder = NotificationCompat.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_notification)
            .setContentTitle(notif.title)
            .setContentText(notif.body)
            .setPriority(notif.priority.toAndroidPriority())
            .setAutoCancel(true)

        // Add action buttons
        for (action in notif.actions) {
            val intent = createActionIntent(action)
            val pendingIntent = PendingIntent.getActivity(
                this, 0, intent, PendingIntent.FLAG_IMMUTABLE
            )

            builder.addAction(0, action.label, pendingIntent)
        }

        val notificationManager = getSystemService(NotificationManager::class.java)
        notificationManager.notify(notif.id.hashCode(), builder.build())
    }
}

// P2P connection to desktop (using libp2p via JNI)
class P2PNotificationSync {
    private external fun startP2pNode(): Long
    private external fun connectToPeer(nodeId: Long, peerId: String)
    private external fun sendNotification(nodeId: Long, notification: ByteArray)

    companion object {
        init {
            System.loadLibrary("codebridge_mobile")
        }
    }
}
```

```swift
// iOS companion app (Swift)
import UserNotifications
import SwiftUI

class NotificationManager: ObservableObject {
    @Published var notifications: [AppNotification] = []

    func requestPermissions() {
        UNUserNotificationCenter.current().requestAuthorization(
            options: [.alert, .badge, .sound]
        ) { granted, error in
            if granted {
                print("Notification permission granted")
            }
        }
    }

    func displayNotification(_ notification: AppNotification) {
        let content = UNMutableNotificationContent()
        content.title = notification.title
        content.body = notification.body
        content.sound = .default

        // Add actions
        var actions: [UNNotificationAction] = []
        for action in notification.actions {
            let notificationAction = UNNotificationAction(
                identifier: action.id,
                title: action.label,
                options: .foreground
            )
            actions.append(notificationAction)
        }

        let category = UNNotificationCategory(
            identifier: "NOTIFICATION_CATEGORY",
            actions: actions,
            intentIdentifiers: [],
            options: []
        )

        UNUserNotificationCenter.current().setNotificationCategories([category])
        content.categoryIdentifier = "NOTIFICATION_CATEGORY"

        let request = UNNotificationRequest(
            identifier: notification.id,
            content: content,
            trigger: nil
        )

        UNUserNotificationCenter.current().add(request)
    }
}

// P2P sync using Rust core via UniFFI
import CodeBridgeCore

class P2PSync: ObservableObject {
    private var bridge: NotificationBridge

    init() {
        self.bridge = NotificationBridge()
    }

    func connect(to peerId: String) {
        bridge.connectToPeer(peerId: peerId)
    }

    func sendNotification(_ notification: AppNotification) {
        bridge.sendNotification(notification: notification)
    }
}
```

---

## 8. Development Roadmap

### Phase 1: Foundation (Weeks 1-4)

**Deliverables:**
- [ ] Notification capture on macOS (NSUserNotificationCenter)
- [ ] Notification capture on Linux (D-Bus)
- [ ] Basic SQLite storage
- [ ] P2P sync using existing libp2p infrastructure
- [ ] Simple notification display

**Milestones:**
- Week 1: Platform API research and POC
- Week 2: Capture implementation for both platforms
- Week 3: P2P sync protocol design
- Week 4: Basic end-to-end demo

### Phase 2: Developer Integrations (Weeks 5-8)

**Deliverables:**
- [ ] GitHub Actions monitoring
- [ ] Local build monitoring (cargo, npm)
- [ ] Test failure detection
- [ ] PR review notifications
- [ ] Basic action buttons

**Milestones:**
- Week 5: GitHub integration
- Week 6: Local build monitoring
- Week 7: Test runner integration
- Week 8: Action button framework

### Phase 3: Smart Features (Weeks 9-12)

**Deliverables:**
- [ ] AI priority sorting (ONNX model)
- [ ] Notification batching
- [ ] Focus mode
- [ ] Custom sounds
- [ ] DND sync

**Milestones:**
- Week 9: ML model training and integration
- Week 10: Batching engine
- Week 11: Focus mode implementation
- Week 12: Sound system and polish

### Phase 4: Communication & Monitoring (Weeks 13-16)

**Deliverables:**
- [ ] Slack integration
- [ ] Discord integration
- [ ] Email monitoring
- [ ] Calendar reminders
- [ ] Datadog/Prometheus monitoring

**Milestones:**
- Week 13: Slack and Discord
- Week 14: Email and calendar
- Week 15: Monitoring integrations
- Week 16: Testing and refinement

### Phase 5: Mobile Apps (Weeks 17-20)

**Deliverables:**
- [ ] iOS companion app
- [ ] Android companion app
- [ ] Push notification relay
- [ ] Mobile action handling
- [ ] Watch app (iOS, Wear OS)

**Milestones:**
- Week 17: iOS app
- Week 18: Android app
- Week 19: Watch apps
- Week 20: Final polish and release

---

## 9. Technology Stack Summary

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Core Language** | Rust | Performance, safety, cross-platform |
| **P2P Network** | libp2p (existing) | Reuse Code Bridge infrastructure |
| **Desktop UI (macOS)** | SwiftUI + Rust | Native UX, performance |
| **Desktop UI (Linux)** | Tauri + Rust | Cross-platform, lightweight |
| **Mobile (iOS)** | SwiftUI + UniFFI | Native, shared Rust core |
| **Mobile (Android)** | Kotlin + JNI | Native, shared Rust core |
| **Database** | SQLite + FTS5 | Fast full-text search |
| **ML Framework** | ONNX Runtime | Cross-platform inference |
| **Serialization** | bincode | Fast, compact |
| **Async Runtime** | tokio | Standard Rust async |

---

## 10. Security and Privacy

### Data Handling

1. **Local-First**: All notifications stored locally by default
2. **E2E Encryption**: P2P sync uses ChaCha20-Poly1305
3. **No Cloud**: No third-party servers (except opt-in integrations)
4. **User Control**: Fine-grained filter controls

### Sensitive Data

1. **Credential Detection**: Scan notification content for API keys, passwords
2. **Auto-Redact**: Option to redact sensitive patterns before sync
3. **App Blacklist**: Exclude password managers, banking apps
4. **Manual Review**: Require confirmation before syncing certain categories

### Permissions

1. **Minimal Permissions**: Only request what's needed
2. **Granular Control**: Enable/disable each integration separately
3. **Audit Log**: Track all notification sync events
4. **Revocable Access**: Easy to disconnect devices

---

## 11. Competitive Analysis

### vs. KDE Connect / GSConnect

| Feature | Code Bridge Notifications | KDE Connect |
|---------|--------------------------|-------------|
| **Platform Support** | macOS, Linux, iOS, Android | Linux, Android primarily |
| **Developer Focus** | Build/CI/monitoring integration | General notifications |
| **Smart Features** | AI priority, batching, focus | Basic sync |
| **P2P Architecture** | libp2p (robust) | Custom protocol |
| **Action Buttons** | Full API integration | Limited |
| **Offline Support** | Full CRDT sync | Basic |

### vs. Pushbullet

| Feature | Code Bridge Notifications | Pushbullet |
|---------|--------------------------|------------|
| **Privacy** | Local-first, no cloud | Cloud-based |
| **Cost** | Free, open source | Freemium |
| **Developer Tools** | Build/CI/monitoring | Consumer focus |
| **Customization** | Full control | Limited |
| **Data Ownership** | User owns all data | Company servers |

### vs. Notification Center Only

| Feature | Code Bridge Notifications | Native Notification Center |
|---------|--------------------------|----------------------------|
| **Cross-Device** | Full P2P sync | None |
| **Build Integration** | Native support | Manual setup |
| **Smart Filtering** | AI-powered | Basic |
| **History Search** | Full-text search | Time-limited |
| **Action Buttons** | API integration | App-dependent |

---

## 12. Success Metrics

### Adoption Metrics
- Active devices per user
- Daily notification sync volume
- Integration usage (GitHub, Slack, etc.)

### Performance Metrics
- Notification sync latency (<500ms target)
- Battery impact (<2% target on mobile)
- Network usage (minimal bandwidth)

### User Satisfaction
- Notification relevance score (user feedback)
- Focus mode effectiveness (fewer interruptions)
- Time saved via action buttons

### Technical Metrics
- P2P connection success rate (>90%)
- ML model accuracy for priority (>80%)
- Crash-free rate (>99.5%)

---

## 13. Future Enhancements

### Year 1+
- **Voice Notifications**: Text-to-speech for critical alerts
- **AR Glasses**: Notification overlay for smart glasses
- **AI Summarization**: Daily digest with AI summary
- **Team Notifications**: Shared notification channels
- **API/Webhook**: Programmable notification endpoints

### Research Areas
- **Federated Learning**: Privacy-preserving priority model training
- **Blockchain**: Immutable notification audit trail (optional)
- **Quantum-Safe**: Post-quantum encryption for long-term privacy

---

## 14. Resources and References

### System Notification APIs
- [macOS UserNotifications Framework](https://developer.apple.com/documentation/usernotifications)
- [Linux D-Bus Notifications Specification](https://specifications.freedesktop.org/notification-spec/latest/)
- [Windows WinRT Notifications](https://docs.microsoft.com/en-us/windows/apps/design/shell/tiles-and-notifications/)

### Developer Tools
- [GitHub REST API - Notifications](https://docs.github.com/en/rest/activity/notifications)
- [Slack Events API](https://api.slack.com/events-api)
- [Discord Gateway](https://discord.com/developers/docs/topics/gateway)
- [Datadog API](https://docs.datadoghq.com/api/)

### P2P Technologies
- [libp2p Documentation](https://docs.libp2p.io/)
- [libp2p Gossipsub](https://github.com/libp2p/specs/blob/master/pubsub/gossipsub/README.md)

### Machine Learning
- [ONNX Runtime](https://onnxruntime.ai/)
- [PyTorch for Rust](https://github.com/LaurentMazare/tch-rs)

### Cross-Platform
- [Tauri Documentation](https://tauri.app/)
- [UniFFI Guide](https://mozilla.github.io/uniffi-rs/)

---

## Conclusion

A cross-platform notification bridge for developers addresses a critical gap in the developer tooling ecosystem. By combining:

1. **System-level notification capture** across all platforms
2. **P2P synchronization** using robust libp2p infrastructure
3. **Developer-focused integrations** (GitHub, CI/CD, monitoring)
4. **Smart features** (AI priority, batching, focus mode)
5. **Action buttons** for one-click workflows

This creates a compelling productivity tool that:
- **Reduces context switching** (act on notifications from any device)
- **Eliminates notification fatigue** (smart filtering and batching)
- **Enhances focus** (intelligent DND integration)
- **Improves response time** (on-call alerts to mobile)
- **Maintains privacy** (local-first, P2P architecture)

The integration with existing Code Bridge infrastructure (libp2p, P2P sync, CRDT) makes this a natural extension that leverages already-built components while adding significant value for developers working across multiple devices.

**Estimated Development Time**: 20 weeks (5 months) for full implementation
**Risk Level**: Medium - Relies on platform APIs and ML, but core P2P stack is proven
**Unique Value**: Only P2P notification bridge designed specifically for developers

---

**Document Version:** 1.0
**Research Compiled By:** Claude Code (Sonnet 4.5)
**Date:** December 20, 2025
**Status:** Research Complete - Ready for Implementation Planning
