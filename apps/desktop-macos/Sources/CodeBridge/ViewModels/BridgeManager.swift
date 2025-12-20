// Bridge Manager - Main ViewModel
// Connects SwiftUI to the Rust core library

import Foundation
import SwiftUI

@MainActor
class BridgeManager: ObservableObject {
    // Published state
    @Published var deviceName: String = "Mac mini"
    @Published var deviceId: String = ""
    @Published var peerCount: Int = 0
    @Published var fileCount: Int = 0
    @Published var isSyncing: Bool = false
    @Published var isConnected: Bool = false

    @Published var peers: [Peer] = []
    @Published var files: [TrackedFile] = []
    @Published var screenshots: [Screenshot] = []
    @Published var recentActivity: [String] = []

    // Configuration
    @Published var watchPaths: [String] = []
    @Published var ignorePatterns: [String] = [".git", "node_modules", "target"]

    init() {
        // Initialize bridge
        Task {
            await initializeBridge()
        }
    }

    func initializeBridge() async {
        // TODO: Initialize Rust bridge via FFI
        // For now, use mock data

        deviceName = Host.current().localizedName ?? "Unknown"
        deviceId = UUID().uuidString.prefix(8).uppercased()
        peerCount = 0
        fileCount = 0
        isConnected = true

        // Add sample activity
        recentActivity = [
            "Bridge initialized",
            "Listening for peers...",
        ]
    }

    func sync() {
        isSyncing = true
        recentActivity.insert("Sync started", at: 0)

        // Simulate sync
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            await MainActor.run {
                self.isSyncing = false
                self.recentActivity.insert("Sync complete", at: 0)
            }
        }
    }

    func addFiles() {
        let panel = NSOpenPanel()
        panel.canChooseFiles = true
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = true

        panel.begin { response in
            if response == .OK {
                for url in panel.urls {
                    self.addPath(url.path)
                }
            }
        }
    }

    func addPath(_ path: String) {
        // TODO: Add to Rust core
        recentActivity.insert("Added: \(path)", at: 0)
        fileCount += 1
    }

    func discoverPeers() {
        recentActivity.insert("Discovering peers...", at: 0)

        // Simulate discovery
        Task {
            try? await Task.sleep(nanoseconds: 3_000_000_000)
            await MainActor.run {
                self.recentActivity.insert("Discovery complete", at: 0)
            }
        }
    }

    func removePeer(_ peer: Peer) {
        peers.removeAll { $0.id == peer.id }
        peerCount = peers.count
        recentActivity.insert("Removed peer: \(peer.name)", at: 0)
    }

    func shareFile(_ file: TrackedFile) {
        recentActivity.insert("Sharing: \(file.name)", at: 0)
    }
}

// Data Models

struct Peer: Identifiable, Hashable {
    let id: String
    let name: String
    let addresses: [String]
    var isConnected: Bool
    var lastSeen: Date
}

struct TrackedFile: Identifiable, Hashable {
    let id: String
    let name: String
    let path: String
    let size: Int64
    let hash: String
    let mimeType: String?
    let modifiedAt: Date
}

struct Screenshot: Identifiable, Hashable {
    let id: String
    let name: String
    let path: String
    let size: Int64
    let capturedAt: Date
    let ocrText: String?
}
