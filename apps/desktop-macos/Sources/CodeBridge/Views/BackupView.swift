// BackupView - Backup Management UI for Code Bridge
// Displays backup jobs, history, and allows backup configuration

import SwiftUI

struct BackupView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @State private var selectedTab = 0
    @State private var showAddJob = false
    @State private var editingJob: BackupJob?

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Backup")
                        .font(.title2)
                        .fontWeight(.semibold)
                    Text("Backup your codebases to QNAP NAS")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                // Connection status indicator
                HStack(spacing: 6) {
                    Circle()
                        .fill(bridgeManager.nasConnectionStatus.color)
                        .frame(width: 8, height: 8)
                    Text(bridgeManager.nasConnectionStatus.rawValue)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
                .padding(.horizontal, 10)
                .padding(.vertical, 4)
                .background(Color.secondary.opacity(0.1))
                .cornerRadius(12)
            }
            .padding()

            // Tab selector
            Picker("", selection: $selectedTab) {
                Text("Jobs").tag(0)
                Text("History").tag(1)
                Text("Stats").tag(2)
            }
            .pickerStyle(.segmented)
            .padding(.horizontal)

            Divider()
                .padding(.top, 8)

            // Content
            switch selectedTab {
            case 0:
                BackupJobsView(
                    showAddJob: $showAddJob,
                    editingJob: $editingJob
                )
            case 1:
                BackupHistoryView()
            case 2:
                BackupStatsView()
            default:
                EmptyView()
            }
        }
        .sheet(isPresented: $showAddJob) {
            BackupJobEditor(job: nil) { newJob in
                bridgeManager.backupService.addJob(newJob)
            }
        }
        .sheet(item: $editingJob) { job in
            BackupJobEditor(job: job) { updatedJob in
                bridgeManager.backupService.updateJob(updatedJob)
            }
        }
    }
}

// MARK: - Backup Jobs View

struct BackupJobsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager
    @Binding var showAddJob: Bool
    @Binding var editingJob: BackupJob?

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 12) {
                // Current backup progress
                if bridgeManager.backupService.isBackupRunning,
                   let currentJob = bridgeManager.backupService.currentBackupJob {
                    CurrentBackupCard(
                        job: currentJob,
                        progress: bridgeManager.backupService.currentBackupProgress,
                        onCancel: {
                            bridgeManager.backupService.cancelCurrentBackup()
                        }
                    )
                }

                // Job list
                if bridgeManager.backupService.backupJobs.isEmpty {
                    EmptyBackupJobsView(onAdd: { showAddJob = true })
                } else {
                    ForEach(bridgeManager.backupService.backupJobs) { job in
                        BackupJobCard(
                            job: job,
                            connection: bridgeManager.nasConnections.first(where: { $0.id == job.destinationConnectionId }),
                            isRunning: bridgeManager.backupService.currentBackupJob?.id == job.id,
                            onRun: {
                                Task {
                                    await bridgeManager.backupService.runBackup(job)
                                }
                            },
                            onEdit: { editingJob = job },
                            onDelete: { bridgeManager.backupService.removeJob(job.id) },
                            onToggle: { bridgeManager.backupService.toggleJobEnabled(job.id) }
                        )
                    }
                }
            }
            .padding()
        }
        .overlay(alignment: .bottomTrailing) {
            Button(action: { showAddJob = true }) {
                Image(systemName: "plus")
                    .font(.title2)
                    .foregroundColor(.white)
                    .frame(width: 44, height: 44)
                    .background(Color.accentColor)
                    .clipShape(Circle())
                    .shadow(radius: 4)
            }
            .buttonStyle(.plain)
            .padding()
        }
    }
}

// MARK: - Current Backup Card

struct CurrentBackupCard: View {
    let job: BackupJob
    let progress: Double
    let onCancel: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Image(systemName: "arrow.triangle.2.circlepath")
                    .foregroundColor(.blue)
                Text("Backup in Progress")
                    .font(.headline)
                Spacer()
                Button("Cancel", action: onCancel)
                    .buttonStyle(.bordered)
            }

            Text(job.name)
                .font(.subheadline)
                .foregroundColor(.secondary)

            ProgressView(value: progress)
                .progressViewStyle(.linear)

            Text("\(Int(progress * 100))% complete")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color.blue.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Empty State

