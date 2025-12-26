// Bridge Manager - Main ViewModel
// Connects SwiftUI to the Rust core library

import Foundation
import SwiftUI

@MainActor
class BridgeManager: ObservableObject {
    // Published state
    @Published var deviceName: String = "Mac mini"
    @Published var deviceId: String = ""
    @Published var peerCount: Int = 0
    @Published var fileCount: Int = 0
    @Published var isSyncing: Bool = false
    @Published var isConnected: Bool = false

    @Published var peers: [Peer] = []
    @Published var files: [TrackedFile] = []
    @Published var screenshots: [Screenshot] = []
    @Published var recentActivity: [String] = []

    // Clipboard
    @Published var clipboardHistory: [ClipboardEntry] = []

    // Notifications
    @Published var notifications: [AppNotification] = []

    // Terminal
    @Published var terminalRecordings: [TerminalRecording] = []
    @Published var commandHistory: [CommandHistoryEntry] = []
    @Published var recentDirectories: [String] = []
    @Published var shellAliases: [ShellAlias] = []
    @Published var shellFunctions: [ShellFunction] = []
    @Published var environmentVars: [EnvironmentVar] = []

    // Configuration
    @Published var watchPaths: [String] = []
    @Published var ignorePatterns: [String] = [".git", "node_modules", "target"]

    // MARK: - NAS/Backup Properties
    @Published var nasConnections: [QNAPConnection] = []
    @Published var nasConnectionStatus: NASConnectionStatus = .disconnected

    // Services
    let qnapService: QNAPService
    var backupService: BackupService!
    var screenshotService: ScreenshotService!
    var hotkeyManager: HotkeyManager!

    // Persistence keys
    private let nasConnectionsKey = "CodeBridge.NASConnections"

    init() {
        // Initialize services
        self.qnapService = QNAPService()

        // Load persisted NAS connections first
        if let data = UserDefaults.standard.data(forKey: nasConnectionsKey),
           let connections = try? JSONDecoder().decode([QNAPConnection].self, from: data) {
            nasConnections = connections
        }

        // Initialize backup service with connections closure
        self.backupService = BackupService(qnapService: qnapService, connections: { [weak self] in
            self?.nasConnections ?? []
        })

        // Initialize screenshot service
        self.screenshotService = ScreenshotService(qnapService: qnapService, connections: { [weak self] in
            self?.nasConnections ?? []
        })

        // Initialize hotkey manager
        self.hotkeyManager = HotkeyManager()
        self.hotkeyManager.onScreenshotHotkey = { [weak self] mode in
            Task { @MainActor in
                await self?.screenshotService.captureScreenshot(mode: mode)
            }
        }

        // Initialize bridge and auto-connect to NAS
        Task {
            await initializeBridge()
            await autoConnectToNAS()
        }
    }

    /// Auto-connect to the default NAS connection on startup
    private func autoConnectToNAS() async {
        guard let defaultConnection = nasConnections.first(where: { $0.isDefault }) ?? nasConnections.first else {
            // No NAS connections configured
            return
        }

        // Only auto-connect if we have valid credentials
        guard !defaultConnection.host.isEmpty,
              !defaultConnection.username.isEmpty else {
            return
        }

        await connectToNAS(defaultConnection)
    }

    func initializeBridge() async {
        // TODO: Initialize Rust bridge via FFI
        // For now, use mock data

        deviceName = Host.current().localizedName ?? "Unknown"
        deviceId = UUID().uuidString.prefix(8).uppercased()
        peerCount = 0
        fileCount = 0
        isConnected = true

        // Add sample activity
        recentActivity = [
            "Bridge initialized",
            "Listening for peers...",
        ]

        // Load sample data for new features
        loadSampleData()
    }

