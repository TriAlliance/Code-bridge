// Screenshots View - Screenshot Capture and Management
// Capture screenshots and sync them to QNAP NAS

import SwiftUI
import Foundation

struct ScreenshotsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var searchText = ""
    @State private var selectedScreenshot: CapturedScreenshot?
    @State private var showSettings = false
    @State private var showDeleteConfirm = false
    @State private var screenshotToDelete: CapturedScreenshot?

    var filteredScreenshots: [CapturedScreenshot] {
        if searchText.isEmpty {
            return bridgeManager.screenshotService.screenshots
        }
        return bridgeManager.screenshotService.searchScreenshots(searchText)
    }

    var body: some View {
        ZStack {
            VStack(spacing: 0) {
                // Header with capture buttons
                ScreenshotHeader(
                    searchText: $searchText,
                    showSettings: $showSettings
                )

                Divider()

                // Main content
                if bridgeManager.screenshotService.screenshots.isEmpty {
                    EmptyScreenshotsView()
                } else {
                    HSplitView {
                        // Screenshot grid
                        ScreenshotGrid(
                            screenshots: filteredScreenshots,
                            selectedScreenshot: $selectedScreenshot
                        )
                        .frame(minWidth: 400)

                        // Detail panel
                        if let screenshot = selectedScreenshot {
                            ScreenshotDetailPanel(
                                screenshot: screenshot,
                                onDelete: {
                                    screenshotToDelete = screenshot
                                    showDeleteConfirm = true
                                }
                            )
                            .frame(minWidth: 280, maxWidth: 320)
                        }
                    }
                }
            }

            // Screenshot preview overlay
            if bridgeManager.screenshotService.showPreview,
               let pending = bridgeManager.screenshotService.pendingScreenshot {
                ScreenshotPreviewOverlay(pending: pending)
            }
        }
        .navigationTitle("Screenshots")
        .sheet(isPresented: $showSettings) {
            ScreenshotSettingsSheet()
        }
        .alert("Delete Screenshot?", isPresented: $showDeleteConfirm) {
            Button("Cancel", role: .cancel) { }
            Button("Delete", role: .destructive) {
                if let screenshot = screenshotToDelete {
                    bridgeManager.screenshotService.deleteScreenshot(screenshot.id)
                    if selectedScreenshot?.id == screenshot.id {
                        selectedScreenshot = nil
                    }
                }
            }
        } message: {
            Text("This will permanently delete the screenshot file.")
        }
    }
}

// MARK: - Screenshot Preview Overlay

struct ScreenshotPreviewOverlay: View {
    let pending: PendingScreenshot
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var uploadToNAS: Bool = true