struct EmptyBackupJobsView: View {
    let onAdd: () -> Void

    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "externaldrive.badge.timemachine")
                .font(.system(size: 48))
                .foregroundColor(.secondary)

            Text("No Backup Jobs")
                .font(.title3)
                .fontWeight(.medium)

            Text("Create a backup job to automatically sync your codebases to your QNAP NAS")
                .font(.caption)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)

            Button(action: onAdd) {
                Label("Create Backup Job", systemImage: "plus")
            }
            .buttonStyle(.borderedProminent)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 60)
    }
}

// MARK: - Backup Job Card

struct BackupJobCard: View {
    let job: BackupJob
    let connection: QNAPConnection?
    let isRunning: Bool
    let onRun: () -> Void
    let onEdit: () -> Void
    let onDelete: () -> Void
    let onToggle: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            // Header
            HStack {
                Toggle("", isOn: .init(
                    get: { job.isEnabled },
                    set: { _ in onToggle() }
                ))
                .toggleStyle(.switch)
                .labelsHidden()

                VStack(alignment: .leading, spacing: 2) {
                    Text(job.name)
                        .font(.headline)
                    Text(job.backupSchedule.description)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                Spacer()

                HStack(spacing: 8) {
                    Button(action: onRun) {
                        Image(systemName: "play.fill")
                    }
                    .buttonStyle(.bordered)
                    .disabled(isRunning || !job.isEnabled)

                    Button(action: onEdit) {
                        Image(systemName: "pencil")
                    }
                    .buttonStyle(.bordered)

                    Button(action: onDelete) {
                        Image(systemName: "trash")
                    }
                    .buttonStyle(.bordered)
                    .tint(.red)
                }
            }

            Divider()

            // Source paths
            VStack(alignment: .leading, spacing: 4) {
                Text("Source Paths")
                    .font(.caption)
                    .foregroundColor(.secondary)

                if job.sourcePaths.isEmpty {
                    Text("No paths configured")
                        .font(.caption)
                        .foregroundColor(.orange)
                } else {
                    ForEach(job.sourcePaths, id: \.self) { path in
                        HStack(spacing: 4) {
                            Image(systemName: "folder.fill")
                                .font(.caption)
                            Text(path)
                                .font(.caption)
                                .lineLimit(1)
                                .truncationMode(.middle)
                        }
                    }
                }
            }

            // Destination
            VStack(alignment: .leading, spacing: 4) {
                Text("Destination")
                    .font(.caption)
                    .foregroundColor(.secondary)

                if let conn = connection {
                    HStack(spacing: 4) {
                        Image(systemName: "externaldrive.fill")
                            .font(.caption)
                        Text("\(conn.name): \(job.destinationPath)")
                            .font(.caption)
                            .lineLimit(1)
                    }
                } else {
                    Text("No destination configured")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }

            // Last run status
            if let lastRun = job.lastRunAt {
                HStack {
                    Image(systemName: job.lastStatus.icon)
                        .foregroundColor(job.lastStatus.color)
                    Text("Last run: \(lastRun.formatted(date: .abbreviated, time: .shortened))")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding()
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(12)
        .opacity(job.isEnabled ? 1 : 0.6)
    }
}

// MARK: - Backup History View

struct BackupHistoryView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 8) {
                if bridgeManager.backupService.backupHistory.isEmpty {
                    VStack(spacing: 12) {
                        Image(systemName: "clock.arrow.circlepath")
                            .font(.system(size: 36))
                            .foregroundColor(.secondary)
                        Text("No Backup History")
                            .font(.headline)
                        Text("Completed backups will appear here")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 60)
                } else {
                    ForEach(bridgeManager.backupService.backupHistory) { entry in
                        BackupHistoryRow(entry: entry)
                    }
                }
            }
            .padding()
        }
        .overlay(alignment: .bottomTrailing) {
            if !bridgeManager.backupService.backupHistory.isEmpty {
                Button(action: { bridgeManager.backupService.clearHistory() }) {
                    Label("Clear History", systemImage: "trash")
                        .font(.caption)
                }
                .buttonStyle(.bordered)
                .padding()
            }
        }
    }
}

struct BackupHistoryRow: View {
    let entry: BackupHistoryEntry