    func sync() {
        isSyncing = true
        recentActivity.insert("Sync started", at: 0)

        // Simulate sync
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            await MainActor.run {
                self.isSyncing = false
                self.recentActivity.insert("Sync complete", at: 0)
            }
        }
    }

    func addFiles() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = true

        panel.begin { response in
            if response == .OK {
                for url in panel.urls {
                    self.addPath(url.path)
                }
            }
        }
    }

    func addPath(_ path: String) {
        // TODO: Add to Rust core
        recentActivity.insert("Added: \(path)", at: 0)
        fileCount += 1
    }

    func discoverPeers() {
        recentActivity.insert("Discovering peers...", at: 0)

        // Simulate discovery
        Task {
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            await MainActor.run {
                self.recentActivity.insert("Discovery complete", at: 0)
            }
        }
    }

    func removePeer(_ peer: Peer) {
        peers.removeAll { $0.id == peer.id }
        peerCount = peers.count
        recentActivity.insert("Removed peer: \(peer.name)", at: 0)
    }

    func shareFile(_ file: TrackedFile) {
        recentActivity.insert("Sharing: \(file.name)", at: 0)
    }

    // MARK: - Clipboard Methods

    func clearClipboardHistory() {
        clipboardHistory.removeAll()
        recentActivity.insert("Cleared clipboard history", at: 0)
    }

    func toggleClipboardFavorite(_ id: String) {
        if let index = clipboardHistory.firstIndex(where: { $0.id == id }) {
            clipboardHistory[index].isFavorite.toggle()
        }
    }

    // MARK: - Notification Methods

    func markAllNotificationsRead() {
        for i in notifications.indices {
            notifications[i].isRead = true
        }
    }

    func markNotificationRead(_ id: String) {
        if let index = notifications.firstIndex(where: { $0.id == id }) {
            notifications[index].isRead = true
        }
    }

    func dismissNotification(_ id: String) {
        notifications.removeAll { $0.id == id }
    }

    func executeNotificationAction(_ notificationId: String, actionId: String) {
        recentActivity.insert("Executed action: \(actionId)", at: 0)
        markNotificationRead(notificationId)
    }

    // MARK: - Terminal Methods

    func startTerminalRecording() {
        recentActivity.insert("Started terminal recording", at: 0)
    }

    func stopTerminalRecording() {
        let recording = TerminalRecording(
            id: UUID().uuidString,
            title: "Recording \(terminalRecordings.count + 1)",
            shell: "zsh",
            recordedAt: Date(),
            duration: Double.random(in: 30...300),
            outputPreview: "$ ls -la\ntotal 0\ndrwxr-xr-x  5 user  staff  160 Jan  1 12:00 .\ndrwxr-xr-x  3 user  staff   96 Jan  1 12:00 ..\n"
        )
        terminalRecordings.insert(recording, at: 0)
        recentActivity.insert("Stopped terminal recording", at: 0)
    }

    func syncCommandHistory() {
        recentActivity.insert("Syncing command history...", at: 0)
        Task {
            try? await Task.sleep(nanoseconds: 1_000_000_000)
            await MainActor.run {
                self.recentActivity.insert("Command history synced", at: 0)
            }
        }
    }

    // MARK: - Sample Data

    func loadSampleData() {
        // Sample clipboard entries
        clipboardHistory = [
            ClipboardEntry(
                id: "clip1",
                contentType: "code",
                preview: "func main() { println!(\"Hello\"); }",
                timestamp: Date().addingTimeInterval(-3600),
                sourceDevice: deviceId,
                isFavorite: true,
                isPinned: false,
                textContent: "func main() {\n    println!(\"Hello, World!\");\n}",
                imageData: nil,
                language: "rust",
                appSource: "VSCode",
                dataSize: 45
            ),
            ClipboardEntry(
                id: "clip2",
                contentType: "url",
                preview: "https://github.com/anthropics/claude-code",
                timestamp: Date().addingTimeInterval(-7200),
                sourceDevice: "Ubuntu-Dev",
                isFavorite: false,
                isPinned: false,
                textContent: "https://github.com/anthropics/claude-code",
                imageData: nil,
                language: nil,
                appSource: "Firefox",
                dataSize: 42
            ),
            ClipboardEntry(
                id: "clip3",
                contentType: "text",
                preview: "Meeting notes from standup...",
                timestamp: Date().addingTimeInterval(-86400),
                sourceDevice: deviceId,
                isFavorite: false,
                isPinned: false,
                textContent: "Meeting notes from standup:\n- Discussed project timeline\n- Reviewed sprint goals\n- Assigned new tasks",
                imageData: nil,
                language: nil,
                appSource: "Notes",
                dataSize: 120
            )
        ]

        // Sample notifications
        notifications = [
            AppNotification(
                id: "notif1",
                appName: "GitHub Actions",
                title: "Build Succeeded",
                subtitle: "main branch",
                body: "CI/CD pipeline completed successfully for commit abc123",
                timestamp: Date().addingTimeInterval(-1800),
                priority: .normal,
                category: .build,
                sourceDevice: deviceId,
                actions: [
                    NotificationAction(id: "view", label: "View"),
                    NotificationAction(id: "deploy", label: "Deploy")
                ],
                isRead: false
            ),
            AppNotification(
                id: "notif2",
                appName: "GitHub",
                title: "New PR Review",
                subtitle: nil,
                body: "Alex commented on your pull request #42",
                timestamp: Date().addingTimeInterval(-3600),
                priority: .high,
                category: .pullRequest,
                sourceDevice: "Ubuntu-Dev",
                actions: [
                    NotificationAction(id: "view", label: "View PR")
                ],
                isRead: false
            ),
            AppNotification(
                id: "notif3",
                appName: "Security Scanner",
                title: "Vulnerability Found",
                subtitle: "High Severity",
                body: "Detected outdated dependency with known CVE",
                timestamp: Date().addingTimeInterval(-7200),
                priority: .critical,
                category: .security,
                sourceDevice: deviceId,
                actions: [
                    NotificationAction(id: "fix", label: "Fix Now"),
                    NotificationAction(id: "ignore", label: "Ignore")
                ],
                isRead: true
            )
        ]

        // Sample terminal recordings
        terminalRecordings = [
            TerminalRecording(
                id: "rec1",
                title: "Project Setup",
                shell: "zsh",
                recordedAt: Date().addingTimeInterval(-86400),
                duration: 245,
                outputPreview: "$ cargo new my-project\n     Created binary (application) `my-project` package\n$ cd my-project\n$ cargo build\n   Compiling my-project v0.1.0\n    Finished dev [unoptimized + debuginfo] target(s)"
            ),
            TerminalRecording(
                id: "rec2",
                title: "Debug Session",
                shell: "bash",
                recordedAt: Date().addingTimeInterval(-172800),
                duration: 180,
                outputPreview: "$ npm run test\n\n> test\n> jest\n\nPASS  src/utils.test.ts\n  ✓ should format date correctly (5 ms)\n  ✓ should parse JSON safely (3 ms)"
            )
        ]

        // Sample command history
        commandHistory = [
            CommandHistoryEntry(
                id: "cmd1",
                command: "cargo build --release",
                workingDir: "~/Projects/code-bridge",
                exitCode: 0,
                duration: 45.2,
                timestamp: Date().addingTimeInterval(-300),
                sourceDevice: deviceId
            ),
            CommandHistoryEntry(
                id: "cmd2",
                command: "git push origin main",
                workingDir: "~/Projects/code-bridge",
                exitCode: 0,
                duration: 2.1,
                timestamp: Date().addingTimeInterval(-600),
                sourceDevice: deviceId
            ),
            CommandHistoryEntry(
                id: "cmd3",
                command: "npm install",
                workingDir: "~/Projects/frontend",
                exitCode: 0,
                duration: 12.5,
                timestamp: Date().addingTimeInterval(-900),
                sourceDevice: "Ubuntu-Dev"
            )
        ]

        // Sample directories
        recentDirectories = [
            "~/Projects/code-bridge",
            "~/Projects/frontend",
            "~/Documents"
        ]

        // Sample aliases
        shellAliases = [
            ShellAlias(name: "ll", command: "ls -la"),
            ShellAlias(name: "gs", command: "git status"),
            ShellAlias(name: "gp", command: "git push"),
            ShellAlias(name: "cb", command: "cargo build")
        ]

        // Sample functions
        shellFunctions = [
            ShellFunction(name: "mkcd", body: "mkdir -p $1 && cd $1"),
            ShellFunction(name: "gitlog", body: "git log --oneline -n ${1:-10}")
        ]

        // Sample environment vars
        environmentVars = [
            EnvironmentVar(name: "PATH", value: "/usr/local/bin:/usr/bin:/bin", isSensitive: false),
            EnvironmentVar(name: "EDITOR", value: "nvim", isSensitive: false),
            EnvironmentVar(name: "GITHUB_TOKEN", value: "••••••••", isSensitive: true)
        ]
    }

    // MARK: - NAS Connection Management

    /// Load NAS connections from persistence
    private func loadNASConnections() {
        if let data = UserDefaults.standard.data(forKey: nasConnectionsKey),
           let connections = try? JSONDecoder().decode([QNAPConnection].self, from: data) {
            nasConnections = connections
        }
    }

    /// Save NAS connections to persistence
    private func saveNASConnections() {
        if let data = try? JSONEncoder().encode(nasConnections) {
            UserDefaults.standard.set(data, forKey: nasConnectionsKey)
        }
    }

    /// Add a new NAS connection
    func addNASConnection(_ connection: QNAPConnection) {
        var newConnection = connection

        // If this is the first connection, make it default
        if nasConnections.isEmpty {
            newConnection.isDefault = true
        }

        nasConnections.append(newConnection)
        saveNASConnections()
        recentActivity.insert("Added NAS connection: \(connection.name)", at: 0)
    }

    /// Update an existing NAS connection
    func updateNASConnection(_ connection: QNAPConnection) {
        if let index = nasConnections.firstIndex(where: { $0.id == connection.id }) {
            nasConnections[index] = connection
            saveNASConnections()
            recentActivity.insert("Updated NAS connection: \(connection.name)", at: 0)
        }
    }

    /// Remove a NAS connection
    func removeNASConnection(_ connectionId: String) {
        if let index = nasConnections.firstIndex(where: { $0.id == connectionId }) {
            let name = nasConnections[index].name
            nasConnections.remove(at: index)

            // If we removed the default, set a new default
            if !nasConnections.isEmpty && !nasConnections.contains(where: { $0.isDefault }) {
                nasConnections[0].isDefault = true
            }

            saveNASConnections()
            recentActivity.insert("Removed NAS connection: \(name)", at: 0)
        }
    }

    /// Set a connection as the default
    func setDefaultNASConnection(_ connectionId: String) {
        for i in nasConnections.indices {
            nasConnections[i].isDefault = (nasConnections[i].id == connectionId)
        }
        saveNASConnections()
    }

    /// Connect to a NAS
    func connectToNAS(_ connection: QNAPConnection) async {
        nasConnectionStatus = .connecting
        recentActivity.insert("Connecting to \(connection.name)...", at: 0)

        do {
            try await qnapService.connect(using: connection)
            nasConnectionStatus = .connected
            recentActivity.insert("Connected to \(connection.name)", at: 0)
        } catch {
            nasConnectionStatus = .error
            recentActivity.insert("Failed to connect: \(error.localizedDescription)", at: 0)
        }
    }

    /// Disconnect from NAS
    func disconnectFromNAS() async {
        await qnapService.disconnect()
        nasConnectionStatus = .disconnected
        recentActivity.insert("Disconnected from NAS", at: 0)
    }

    /// Get the default NAS connection
    var defaultNASConnection: QNAPConnection? {
        nasConnections.first(where: { $0.isDefault })
    }
}