    var body: some View {
        ZStack {
            // Dimmed background - blocks interaction with views behind
            Color.black.opacity(0.6)
                .ignoresSafeArea()
                .contentShape(Rectangle())
                .onTapGesture {
                    // Don't dismiss on background tap - just consume the tap
                }

            // Preview panel
            VStack(spacing: 0) {
                // Header
                HStack {
                    Text("Screenshot Preview")
                        .font(.headline)
                    Spacer()
                    Button(action: {
                        bridgeManager.screenshotService.cancelPendingScreenshot()
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
                    // Copy to clipboard only
                    Button(action: {
                        bridgeManager.screenshotService.copyPendingToClipboard()
                        bridgeManager.screenshotService.cancelPendingScreenshot()
                    }) {
                        Label("Copy Only", systemImage: "doc.on.doc")
                    }
                    .buttonStyle(.bordered)

                    Spacer()

                    // Upload toggle
                    Toggle("Upload to NAS", isOn: $uploadToNAS)
                        .toggleStyle(.checkbox)

                    // Cancel
                    Button("Discard") {
                        bridgeManager.screenshotService.cancelPendingScreenshot()
                    }
                    .buttonStyle(.bordered)

                    // Confirm
                    Button(action: {
                        Task {
                            await bridgeManager.screenshotService.confirmPendingScreenshot(uploadToNAS: uploadToNAS)
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
            .background(Color(nsColor: .windowBackgroundColor))
            .cornerRadius(12)
            .shadow(radius: 20)
        }
        .allowsHitTesting(true)
        .transition(.opacity)
        .animation(.easeInOut(duration: 0.2), value: bridgeManager.screenshotService.showPreview)
    }
}

// MARK: - Screenshot Header

struct ScreenshotHeader: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @Binding var searchText: String
    @Binding var showSettings: Bool

    var body: some View {
        HStack(spacing: 16) {
            // Capture buttons
            HStack(spacing: 8) {
                CaptureButton(
                    mode: .fullScreen,
                    isCapturing: bridgeManager.screenshotService.isCapturing
                ) {
                    Task {
                        await bridgeManager.screenshotService.captureFullScreen()
                    }
                }

                CaptureButton(
                    mode: .selection,
                    isCapturing: bridgeManager.screenshotService.isCapturing
                ) {
                    print("[Button] Selection button clicked!")
                    Task {
                        print("[Button] Starting capture task...")
                        await bridgeManager.screenshotService.captureSelection()
                        print("[Button] Capture task finished")
                    }
                }

                CaptureButton(
                    mode: .window,
                    isCapturing: bridgeManager.screenshotService.isCapturing
                ) {
                    Task {
                        await bridgeManager.screenshotService.captureWindow()
                    }
                }
            }

            Divider()
                .frame(height: 24)

            // Search
            TextField("Search screenshots...", text: $searchText)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 250)

            Spacer()

            // Sync status
            HStack(spacing: 4) {
                let unsyncedCount = bridgeManager.screenshotService.screenshots.filter { !$0.isSyncedToNAS }.count
                if unsyncedCount > 0 {
                    Button(action: {
                        Task {
                            await bridgeManager.screenshotService.uploadAllToNAS()
                        }
                    }) {
                        Label("\(unsyncedCount) to sync", systemImage: "arrow.triangle.2.circlepath")
                            .font(.caption)
                    }
                    .buttonStyle(.bordered)
                } else {
                    Label("All synced", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                }
            }

            // Settings button
            Button(action: { showSettings = true }) {
                Image(systemName: "gear")
            }
            .buttonStyle(.bordered)
        }
        .padding()
    }
}

// MARK: - Capture Button

func logToFile(_ message: String) {
    let logPath = "/tmp/codebridge_app.log"
    let timestamp = DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
    let line = "[\(timestamp)] \(message)\n"
    if let data = line.data(using: .utf8) {
        if FileManager.default.fileExists(atPath: logPath) {
            if let fileHandle = FileHandle(forWritingAtPath: logPath) {
                fileHandle.seekToEndOfFile()
                fileHandle.write(data)
                fileHandle.closeFile()
            }
        } else {
            FileManager.default.createFile(atPath: logPath, contents: data)
        }
    }
}

struct CaptureButton: View {
    let mode: ScreenshotCaptureMode
    let isCapturing: Bool
    let action: () -> Void

    var body: some View {
        Button(action: {
            logToFile("[CaptureButton] \(mode.rawValue) button PRESSED, isCapturing: \(isCapturing)")
            action()
        }) {
            VStack(spacing: 4) {
                Image(systemName: mode.icon)
                    .font(.title3)
                Text(mode.rawValue)
                    .font(.caption2)
            }
            .frame(width: 80, height: 50)
        }
        .buttonStyle(.bordered)
        .disabled(isCapturing)
        .keyboardShortcut(keyboardShortcutForMode)
        .help("\(mode.rawValue) (\(mode.shortcut))")
        .onAppear {
            logToFile("[CaptureButton] \(mode.rawValue) button APPEARED, isCapturing: \(isCapturing)")
        }
    }

    private var keyboardShortcutForMode: KeyboardShortcut {
        switch mode {
        case .fullScreen:
            return KeyboardShortcut("3", modifiers: [.command, .shift])
        case .selection:
            return KeyboardShortcut("4", modifiers: [.command, .shift])
        case .window:
            return KeyboardShortcut("5", modifiers: [.command, .shift])
        }
    }
}

// MARK: - Empty State

struct EmptyScreenshotsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "camera.viewfinder")
                .font(.system(size: 64))
                .foregroundColor(.secondary)

            Text("No Screenshots Yet")
                .font(.title2)
                .fontWeight(.medium)

            Text("Capture screenshots using the buttons above.\nThey will automatically sync to your QNAP NAS.")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)

            HStack(spacing: 16) {
                ForEach(ScreenshotCaptureMode.allCases) { mode in
                    VStack(spacing: 8) {
                        Button(action: {
                            Task {
                                await bridgeManager.screenshotService.captureScreenshot(mode: mode)
                            }
                        }) {
                            Image(systemName: mode.icon)
                                .font(.title)
                                .frame(width: 60, height: 60)
                        }
                        .buttonStyle(.bordered)

                        Text(mode.rawValue)
                            .font(.caption)
                        Text(mode.shortcut)
                            .font(.caption2)
                            .foregroundColor(.secondary)
                    }
                }
            }
            .padding(.top, 20)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}

// MARK: - Screenshot Grid

struct ScreenshotGrid: View {
    let screenshots: [CapturedScreenshot]
    @Binding var selectedScreenshot: CapturedScreenshot?
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        ScrollView {
            LazyVGrid(columns: [
                GridItem(.adaptive(minimum: 180, maximum: 220))
            ], spacing: 16) {
                ForEach(screenshots) { screenshot in
                    ScreenshotThumbnail(
                        screenshot: screenshot,
                        isSelected: selectedScreenshot?.id == screenshot.id
                    )
                    .onTapGesture {
                        selectedScreenshot = screenshot
                    }
                    .contextMenu {
                        Button(action: {
                            bridgeManager.screenshotService.copyToClipboard(screenshot)
                        }) {
                            Label("Copy", systemImage: "doc.on.doc")
                        }

                        Button(action: {
                            bridgeManager.screenshotService.openScreenshot(screenshot)
                        }) {
                            Label("Open", systemImage: "arrow.up.right.square")
                        }

                        Button(action: {
                            bridgeManager.screenshotService.revealInFinder(screenshot)
                        }) {
                            Label("Show in Finder", systemImage: "folder")
                        }

                        Divider()

                        if !screenshot.isSyncedToNAS {
                            Button(action: {
                                Task {
                                    await bridgeManager.screenshotService.uploadToNAS(screenshot)
                                }
                            }) {
                                Label("Upload to NAS", systemImage: "arrow.up.to.line")
                            }
                        }

                        Divider()

                        Button(role: .destructive, action: {
                            bridgeManager.screenshotService.deleteScreenshot(screenshot.id)
                            if selectedScreenshot?.id == screenshot.id {
                                selectedScreenshot = nil
                            }
                        }) {
                            Label("Delete", systemImage: "trash")
                        }
                    }
                }
            }
            .padding()
        }
    }
}

// MARK: - Screenshot Thumbnail

struct ScreenshotThumbnail: View {
    let screenshot: CapturedScreenshot
    let isSelected: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            // Thumbnail
            ZStack(alignment: .topTrailing) {
                if let image = NSImage(contentsOfFile: screenshot.localPath) {
                    Image(nsImage: image)
                        .resizable()
                        .aspectRatio(contentMode: .fill)
                        .frame(height: 120)
                        .clipped()
                        .cornerRadius(8)
                } else {
                    Rectangle()
                        .fill(Color.secondary.opacity(0.2))
                        .frame(height: 120)
                        .cornerRadius(8)
                        .overlay {
                            Image(systemName: "photo")
                                .font(.title)
                                .foregroundColor(.secondary)
                        }
                }

                // Sync status badge
                HStack(spacing: 4) {
                    if screenshot.isUploading {
                        ProgressView()
                            .scaleEffect(0.6)
                    } else if screenshot.isSyncedToNAS {
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(.green)
                    } else {
                        Image(systemName: "arrow.up.circle")
                            .foregroundColor(.orange)
                    }
                }
                .padding(6)
                .background(.ultraThinMaterial)
                .cornerRadius(12)
                .padding(6)
            }

            // Info
            VStack(alignment: .leading, spacing: 2) {
                Text(screenshot.filename)
                    .font(.caption)
                    .fontWeight(.medium)
                    .lineLimit(1)

                HStack {
                    Text(screenshot.capturedAt, style: .relative)
                    Text("•")
                    Text(screenshot.formattedSize)
                }
                .font(.caption2)
                .foregroundColor(.secondary)
            }
        }
        .padding(8)
        .background(isSelected ? Color.accentColor.opacity(0.1) : Color.secondary.opacity(0.05))
        .cornerRadius(12)
        .overlay(
            RoundedRectangle(cornerRadius: 12)
                .stroke(isSelected ? Color.accentColor : Color.clear, lineWidth: 2)
        )
    }
}

