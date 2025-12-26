// Main Content View

import SwiftUI

struct ContentView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedTab = 0

    var body: some View {
        NavigationSplitView {
            Sidebar(selectedTab: $selectedTab)
        } detail: {
            switch selectedTab {
            case 0:
                DashboardView()
            case 1:
                FilesView()
            case 2:
                PeersView()
            case 3:
                ScreenshotsView()
            case 4:
                ClipboardView()
            case 5:
                NotificationsView()
            case 6:
                TerminalView()
            case 7:
                TransformView()
            case 8:
                BackupView()
            default:
                DashboardView()
            }
        }
        .frame(minWidth: 900, minHeight: 700)
        // Global screenshot preview - use ScreenshotSheetPresenter to observe service
        .background {
            ScreenshotSheetPresenter()
        }
    }
}

/// Presents screenshot preview sheet by observing ScreenshotService directly
struct ScreenshotSheetPresenter: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        ScreenshotSheetObserver(service: bridgeManager.screenshotService)
            .environmentObject(bridgeManager)
    }
}

struct ScreenshotSheetObserver: View {
    @ObservedObject var service: ScreenshotService
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        Color.clear
            .sheet(isPresented: $service.showPreview) {
                if let pending = service.pendingScreenshot {
                    ScreenshotPreviewSheet(pending: pending)
                        .environmentObject(bridgeManager)
                }
            }
    }
}

/// Container view that observes ScreenshotService directly
struct ScreenshotPreviewContainer: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        // Force view update by observing the service
        ScreenshotPreviewObserver(service: bridgeManager.screenshotService)
    }
}

struct ScreenshotPreviewObserver: View {
    @ObservedObject var service: ScreenshotService

    var body: some View {
        if service.showPreview, let pending = service.pendingScreenshot {
            ScreenshotPreviewOverlay(pending: pending)
        }
    }
}

/// Sheet view for screenshot preview with proper modal interaction
struct ScreenshotPreviewSheet: View {
    let pending: PendingScreenshot
    @EnvironmentObject var bridgeManager: BridgeManager
    @Environment(\.dismiss) private var dismiss
    @State private var uploadToNAS: Bool = true

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Screenshot Preview")
                    .font(.headline)
                Spacer()
                Button(action: {
                    dismiss()
                }) {
                    Image(systemName: "xmark.circle.fill")
                        .font(.title2)
                        .foregroundColor(.secondary)
                }
                .buttonStyle(.plain)
            }
            .padding()

            Divider()

            // Image preview
            ScrollView {
                Image(nsImage: pending.image)
                    .resizable()
                    .aspectRatio(contentMode: .fit)
                    .cornerRadius(8)
                    .padding()
            }
            .frame(maxHeight: 400)

            Divider()

            // Info bar
            HStack {
                Label("\(pending.width) × \(pending.height)", systemImage: "aspectratio")
                    .font(.caption)

                Divider()
                    .frame(height: 16)

                Label(ByteCountFormatter.string(fromByteCount: pending.fileSize, countStyle: .file), systemImage: "doc")
                    .font(.caption)

                if let ocrText = pending.ocrText, !ocrText.isEmpty {
                    Divider()
                        .frame(height: 16)
                    Label("Text detected", systemImage: "text.viewfinder")
                        .font(.caption)
                        .foregroundColor(.green)
                }

                Spacer()
            }
            .padding(.horizontal)
            .padding(.vertical, 8)

            Divider()

            // Actions
            HStack(spacing: 12) {
                Button(action: {
                    bridgeManager.screenshotService.copyPendingToClipboard()
                    dismiss()
                }) {
                    Label("Copy Only", systemImage: "doc.on.doc")
                }
                .buttonStyle(.bordered)

                Spacer()

                Toggle("Upload to NAS", isOn: $uploadToNAS)
                    .toggleStyle(.checkbox)

                Button(action: {
                    dismiss()
                }) {
                    Text("Discard")
                }
                .buttonStyle(.bordered)

                Button(action: {
                    Task {
                        await bridgeManager.screenshotService.confirmPendingScreenshot(uploadToNAS: uploadToNAS)
                        dismiss()
                    }
                }) {
                    Label("Save", systemImage: "square.and.arrow.down")
                }
                .buttonStyle(.borderedProminent)
                .keyboardShortcut(.return, modifiers: [])
            }
            .padding()
        }
        .frame(width: 600)
    }
}

