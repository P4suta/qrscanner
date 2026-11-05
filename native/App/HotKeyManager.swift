import AppKit
import Carbon
import Combine

struct KeyboardShortcut: Equatable {
    let keyCode: UInt32
    let modifiers: UInt
    let keyLabel: String

    static let defaultValue = KeyboardShortcut(
        keyCode: UInt32(kVK_ANSI_Q), modifiers: NSEvent.ModifierFlags([.control, .option]).rawValue, keyLabel: "Q")

    var flags: NSEvent.ModifierFlags { NSEvent.ModifierFlags(rawValue: modifiers) }
    var display: String {
        let symbols: [(NSEvent.ModifierFlags, String)] = [(.control, "⌃"), (.option, "⌥"), (.shift, "⇧"), (.command, "⌘")]
        return symbols.filter { flags.contains($0.0) }.map(\.1).joined() + keyLabel
    }

    var isValid: Bool {
        let allowed: NSEvent.ModifierFlags = [.control, .option, .shift, .command]
        return keyCode < 128 && !keyLabel.isEmpty && keyLabel.count <= 16
            && !keyLabel.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) })
            && flags.isSubset(of: allowed) && !flags.isDisjoint(with: [.control, .option, .command])
    }

    var carbonModifiers: UInt32 {
        var result: UInt32 = 0
        if flags.contains(.command) { result |= UInt32(cmdKey) }
        if flags.contains(.control) { result |= UInt32(controlKey) }
        if flags.contains(.option) { result |= UInt32(optionKey) }
        if flags.contains(.shift) { result |= UInt32(shiftKey) }
        return result
    }
}

enum ShortcutRecordingState: Equatable {
    case idle
    case recording
    case unavailable(KeyboardShortcut)
}

@MainActor
struct HotKeyRegistrationClient {
    let installHandler: (@escaping () -> Void) -> Bool
    let register: (KeyboardShortcut) -> (() -> Void)?
    let removeHandler: () -> Void

    static var live: HotKeyRegistrationClient {
        let registration = CarbonHotKeyRegistration()
        return HotKeyRegistrationClient(
            installHandler: registration.installHandler,
            register: registration.register,
            removeHandler: registration.removeHandler)
    }
}

@MainActor
final class HotKeyManager: ObservableObject {
    @Published private(set) var shortcut: KeyboardShortcut
    @Published private(set) var error: String?
    @Published private(set) var isRegistered = false
    @Published private(set) var recordingState: ShortcutRecordingState = .idle
    var onTrigger: (() -> Void)?
    private var unregister: (() -> Void)?
    private var hasHandler = false
    private let registration: HotKeyRegistrationClient
    private let defaults: UserDefaults

    var isRecording: Bool { recordingState == .recording }
    var isDefaultActive: Bool { isRegistered && shortcut == .defaultValue }
    var failedCandidate: KeyboardShortcut? {
        guard case let .unavailable(candidate) = recordingState else { return nil }
        return candidate
    }
    var recorderTitle: String {
        isRecording ? String(localized: "Press shortcut…") : (failedCandidate ?? shortcut).display
    }
    var helperText: String {
        if isRecording { return String(localized: "Include Command, Control, or Option. Press Esc to cancel.") }
        if !isRegistered { return String(localized: "No shortcut is active. Use Scan Screen in the menu bar.") }
        if failedCandidate != nil { return String(localized: "Your current shortcut \(shortcut.display) is still active.") }
        return String(localized: "Use this shortcut to scan all visible displays, even when another app is active.")
    }

    init(defaults: UserDefaults = .standard, registration: HotKeyRegistrationClient = .live) {
        self.defaults = defaults
        self.registration = registration
        if let label = defaults.string(forKey: "shortcut.label"),
           let code = defaults.object(forKey: "shortcut.code") as? NSNumber,
           let modifiers = defaults.object(forKey: "shortcut.modifiers") as? NSNumber,
           let exactCode = UInt32(exactly: code.uint64Value),
           let exactModifiers = UInt(exactly: modifiers.uint64Value) {
            let saved = KeyboardShortcut(keyCode: exactCode, modifiers: exactModifiers, keyLabel: label)
            shortcut = saved.isValid ? saved : .defaultValue
            if !saved.isValid { error = String(localized: "The saved shortcut was invalid. The default shortcut has been restored.") }
        } else {
            shortcut = .defaultValue
        }
    }

