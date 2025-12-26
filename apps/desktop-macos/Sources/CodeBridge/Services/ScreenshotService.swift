// ScreenshotService - Screenshot Capture and Management
// Captures screenshots and syncs them to QNAP NAS

import Foundation
import AppKit
import Vision
import os.log

private let logger = Logger(subsystem: "com.codebridge.app", category: "Screenshot")

/// Capture mode for screenshots
enum ScreenshotCaptureMode: String, CaseIterable, Identifiable {
    case fullScreen = "Full Screen"
    case selection = "Selection"
    case window = "Window"

    var id: String { rawValue }

    var icon: String {
        switch self {
        case .fullScreen: return "rectangle.dashed"
        case .selection: return "crop"
        case .window: return "macwindow"
        }
    }

    var shortcut: String {
        switch self {
        case .fullScreen: return "⌘⇧3"
        case .selection: return "⌘⇧4"
        case .window: return "⌘⇧4 + Space"
        }
    }
}

/// Screenshot with metadata
struct CapturedScreenshot: Identifiable, Codable {
    let id: String
    let filename: String
    let localPath: String
    var remotePath: String?
    let capturedAt: Date
    let width: Int
    let height: Int
    let fileSize: Int64
    var ocrText: String?
    var tags: [String]
    var isSyncedToNAS: Bool
    var isUploading: Bool

    init(
        id: String = UUID().uuidString,
        filename: String,
        localPath: String,
        remotePath: String? = nil,
        capturedAt: Date = Date(),
        width: Int = 0,
        height: Int = 0,
        fileSize: Int64 = 0,
        ocrText: String? = nil,
        tags: [String] = [],
        isSyncedToNAS: Bool = false,
        isUploading: Bool = false
    ) {
        self.id = id
        self.filename = filename
        self.localPath = localPath
        self.remotePath = remotePath
        self.capturedAt = capturedAt
        self.width = width
        self.height = height
        self.fileSize = fileSize
        self.ocrText = ocrText
        self.tags = tags
        self.isSyncedToNAS = isSyncedToNAS
        self.isUploading = isUploading
    }

    var formattedSize: String {
        ByteCountFormatter.string(fromByteCount: fileSize, countStyle: .file)
    }

    var resolution: String {
        "\(width) × \(height)"
    }
}

/// Pending screenshot awaiting confirmation
struct PendingScreenshot {
    let id: String
    let tempPath: String
    let image: NSImage
    let width: Int
    let height: Int
    let fileSize: Int64
    let capturedAt: Date
    var ocrText: String?

    init(tempPath: String, image: NSImage, width: Int, height: Int, fileSize: Int64, ocrText: String? = nil) {
        self.id = UUID().uuidString
        self.tempPath = tempPath
        self.image = image
        self.width = width
        self.height = height
        self.fileSize = fileSize
        self.capturedAt = Date()
        self.ocrText = ocrText
    }
}

/// Service for capturing and managing screenshots
@MainActor
class ScreenshotService: ObservableObject {
    // MARK: - Published State
    @Published var screenshots: [CapturedScreenshot] = []
    @Published var isCapturing: Bool = false
    @Published var lastCapturedScreenshot: CapturedScreenshot?
    @Published var autoUploadToNAS: Bool = true
    @Published var performOCR: Bool = true
    @Published var screenshotFormat: String = "png"
    @Published var localSavePath: String
    @Published var nasSavePath: String = "/Screenshots"

    // Preview mode state
    @Published var showPreview: Bool = false
    @Published var pendingScreenshot: PendingScreenshot?
    @Published var previewEnabled: Bool = true  // User setting to enable/disable preview

    // MARK: - Dependencies
    private let qnapService: QNAPService
    private let getConnections: () -> [QNAPConnection]

    // MARK: - Persistence
    private let screenshotsKey = "CodeBridge.Screenshots"
    private let settingsKey = "CodeBridge.ScreenshotSettings"

