// Notifications View

import SwiftUI

struct NotificationsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedCategory: NotificationCategory? = nil
    @State private var showingSettings = false

    var filteredNotifications: [AppNotification] {
        if let category = selectedCategory {
            return bridgeManager.notifications.filter { $0.category == category }
        }
        return bridgeManager.notifications
    }

    var unreadCount: Int {
        bridgeManager.notifications.filter { !$0.isRead }.count
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                // Category filter
                Picker("Category", selection: $selectedCategory) {
                    Text("All").tag(nil as NotificationCategory?)
                    ForEach(NotificationCategory.allCases, id: \.self) { category in
                        Label(category.displayName, systemImage: category.icon)
                            .tag(category as NotificationCategory?)
                    }
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 500)

                Spacer()

                if unreadCount > 0 {
                    Button("Mark All Read") {
                        bridgeManager.markAllNotificationsRead()
                    }
                    .buttonStyle(.bordered)
                }

                Button(action: { showingSettings = true }) {
                    Image(systemName: "gear")
                }
                .buttonStyle(.bordered)
            }
            .padding()

            Divider()

            if filteredNotifications.isEmpty {
                EmptyNotificationsView()
            } else {
                List {
                    ForEach(filteredNotifications) { notification in
                        NotificationRow(notification: notification)
                            .listRowSeparator(.visible)
                    }
                    .onDelete { indexSet in
                        for index in indexSet {
                            bridgeManager.dismissNotification(filteredNotifications[index].id)
                        }
                    }
                }
            }
        }
        .navigationTitle("Notifications")
        .sheet(isPresented: $showingSettings) {
            NotificationSettingsView()
        }
    }
}

struct EmptyNotificationsView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "bell.slash")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            Text("No notifications")
                .font(.title2)
            Text("You're all caught up!")
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct NotificationRow: View {
    let notification: AppNotification
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var isHovering = false

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            // Priority indicator
            Circle()
                .fill(notification.isRead ? Color.clear : priorityColor)
                .frame(width: 8, height: 8)
                .padding(.top, 6)

            // Icon
            ZStack {
                Circle()
                    .fill(notification.category.color.opacity(0.2))
                    .frame(width: 40, height: 40)

                Image(systemName: notification.category.icon)
                    .foregroundColor(notification.category.color)
            }

            // Content
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(notification.appName)
                        .font(.caption)
                        .foregroundColor(.secondary)

                    Spacer()

                    Text(notification.timestamp, style: .relative)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Text(notification.title)
                    .font(.headline)

                if let subtitle = notification.subtitle {
                    Text(subtitle)
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }

                Text(notification.body)
                    .font(.body)
                    .lineLimit(2)

                // Actions
                if !notification.actions.isEmpty && isHovering {
                    HStack {
                        ForEach(notification.actions, id: \.id) { action in
                            Button(action.label) {
                                bridgeManager.executeNotificationAction(notification.id, actionId: action.id)
                            }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        }
                    }
                    .padding(.top, 4)
                }
            }

            // Source device
            if notification.sourceDevice != bridgeManager.deviceId {
                Image(systemName: "laptopcomputer")
                    .foregroundColor(.blue)
                    .font(.caption)
            }
        }
        .padding(.vertical, 8)
        .contentShape(Rectangle())
        .onHover { hovering in
            isHovering = hovering
        }
        .onTapGesture {
            bridgeManager.markNotificationRead(notification.id)
        }
        .opacity(notification.isRead ? 0.7 : 1.0)
    }

    var priorityColor: Color {
        switch notification.priority {
        case .critical: return .red
        case .high: return .orange
        case .normal: return .blue
        case .low: return .gray
        }
    }
}

struct NotificationSettingsView: View {
    @Environment(\.dismiss) var dismiss
    @State private var enableBuildNotifications = true
    @State private var enableTestNotifications = true
    @State private var enablePRNotifications = true
    @State private var enableSecurityAlerts = true
    @State private var quietHoursEnabled = false
    @State private var quietStart = Date()
    @State private var quietEnd = Date()

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text("Notification Settings")
                    .font(.headline)
                Spacer()
                Button("Done") { dismiss() }
                    .buttonStyle(.borderedProminent)
            }
            .padding()

            Form {
                Section("Developer Notifications") {
                    Toggle("Build Status", isOn: $enableBuildNotifications)
                    Toggle("Test Results", isOn: $enableTestNotifications)
                    Toggle("Pull Requests", isOn: $enablePRNotifications)
                    Toggle("Security Alerts", isOn: $enableSecurityAlerts)
                }

                Section("Quiet Hours") {
                    Toggle("Enable Quiet Hours", isOn: $quietHoursEnabled)

                    if quietHoursEnabled {
                        DatePicker("Start", selection: $quietStart, displayedComponents: .hourAndMinute)
                        DatePicker("End", selection: $quietEnd, displayedComponents: .hourAndMinute)
                    }
                }

                Section("Integrations") {
                    HStack {
                        Text("GitHub")
                        Spacer()
                        Button("Configure") { }
                            .buttonStyle(.bordered)
                    }

                    HStack {
                        Text("GitLab")
                        Spacer()
                        Button("Configure") { }
                            .buttonStyle(.bordered)
                    }
                }
            }
            .formStyle(.grouped)
        }
        .frame(width: 500, height: 500)
    }
}

// Models
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

#Preview {
    NotificationsView()
        .environmentObject(BridgeManager())
}
