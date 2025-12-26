// BackupService - Backup Management for Code Bridge
// Manages backup jobs, scheduling, and execution

import Foundation

/// Service for managing backups to QNAP NAS
@MainActor
class BackupService: ObservableObject {
    // MARK: - Published State
    @Published var backupJobs: [BackupJob] = []
    @Published var backupHistory: [BackupHistoryEntry] = []
    @Published var backupStats: BackupStats = BackupStats()
    @Published var currentBackupProgress: Double = 0
    @Published var currentBackupJob: BackupJob?
    @Published var isBackupRunning: Bool = false
    @Published var lastError: String?

    // MARK: - Dependencies
    private let qnapService: QNAPService
    private let connections: () -> [QNAPConnection]
    private var scheduledTimers: [String: Timer] = [:]

    // MARK: - Persistence Keys
    private let backupJobsKey = "CodeBridge.BackupJobs"
    private let backupHistoryKey = "CodeBridge.BackupHistory"
    private let backupStatsKey = "CodeBridge.BackupStats"

    init(qnapService: QNAPService, connections: @escaping () -> [QNAPConnection]) {
        self.qnapService = qnapService
        self.connections = connections
        loadPersistedData()
        setupScheduledBackups()
    }

    // MARK: - Job Management

    /// Add a new backup job
    func addJob(_ job: BackupJob) {
        backupJobs.append(job)
        saveJobs()
        scheduleJob(job)
    }

    /// Update an existing backup job
    func updateJob(_ job: BackupJob) {
        if let index = backupJobs.firstIndex(where: { $0.id == job.id }) {
            // Cancel existing schedule
            cancelSchedule(for: job.id)

            backupJobs[index] = job
            saveJobs()

            // Reschedule if enabled
            if job.isEnabled {
                scheduleJob(job)
            }
        }
    }

    /// Remove a backup job
    func removeJob(_ jobId: String) {
        cancelSchedule(for: jobId)
        backupJobs.removeAll { $0.id == jobId }
        saveJobs()
    }

    /// Toggle job enabled state
    func toggleJobEnabled(_ jobId: String) {
        if let index = backupJobs.firstIndex(where: { $0.id == jobId }) {
            backupJobs[index].isEnabled.toggle()
            saveJobs()

            if backupJobs[index].isEnabled {
                scheduleJob(backupJobs[index])
            } else {
                cancelSchedule(for: jobId)
            }
        }
    }

    // MARK: - Backup Execution