    init(qnapService: QNAPService, connections: @escaping () -> [QNAPConnection]) {
        self.qnapService = qnapService
        self.getConnections = connections

        // Default local save path
        let desktopPath = FileManager.default.urls(for: .desktopDirectory, in: .userDomainMask).first?.path ?? "~/Desktop"
        self.localSavePath = "\(desktopPath)/CodeBridge Screenshots"

        // Create local screenshot directory
        try? FileManager.default.createDirectory(atPath: localSavePath, withIntermediateDirectories: true)

        loadScreenshots()
        loadSettings()

        NSLog("[ScreenshotService] Initialized - isCapturing: %d, previewEnabled: %d, screenshots count: %d",
              isCapturing, previewEnabled, screenshots.count)
    }

    // MARK: - Screenshot Capture

    /// Capture a screenshot with the specified mode
    /// If previewEnabled is true, shows a preview before saving
    func captureScreenshot(mode: ScreenshotCaptureMode, skipPreview: Bool = false) async -> CapturedScreenshot? {
        isCapturing = true
        logger.info("[Screenshot] Starting capture, mode: \(mode.rawValue), previewEnabled: \(self.previewEnabled), skipPreview: \(skipPreview)")
        NSLog("[Screenshot] Starting capture, mode: %@", mode.rawValue)

        // Capture to temp location first if preview is enabled
        let shouldPreview = previewEnabled && !skipPreview
        let tempPath = shouldPreview ? NSTemporaryDirectory() + "codebridge_preview_\(UUID().uuidString).\(screenshotFormat)" : nil
        logger.info("[Screenshot] shouldPreview: \(shouldPreview), tempPath: \(tempPath ?? "nil")")
        NSLog("[Screenshot] shouldPreview: %d", shouldPreview)

        // Generate filename with timestamp
        let timestamp = DateFormatter.localizedString(from: Date(), dateStyle: .none, timeStyle: .medium)
            .replacingOccurrences(of: ":", with: "-")
        let dateStr = DateFormatter.localizedString(from: Date(), dateStyle: .short, timeStyle: .none)
            .replacingOccurrences(of: "/", with: "-")
        let filename = "Screenshot \(dateStr) at \(timestamp).\(screenshotFormat)"
        let filepath = tempPath ?? "\(localSavePath)/\(filename)"

        // Build screencapture command
        var arguments: [String] = []

        switch mode {
        case .fullScreen:
            arguments = ["-x", filepath]  // -x for no sound
        case .selection:
            arguments = ["-x", "-i", filepath]  // -i for interactive selection
        case .window:
            arguments = ["-x", "-i", "-w", filepath]  // -w for window mode
        }

        // Add format option
        if screenshotFormat != "png" {
            arguments.insert(contentsOf: ["-t", screenshotFormat], at: 0)
        }

        // Execute screencapture
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
        process.arguments = arguments
        NSLog("[Screenshot] Running screencapture with args: %@", arguments.joined(separator: " "))
        NSLog("[Screenshot] filepath: %@", filepath)

        do {
            try process.run()
            NSLog("[Screenshot] Process started, waiting...")

            // Wait for process asynchronously to not block main thread
            await withCheckedContinuation { continuation in
                DispatchQueue.global(qos: .userInitiated).async {
                    process.waitUntilExit()
                    DispatchQueue.main.async {
                        continuation.resume()
                    }
                }
            }

            NSLog("[Screenshot] screencapture exited with code: %d", process.terminationStatus)

            // Check if file was created (user might have cancelled)
            let fileExists = FileManager.default.fileExists(atPath: filepath)
            NSLog("[Screenshot] File exists at %@: %d", filepath, fileExists)
            guard fileExists else {
                NSLog("[Screenshot] File not created, user likely cancelled")
                isCapturing = false
                return nil
            }

            // Get file attributes
            let attrs = try FileManager.default.attributesOfItem(atPath: filepath)
            let fileSize = attrs[.size] as? Int64 ?? 0

            // Get image
            guard let image = NSImage(contentsOfFile: filepath) else {
                isCapturing = false
                return nil
            }
            let width = Int(image.size.width)
            let height = Int(image.size.height)

            // If preview is enabled, show preview window
            if shouldPreview {
                NSLog("[Screenshot] Creating pending screenshot for preview")
                var pending = PendingScreenshot(
                    tempPath: filepath,
                    image: image,
                    width: width,
                    height: height,
                    fileSize: fileSize
                )

                // Perform OCR in background
                if performOCR {
                    NSLog("[Screenshot] Performing OCR...")
                    pending.ocrText = await performOCROnImage(filepath)
                }

                NSLog("[Screenshot] Setting pendingScreenshot and showPreview = true")
                pendingScreenshot = pending
                showPreview = true
                isCapturing = false

                NSLog("[Screenshot] Preview should now be visible, showPreview: %d", showPreview)
                // Return nil here - actual screenshot will be created on confirmation
                return nil
            }

            // Direct save mode (preview disabled)
            var screenshot = CapturedScreenshot(
                filename: filename,
                localPath: filepath,
                width: width,
                height: height,
                fileSize: fileSize
            )

            // Perform OCR if enabled
            if performOCR {
                screenshot.ocrText = await performOCROnImage(filepath)
            }

            // Add to list
            screenshots.insert(screenshot, at: 0)
            lastCapturedScreenshot = screenshot
            saveScreenshots()

            // Auto-upload to NAS if enabled
            if autoUploadToNAS {
                Task {
                    await uploadToNAS(screenshot)
                }
            }

            isCapturing = false
            return screenshot

        } catch {
            print("Screenshot capture failed: \(error)")
            isCapturing = false
            return nil
        }
    }