// MARK: - Screenshot Detail Panel

struct ScreenshotDetailPanel: View {
    let screenshot: CapturedScreenshot
    let onDelete: () -> Void
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                // Preview
                if let image = NSImage(contentsOfFile: screenshot.localPath) {
                    Image(nsImage: image)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .cornerRadius(8)
                        .onTapGesture(count: 2) {
                            bridgeManager.screenshotService.openScreenshot(screenshot)
                        }
                }

                // Actions
                HStack(spacing: 8) {
                    Button(action: {
                        bridgeManager.screenshotService.copyToClipboard(screenshot)
                    }) {
                        Label("Copy", systemImage: "doc.on.doc")
                    }

                    Button(action: {
                        bridgeManager.screenshotService.revealInFinder(screenshot)
                    }) {
                        Label("Finder", systemImage: "folder")
                    }

                    Spacer()

                    Button(role: .destructive, action: onDelete) {
                        Image(systemName: "trash")
                    }
                }
                .buttonStyle(.bordered)

                Divider()

                // Details
                Group {
                    DetailRow(label: "Filename", value: screenshot.filename)
                    DetailRow(label: "Resolution", value: screenshot.resolution)
                    DetailRow(label: "Size", value: screenshot.formattedSize)
                    DetailRow(label: "Captured", value: screenshot.capturedAt.formatted())

                    HStack {
                        Text("Synced to NAS")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Spacer()
                        if screenshot.isUploading {
                            ProgressView()
                                .scaleEffect(0.7)
                            Text("Uploading...")
                                .font(.caption)
                        } else if screenshot.isSyncedToNAS {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                            Text("Yes")
                                .font(.caption)
                        } else {
                            Button("Upload Now") {
                                Task {
                                    await bridgeManager.screenshotService.uploadToNAS(screenshot)
                                }
                            }
                            .buttonStyle(.bordered)
                            .controlSize(.small)
                        }
                    }
                }