    /// Run a backup job immediately
    func runBackup(_ job: BackupJob) async {
        guard !isBackupRunning else {
            lastError = "A backup is already running"
            return
        }

        isBackupRunning = true
        currentBackupJob = job
        currentBackupProgress = 0
        lastError = nil

        let startTime = Date()
        var filesBackedUp = 0
        var totalSize: Int64 = 0
        var errorMessage: String?

        do {
            // Get the connection for this job
            guard let connection = connections().first(where: { $0.id == job.destinationConnectionId }) else {
                throw BackupError.connectionNotFound
            }

            // Connect to NAS if not already connected
            if qnapService.connectionStatus != .connected {
                try await qnapService.connect(using: connection)
            }

            // Ensure backup directory exists
            try await qnapService.createDirectory(
                path: connection.sharePath,
                name: job.destinationPath.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            )

            // Calculate total files for progress
            let allFiles = try await collectFilesToBackup(job: job)
            let totalFiles = allFiles.count

            // Backup each file
            for (index, fileURL) in allFiles.enumerated() {
                // Check if file matches exclude patterns
                if shouldExclude(file: fileURL, patterns: job.excludePatterns) {
                    continue
                }

                let relativePath = makeRelativePath(fileURL: fileURL, sourcePaths: job.sourcePaths)
                let remotePath = "\(connection.sharePath)/\(job.destinationPath)/\(relativePath)"

                try await qnapService.uploadFile(
                    localPath: fileURL.path,
                    remotePath: remotePath
                ) { progress in
                    // Update overall progress
                    let fileProgress = Double(index) / Double(totalFiles)
                    let subProgress = progress / Double(totalFiles)
                    Task { @MainActor in
                        self.currentBackupProgress = fileProgress + subProgress
                    }
                }

                filesBackedUp += 1
                if let size = try? FileManager.default.attributesOfItem(atPath: fileURL.path)[.size] as? Int64 {
                    totalSize += size
                }
            }

            currentBackupProgress = 1.0

            // Update job status
            if let index = backupJobs.firstIndex(where: { $0.id == job.id }) {
                backupJobs[index].lastRunAt = Date()
                backupJobs[index].lastStatus = .completed
                saveJobs()
            }

        } catch {
            errorMessage = error.localizedDescription
            lastError = errorMessage

            // Update job status to failed
            if let index = backupJobs.firstIndex(where: { $0.id == job.id }) {
                backupJobs[index].lastRunAt = Date()
                backupJobs[index].lastStatus = .failed
                saveJobs()
            }
        }

        // Record history
        let historyEntry = BackupHistoryEntry(
            id: UUID().uuidString,
            jobId: job.id,
            jobName: job.name,
            startedAt: startTime,
            completedAt: Date(),
            status: errorMessage == nil ? .completed : .failed,
            filesBackedUp: filesBackedUp,
            totalSize: totalSize,
            errorMessage: errorMessage
        )

        backupHistory.insert(historyEntry, at: 0)

        // Keep only last 100 history entries
        if backupHistory.count > 100 {
            backupHistory = Array(backupHistory.prefix(100))
        }

        saveHistory()

        // Update stats
        updateStats(entry: historyEntry)

        isBackupRunning = false
        currentBackupJob = nil
        currentBackupProgress = 0
    }

    /// Cancel the currently running backup
    func cancelCurrentBackup() {
        if let job = currentBackupJob {
            isBackupRunning = false
            currentBackupProgress = 0
            currentBackupJob = nil

            if let index = backupJobs.firstIndex(where: { $0.id == job.id }) {
                backupJobs[index].lastStatus = .paused
                saveJobs()
            }
        }
    }

    // MARK: - File Collection

    private func collectFilesToBackup(job: BackupJob) async throws -> [URL] {
        var allFiles: [URL] = []
        let fileManager = FileManager.default

        for sourcePath in job.sourcePaths {
            let sourceURL = URL(fileURLWithPath: sourcePath)

            guard fileManager.fileExists(atPath: sourcePath) else {
                continue
            }

            var isDirectory: ObjCBool = false
            fileManager.fileExists(atPath: sourcePath, isDirectory: &isDirectory)

            if isDirectory.boolValue {
                // Enumerate directory contents
                if let enumerator = fileManager.enumerator(
                    at: sourceURL,
                    includingPropertiesForKeys: [.isRegularFileKey, .fileSizeKey],
                    options: [.skipsHiddenFiles]
                ) {
                    for case let fileURL as URL in enumerator {
                        if !shouldExclude(file: fileURL, patterns: job.excludePatterns) {
                            var isFile: ObjCBool = false
                            if fileManager.fileExists(atPath: fileURL.path, isDirectory: &isFile),
                               !isFile.boolValue {
                                allFiles.append(fileURL)
                            }
                        } else {
                            // Skip excluded directories entirely
                            enumerator.skipDescendants()
                        }
                    }
                }
            } else {
                allFiles.append(sourceURL)
            }
        }

        return allFiles
    }

    private func shouldExclude(file: URL, patterns: [String]) -> Bool {
        let filename = file.lastPathComponent
        let path = file.path

        for pattern in patterns {
            // Check filename match
            if filename == pattern {
                return true
            }

            // Check glob-style pattern
            if pattern.contains("*") {
                let regex = pattern
                    .replacingOccurrences(of: ".", with: "\\.")
                    .replacingOccurrences(of: "*", with: ".*")

                if let _ = filename.range(of: regex, options: .regularExpression) {
                    return true
                }
            }

            // Check if path contains the pattern
            if path.contains("/\(pattern)/") || path.hasSuffix("/\(pattern)") {
                return true
            }
        }

        return false
    }