    // MARK: - Preview Actions

    /// Confirm and save the pending screenshot
    func confirmPendingScreenshot(uploadToNAS: Bool = true) async -> CapturedScreenshot? {
        NSLog("[Screenshot] confirmPendingScreenshot called, uploadToNAS: %d", uploadToNAS)
        guard let pending = pendingScreenshot else {
            NSLog("[Screenshot] No pending screenshot to confirm")
            return nil
        }

        // Generate final filename
        let timestamp = DateFormatter.localizedString(from: pending.capturedAt, dateStyle: .none, timeStyle: .medium)
            .replacingOccurrences(of: ":", with: "-")
        let dateStr = DateFormatter.localizedString(from: pending.capturedAt, dateStyle: .short, timeStyle: .none)
            .replacingOccurrences(of: "/", with: "-")
        let filename = "Screenshot \(dateStr) at \(timestamp).\(screenshotFormat)"
        let finalPath = "\(localSavePath)/\(filename)"

        do {
            // Move from temp to final location
            NSLog("[Screenshot] Moving file from %@ to %@", pending.tempPath, finalPath)
            try FileManager.default.moveItem(atPath: pending.tempPath, toPath: finalPath)
            NSLog("[Screenshot] File moved successfully")

            // Create screenshot record
            let screenshot = CapturedScreenshot(
                filename: filename,
                localPath: finalPath,
                width: pending.width,
                height: pending.height,
                fileSize: pending.fileSize,
                ocrText: pending.ocrText
            )

            // Add to list
            screenshots.insert(screenshot, at: 0)
            lastCapturedScreenshot = screenshot
            saveScreenshots()

            // Clear preview state
            pendingScreenshot = nil
            showPreview = false

            // Upload to NAS if requested
            if uploadToNAS && autoUploadToNAS {
                NSLog("[Screenshot] Starting NAS upload for %@", filename)
                Task {
                    await self.uploadToNAS(screenshot)
                }
            } else {
                NSLog("[Screenshot] NAS upload skipped - uploadToNAS: %d, autoUploadToNAS: %d", uploadToNAS, autoUploadToNAS)
            }

            return screenshot

        } catch {
            NSLog("[Screenshot] Failed to save: %@", error.localizedDescription)
            return nil
        }
    }

    /// Cancel the pending screenshot (discard temp file)
    func cancelPendingScreenshot() {
        guard let pending = pendingScreenshot else { return }

        // Delete temp file
        try? FileManager.default.removeItem(atPath: pending.tempPath)

        // Clear preview state
        pendingScreenshot = nil
        showPreview = false
    }

    /// Copy pending screenshot to clipboard without saving
    func copyPendingToClipboard() {
        guard let pending = pendingScreenshot else { return }

        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.writeObjects([pending.image])
    }

    /// Capture full screen screenshot (convenience method)
    func captureFullScreen() async -> CapturedScreenshot? {
        await captureScreenshot(mode: .fullScreen)
    }

