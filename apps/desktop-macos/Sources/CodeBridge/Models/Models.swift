// Coachly Code Bridge - Data Models
// Consolidated model definitions for the macOS app

import SwiftUI

// MARK: - Clipboard Models

struct ClipboardEntry: Identifiable, Hashable {
    let id: String
    let contentType: String
    let preview: String?
    let timestamp: Date
    let sourceDevice: String
    var isFavorite: Bool
    var isPinned: Bool
    let textContent: String?
    let imageData: Data?
    let language: String?
    let appSource: String?
    let dataSize: Int

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }

    static func == (lhs: ClipboardEntry, rhs: ClipboardEntry) -> Bool {
        lhs.id == rhs.id
    }
}

// MARK: - Notification Models

enum NotificationCategory: String, CaseIterable {
    case build = "build"
    case test = "test"
    case pullRequest = "pr"
    case security = "security"
    case system = "system"
    case chat = "chat"

    var displayName: String {
        switch self {
        case .build: return "Builds"
        case .test: return "Tests"
        case .pullRequest: return "PRs"
        case .security: return "Security"
        case .system: return "System"
        case .chat: return "Chat"
        }
    }

    var icon: String {
        switch self {
        case .build: return "hammer"
        case .test: return "testtube.2"
        case .pullRequest: return "arrow.triangle.pull"
        case .security: return "shield"
        case .system: return "gear"
        case .chat: return "bubble.left"
        }
    }

    var color: Color {
        switch self {
        case .build: return .blue
        case .test: return .purple
        case .pullRequest: return .green
        case .security: return .red
        case .system: return .gray
        case .chat: return .orange
        }
    }
}

enum NotificationPriority: String {
    case low, normal, high, critical
}

struct NotificationAction: Identifiable {
    let id: String
    let label: String
}

struct AppNotification: Identifiable {
    let id: String
    let appName: String
    let title: String
    let subtitle: String?
    let body: String
    let timestamp: Date
    let priority: NotificationPriority
    let category: NotificationCategory
    let sourceDevice: String
    let actions: [NotificationAction]
    var isRead: Bool
}

// MARK: - Terminal Models

struct TerminalRecording: Identifiable, Hashable {
    let id: String
    let title: String
    let shell: String
    let recordedAt: Date
    let duration: TimeInterval
    let outputPreview: String

    func hash(into hasher: inout Hasher) {
        hasher.combine(id)
    }

    static func == (lhs: TerminalRecording, rhs: TerminalRecording) -> Bool {
        lhs.id == rhs.id
    }
}

struct CommandHistoryEntry: Identifiable {
    let id: String
    let command: String
    let workingDir: String
    let exitCode: Int?
    let duration: Double?
    let timestamp: Date
    let sourceDevice: String
}

struct ShellAlias: Identifiable {
    var id: String { name }
    let name: String
    let command: String
}

struct ShellFunction: Identifiable {
    var id: String { name }
    let name: String
    let body: String
}

struct EnvironmentVar: Identifiable {
    var id: String { name }
    let name: String
    let value: String
    let isSensitive: Bool
}

// MARK: - Core Models

struct Peer: Identifiable, Hashable {
    let id: String
    let name: String
    let addresses: [String]
    var isConnected: Bool
    var lastSeen: Date
}

struct TrackedFile: Identifiable, Hashable {
    let id: String
    let name: String
    let path: String
    let size: Int64
    let hash: String
    let mimeType: String?
    let modifiedAt: Date
}

struct Screenshot: Identifiable, Hashable {
    let id: String
    let name: String
    let path: String
    let size: Int64
    let capturedAt: Date
    let ocrText: String?
}

// MARK: - QNAP NAS Models

/// Protocol options for connecting to QNAP NAS
enum QNAPProtocol: String, CaseIterable, Identifiable {
    case smb3 = "SMB3"
    case webdav = "WebDAV"
    case nfs = "NFS"
    case api = "File Station API"

    var id: String { rawValue }

    var description: String {
        switch self {
        case .smb3: return "SMB3 - Best for Windows/macOS/Linux"
        case .webdav: return "WebDAV - HTTP-based, firewall-friendly"
        case .nfs: return "NFS - Best for Linux systems"
        case .api: return "File Station API - Direct API access"
        }
    }

    var icon: String {
        switch self {
        case .smb3: return "externaldrive.connected.to.line.below"
        case .webdav: return "globe"
        case .nfs: return "server.rack"
        case .api: return "arrow.left.arrow.right"
        }
    }

    var defaultPort: Int {
        switch self {
        case .smb3: return 445
        case .webdav: return 443
        case .nfs: return 2049
        case .api: return 8080
        }
    }
}

/// Connection configuration for QNAP NAS
struct QNAPConnection: Identifiable, Codable {
    let id: String
    var name: String
    var host: String
    var port: Int
    var username: String
    var password: String  // In production, use Keychain
    var sharePath: String
    var protocolType: String  // Stored as string for Codable
    var useSSL: Bool
    var isDefault: Bool

    var qnapProtocol: QNAPProtocol {
        get { QNAPProtocol(rawValue: protocolType) ?? .smb3 }
        set { protocolType = newValue.rawValue }
    }