    var body: some View {
        HStack {
            Image(systemName: entry.status.icon)
                .foregroundColor(entry.status.color)
                .frame(width: 24)

            VStack(alignment: .leading, spacing: 2) {
                Text(entry.jobName)
                    .font(.subheadline)
                Text(entry.startedAt.formatted(date: .abbreviated, time: .shortened))
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Spacer()

            VStack(alignment: .trailing, spacing: 2) {
                Text("\(entry.filesBackedUp) files")
                    .font(.caption)
                Text(entry.formattedSize)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }

            Text(entry.formattedDuration)
                .font(.caption)
                .foregroundColor(.secondary)
                .frame(width: 60, alignment: .trailing)
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(Color.secondary.opacity(0.05))
        .cornerRadius(8)
    }
}

// MARK: - Backup Stats View

struct BackupStatsView: View {
    @EnvironmentObject var bridgeManager: BridgeManager

    var stats: BackupStats {
        bridgeManager.backupService.backupStats
    }

    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Overview cards
                LazyVGrid(columns: [
                    GridItem(.flexible()),
                    GridItem(.flexible())
                ], spacing: 16) {
                    StatCard(
                        title: "Total Backups",
                        value: "\(stats.totalBackups)",
                        icon: "arrow.triangle.2.circlepath",
                        color: .blue
                    )

                    StatCard(
                        title: "Success Rate",
                        value: String(format: "%.1f%%", stats.successRate),
                        icon: "checkmark.circle",
                        color: .green
                    )

                    StatCard(
                        title: "Data Backed Up",
                        value: ByteCountFormatter.string(fromByteCount: stats.totalDataBackedUp, countStyle: .file),
                        icon: "externaldrive",
                        color: .purple
                    )

                    StatCard(
                        title: "Failed Backups",
                        value: "\(stats.failedBackups)",
                        icon: "xmark.circle",
                        color: .red
                    )
                }

                // Last backup info
                if let lastDate = stats.lastBackupDate {
                    HStack {
                        Image(systemName: "clock")
                            .foregroundColor(.secondary)
                        Text("Last backup: \(lastDate.formatted(date: .abbreviated, time: .shortened))")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.secondary.opacity(0.05))
                    .cornerRadius(12)
                }

                // Reset button
                Button(action: { bridgeManager.backupService.resetStats() }) {
                    Label("Reset Statistics", systemImage: "arrow.counterclockwise")
                }
                .buttonStyle(.bordered)
            }
            .padding()
        }
    }
}

struct StatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color

    var body: some View {
        VStack(spacing: 8) {
            Image(systemName: icon)
                .font(.title)
                .foregroundColor(color)

            Text(value)
                .font(.title2)
                .fontWeight(.bold)

            Text(title)
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity)
        .padding()
        .background(color.opacity(0.1))
        .cornerRadius(12)
    }
}

// MARK: - Backup Job Editor

struct BackupJobEditor: View {
    let job: BackupJob?
    let onSave: (BackupJob) -> Void

    @EnvironmentObject var bridgeManager: BridgeManager
    @Environment(\.dismiss) private var dismiss

    @State private var name: String = ""
    @State private var sourcePaths: [String] = []
    @State private var destinationConnectionId: String = ""
    @State private var destinationPath: String = "/Backups/CodeBridge"
    @State private var schedule: BackupSchedule = .daily
    @State private var isEnabled: Bool = true
    @State private var excludePatterns: [String] = [".git", "node_modules", "target", ".build"]
    @State private var retainVersions: Int = 5
    @State private var compressBackups: Bool = true
    @State private var newExcludePattern: String = ""