    func start() {
        guard !isRegistered else { return }
        register(shortcut, persist: false)
    }

    @discardableResult
    func update(_ newShortcut: KeyboardShortcut) -> Bool {
        guard newShortcut.isValid else {
            reportInvalidKey()
            return false
        }
        if newShortcut.keyCode == shortcut.keyCode && newShortcut.modifiers == shortcut.modifiers && isRegistered {
            cancelRecording()
            return true
        }
        return register(newShortcut, persist: true)
    }

    func beginRecording() {
        error = nil
        recordingState = .recording
    }

    func cancelRecording() {
        recordingState = .idle
        error = nil
    }

    func reportInvalidKey() {
        error = String(localized: "Include Command, Control, or Option, then press a character key or Space.")
    }

    func stop() {
        unregister?()
        unregister = nil
        if hasHandler { registration.removeHandler() }
        hasHandler = false
        isRegistered = false
        cancelRecording()
    }

    private func receiveHotKey() {
        guard isRegistered else { return }
        if isRecording {
            cancelRecording()
        } else {
            onTrigger?()
        }
    }

    @discardableResult
    private func register(_ candidate: KeyboardShortcut, persist: Bool) -> Bool {
        if !hasHandler {
            hasHandler = registration.installHandler { [weak self] in self?.receiveHotKey() }
        }
        guard hasHandler else {
            recordingState = .unavailable(candidate)
            ProcessingLog.event(.shortcut, .failure)
            error = String(localized: "Couldn’t register the shortcut. Use Scan Screen in the menu bar.")
            return false
        }
        guard let releaseCandidate = registration.register(candidate) else {
            recordingState = .unavailable(candidate)
            ProcessingLog.event(.shortcut, .failure)
            error = String(localized: "\(candidate.display) is unavailable. Choose another shortcut.")
            return false
        }
        unregister?()
        unregister = releaseCandidate
        shortcut = candidate
        isRegistered = true
        ProcessingLog.event(.shortcut, .success)
        cancelRecording()
        if persist {
            defaults.set(candidate.keyCode, forKey: "shortcut.code")
            defaults.set(candidate.modifiers, forKey: "shortcut.modifiers")
            defaults.set(candidate.keyLabel, forKey: "shortcut.label")
        }
        return true
    }
}

@MainActor
private final class CarbonHotKeyRegistration {
    private var handler: EventHandlerRef?
    private var onTrigger: (() -> Void)?

    func installHandler(_ onTrigger: @escaping () -> Void) -> Bool {
        self.onTrigger = onTrigger
        var eventType = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let callback: EventHandlerUPP = { _, _, context in
            guard let context else { return OSStatus(eventNotHandledErr) }
            let registration = Unmanaged<CarbonHotKeyRegistration>.fromOpaque(context).takeUnretainedValue()
            MainActor.assumeIsolated { registration.onTrigger?() }
            return noErr
        }
        let status = InstallEventHandler(GetApplicationEventTarget(), callback, 1, &eventType,
                                        Unmanaged.passUnretained(self).toOpaque(), &handler)
        return status == noErr && handler != nil
    }

    func register(_ candidate: KeyboardShortcut) -> (() -> Void)? {
        var reference: EventHotKeyRef?
        let identity = EventHotKeyID(signature: 0x51525343, id: 1)
        let status = RegisterEventHotKey(candidate.keyCode, candidate.carbonModifiers, identity,
                                        GetApplicationEventTarget(), OptionBits(kEventHotKeyExclusive), &reference)
        guard status == noErr, let reference else { return nil }
        return { UnregisterEventHotKey(reference) }
    }

    func removeHandler() {
        if let handler { RemoveEventHandler(handler) }
        handler = nil
        onTrigger = nil
    }
}
