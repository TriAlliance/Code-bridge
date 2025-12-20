// Settings View

import SwiftUI

struct SettingsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        TabView {
            GeneralSettingsView()
                .tabItem {
                    Label("General", systemImage: "gear")
                }

            NetworkSettingsView()
                .tabItem {
                    Label("Network", systemImage: "network")
                }

            SyncSettingsView()
                .tabItem {
                    Label("Sync", systemImage: "arrow.triangle.2.circlepath")
                }
        }
        .frame(width: 500, height: 400)
    }
}

struct GeneralSettingsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var deviceName: String = ""
    @State private var launchAtLogin = true

    var body: some View {
        Form {
            Section {
                TextField("Device Name", text: $deviceName)
                    .onAppear {
                        deviceName = bridgeManager.deviceName
                    }

                Toggle("Launch at Login", isOn: $launchAtLogin)
            }

            Section {
                LabeledContent("Device ID") {
                    Text(bridgeManager.deviceId)
                        .font(.system(.body, design: .monospaced))
                        .foregroundColor(.secondary)
                }

                LabeledContent("Version") {
                    Text("0.1.0")
                        .foregroundColor(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct NetworkSettingsView: View {
    @State private var enableMdns = true
    @State private var enableDht = true
    @State private var enableQuic = true
    @State private var listenPort = "0"

    var body: some View {
        Form {
            Section("Discovery") {
                Toggle("mDNS (Local Network)", isOn: $enableMdns)
                Toggle("DHT (Global)", isOn: $enableDht)
            }

            Section("Transport") {
                Toggle("QUIC (Recommended)", isOn: $enableQuic)

                TextField("Listen Port (0 = random)", text: $listenPort)
            }

            Section("Bootstrap Peers") {
                Text("No custom bootstrap peers configured")
                    .foregroundColor(.secondary)

                Button("Add Peer") { }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

struct SyncSettingsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var realtimeSync = true
    @State private var syncInterval = "30"

    var body: some View {
        Form {
            Section("Sync Behavior") {
                Toggle("Real-time Sync", isOn: $realtimeSync)

                TextField("Sync Interval (seconds)", text: $syncInterval)
                    .disabled(realtimeSync)
            }

            Section("Watch Paths") {
                if bridgeManager.watchPaths.isEmpty {
                    Text("No paths configured")
                        .foregroundColor(.secondary)
                } else {
                    ForEach(bridgeManager.watchPaths, id: \.self) { path in
                        Text(path)
                    }
                }

                Button("Add Path") { }
            }

            Section("Ignore Patterns") {
                ForEach(bridgeManager.ignorePatterns, id: \.self) { pattern in
                    Text(pattern)
                        .font(.system(.body, design: .monospaced))
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

#Preview {
    SettingsView()
        .environmentObject(BridgeManager())
}