    var body: some View {
        VStack(spacing: 0) {
            // Header
            HStack {
                Text(job == nil ? "Create Backup Job" : "Edit Backup Job")
                    .font(.headline)
                Spacer()
                Button("Cancel") { dismiss() }
                    .keyboardShortcut(.escape, modifiers: [])
            }
            .padding()

            Divider()

            // Form
            Form {
                Section("Basic Settings") {
                    TextField("Job Name", text: $name)
                        .textFieldStyle(.roundedBorder)

                    Picker("Schedule", selection: $schedule) {
                        ForEach(BackupSchedule.allCases) { sched in
                            Text(sched.rawValue).tag(sched)
                        }
                    }

                    Toggle("Enabled", isOn: $isEnabled)
                }

                Section("Source Paths") {
                    if sourcePaths.isEmpty {
                        Text("No source paths added")
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(sourcePaths, id: \.self) { path in
                            HStack {
                                Image(systemName: "folder.fill")
                                Text(path)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                                Spacer()
                                Button(action: {
                                    sourcePaths.removeAll { $0 == path }
                                }) {
                                    Image(systemName: "xmark.circle.fill")
                                        .foregroundColor(.secondary)
                                }
                                .buttonStyle(.plain)
                            }
                        }
                    }

                    Button(action: selectSourcePath) {
                        Label("Add Source Path", systemImage: "plus")
                    }
                }

                Section("Destination") {
                    if bridgeManager.nasConnections.isEmpty {
                        Text("No NAS connections configured. Add one in Settings → NAS.")
                            .foregroundColor(.orange)
                            .font(.caption)
                    } else {
                        Picker("NAS Connection", selection: $destinationConnectionId) {
                            Text("Select...").tag("")
                            ForEach(bridgeManager.nasConnections) { conn in
                                Text(conn.name).tag(conn.id)
                            }
                        }

                        TextField("Destination Path", text: $destinationPath)
                            .textFieldStyle(.roundedBorder)
                    }
                }

                Section("Exclude Patterns") {
                    ForEach(excludePatterns, id: \.self) { pattern in
                        HStack {
                            Text(pattern)
                                .font(.system(.body, design: .monospaced))
                            Spacer()
                            Button(action: {
                                excludePatterns.removeAll { $0 == pattern }
                            }) {
                                Image(systemName: "xmark.circle.fill")
                                    .foregroundColor(.secondary)
                            }
                            .buttonStyle(.plain)
                        }
                    }

                    HStack {
                        TextField("Add pattern (e.g., *.log)", text: $newExcludePattern)
                            .textFieldStyle(.roundedBorder)
                        Button(action: {
                            if !newExcludePattern.isEmpty {
                                excludePatterns.append(newExcludePattern)
                                newExcludePattern = ""
                            }
                        }) {
                            Image(systemName: "plus.circle.fill")
                        }
                        .disabled(newExcludePattern.isEmpty)
                    }
                }

                Section("Advanced") {
                    Stepper("Retain \(retainVersions) versions", value: $retainVersions, in: 1...20)
                    Toggle("Compress backups", isOn: $compressBackups)
                }
            }
            .formStyle(.grouped)
            .padding()

            Divider()

            // Footer
            HStack {
                Spacer()
                Button("Save") {
                    let newJob = BackupJob(
                        id: job?.id ?? UUID().uuidString,
                        name: name,
                        sourcePaths: sourcePaths,
                        destinationConnectionId: destinationConnectionId,
                        destinationPath: destinationPath,
                        schedule: schedule,
                        isEnabled: isEnabled,
                        excludePatterns: excludePatterns,
                        lastRunAt: job?.lastRunAt,
                        lastStatus: job?.lastStatus ?? .idle,
                        retainVersions: retainVersions,
                        compressBackups: compressBackups
                    )
                    onSave(newJob)
                    dismiss()
                }
                .keyboardShortcut(.return, modifiers: [.command])
                .disabled(name.isEmpty || sourcePaths.isEmpty || destinationConnectionId.isEmpty)
            }
            .padding()
        }
        .frame(width: 500, height: 650)
        .onAppear {
            if let j = job {
                name = j.name
                sourcePaths = j.sourcePaths
                destinationConnectionId = j.destinationConnectionId
                destinationPath = j.destinationPath
                schedule = j.backupSchedule
                isEnabled = j.isEnabled
                excludePatterns = j.excludePatterns
                retainVersions = j.retainVersions
                compressBackups = j.compressBackups
            } else if let defaultConn = bridgeManager.nasConnections.first(where: { $0.isDefault }) {
                destinationConnectionId = defaultConn.id
            }
        }
    }

    private func selectSourcePath() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = true

        panel.begin { response in
            if response == .OK {
                for url in panel.urls {
                    let path = url.path
                    if !sourcePaths.contains(path) {
                        sourcePaths.append(path)
                    }
                }
            }
        }
    }
}

#Preview {
    BackupView()
        .environmentObject(BridgeManager())
        .frame(width: 600, height: 500)
}
