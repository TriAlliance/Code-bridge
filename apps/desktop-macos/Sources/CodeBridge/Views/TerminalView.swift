// Terminal View - Session Recording and History

import SwiftUI

struct TerminalView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedTab = 0

    var body: some View {
        VStack(spacing: 0) {
            // Tab selector
            Picker("View", selection: $selectedTab) {
                Text("Recordings").tag(0)
                Text("History").tag(1)
                Text("Environment").tag(2)
            }
            .pickerStyle(.segmented)
            .padding()

            Divider()

            switch selectedTab {
            case 0:
                RecordingsListView()
            case 1:
                CommandHistoryView()
            case 2:
                EnvironmentSyncView()
            default:
                EmptyView()
            }
        }
        .navigationTitle("Terminal")
    }
}

struct RecordingsListView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedRecording: TerminalRecording?
    @State private var isRecording = false

    var body: some View {
        HSplitView {
            // List
            VStack {
                // Recording controls
                HStack {
                    if isRecording {
                        Button(action: { stopRecording() }) {
                            Label("Stop Recording", systemImage: "stop.circle.fill")
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.red)

                        Text("Recording...")
                            .foregroundColor(.red)
                            .font(.caption)
                    } else {
                        Button(action: { startRecording() }) {
                            Label("New Recording", systemImage: "record.circle")
                        }
                        .buttonStyle(.borderedProminent)
                    }

                    Spacer()
                }
                .padding()

                List(bridgeManager.terminalRecordings, selection: $selectedRecording) { recording in
                    RecordingRow(recording: recording)
                        .tag(recording)
                }
            }
            .frame(minWidth: 300)

            // Player
            if let recording = selectedRecording {
                RecordingPlayerView(recording: recording)
            } else {
                VStack {
                    Image(systemName: "play.rectangle")
                        .font(.system(size: 48))
                        .foregroundColor(.secondary)
                    Text("Select a recording to play")
                        .foregroundColor(.secondary)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
    }

    func startRecording() {
        isRecording = true
        bridgeManager.startTerminalRecording()
    }

    func stopRecording() {
        isRecording = false
        bridgeManager.stopTerminalRecording()
    }
}

struct RecordingRow: View {
    let recording: TerminalRecording

    var body: some View {
        HStack {
            Image(systemName: "terminal")
                .foregroundColor(.green)

            VStack(alignment: .leading) {
                Text(recording.title)
                    .font(.headline)

                HStack {
                    Text(recording.recordedAt, style: .date)
                    Text("•")
                    Text(formatDuration(recording.duration))
                }
                .font(.caption)
                .foregroundColor(.secondary)
            }

            Spacer()

            Text(recording.shell)
                .font(.caption)
                .padding(.horizontal, 8)
                .padding(.vertical, 2)
                .background(Color.secondary.opacity(0.2))
                .cornerRadius(4)
        }
        .padding(.vertical, 4)
    }

    func formatDuration(_ duration: TimeInterval) -> String {
        let minutes = Int(duration) / 60
        let seconds = Int(duration) % 60
        return String(format: "%d:%02d", minutes, seconds)
    }
}

struct RecordingPlayerView: View {
    let recording: TerminalRecording
    @State private var isPlaying = false
    @State private var playbackSpeed: Double = 1.0
    @State private var currentTime: TimeInterval = 0

    var body: some View {
        VStack(spacing: 0) {
            // Terminal display
            ScrollView {
                Text(recording.outputPreview)
                    .font(.system(.body, design: .monospaced))
                    .foregroundColor(.green)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding()
            }
            .background(Color.black)
            .frame(maxHeight: .infinity)

            Divider()

            // Controls
            HStack {
                Button(action: { isPlaying.toggle() }) {
                    Image(systemName: isPlaying ? "pause.fill" : "play.fill")
                }
                .buttonStyle(.bordered)

                Slider(value: $currentTime, in: 0...recording.duration)
                    .frame(maxWidth: .infinity)

                Text(formatTime(currentTime))
                    .font(.caption)
                    .monospacedDigit()

                Picker("Speed", selection: $playbackSpeed) {
                    Text("0.5x").tag(0.5)
                    Text("1x").tag(1.0)
                    Text("2x").tag(2.0)
                    Text("4x").tag(4.0)
                }
                .pickerStyle(.menu)
                .frame(width: 80)

                Button(action: { exportRecording() }) {
                    Image(systemName: "square.and.arrow.up")
                }
                .buttonStyle(.bordered)
            }
            .padding()
        }
    }

    func formatTime(_ time: TimeInterval) -> String {
        let minutes = Int(time) / 60
        let seconds = Int(time) % 60
        return String(format: "%d:%02d", minutes, seconds)
    }

    func exportRecording() {
        // Export as asciinema format
    }
}

struct CommandHistoryView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var searchText = ""
    @State private var selectedDirectory: String?

    var filteredHistory: [CommandHistoryEntry] {
        var entries = bridgeManager.commandHistory

        if !searchText.isEmpty {
            entries = entries.filter { $0.command.localizedCaseInsensitiveContains(searchText) }
        }

        if let dir = selectedDirectory {
            entries = entries.filter { $0.workingDir == dir }
        }

        return entries
    }

    var body: some View {
        VStack(spacing: 0) {
            // Search and filter
            HStack {
                TextField("Search commands...", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 300)

                Picker("Directory", selection: $selectedDirectory) {
                    Text("All Directories").tag(nil as String?)
                    ForEach(bridgeManager.recentDirectories, id: \.self) { dir in
                        Text(dir).tag(dir as String?)
                    }
                }
                .frame(maxWidth: 200)

                Spacer()

                Button("Sync Now") {
                    bridgeManager.syncCommandHistory()
                }
                .buttonStyle(.bordered)
            }
            .padding()

            Divider()

            List(filteredHistory) { entry in
                CommandHistoryRow(entry: entry)
            }
        }
    }
}

struct CommandHistoryRow: View {
    let entry: CommandHistoryEntry
    @State private var isHovering = false

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 4) {
                Text(entry.command)
                    .font(.system(.body, design: .monospaced))

                HStack {
                    Text(entry.workingDir)
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if let exitCode = entry.exitCode {
                        Text("Exit: \(exitCode)")
                            .font(.caption)
                            .foregroundColor(exitCode == 0 ? .green : .red)
                    }

                    if let duration = entry.duration {
                        Text(String(format: "%.2fs", duration))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
            }

            Spacer()

            if isHovering {
                Button(action: { copyCommand() }) {
                    Image(systemName: "doc.on.doc")
                }
                .buttonStyle(.plain)

                Button(action: { runCommand() }) {
                    Image(systemName: "play")
                }
                .buttonStyle(.plain)
            }

            Text(entry.timestamp, style: .relative)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
        .onHover { hovering in
            isHovering = hovering
        }
    }

    func copyCommand() {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(entry.command, forType: .string)
    }

    func runCommand() {
        // Open terminal with command
    }
}

struct EnvironmentSyncView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedTab = 0

    var body: some View {
        VStack(spacing: 0) {
            Picker("", selection: $selectedTab) {
                Text("Aliases").tag(0)
                Text("Functions").tag(1)
                Text("Environment").tag(2)
            }
            .pickerStyle(.segmented)
            .padding()

            switch selectedTab {
            case 0:
                AliasesView()
            case 1:
                FunctionsView()
            case 2:
                EnvironmentVarsView()
            default:
                EmptyView()
            }
        }
    }
}

struct AliasesView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        List(bridgeManager.shellAliases, id: \.name) { alias in
            HStack {
                Text(alias.name)
                    .font(.system(.body, design: .monospaced))
                    .fontWeight(.bold)

                Text("=")
                    .foregroundColor(.secondary)

                Text(alias.command)
                    .font(.system(.body, design: .monospaced))
                    .foregroundColor(.secondary)
            }
        }
    }
}

struct FunctionsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        List(bridgeManager.shellFunctions, id: \.name) { function in
            VStack(alignment: .leading) {
                Text(function.name)
                    .font(.system(.body, design: .monospaced))
                    .fontWeight(.bold)

                Text(function.body)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(3)
            }
            .padding(.vertical, 4)
        }
    }
}

struct EnvironmentVarsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        List(bridgeManager.environmentVars, id: \.name) { envVar in
            HStack {
                Text(envVar.name)
                    .font(.system(.body, design: .monospaced))
                    .fontWeight(.bold)

                Spacer()

                Text(envVar.value)
                    .font(.system(.body, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
        }
    }
}

#Preview {
    TerminalView()
        .environmentObject(BridgeManager())
}
