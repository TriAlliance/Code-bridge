// Coachly Code Bridge - macOS App
// Native SwiftUI application with Rust core integration

import SwiftUI

@main
struct CodeBridgeApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) var appDelegate
    @StateObject private var bridgeManager = BridgeManager()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(bridgeManager)
        }
        .windowStyle(.hiddenTitleBar)
        .windowResizability(.contentSize)

        // Menu bar extra
        MenuBarExtra {
            MenuBarView()
                .environmentObject(bridgeManager)
        } label: {
            Image(systemName: "arrow.triangle.2.circlepath")
        }
        .menuBarExtraStyle(.window)

        // Settings window
        Settings {
            SettingsView()
                .environmentObject(bridgeManager)
        }
    }
}

class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        print("Code Bridge started")
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        return false // Keep running in menu bar
    }
}
