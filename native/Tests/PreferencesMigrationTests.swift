import Carbon
import XCTest

@MainActor
final class PreferencesMigrationTests: XCTestCase {
    private func defaults() throws -> UserDefaults {
        let name = "QRScanner.MigrationTests.\(UUID().uuidString)"
        let value = try XCTUnwrap(UserDefaults(suiteName: name))
        addTeardownBlock { UserDefaults.standard.removePersistentDomain(forName: name) }
        return value
    }

    func testMigrationPreservesShortcutAndWelcomeAndCanRunAgain() throws {
        let destination = try defaults()
        let source: [String: Any] = ["shortcut.code": UInt32(kVK_ANSI_S),
                                     "shortcut.modifiers": KeyboardShortcut.defaultValue.modifiers,
                                     "shortcut.label": "S", "hasShownWelcome": true, "screenAccess.requested": true]
        PreferencesMigration.apply(source, to: destination)
        let manager = HotKeyManager(defaults: destination)
        XCTAssertEqual(manager.shortcut.keyCode, UInt32(kVK_ANSI_S))
        XCTAssertEqual(manager.shortcut.keyLabel, "S")
        XCTAssertTrue(destination.bool(forKey: "hasShownWelcome"))
        destination.set(false, forKey: "hasShownWelcome")
        destination.set("Q", forKey: "shortcut.label")
        PreferencesMigration.apply(source, to: destination)
        XCTAssertFalse(destination.bool(forKey: "hasShownWelcome"))
        XCTAssertEqual(destination.string(forKey: "shortcut.label"), "Q")
    }

    func testCurrentPreferencesAndInvalidImportedValuesArePreserved() throws {
        let destination = try defaults()
        destination.set(false, forKey: "hasShownWelcome")
        let source: [String: Any] = ["shortcut.code": -1, "shortcut.modifiers": 0,
                                     "shortcut.label": "Invalid", "hasShownWelcome": true,
                                     "screenAccess.requested": "true", "unrelated": "discard"]
        PreferencesMigration.apply(source, to: destination)
        XCTAssertFalse(destination.bool(forKey: "hasShownWelcome"))
        XCTAssertNil(destination.object(forKey: "shortcut.code"))
        XCTAssertNil(destination.object(forKey: "screenAccess.requested"))
        XCTAssertNil(destination.object(forKey: "unrelated"))
        XCTAssertEqual(HotKeyManager(defaults: destination).shortcut, .defaultValue)
    }
}
