import AppKit
import Carbon
import XCTest

final class KeyboardShortcutTests: XCTestCase {
    func testDefaultShortcutIsValidAndMapsToCarbon() {
        let shortcut = KeyboardShortcut.defaultValue
        XCTAssertTrue(shortcut.isValid)
        XCTAssertEqual(shortcut.display, "⌃⌥Q")
        XCTAssertEqual(shortcut.carbonModifiers, UInt32(controlKey | optionKey))
    }

    func testBareKeysAndUnknownModifiersAreRefused() {
        XCTAssertFalse(KeyboardShortcut(keyCode: 12, modifiers: 0, keyLabel: "Q").isValid)
        XCTAssertFalse(KeyboardShortcut(keyCode: 999, modifiers: NSEvent.ModifierFlags.command.rawValue, keyLabel: "Q").isValid)
        XCTAssertFalse(KeyboardShortcut(keyCode: 12, modifiers: UInt.max, keyLabel: "Q").isValid)
    }

    func testControlCharacterLabelsAreRefused() {
        for label in ["\n", "\t", "Q\u{0000}"] {
            let shortcut = KeyboardShortcut(keyCode: 12, modifiers: NSEvent.ModifierFlags.command.rawValue, keyLabel: label)
            XCTAssertFalse(shortcut.isValid, "A saved control-character label must not become a shortcut: \(label.debugDescription)")
        }
    }

    @MainActor
    func testFailedReplacementRetainsBindingAndPreferences() throws {
        try withManager { manager, registration, defaults in
            manager.start()
            manager.beginRecording()
            registration.rejectRegistration = true
            XCTAssertFalse(manager.update(Self.customShortcut))
            XCTAssertEqual(manager.shortcut, .defaultValue)
            XCTAssertTrue(manager.isRegistered)
            XCTAssertEqual(manager.failedCandidate, Self.customShortcut)
            XCTAssertEqual(manager.recorderTitle, Self.customShortcut.display)
            XCTAssertEqual(registration.events, ["install", "register:⌃⌥Q", "register:⌃⌥S"])
            XCTAssertNil(defaults.object(forKey: "shortcut.code"))
            XCTAssertEqual(manager.helperText, "Your current shortcut ⌃⌥Q is still active.")
            manager.cancelRecording()
            XCTAssertEqual(manager.recordingState, .idle)
            XCTAssertEqual(manager.shortcut, .defaultValue)
            XCTAssertTrue(manager.isRegistered)
            XCTAssertNil(manager.error)
        }
    }

    @MainActor
    func testSuccessfulReplacementRegistersBeforeReleasingAndPersisting() throws {
        try withManager { manager, registration, defaults in
            manager.start()
            manager.beginRecording()
            XCTAssertTrue(manager.update(Self.customShortcut))
            XCTAssertEqual(registration.events, ["install", "register:⌃⌥Q", "register:⌃⌥S", "unregister:⌃⌥Q"])
            XCTAssertEqual(manager.shortcut, Self.customShortcut)
            XCTAssertTrue(manager.isRegistered)
            XCTAssertEqual(manager.recordingState, .idle)
            XCTAssertNil(manager.error)
            XCTAssertEqual(defaults.integer(forKey: "shortcut.code"), Int(Self.customShortcut.keyCode))
            XCTAssertEqual(defaults.string(forKey: "shortcut.label"), "S")
            XCTAssertFalse(manager.isDefaultActive)
        }
    }

    @MainActor
    func testFailedRestoreDefaultPreservesSavedCustomBinding() throws {
        try withManager { manager, registration, defaults in
            manager.start()
            XCTAssertTrue(manager.update(Self.customShortcut))
            registration.rejectRegistration = true
            XCTAssertFalse(manager.update(.defaultValue))
            XCTAssertEqual(manager.shortcut, Self.customShortcut)
            XCTAssertEqual(manager.failedCandidate, .defaultValue)
            XCTAssertTrue(manager.isRegistered)
            XCTAssertEqual(defaults.integer(forKey: "shortcut.code"), Int(Self.customShortcut.keyCode))
            XCTAssertEqual(defaults.integer(forKey: "shortcut.modifiers"), Int(Self.customShortcut.modifiers))
            XCTAssertEqual(defaults.string(forKey: "shortcut.label"), "S")
            XCTAssertEqual(manager.helperText, "Your current shortcut ⌃⌥S is still active.")
        }
    }

