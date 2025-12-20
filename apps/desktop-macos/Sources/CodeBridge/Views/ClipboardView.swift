// Clipboard History View

import SwiftUI

struct ClipboardView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var searchText = ""
    @State private var selectedEntry: ClipboardEntry?
    @State private var showingFavoritesOnly = false

    var filteredEntries: [ClipboardEntry] {
        var entries = showingFavoritesOnly
            ? bridgeManager.clipboardHistory.filter { $0.isFavorite }
            : bridgeManager.clipboardHistory

        if !searchText.isEmpty {
            entries = entries.filter {
                $0.preview?.localizedCaseInsensitiveContains(searchText) ?? false
            }
        }

        return entries
    }

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                TextField("Search clipboard history...", text: $searchText)
                    .textFieldStyle(.roundedBorder)
                    .frame(maxWidth: 300)

                Spacer()

                Toggle("Favorites", isOn: $showingFavoritesOnly)
                    .toggleStyle(.button)

                Button(action: { bridgeManager.clearClipboardHistory() }) {
                    Label("Clear", systemImage: "trash")
                }
                .buttonStyle(.bordered)
            }
            .padding()

            Divider()

            if filteredEntries.isEmpty {
                EmptyClipboardView()
            } else {
                HSplitView {
                    // List
                    List(filteredEntries, selection: $selectedEntry) { entry in
                        ClipboardEntryRow(entry: entry)
                            .tag(entry)
                    }
                    .frame(minWidth: 300)

                    // Detail
                    if let entry = selectedEntry {
                        ClipboardDetailView(entry: entry)
                    } else {
                        Text("Select an entry to view details")
                            .foregroundColor(.secondary)
                            .frame(maxWidth: .infinity, maxHeight: .infinity)
                    }
                }
            }
        }
        .navigationTitle("Clipboard")
    }
}

struct EmptyClipboardView: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "doc.on.clipboard")
                .font(.system(size: 48))
                .foregroundColor(.secondary)
            Text("No clipboard history")
                .font(.title2)
            Text("Copy something to see it here")
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

struct ClipboardEntryRow: View {
    let entry: ClipboardEntry
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        HStack {
            // Icon based on type
            Image(systemName: iconForType(entry.contentType))
                .foregroundColor(colorForType(entry.contentType))
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 2) {
                Text(entry.preview ?? "Unknown content")
                    .lineLimit(2)
                    .font(.body)

                HStack {
                    Text(entry.timestamp, style: .relative)
                        .font(.caption)
                        .foregroundColor(.secondary)

                    if entry.sourceDevice != bridgeManager.deviceId {
                        Label(entry.sourceDevice, systemImage: "laptopcomputer")
                            .font(.caption2)
                            .foregroundColor(.blue)
                    }
                }
            }

            Spacer()

            // Actions
            Button(action: { bridgeManager.toggleClipboardFavorite(entry.id) }) {
                Image(systemName: entry.isFavorite ? "star.fill" : "star")
                    .foregroundColor(entry.isFavorite ? .yellow : .gray)
            }
            .buttonStyle(.plain)

            Button(action: { copyToClipboard(entry) }) {
                Image(systemName: "doc.on.doc")
            }
            .buttonStyle(.plain)
        }
        .padding(.vertical, 4)
    }

    func iconForType(_ type: String) -> String {
        switch type {
        case "text": return "doc.text"
        case "code": return "curlybraces"
        case "url": return "link"
        case "image": return "photo"
        default: return "doc"
        }
    }

    func colorForType(_ type: String) -> Color {
        switch type {
        case "text": return .blue
        case "code": return .orange
        case "url": return .purple
        case "image": return .green
        default: return .gray
        }
    }

    func copyToClipboard(_ entry: ClipboardEntry) {
        if let text = entry.textContent {
            NSPasteboard.general.clearContents()
            NSPasteboard.general.setString(text, forType: .string)
        }
    }
}

struct ClipboardDetailView: View {
    let entry: ClipboardEntry
    @State private var showingRawData = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            // Header
            HStack {
                VStack(alignment: .leading) {
                    Text(entry.contentType.capitalized)
                        .font(.headline)
                    Text(entry.timestamp, style: .date)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                Button("Copy") {
                    if let text = entry.textContent {
                        NSPasteboard.general.clearContents()
                        NSPasteboard.general.setString(text, forType: .string)
                    }
                }
                .buttonStyle(.borderedProminent)
            }

            Divider()

            // Content
            ScrollView {
                if entry.contentType == "image", let imageData = entry.imageData {
                    if let nsImage = NSImage(data: imageData) {
                        Image(nsImage: nsImage)
                            .resizable()
                            .aspectRatio(contentMode: .fit)
                            .frame(maxHeight: 300)
                    }
                } else if let text = entry.textContent {
                    if entry.contentType == "code" {
                        CodeBlockView(code: text, language: entry.language ?? "text")
                    } else {
                        Text(text)
                            .font(.system(.body, design: entry.contentType == "code" ? .monospaced : .default))
                            .textSelection(.enabled)
                    }
                }
            }

            Spacer()

            // Metadata
            GroupBox("Details") {
                LabeledContent("Source", value: entry.sourceDevice)
                LabeledContent("Size", value: "\(entry.dataSize) bytes")
                if let app = entry.appSource {
                    LabeledContent("App", value: app)
                }
            }
        }
        .padding()
    }
}

struct CodeBlockView: View {
    let code: String
    let language: String

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(language.uppercased())
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Button("Copy") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(code, forType: .string)
                }
                .buttonStyle(.plain)
                .font(.caption)
            }

            ScrollView(.horizontal, showsIndicators: true) {
                Text(code)
                    .font(.system(.body, design: .monospaced))
                    .textSelection(.enabled)
            }
            .padding()
            .background(Color(NSColor.textBackgroundColor))
            .cornerRadius(8)
        }
    }
}

#Preview {
    ClipboardView()
        .environmentObject(BridgeManager())
}