    /// Capture selection screenshot (convenience method)
    func captureSelection() async -> CapturedScreenshot? {
        await captureScreenshot(mode: .selection)
    }

    /// Capture window screenshot (convenience method)
    func captureWindow() async -> CapturedScreenshot? {
        await captureScreenshot(mode: .window)
    }

    // MARK: - OCR

    private func performOCROnImage(_ imagePath: String) async -> String? {
        guard let image = NSImage(contentsOfFile: imagePath),
              let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
            return nil
        }

        return await withCheckedContinuation { continuation in
            let request = VNRecognizeTextRequest { request, error in
                guard error == nil,
                      let observations = request.results as? [VNRecognizedTextObservation] else {
                    continuation.resume(returning: nil)
                    return
                }

                let text = observations.compactMap { observation in
                    observation.topCandidates(1).first?.string
                }.joined(separator: "\n")

                continuation.resume(returning: text.isEmpty ? nil : text)
            }

            request.recognitionLevel = .accurate
            request.usesLanguageCorrection = true

            let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
            do {
                try handler.perform([request])
            } catch {
                continuation.resume(returning: nil)
            }
        }
    }

    // MARK: - NAS Upload

    /// Upload a screenshot to NAS
    func uploadToNAS(_ screenshot: CapturedScreenshot) async {
        guard let defaultConnection = getConnections().first(where: { $0.isDefault }) else {
            print("[NAS Upload] No default NAS connection configured")
            return
        }

        print("[NAS Upload] Starting upload for: \(screenshot.filename)")

        // Mark as uploading
        if let index = screenshots.firstIndex(where: { $0.id == screenshot.id }) {
            screenshots[index].isUploading = true
        }

        // Try direct SMB copy first (most reliable)
        let smbMountPath = "/Volumes/Coachly"
        let smbDestDir = "\(smbMountPath)\(nasSavePath)"
        let smbDestPath = "\(smbDestDir)/\(screenshot.filename)"

        // Check if SMB volume is mounted
        if FileManager.default.fileExists(atPath: smbMountPath) {
            print("[NAS Upload] SMB volume mounted at \(smbMountPath), using direct copy")

            do {
                // Create Screenshots directory if needed
                if !FileManager.default.fileExists(atPath: smbDestDir) {
                    print("[NAS Upload] Creating directory: \(smbDestDir)")
                    try FileManager.default.createDirectory(atPath: smbDestDir, withIntermediateDirectories: true)
                }

                // Check if file already exists
                if FileManager.default.fileExists(atPath: smbDestPath) {
                    print("[NAS Upload] File already exists, removing old version")
                    try FileManager.default.removeItem(atPath: smbDestPath)
                }

                // Copy file
                print("[NAS Upload] Copying \(screenshot.localPath) to \(smbDestPath)")
                try FileManager.default.copyItem(atPath: screenshot.localPath, toPath: smbDestPath)

                print("[NAS Upload] SUCCESS - File uploaded to NAS")

                // Update screenshot record
                if let index = screenshots.firstIndex(where: { $0.id == screenshot.id }) {
                    screenshots[index].isSyncedToNAS = true
                    screenshots[index].isUploading = false
                    screenshots[index].remotePath = smbDestPath
                    saveScreenshots()
                }
                return

            } catch {
                print("[NAS Upload] Direct copy failed: \(error)")
            }
        } else {
            print("[NAS Upload] SMB volume not mounted at \(smbMountPath)")
        }

        // Try SCP upload (works when SMB isn't mounted)
        NSLog("[NAS Upload] Trying SCP upload...")
        let scpResult = await uploadViaSCP(
            localPath: screenshot.localPath,
            remotePath: "/share\(defaultConnection.sharePath)\(nasSavePath)/\(screenshot.filename)",
            host: defaultConnection.host,
            username: defaultConnection.username,
            password: defaultConnection.password
        )

        if scpResult {
            print("[NAS Upload] SUCCESS via SCP")
            if let index = screenshots.firstIndex(where: { $0.id == screenshot.id }) {
                screenshots[index].isSyncedToNAS = true
                screenshots[index].isUploading = false
                screenshots[index].remotePath = "\(defaultConnection.sharePath)\(nasSavePath)/\(screenshot.filename)"
                saveScreenshots()
            }
            return
        }

        print("[NAS Upload] SCP failed, falling back to QNAPService API")
        do {
            // Connect if not connected
            if qnapService.connectionStatus != .connected {
                try await qnapService.connect(using: defaultConnection)
            }

            // Create screenshots directory on NAS if needed
            let remotePath = "\(defaultConnection.sharePath)\(nasSavePath)"
            try? await qnapService.createDirectory(
                path: defaultConnection.sharePath,
                name: nasSavePath.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            )

            // Upload file
            try await qnapService.uploadFile(
                localPath: screenshot.localPath,
                remotePath: remotePath
            ) { progress in
                print("[NAS Upload] Progress: \(progress * 100)%")
            }

            // Update screenshot record
            if let index = screenshots.firstIndex(where: { $0.id == screenshot.id }) {
                screenshots[index].isSyncedToNAS = true
                screenshots[index].isUploading = false
                screenshots[index].remotePath = "\(remotePath)/\(screenshot.filename)"
                saveScreenshots()
            }
            print("[NAS Upload] SUCCESS via API")

        } catch {
            print("[NAS Upload] FAILED: \(error)")
            if let index = screenshots.firstIndex(where: { $0.id == screenshot.id }) {
                screenshots[index].isUploading = false
            }
        }
    }

    /// Upload file via SCP using expect script to handle password
    private func uploadViaSCP(localPath: String, remotePath: String, host: String, username: String, password: String) async -> Bool {
        NSLog("[SCP Upload] Uploading %@ to %@@%@:%@", localPath, username, host, remotePath)

        // Create expect script for SCP with password
        // Escape quotes in paths for the expect script
        let escapedLocalPath = localPath.replacingOccurrences(of: "\"", with: "\\\"")
        let escapedRemotePath = remotePath.replacingOccurrences(of: "\"", with: "\\\"")

        let expectScript = """
        set timeout 60
        spawn scp -o StrictHostKeyChecking=no "\(escapedLocalPath)" "\(username)@\(host):\(escapedRemotePath)"
        expect {
            "password:" {
                send "\(password)\\r"
                expect {
                    "100%" { exit 0 }
                    "Permission denied" { exit 1 }
                    timeout { exit 2 }
                    eof { exit 0 }
                }
            }
            timeout { exit 2 }
        }
        """

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/expect")
        process.arguments = ["-c", expectScript]

        let pipe = Pipe()
        process.standardOutput = pipe
        process.standardError = pipe

        do {
            try process.run()

            // Wait asynchronously
            await withCheckedContinuation { continuation in
                DispatchQueue.global(qos: .userInitiated).async {
                    process.waitUntilExit()
                    DispatchQueue.main.async {
                        continuation.resume()
                    }
                }
            }

            let outputData = pipe.fileHandleForReading.readDataToEndOfFile()
            let output = String(data: outputData, encoding: .utf8) ?? ""
            NSLog("[SCP Upload] Output: %@", String(output.prefix(500)))

            if process.terminationStatus == 0 {
                NSLog("[SCP Upload] Success!")
                return true
            } else {
                NSLog("[SCP Upload] Failed with status: %d", process.terminationStatus)
                return false
            }
        } catch {
            NSLog("[SCP Upload] Error: %@", error.localizedDescription)
            return false
        }
    }

    /// Upload all unsynced screenshots to NAS
    func uploadAllToNAS() async {
        let unsynced = screenshots.filter { !$0.isSyncedToNAS && !$0.isUploading }
        for screenshot in unsynced {
            await uploadToNAS(screenshot)
        }
    }

    // MARK: - Screenshot Management

    /// Delete a screenshot
    func deleteScreenshot(_ id: String, deleteFile: Bool = true) {
        if let index = screenshots.firstIndex(where: { $0.id == id }) {
            let screenshot = screenshots[index]

            // Delete local file if requested
            if deleteFile {
                try? FileManager.default.removeItem(atPath: screenshot.localPath)
            }

            screenshots.remove(at: index)
            saveScreenshots()
        }
    }

    /// Add tags to a screenshot
    func addTags(_ id: String, tags: [String]) {
        if let index = screenshots.firstIndex(where: { $0.id == id }) {
            screenshots[index].tags.append(contentsOf: tags)
            saveScreenshots()
        }
    }

    /// Search screenshots by OCR text or filename
    func searchScreenshots(_ query: String) -> [CapturedScreenshot] {
        guard !query.isEmpty else { return screenshots }

        let lowercased = query.lowercased()
        return screenshots.filter { screenshot in
            screenshot.filename.lowercased().contains(lowercased) ||
            screenshot.ocrText?.lowercased().contains(lowercased) == true ||
            screenshot.tags.contains { $0.lowercased().contains(lowercased) }
        }
    }

    /// Open screenshot in Finder
    func revealInFinder(_ screenshot: CapturedScreenshot) {
        NSWorkspace.shared.selectFile(screenshot.localPath, inFileViewerRootedAtPath: "")
    }

    /// Copy screenshot to clipboard
    func copyToClipboard(_ screenshot: CapturedScreenshot) {
        guard let image = NSImage(contentsOfFile: screenshot.localPath) else { return }

        let pasteboard = NSPasteboard.general
        pasteboard.clearContents()
        pasteboard.writeObjects([image])
    }

    /// Open screenshot with default app
    func openScreenshot(_ screenshot: CapturedScreenshot) {
        NSWorkspace.shared.open(URL(fileURLWithPath: screenshot.localPath))
    }

    // MARK: - Settings

    func setLocalSavePath(_ path: String) {
        localSavePath = path
        try? FileManager.default.createDirectory(atPath: path, withIntermediateDirectories: true)
        saveSettings()
    }

    // MARK: - Persistence

    private func loadScreenshots() {
        if let data = UserDefaults.standard.data(forKey: screenshotsKey),
           let saved = try? JSONDecoder().decode([CapturedScreenshot].self, from: data) {
            // Filter out screenshots whose files no longer exist
            screenshots = saved.filter { FileManager.default.fileExists(atPath: $0.localPath) }
        }
    }

    private func saveScreenshots() {
        if let data = try? JSONEncoder().encode(screenshots) {
            UserDefaults.standard.set(data, forKey: screenshotsKey)
        }
    }

    private func loadSettings() {
        if let data = UserDefaults.standard.data(forKey: settingsKey),
           let settings = try? JSONDecoder().decode(ScreenshotSettings.self, from: data) {
            autoUploadToNAS = settings.autoUploadToNAS
            performOCR = settings.performOCR
            screenshotFormat = settings.screenshotFormat
            if !settings.localSavePath.isEmpty {
                localSavePath = settings.localSavePath
            }
            nasSavePath = settings.nasSavePath
            previewEnabled = settings.previewEnabled
        }
    }

    private func saveSettings() {
        let settings = ScreenshotSettings(
            autoUploadToNAS: autoUploadToNAS,
            performOCR: performOCR,
            screenshotFormat: screenshotFormat,
            localSavePath: localSavePath,
            nasSavePath: nasSavePath,
            previewEnabled: previewEnabled
        )
        if let data = try? JSONEncoder().encode(settings) {
            UserDefaults.standard.set(data, forKey: settingsKey)
        }
    }
}

// MARK: - Settings Model

private struct ScreenshotSettings: Codable {
    var autoUploadToNAS: Bool
    var performOCR: Bool
    var screenshotFormat: String
    var localSavePath: String
    var nasSavePath: String
    var previewEnabled: Bool

    init(autoUploadToNAS: Bool, performOCR: Bool, screenshotFormat: String, localSavePath: String, nasSavePath: String, previewEnabled: Bool = true) {
        self.autoUploadToNAS = autoUploadToNAS
        self.performOCR = performOCR
        self.screenshotFormat = screenshotFormat
        self.localSavePath = localSavePath
        self.nasSavePath = nasSavePath
        self.previewEnabled = previewEnabled
    }
}

// MARK: - NSImage Extension

extension NSImage {
    var cgImage: CGImage? {
        cgImage(forProposedRect: nil, context: nil, hints: nil)
    }
}