    private func makeRelativePath(fileURL: URL, sourcePaths: [String]) -> String {
        for sourcePath in sourcePaths {
            let sourceURL = URL(fileURLWithPath: sourcePath)
            if fileURL.path.hasPrefix(sourceURL.path) {
                var relative = String(fileURL.path.dropFirst(sourceURL.path.count))
                if relative.hasPrefix("/") {
                    relative = String(relative.dropFirst())
                }
                return relative
            }
        }
        return fileURL.lastPathComponent
    }

    // MARK: - Scheduling

    private func setupScheduledBackups() {
        for job in backupJobs where job.isEnabled {
            scheduleJob(job)
        }
    }

    private func scheduleJob(_ job: BackupJob) {
        guard job.isEnabled else { return }

        cancelSchedule(for: job.id)

        let interval: TimeInterval
        switch job.backupSchedule {
        case .manual:
            return // No automatic scheduling
        case .hourly:
            interval = 3600
        case .daily:
            interval = 86400
        case .weekly:
            interval = 604800
        }

        let timer = Timer.scheduledTimer(withTimeInterval: interval, repeats: true) { [weak self] _ in
            Task { @MainActor [weak self] in
                guard let self = self else { return }
                if let currentJob = self.backupJobs.first(where: { $0.id == job.id }) {
                    await self.runBackup(currentJob)
                }
            }
        }

        scheduledTimers[job.id] = timer
    }

    private func cancelSchedule(for jobId: String) {
        scheduledTimers[jobId]?.invalidate()
        scheduledTimers.removeValue(forKey: jobId)
    }

    // MARK: - Statistics

    private func updateStats(entry: BackupHistoryEntry) {
        backupStats.totalBackups += 1

        if entry.status == .completed {
            backupStats.successfulBackups += 1
        } else {
            backupStats.failedBackups += 1
        }

        backupStats.totalDataBackedUp += entry.totalSize
        backupStats.lastBackupDate = entry.completedAt

        saveStats()
    }

    /// Clear backup history
    func clearHistory() {
        backupHistory.removeAll()
        saveHistory()
    }

    /// Reset statistics
    func resetStats() {
        backupStats = BackupStats()
        saveStats()
    }

    // MARK: - Persistence

    private func loadPersistedData() {
        // Load backup jobs
        if let data = UserDefaults.standard.data(forKey: backupJobsKey),
           let jobs = try? JSONDecoder().decode([BackupJob].self, from: data) {
            backupJobs = jobs
        }

        // Load backup history
        if let data = UserDefaults.standard.data(forKey: backupHistoryKey),
           let history = try? JSONDecoder().decode([BackupHistoryEntry].self, from: data) {
            backupHistory = history
        }

        // Load backup stats
        if let data = UserDefaults.standard.data(forKey: backupStatsKey),
           let stats = try? JSONDecoder().decode(BackupStats.self, from: data) {
            backupStats = stats
        }
    }

    private func saveJobs() {
        if let data = try? JSONEncoder().encode(backupJobs) {
            UserDefaults.standard.set(data, forKey: backupJobsKey)
        }
    }

    private func saveHistory() {
        if let data = try? JSONEncoder().encode(backupHistory) {
            UserDefaults.standard.set(data, forKey: backupHistoryKey)
        }
    }

    private func saveStats() {
        if let data = try? JSONEncoder().encode(backupStats) {
            UserDefaults.standard.set(data, forKey: backupStatsKey)
        }
    }
}

// MARK: - Backup Errors

enum BackupError: LocalizedError {
    case connectionNotFound
    case backupInProgress
    case noFilesToBackup
    case cancelled

    var errorDescription: String? {
        switch self {
        case .connectionNotFound:
            return "NAS connection not found for this backup job"
        case .backupInProgress:
            return "A backup is already in progress"
        case .noFilesToBackup:
            return "No files found to backup"
        case .cancelled:
            return "Backup was cancelled"
        }
    }
}
