// HotkeyManager - Global Hotkey Support
// Registers system-wide keyboard shortcuts for quick actions

import Foundation
import AppKit
import Carbon.HIToolbox

/// Manager for global keyboard shortcuts
@MainActor
class HotkeyManager: ObservableObject {
    // MARK: - Published State
    @Published var isEnabled: Bool = true
    @Published var lastTriggeredHotkey: String?

    // MARK: - Callbacks
    var onScreenshotHotkey: ((ScreenshotCaptureMode) -> Void)?

    // MARK: - Private
    private var globalMonitor: Any?
    private var localMonitor: Any?

    // Hotkey configurations
    struct Hotkey {
        let key: UInt16
        let modifiers: NSEvent.ModifierFlags
        let action: String
        let description: String
    }

    private let hotkeys: [Hotkey] = [
        // ⌘⇧S - Quick screenshot (selection mode)
        Hotkey(key: UInt16(kVK_ANSI_S), modifiers: [.command, .shift], action: "screenshot_selection", description: "⌘⇧S - Quick Selection Screenshot"),
        // ⌘⇧A - Full screen screenshot
        Hotkey(key: UInt16(kVK_ANSI_A), modifiers: [.command, .shift, .option], action: "screenshot_fullscreen", description: "⌘⇧⌥A - Full Screen Screenshot"),
        // ⌘⇧W - Window screenshot
        Hotkey(key: UInt16(kVK_ANSI_W), modifiers: [.command, .shift, .option], action: "screenshot_window", description: "⌘⇧⌥W - Window Screenshot"),
    ]

    init() {
        setupMonitors()
    }

    // MARK: - Setup

    private func setupMonitors() {
        // Global monitor for when app is in background
        globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
            Task { @MainActor in
                self?.handleKeyEvent(event)
            }
        }

        // Local monitor for when app is in foreground
        localMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            Task { @MainActor in
                self?.handleKeyEvent(event)
            }
            return event
        }
    }

    func removeMonitors() {
        if let monitor = globalMonitor {
            NSEvent.removeMonitor(monitor)
            globalMonitor = nil
        }
        if let monitor = localMonitor {
            NSEvent.removeMonitor(monitor)
            localMonitor = nil
        }
    }

    // MARK: - Event Handling

    private func handleKeyEvent(_ event: NSEvent) {
        guard isEnabled else { return }

        for hotkey in hotkeys {
            if event.keyCode == hotkey.key && event.modifierFlags.intersection(.deviceIndependentFlagsMask) == hotkey.modifiers {
                triggerAction(hotkey.action)
                lastTriggeredHotkey = hotkey.description
                return
            }
        }
    }

    private func triggerAction(_ action: String) {
        switch action {
        case "screenshot_selection":
            onScreenshotHotkey?(.selection)
        case "screenshot_fullscreen":
            onScreenshotHotkey?(.fullScreen)
        case "screenshot_window":
            onScreenshotHotkey?(.window)
        default:
            break
        }
    }

    // MARK: - Public Methods

    func enable() {
        isEnabled = true
    }

    func disable() {
        isEnabled = false
    }

    /// Get list of registered hotkeys for display
    var registeredHotkeys: [(shortcut: String, description: String)] {
        hotkeys.map { ($0.description.components(separatedBy: " - ").first ?? "", $0.description.components(separatedBy: " - ").last ?? "") }
    }
}
