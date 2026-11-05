import SwiftUI

@main
struct QRScannerApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @AppStorage("textScale") private var textScale = 1.0

    var body: some Scene {
        MenuBarExtra("QRScanner", systemImage: "qrcode.viewfinder") {
            ScannerMenu(model: delegate.model, hotKey: delegate.hotKey)
        }
        Settings {
            SettingsView(hotKey: delegate.hotKey)
                .environment(\.appTextScale, textScale)
        }
        .windowResizability(.contentSize)
    }
}

private struct ScannerMenu: View {
    @ObservedObject var model: AppModel
    @ObservedObject var hotKey: HotKeyManager

    var body: some View {
        Group {
            if hotKey.isRegistered, let key = menuKey {
                Button("Scan Screen", action: model.scan).keyboardShortcut(key, modifiers: menuModifiers)
            } else {
                Button("Scan Screen", action: model.scan)
            }
        }
        .disabled(hotKey.isRecording)
        Divider()
        SettingsLink().keyboardShortcut(",", modifiers: .command)
        Divider()
        Button("Quit QRScanner") { NSApplication.shared.terminate(nil) }.keyboardShortcut("q")
    }

    private var menuKey: KeyEquivalent? {
        if hotKey.shortcut.keyLabel == "Space" { return .space }
        return hotKey.shortcut.keyLabel.lowercased().first.map { KeyEquivalent($0) }
    }

    private var menuModifiers: EventModifiers {
        var result: EventModifiers = []
        if hotKey.shortcut.flags.contains(.command) { result.insert(.command) }
        if hotKey.shortcut.flags.contains(.control) { result.insert(.control) }
        if hotKey.shortcut.flags.contains(.option) { result.insert(.option) }
        if hotKey.shortcut.flags.contains(.shift) { result.insert(.shift) }
        return result
    }
}
