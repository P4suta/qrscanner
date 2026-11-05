import XCTest

@MainActor
final class PreferencesMigrationTests: XCTestCase {
    private func defaults() throws -> UserDefaults {
        let name = "QRScanner.MigrationTests.\(UUID().uuidString)"
        let value = try XCTUnwrap(UserDefaults(suiteName: name))
        addTeardownBlock { UserDefaults.standard.removePersistentDomain(forName: name) }
        return value
    }

    func testMigrationPreservesWelcomeAndPermissionAndCanRunAgain() throws {
        let destination = try defaults()
        let source: [String: Any] = ["hasShownWelcome": true, "screenAccess.requested": true]
        PreferencesMigration.apply(source, to: destination)
        XCTAssertTrue(destination.bool(forKey: "hasShownWelcome"))
        destination.set(false, forKey: "hasShownWelcome")
        PreferencesMigration.apply(source, to: destination)
        XCTAssertFalse(destination.bool(forKey: "hasShownWelcome"))
        XCTAssertTrue(destination.bool(forKey: "screenAccess.requested"))
    }

    func testCurrentPreferencesAndInvalidImportedValuesArePreserved() throws {
        let destination = try defaults()
        destination.set(false, forKey: "hasShownWelcome")
        let source: [String: Any] = ["hasShownWelcome": true,
                                     "screenAccess.requested": "true", "unrelated": "discard"]
        PreferencesMigration.apply(source, to: destination)
        XCTAssertFalse(destination.bool(forKey: "hasShownWelcome"))
        XCTAssertNil(destination.object(forKey: "screenAccess.requested"))
        XCTAssertNil(destination.object(forKey: "unrelated"))
    }
}