    @MainActor
    func testStartupRegistrationFailureNeverClaimsActiveBinding() throws {
        try withManager { manager, registration, defaults in
            registration.rejectRegistration = true
            manager.start()
            XCTAssertFalse(manager.isRegistered)
            XCTAssertFalse(manager.isDefaultActive)
            XCTAssertEqual(manager.failedCandidate, .defaultValue)
            XCTAssertEqual(manager.helperText, "No shortcut is active. Use Scan Screen in the menu bar.")
            XCTAssertNil(defaults.object(forKey: "shortcut.code"))
            registration.rejectRegistration = false
            XCTAssertTrue(manager.update(.defaultValue))
            XCTAssertTrue(manager.isDefaultActive)
        }
    }

    @MainActor
    func testHandlerFailureDoesNotRegisterOrPersistShortcut() throws {
        try withManager { manager, registration, defaults in
            registration.allowHandler = false
            manager.start()
            XCTAssertFalse(manager.isRegistered)
            XCTAssertEqual(registration.events, ["install"])
            XCTAssertNotNil(manager.error)
            XCTAssertNil(defaults.object(forKey: "shortcut.code"))
            registration.allowHandler = true
            XCTAssertTrue(manager.update(Self.customShortcut))
            XCTAssertTrue(manager.isRegistered)
        }
    }

    @MainActor
    func testRecordingConsumesExistingHotKeyWithoutScanning() throws {
        try withManager { manager, registration, _ in
            var scans = 0
            manager.onTrigger = { scans += 1 }
            manager.start()
            registration.trigger?()
            XCTAssertEqual(scans, 1)
            manager.beginRecording()
            registration.trigger?()
            XCTAssertEqual(scans, 1)
            XCTAssertEqual(manager.recordingState, .idle)
            XCTAssertEqual(registration.events, ["install", "register:⌃⌥Q"])
            registration.trigger?()
            XCTAssertEqual(scans, 2)
        }
    }

    @MainActor
    func testCancelAndInvalidInputDoNotReplaceActiveBinding() throws {
        try withManager { manager, registration, defaults in
            manager.start()
            manager.beginRecording()
            let invalid = KeyboardShortcut(keyCode: 12, modifiers: 0, keyLabel: "Q")
            XCTAssertFalse(manager.update(invalid))
            XCTAssertTrue(manager.isRecording)
            XCTAssertNotNil(manager.error)
            manager.cancelRecording()
            XCTAssertEqual(manager.recordingState, .idle)
            XCTAssertTrue(manager.isDefaultActive)
            XCTAssertNil(manager.error)
            XCTAssertEqual(registration.events, ["install", "register:⌃⌥Q"])
            XCTAssertNil(defaults.object(forKey: "shortcut.code"))
        }
    }

    @MainActor
    func testStartAndStopAreIdempotentAndLateTriggerCannotScan() throws {
        try withManager { manager, registration, _ in
            var scans = 0
            manager.onTrigger = { scans += 1 }
            manager.start()
            manager.start()
            manager.stop()
            manager.stop()
            registration.trigger?()
            XCTAssertEqual(scans, 0)
            XCTAssertFalse(manager.isRegistered)
            XCTAssertEqual(registration.events, ["install", "register:⌃⌥Q", "unregister:⌃⌥Q", "remove"])
        }
    }

    private static let customShortcut = KeyboardShortcut(
        keyCode: UInt32(kVK_ANSI_S), modifiers: NSEvent.ModifierFlags([.control, .option]).rawValue, keyLabel: "S")

    @MainActor
    private func withManager(
        name: String = #function,
        _ body: (HotKeyManager, RegistrationFixture, UserDefaults) throws -> Void
    ) throws {
        let domain = "com.p4suta.qrscanner.tests.\(name)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: domain))
        defaults.removePersistentDomain(forName: domain)
        defer { defaults.removePersistentDomain(forName: domain) }
        let registration = RegistrationFixture()
        let manager = HotKeyManager(defaults: defaults, registration: registration.client)
        defer { manager.stop() }
        try body(manager, registration, defaults)
    }
}

@MainActor
private final class RegistrationFixture {
    var rejectRegistration = false
    var allowHandler = true
    var events: [String] = []
    var trigger: (() -> Void)?

    var client: HotKeyRegistrationClient {
        HotKeyRegistrationClient(
            installHandler: { callback in
                self.events.append("install")
                guard self.allowHandler else { return false }
                self.trigger = callback
                return true
            },
            register: { shortcut in
                self.events.append("register:\(shortcut.display)")
                guard !self.rejectRegistration else { return nil }
                return { self.events.append("unregister:\(shortcut.display)") }
            },
            removeHandler: { self.events.append("remove") })
    }
}
