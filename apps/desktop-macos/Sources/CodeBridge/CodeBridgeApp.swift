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
        .commands {
            // Screenshot menu commands
            CommandMenu("Screenshots") {
                Button("Capture Full Screen") {
                    Task {
                        await bridgeManager.screenshotService.captureFullScreen()
                    }
                }
                .keyboardShortcut("3", modifiers: [.command, .shift])

                Button("Capture Selection") {
                    Task {
                        await bridgeManager.screenshotService.captureSelection()
                    }
                }
                .keyboardShortcut("4", modifiers: [.command, .shift])

                Button("Capture Window") {
                    Task {
                        await bridgeManager.screenshotService.captureWindow()
                    }
                }
                .keyboardShortcut("5", modifiers: [.command, .shift])

                Divider()

                Button("Upload All to NAS") {
                    Task {
                        await bridgeManager.screenshotService.uploadAllToNAS()
                    }
                }
                .keyboardShortcut("u", modifiers: [.command, .shift])
            }
        }

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
