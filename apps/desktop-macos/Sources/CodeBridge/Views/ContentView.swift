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
            default:
                DashboardView()
            }
        }
        .frame(minWidth: 800, minHeight: 600)
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

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title)
                .foregroundColor(.accentColor)

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
