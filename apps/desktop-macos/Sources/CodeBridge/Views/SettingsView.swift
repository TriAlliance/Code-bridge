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

            NASSettingsView()
                .tabItem {
                    Label("NAS", systemImage: "externaldrive.connected.to.line.below")
                }
        }
        .frame(width: 600, height: 500)
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

// MARK: - NAS Settings View

struct NASSettingsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var showAddConnection = false
    @State private var editingConnection: QNAPConnection?
    @State private var testingConnectionId: String?
    @State private var testResult: (success: Bool, message: String)?

    var body: some View {
        Form {
            Section("NAS Connections") {
                if bridgeManager.nasConnections.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "externaldrive.badge.plus")
                            .font(.system(size: 32))
                            .foregroundColor(.secondary)
                        Text("No NAS connections configured")
                            .foregroundColor(.secondary)
                        Text("Add a QNAP NAS to enable backup functionality")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 20)
                } else {
                    ForEach(bridgeManager.nasConnections) { connection in
                        NASConnectionRow(
                            connection: connection,
                            isDefault: connection.isDefault,
                            isTesting: testingConnectionId == connection.id,
                            onEdit: { editingConnection = connection },
                            onDelete: { bridgeManager.removeNASConnection(connection.id) },
                            onTest: { testConnection(connection) },
                            onSetDefault: { bridgeManager.setDefaultNASConnection(connection.id) }
                        )
                    }
                }

                Button(action: { showAddConnection = true }) {
                    Label("Add NAS Connection", systemImage: "plus")
                }
            }

            if let result = testResult {
                Section {
                    HStack {
                        Image(systemName: result.success ? "checkmark.circle.fill" : "xmark.circle.fill")
                            .foregroundColor(result.success ? .green : .red)
                        Text(result.message)
                            .font(.caption)
                    }
                }
            }

            Section("Connection Status") {
                HStack {
                    Image(systemName: bridgeManager.nasConnectionStatus.icon)
                        .foregroundColor(bridgeManager.nasConnectionStatus.color)
                    Text(bridgeManager.nasConnectionStatus.rawValue)
                    Spacer()
                    if bridgeManager.nasConnectionStatus == .connected {
                        Button("Disconnect") {
                            Task {
                                await bridgeManager.disconnectFromNAS()
                            }
                        }
                    } else if let defaultConn = bridgeManager.nasConnections.first(where: { $0.isDefault }) {
                        Button("Connect") {
                            Task {
                                await bridgeManager.connectToNAS(defaultConn)
                            }
                        }
                    }
                }
            }

            Section("About QNAP Integration") {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Supported Protocols")
                        .font(.headline)

                    ForEach(QNAPProtocol.allCases) { proto in
                        HStack {
                            Image(systemName: proto.icon)
                                .frame(width: 20)
                            VStack(alignment: .leading) {
                                Text(proto.rawValue)
                                    .font(.subheadline)
                                Text(proto.description)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                }
                .padding(.vertical, 4)
            }
        }
        .formStyle(.grouped)
        .padding()
        .sheet(isPresented: $showAddConnection) {
            NASConnectionEditor(connection: nil) { newConnection in
                bridgeManager.addNASConnection(newConnection)
            }
        }
        .sheet(item: $editingConnection) { connection in
            NASConnectionEditor(connection: connection) { updatedConnection in
                bridgeManager.updateNASConnection(updatedConnection)
            }
        }
    }

    private func testConnection(_ connection: QNAPConnection) {
        testingConnectionId = connection.id
        testResult = nil

        Task {
            let result = await bridgeManager.qnapService.testConnection(connection)
            await MainActor.run {
                testResult = result
                testingConnectionId = nil
            }
        }
    }
}

// MARK: - NAS Connection Row

struct NASConnectionRow: View {
    let connection: QNAPConnection
    let isDefault: Bool
    let isTesting: Bool
    let onEdit: () -> Void
    let onDelete: () -> Void
    let onTest: () -> Void
    let onSetDefault: () -> Void

