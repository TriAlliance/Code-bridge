// Menu Bar View

import SwiftUI

struct MenuBarView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        VStack(spacing: 12) {
            // Status
            HStack {
                Circle()
                    .fill(bridgeManager.isConnected ? Color.green : Color.gray)
                    .frame(width: 8, height: 8)
                Text(bridgeManager.isConnected ? "Connected" : "Offline")
                    .font(.caption)
                Spacer()
                Text("\(bridgeManager.peerCount) peers")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding(.horizontal)

            Divider()

            // Quick stats
            HStack(spacing: 20) {
                VStack {
                    Text("\(bridgeManager.fileCount)")
                        .font(.title2)
                        .fontWeight(.bold)
                    Text("Files")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }

                VStack {
                    Text("\(bridgeManager.peerCount)")
                        .font(.title2)
                        .fontWeight(.bold)
                    Text("Peers")
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.vertical, 4)

            Divider()

            // Actions
            Button(action: { bridgeManager.sync() }) {
                HStack {
                    Image(systemName: "arrow.triangle.2.circlepath")
                    Text("Sync Now")
                    Spacer()
                    if bridgeManager.isSyncing {
                        ProgressView()
                            .scaleEffect(0.5)
                    }
                }
            }
            .buttonStyle(.plain)
            .padding(.horizontal)

            Button(action: { bridgeManager.discoverPeers() }) {
                HStack {
                    Image(systemName: "magnifyingglass")
                    Text("Discover Peers")
                    Spacer()
                }
            }
            .buttonStyle(.plain)
            .padding(.horizontal)

            Divider()

            // Recent activity
            if !bridgeManager.recentActivity.isEmpty {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Recent")
                        .font(.caption2)
                        .foregroundColor(.secondary)

                    ForEach(bridgeManager.recentActivity.prefix(3), id: \.self) { activity in
                        Text(activity)
                            .font(.caption)
                            .lineLimit(1)
                    }
                }
                .padding(.horizontal)

                Divider()
            }

            // Footer
            HStack {
                Button("Open Code Bridge") {
                    NSApp.activate(ignoringOtherApps: true)
                    if let window = NSApp.windows.first {
                        window.makeKeyAndOrderFront(nil)
                    }
                }
                .buttonStyle(.plain)
                .font(.caption)

                Spacer()

                Button("Quit") {
                    NSApp.terminate(nil)
                }
                .buttonStyle(.plain)
                .font(.caption)
                .foregroundColor(.secondary)
            }
            .padding(.horizontal)
        }
        .padding(.vertical)
        .frame(width: 250)
    }
}
