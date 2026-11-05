import AppKit
import Carbon
import SwiftUI

struct SettingsView: View {
    @ObservedObject var hotKey: HotKeyManager
    @AppStorage("textScale") private var textScale = 1.0
    @Environment(\.appTextScale) private var scale

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            Text("Keyboard Shortcut")
                .appFont(.headline)
                .accessibilityAddTraits(.isHeader)
            ViewThatFits(in: .horizontal) {
                HStack(spacing: 12) {
                    Text("Scan Screen")
                    Spacer()
                    ShortcutRecorder(hotKey: hotKey).frame(width: min(280, 144 * scale), height: 28 * scale)
                }
                VStack(alignment: .leading, spacing: 12) {
                    Text("Scan Screen")
                    ShortcutRecorder(hotKey: hotKey).frame(width: min(280, 144 * scale), height: 28 * scale)
                }
            }
            Text(hotKey.helperText)
                .appFont(.callout).foregroundStyle(.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if let error = hotKey.error {
                Label(error, systemImage: "exclamationmark.circle")
                    .appFont(.body).foregroundStyle(.primary)
                    .fixedSize(horizontal: false, vertical: true)
                    .accessibilityLabel(error)
            }
            NativeActions {
                if hotKey.recordingState != .idle {
                    NativeActionButton("Cancel") { hotKey.cancelRecording() }
                    if hotKey.failedCandidate != nil {
                        NativeActionButton("Try Another Shortcut", primary: true) { hotKey.beginRecording() }
                    }
                } else {
                    NativeActionButton("Restore Default") { hotKey.update(.defaultValue) }
                        .disabled(hotKey.isDefaultActive)
                }
            }
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Text Size")
                    Spacer()
                    Text(textScale, format: .percent)
                }
                Slider(value: $textScale, in: 1...2, step: 0.25) {
                    Text("Text Size")
                }
                .labelsHidden()
                .accessibilityValue(Text(textScale, format: .percent))
                .accessibilityIdentifier("textSize")
            }
        }
        .appFont(.body)
        .padding(20)
        .frame(minWidth: 320, idealWidth: 480, maxWidth: 560)
        .navigationTitle("QRScanner Settings")
        .onExitCommand { hotKey.cancelRecording() }
        .onDisappear { hotKey.cancelRecording() }
    }
}

private struct ShortcutRecorder: NSViewRepresentable {
    @ObservedObject var hotKey: HotKeyManager
    @Environment(\.appTextScale) private var scale

    func makeNSView(context: Context) -> RecorderButton {
        let button = RecorderButton()
        button.manager = hotKey
        button.bezelStyle = .flexiblePush
        button.controlSize = .regular
        button.font = .appFont(.body, scale: scale)
        button.focusRingType = .exterior
        button.setAccessibilityElement(true)
        button.setAccessibilityRole(.button)
        button.target = button
        button.action = #selector(RecorderButton.beginRecording)
        button.setAccessibilityLabel(String(localized: "Scan Screen keyboard shortcut"))
        button.setAccessibilityIdentifier("shortcutRecorder")
        return button
    }

    func updateNSView(_ nsView: RecorderButton, context: Context) {
        nsView.title = hotKey.recorderTitle
        nsView.controlSize = scale > 1.5 ? .extraLarge : (scale > 1 ? .large : .regular)
        nsView.font = .appFont(.body, scale: scale)
        nsView.setAccessibilityValue(hotKey.recorderTitle)
        nsView.setAccessibilityHelp(hotKey.helperText)
        if hotKey.isRecording, nsView.window?.firstResponder !== nsView {
            nsView.window?.makeFirstResponder(nsView)
        }
    }
}

@MainActor
private final class RecorderButton: NSButton {
    weak var manager: HotKeyManager?
    override var acceptsFirstResponder: Bool { true }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        NotificationCenter.default.removeObserver(self, name: NSWindow.didResignKeyNotification, object: nil)
        if let window {
            NotificationCenter.default.addObserver(self, selector: #selector(windowResignedKey),
                                                   name: NSWindow.didResignKeyNotification, object: window)
        }
    }

    @objc private func windowResignedKey() {
        if manager?.isRecording == true { manager?.cancelRecording() }
        refreshTitle()
    }

    @objc func beginRecording() {
        manager?.beginRecording()
        refreshTitle()
        window?.makeFirstResponder(self)
    }

    override func resignFirstResponder() -> Bool {
        if manager?.isRecording == true { manager?.cancelRecording() }
        refreshTitle()
        return super.resignFirstResponder()
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        guard manager?.isRecording == true else { return super.performKeyEquivalent(with: event) }
        keyDown(with: event)
        return true
    }

    override func keyDown(with event: NSEvent) {
        guard let manager, manager.isRecording else { super.keyDown(with: event); return }
        if event.keyCode == UInt16(kVK_Escape) {
            manager.cancelRecording()
            refreshTitle()
            return
        }
        let flags = event.modifierFlags.intersection([.command, .control, .option, .shift])
        if event.keyCode == UInt16(kVK_Tab), flags.isSubset(of: [.shift]) {
            manager.cancelRecording()
            refreshTitle()
            if flags.contains(.shift) { window?.selectPreviousKeyView(self) } else { window?.selectNextKeyView(self) }
            return
        }
        let label = event.keyCode == UInt16(kVK_Space) ? "Space" : event.charactersIgnoringModifiers?.uppercased()
        guard let label, !label.isEmpty,
              !label.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }) else {
            manager.reportInvalidKey()
            return
        }
        let shortcut = KeyboardShortcut(keyCode: UInt32(event.keyCode), modifiers: flags.rawValue, keyLabel: label)
        manager.update(shortcut)
        refreshTitle()
    }

    private func refreshTitle() {
        title = manager?.recorderTitle ?? String(localized: "Record Shortcut")
        setAccessibilityValue(title)
        setAccessibilityHelp(manager?.helperText)
    }
}