    var body: some View {
        HStack {
            Image(systemName: connection.qnapProtocol.icon)
                .font(.title2)
                .foregroundColor(.accentColor)
                .frame(width: 30)

            VStack(alignment: .leading, spacing: 2) {
                HStack {
                    Text(connection.name)
                        .font(.headline)
                    if isDefault {
                        Text("Default")
                            .font(.caption2)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(Color.accentColor.opacity(0.2))
                            .cornerRadius(4)
                    }
                }
                Text("\(connection.host):\(connection.port)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Text(connection.qnapProtocol.rawValue)
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }

            Spacer()

            if isTesting {
                ProgressView()
                    .scaleEffect(0.7)
            } else {
                HStack(spacing: 8) {
                    Button(action: onTest) {
                        Image(systemName: "antenna.radiowaves.left.and.right")
                    }
                    .buttonStyle(.borderless)
                    .help("Test Connection")

                    if !isDefault {
                        Button(action: onSetDefault) {
                            Image(systemName: "star")
                        }
                        .buttonStyle(.borderless)
                        .help("Set as Default")
                    }

                    Button(action: onEdit) {
                        Image(systemName: "pencil")
                    }
                    .buttonStyle(.borderless)
                    .help("Edit")

                    Button(action: onDelete) {
                        Image(systemName: "trash")
                    }
                    .buttonStyle(.borderless)
                    .foregroundColor(.red)
                    .help("Delete")
                }
            }
        }
        .padding(.vertical, 4)
    }
}

// MARK: - NAS Connection Editor

struct NASConnectionEditor: View {
    let connection: QNAPConnection?
    let onSave: (QNAPConnection) -> Void

    @Environment(\.dismiss) private var dismiss

    @State private var name: String = ""
    @State private var host: String = ""
    @State private var port: String = ""
    @State private var username: String = ""
    @State private var password: String = ""
    @State private var sharePath: String = "/Development"
    @State private var selectedProtocol: QNAPProtocol = .smb3
    @State private var useSSL: Bool = true

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text(connection == nil ? "Add NAS Connection" : "Edit NAS Connection")
                    .font(.headline)
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.escape, modifiers: [])
            }
            .padding()

            Divider()

            // Form - using VStack instead of Form to fix text input issues
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    // Connection Details
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Connection Details")
                            .font(.headline)

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Name").font(.caption).foregroundColor(.secondary)
                            TextField("", text: $name, prompt: Text("My QNAP NAS"))
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Host (IP or hostname)").font(.caption).foregroundColor(.secondary)
                            TextField("", text: $host, prompt: Text("192.168.1.100"))
                        }

                        HStack {
                            VStack(alignment: .leading, spacing: 4) {
                                Text("Port").font(.caption).foregroundColor(.secondary)
                                TextField("", text: $port, prompt: Text("445"))
                                    .frame(width: 80)
                            }

                            VStack(alignment: .leading, spacing: 4) {
                                Text("Protocol").font(.caption).foregroundColor(.secondary)
                                Picker("", selection: $selectedProtocol) {
                                    ForEach(QNAPProtocol.allCases) { proto in
                                        Text(proto.rawValue).tag(proto)
                                    }
                                }
                                .labelsHidden()
                                .onChange(of: selectedProtocol) { _, newValue in
                                    port = String(newValue.defaultPort)
                                }
                            }
                        }

                        Toggle("Use SSL/TLS", isOn: $useSSL)
                    }

                    Divider()

                    // Authentication
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Authentication")
                            .font(.headline)

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Username").font(.caption).foregroundColor(.secondary)
                            TextField("", text: $username, prompt: Text("admin"))
                        }

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Password").font(.caption).foregroundColor(.secondary)
                            SecureField("", text: $password, prompt: Text("Enter password"))
                        }
                    }

                    Divider()

                    // Share Path
                    VStack(alignment: .leading, spacing: 12) {
                        Text("Share Path")
                            .font(.headline)

                        VStack(alignment: .leading, spacing: 4) {
                            Text("Share Path").font(.caption).foregroundColor(.secondary)
                            TextField("", text: $sharePath, prompt: Text("/Development"))
                        }
                        Text("The root path on the NAS for backups")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                .textFieldStyle(.plain)
                .padding()
            }

            Divider()

            // Footer
            HStack {
                Spacer()
                Button("Save") {
                    let newConnection = QNAPConnection(
                        id: connection?.id ?? UUID().uuidString,
                        name: name,
                        host: host,
                        port: Int(port) ?? selectedProtocol.defaultPort,
                        username: username,
                        password: password,
                        sharePath: sharePath,
                        protocolType: selectedProtocol,
                        useSSL: useSSL,
                        isDefault: connection?.isDefault ?? false
                    )
                    onSave(newConnection)
                    dismiss()
                }
                .keyboardShortcut(.return, modifiers: [.command])
                .disabled(name.isEmpty || host.isEmpty || username.isEmpty)
            }
            .padding()
        }
        .frame(width: 450, height: 500)
        .onAppear {
            if let conn = connection {
                name = conn.name
                host = conn.host
                port = String(conn.port)
                username = conn.username
                password = conn.password
                sharePath = conn.sharePath
                selectedProtocol = conn.qnapProtocol
                useSSL = conn.useSSL
            } else {
                port = String(selectedProtocol.defaultPort)
            }
        }
    }
}

#Preview {
    SettingsView()
        .environmentObject(BridgeManager())
}