                // OCR Text
                if let ocrText = screenshot.ocrText, !ocrText.isEmpty {
                    Divider()

                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("Detected Text")
                                .font(.headline)
                            Spacer()
                            Button(action: {
                                NSPasteboard.general.clearContents()
                                NSPasteboard.general.setString(ocrText, forType: .string)
                            }) {
                                Image(systemName: "doc.on.doc")
                            }
                            .buttonStyle(.borderless)
                            .help("Copy text")
                        }

                        Text(ocrText)
                            .font(.caption)
                            .padding(8)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(Color.secondary.opacity(0.1))
                            .cornerRadius(8)
                    }
                }

                // Tags
                if !screenshot.tags.isEmpty {
                    Divider()

                    VStack(alignment: .leading, spacing: 8) {
                        Text("Tags")
                            .font(.headline)

                        FlowLayout(spacing: 4) {
                            ForEach(screenshot.tags, id: \.self) { tag in
                                Text(tag)
                                    .font(.caption)
                                    .padding(.horizontal, 8)
                                    .padding(.vertical, 4)
                                    .background(Color.accentColor.opacity(0.2))
                                    .cornerRadius(8)
                            }
                        }
                    }
                }
            }
            .padding()
        }
        .background(Color.secondary.opacity(0.05))
    }
}

struct DetailRow: View {
    let label: String
    let value: String

    var body: some View {
        HStack {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
            Text(value)
                .font(.caption)
                .lineLimit(1)
        }
    }
}

// MARK: - Flow Layout for Tags

struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let result = FlowResult(in: proposal.width ?? 0, subviews: subviews, spacing: spacing)
        return result.size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = FlowResult(in: bounds.width, subviews: subviews, spacing: spacing)
        for (index, subview) in subviews.enumerated() {
            subview.place(at: CGPoint(x: bounds.minX + result.positions[index].x,
                                       y: bounds.minY + result.positions[index].y),
                         proposal: .unspecified)
        }
    }

    struct FlowResult {
        var size: CGSize = .zero
        var positions: [CGPoint] = []

        init(in width: CGFloat, subviews: Subviews, spacing: CGFloat) {
            var x: CGFloat = 0
            var y: CGFloat = 0
            var lineHeight: CGFloat = 0

            for subview in subviews {
                let size = subview.sizeThatFits(.unspecified)

                if x + size.width > width && x > 0 {
                    x = 0
                    y += lineHeight + spacing
                    lineHeight = 0
                }

                positions.append(CGPoint(x: x, y: y))
                lineHeight = max(lineHeight, size.height)
                x += size.width + spacing
            }

            self.size = CGSize(width: width, height: y + lineHeight)
        }
    }
}

// MARK: - Settings Sheet

struct ScreenshotSettingsSheet: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text("Screenshot Settings")
                    .font(.headline)
                Spacer()
                Button("Done") { dismiss() }
            }
            .padding()

            Divider()

            Form {
                Section("Capture Settings") {
                    Picker("Format", selection: $bridgeManager.screenshotService.screenshotFormat) {
                        Text("PNG").tag("png")
                        Text("JPEG").tag("jpg")
                        Text("TIFF").tag("tiff")
                    }

                    Toggle("Show preview before saving", isOn: $bridgeManager.screenshotService.previewEnabled)

                    Toggle("Perform OCR on capture", isOn: $bridgeManager.screenshotService.performOCR)
                }

                Section("Keyboard Shortcuts") {
                    VStack(alignment: .leading, spacing: 8) {
                        HotkeyRow(shortcut: "⌘⇧S", description: "Quick Selection Screenshot")
                        HotkeyRow(shortcut: "⌘⇧⌥A", description: "Full Screen Screenshot")
                        HotkeyRow(shortcut: "⌘⇧⌥W", description: "Window Screenshot")
                    }
                    .padding(.vertical, 4)

                    Toggle("Enable global hotkeys", isOn: $bridgeManager.hotkeyManager.isEnabled)
                }

                Section("Storage") {
                    HStack {
                        Text("Local Save Path")
                        Spacer()
                        Text(bridgeManager.screenshotService.localSavePath)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                        Button("Change...") {
                            selectLocalPath()
                        }
                    }

                    HStack {
                        Text("NAS Path")
                        Spacer()
                        TextField("", text: $bridgeManager.screenshotService.nasSavePath)
                            .textFieldStyle(.roundedBorder)
                            .frame(width: 150)
                    }
                }

                Section("Sync") {
                    Toggle("Auto-upload to NAS", isOn: $bridgeManager.screenshotService.autoUploadToNAS)

                    HStack {
                        Text("Screenshots")
                        Spacer()
                        Text("\(bridgeManager.screenshotService.screenshots.count) total")
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("Synced to NAS")
                        Spacer()
                        let synced = bridgeManager.screenshotService.screenshots.filter { $0.isSyncedToNAS }.count
                        Text("\(synced)")
                            .foregroundColor(.secondary)
                    }
                }

                Section {
                    Button("Open Screenshot Folder") {
                        NSWorkspace.shared.open(URL(fileURLWithPath: bridgeManager.screenshotService.localSavePath))
                    }
                }
            }
            .formStyle(.grouped)
            .padding()
        }
        .frame(width: 450, height: 500)
    }

    private func selectLocalPath() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false

        panel.begin { response in
            if response == .OK, let url = panel.url {
                bridgeManager.screenshotService.setLocalSavePath(url.path)
            }
        }
    }
}

struct HotkeyRow: View {
    let shortcut: String
    let description: String

    var body: some View {
        HStack {
            Text(shortcut)
                .font(.system(.body, design: .monospaced))
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(Color.secondary.opacity(0.2))
                .cornerRadius(4)
            Text(description)
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }
}

#Preview {
    ScreenshotsView()
        .environmentObject(BridgeManager())
        .frame(width: 800, height: 600)
}