    init(
        id: String = UUID().uuidString,
        name: String = "QNAP NAS",
        host: String = "",
        port: Int = 445,
        username: String = "",
        password: String = "",
        sharePath: String = "/Development",
        protocolType: QNAPProtocol = .smb3,
        useSSL: Bool = true,
        isDefault: Bool = false
    ) {
        self.id = id
        self.name = name
        self.host = host
        self.port = port
        self.username = username
        self.password = password
        self.sharePath = sharePath
        self.protocolType = protocolType.rawValue
        self.useSSL = useSSL
        self.isDefault = isDefault
    }
}

/// Connection status for NAS
enum NASConnectionStatus: String {
    case disconnected = "Disconnected"
    case connecting = "Connecting..."
    case connected = "Connected"
    case error = "Error"

    var color: Color {
        switch self {
        case .disconnected: return .secondary
        case .connecting: return .orange
        case .connected: return .green
        case .error: return .red
        }
    }

    var icon: String {
        switch self {
        case .disconnected: return "circle"
        case .connecting: return "arrow.triangle.2.circlepath"
        case .connected: return "checkmark.circle.fill"
        case .error: return "exclamationmark.triangle.fill"
        }
    }
}

/// Backup schedule options
enum BackupSchedule: String, CaseIterable, Identifiable {
    case manual = "Manual"
    case hourly = "Hourly"
    case daily = "Daily"
    case weekly = "Weekly"

    var id: String { rawValue }

    var description: String {
        switch self {
        case .manual: return "Run backups manually"
        case .hourly: return "Every hour"
        case .daily: return "Once per day"
        case .weekly: return "Once per week"
        }
    }
}

/// Status of a backup job
enum BackupStatus: String, Codable {
    case idle = "Idle"
    case running = "Running"
    case completed = "Completed"
    case failed = "Failed"
    case paused = "Paused"

    var color: Color {
        switch self {
        case .idle: return .secondary
        case .running: return .blue
        case .completed: return .green
        case .failed: return .red
        case .paused: return .orange
        }
    }

    var icon: String {
        switch self {
        case .idle: return "clock"
        case .running: return "arrow.triangle.2.circlepath"
        case .completed: return "checkmark.circle.fill"
        case .failed: return "xmark.circle.fill"
        case .paused: return "pause.circle.fill"
        }
    }
}

/// A configured backup job
struct BackupJob: Identifiable, Codable {
    let id: String
    var name: String
    var sourcePaths: [String]
    var destinationConnectionId: String
    var destinationPath: String
    var schedule: String  // Stored as string for Codable
    var isEnabled: Bool
    var excludePatterns: [String]
    var lastRunAt: Date?
    var lastStatus: BackupStatus
    var retainVersions: Int
    var compressBackups: Bool

    var backupSchedule: BackupSchedule {
        get { BackupSchedule(rawValue: schedule) ?? .manual }
        set { schedule = newValue.rawValue }
    }

    init(
        id: String = UUID().uuidString,
        name: String = "New Backup",
        sourcePaths: [String] = [],
        destinationConnectionId: String = "",
        destinationPath: String = "/Backups/CodeBridge",
        schedule: BackupSchedule = .daily,
        isEnabled: Bool = true,
        excludePatterns: [String] = [".git", "node_modules", "target", ".build", "*.log"],
        lastRunAt: Date? = nil,
        lastStatus: BackupStatus = .idle,
        retainVersions: Int = 5,
        compressBackups: Bool = true
    ) {
        self.id = id
        self.name = name
        self.sourcePaths = sourcePaths
        self.destinationConnectionId = destinationConnectionId
        self.destinationPath = destinationPath
        self.schedule = schedule.rawValue
        self.isEnabled = isEnabled
        self.excludePatterns = excludePatterns
        self.lastRunAt = lastRunAt
        self.lastStatus = lastStatus
        self.retainVersions = retainVersions
        self.compressBackups = compressBackups
    }
}

/// Record of a completed backup
struct BackupHistoryEntry: Identifiable, Codable {
    let id: String
    let jobId: String
    let jobName: String
    let startedAt: Date
    let completedAt: Date
    let status: BackupStatus
    let filesBackedUp: Int
    let totalSize: Int64
    let errorMessage: String?

    var duration: TimeInterval {
        completedAt.timeIntervalSince(startedAt)
    }

    var formattedDuration: String {
        let formatter = DateComponentsFormatter()
        formatter.allowedUnits = [.hour, .minute, .second]
        formatter.unitsStyle = .abbreviated
        return formatter.string(from: duration) ?? "N/A"
    }

    var formattedSize: String {
        ByteCountFormatter.string(fromByteCount: totalSize, countStyle: .file)
    }
}

/// Statistics for backup operations
struct BackupStats: Codable {
    var totalBackups: Int
    var successfulBackups: Int
    var failedBackups: Int
    var totalDataBackedUp: Int64
    var lastBackupDate: Date?

    var successRate: Double {
        guard totalBackups > 0 else { return 0 }
        return Double(successfulBackups) / Double(totalBackups) * 100
    }

    init(
        totalBackups: Int = 0,
        successfulBackups: Int = 0,
        failedBackups: Int = 0,
        totalDataBackedUp: Int64 = 0,
        lastBackupDate: Date? = nil
    ) {
        self.totalBackups = totalBackups
        self.successfulBackups = successfulBackups
        self.failedBackups = failedBackups
        self.totalDataBackedUp = totalDataBackedUp
        self.lastBackupDate = lastBackupDate
    }
}