struct Sidebar: View {
    @Binding var selectedTab: Int
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        List(selection: $selectedTab) {
            Section("Overview") {
                Label("Dashboard", systemImage: "square.grid.2x2")
                    .tag(0)
            }

            Section("Content") {
                Label("Files", systemImage: "folder")
                    .tag(1)
                Label("Peers", systemImage: "network")
                    .tag(2)
                Label("Screenshots", systemImage: "photo")
                    .tag(3)
            }

            Section("Productivity") {
                Label {
                    HStack {
                        Text("Clipboard")
                        if bridgeManager.clipboardHistory.count > 0 {
                            Text("\(bridgeManager.clipboardHistory.count)")
                                .font(.caption2)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Color.secondary.opacity(0.3))
                                .cornerRadius(8)
                        }
                    }
                } icon: {
                    Image(systemName: "doc.on.clipboard")
                }
                .tag(4)

                Label {
                    HStack {
                        Text("Notifications")
                        let unread = bridgeManager.notifications.filter { !$0.isRead }.count
                        if unread > 0 {
                            Text("\(unread)")
                                .font(.caption2)
                                .padding(.horizontal, 6)
                                .padding(.vertical, 2)
                                .background(Color.red)
                                .foregroundColor(.white)
                                .cornerRadius(8)
                        }
                    }
                } icon: {
                    Image(systemName: "bell")
                }
                .tag(5)

                Label("Terminal", systemImage: "terminal")
                    .tag(6)
            }

            Section("Tools") {
                Label("Transform", systemImage: "arrow.triangle.2.circlepath")
                    .tag(7)

                Label {
                    HStack {
                        Text("Backup")
                        if bridgeManager.backupService.isBackupRunning {
                            ProgressView()
                                .scaleEffect(0.5)
                        }
                    }
                } icon: {
                    Image(systemName: "externaldrive.badge.timemachine")
                }
                .tag(8)
            }
        }
        .listStyle(.sidebar)
        .frame(minWidth: 200)
        .toolbar {
            ToolbarItem {
                Button(action: { bridgeManager.sync() }) {
                    Image(systemName: "arrow.triangle.2.circlepath")
                }
                .help("Sync Now")
            }
        }
    }
}

struct DashboardView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Status Cards
                HStack(spacing: 16) {
                    StatusCard(
                        title: "Device",
                        value: bridgeManager.deviceName,
                        icon: "desktopcomputer"
                    )

                    StatusCard(
                        title: "Peers",
                        value: "\(bridgeManager.peerCount)",
                        icon: "person.2"
                    )

                    StatusCard(
                        title: "Files",
                        value: "\(bridgeManager.fileCount)",
                        icon: "doc.on.doc"
                    )

                    StatusCard(
                        title: "Status",
                        value: bridgeManager.isSyncing ? "Syncing" : "Ready",
                        icon: bridgeManager.isSyncing ? "arrow.triangle.2.circlepath" : "checkmark.circle"
                    )

                    StatusCard(
                        title: "NAS",
                        value: bridgeManager.nasConnectionStatus.rawValue,
                        icon: bridgeManager.nasConnectionStatus.icon,
                        color: bridgeManager.nasConnectionStatus.color
                    )
                }
                .padding()

                // Quick Actions
                GroupBox("Quick Actions") {
                    HStack(spacing: 20) {
                        ActionButton(title: "Sync Now", icon: "arrow.triangle.2.circlepath") {
                            bridgeManager.sync()
                        }

                        ActionButton(title: "Add Files", icon: "plus.circle") {
                            bridgeManager.addFiles()
                        }

                        ActionButton(title: "Discover Peers", icon: "magnifyingglass") {
                            bridgeManager.discoverPeers()
                        }
                    }
                    .padding()
                }
                .padding(.horizontal)

                // Recent Activity
                GroupBox("Recent Activity") {
                    if bridgeManager.recentActivity.isEmpty {
                        Text("No recent activity")
                            .foregroundColor(.secondary)
                            .padding()
                    } else {
                        ForEach(bridgeManager.recentActivity, id: \.self) { activity in
                            HStack {
                                Image(systemName: "clock")
                                    .foregroundColor(.secondary)
                                Text(activity)
                                Spacer()
                            }
                            .padding(.vertical, 4)
                        }
                        .padding()
                    }
                }
                .padding(.horizontal)

                Spacer()
            }
        }
        .navigationTitle("Dashboard")
    }
}

struct StatusCard: View {
    let title: String
    let value: String
    let icon: String
    var color: Color = .accentColor

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title)
                .foregroundColor(color)

            Text(value)
                .font(.title2)
                .fontWeight(.semibold)

            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(.regularMaterial)
        .cornerRadius(12)
    }
}

struct ActionButton: View {
    let title: String
    let icon: String
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            VStack(spacing: 8) {
                Image(systemName: icon)
                    .font(.title2)
                Text(title)
                    .font(.caption)
            }
            .frame(width: 100, height: 80)
        }
        .buttonStyle(.bordered)
    }
}

#Preview {
    ContentView()
        .environmentObject(BridgeManager())
}
