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
