import XCTest

@MainActor
final class AppDelegateTests: XCTestCase {
    func testFailedRelaunchRestoresShortcutWithoutTerminating() async {
        var registrations = 0
        var releases = 0
        var terminations = 0
        let manager = HotKeyManager(registration: HotKeyRegistrationClient(installHandler: { _ in true }, register: { _ in
            registrations += 1
            return { releases += 1 }
        }, removeHandler: {}))
        manager.start()
        let succeeded = await AppDelegate.relaunch(hotKey: manager, openApplication: {
            XCTAssertFalse(manager.isRegistered)
            XCTAssertEqual(releases, 1)
            return false
        }, terminate: { terminations += 1 })
        XCTAssertFalse(succeeded)
        XCTAssertTrue(manager.isRegistered)
        XCTAssertEqual(registrations, 2)
        XCTAssertEqual(terminations, 0)
        manager.stop()
    }

    func testConfirmedRelaunchTerminatesAfterReleasingShortcut() async {
        var terminations = 0
        let manager = HotKeyManager(registration: HotKeyRegistrationClient(installHandler: { _ in true }, register: { _ in {} }, removeHandler: {}))
        manager.start()
        let succeeded = await AppDelegate.relaunch(hotKey: manager, openApplication: {
            XCTAssertFalse(manager.isRegistered)
            return true
        }, terminate: { terminations += 1 })
        XCTAssertTrue(succeeded)
        XCTAssertFalse(manager.isRegistered)
        XCTAssertEqual(terminations, 1)
    }
}
