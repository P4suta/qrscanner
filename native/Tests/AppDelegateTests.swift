import XCTest

@MainActor
final class AppDelegateTests: XCTestCase {
    func testFailedRelaunchKeepsTheCurrentApplicationRunning() async {
        var terminations = 0
        let succeeded = await AppDelegate.relaunch(openApplication: { false }, terminate: { terminations += 1 })
        XCTAssertFalse(succeeded)
        XCTAssertEqual(terminations, 0)
    }

    func testConfirmedRelaunchTerminatesAfterOpeningTheReplacement() async {
        var operations: [String] = []
        let succeeded = await AppDelegate.relaunch(openApplication: {
            operations.append("opened")
            return true
        }, terminate: { operations.append("terminated") })
        XCTAssertTrue(succeeded)
        XCTAssertEqual(operations, ["opened", "terminated"])
    }
}
