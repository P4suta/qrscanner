import Combine
import XCTest

@MainActor
private final class RelaunchProbe {
    let started = XCTestExpectation(description: "Relaunch requested")
    private(set) var requests = 0
    private var completions: [CheckedContinuation<Bool, Never>] = []

    func launch() async -> Bool {
        requests += 1
        return await withCheckedContinuation { continuation in
            completions.append(continuation)
            started.fulfill()
        }
    }

    func complete(_ succeeded: Bool) {
        let pending = completions
        completions = []
        pending.forEach { $0.resume(returning: succeeded) }
    }
}

extension AppModelTests {
    func testPendingRelaunchRejectsDuplicateRequestsAndRetainsFailure() async throws {
        let probe = RelaunchProbe()
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, requestScreenAccess: { true },
                             relaunch: probe.launch, defaults: try isolatedDefaults())
        model.scan()
        model.requestScreenAccess()
        model.relaunch()
        model.relaunch()
        XCTAssertTrue(model.isRelaunching)
        await waitFor(probe.started)
        XCTAssertEqual(probe.requests, 1)
        await awaitRelaunch(model) { probe.complete(false) }
        XCTAssertFalse(model.isRelaunching)
        XCTAssertEqual(model.permissionPresentation, .relaunchRequired)
        XCTAssertEqual(model.permissionActionFailure, .relaunch)
        model.cancel()
    }

    func testRelaunchFailureAfterCancellationCannotChangeNewPermissionState() async throws {
        let probe = RelaunchProbe()
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, requestScreenAccess: { true },
                             relaunch: probe.launch, defaults: try isolatedDefaults())
        model.scan()
        model.requestScreenAccess()
        model.relaunch()
        await waitFor(probe.started)
        model.cancel()
        model.scan()
        let newGeneration = model.state.generation
        model.relaunch()
        XCTAssertEqual(probe.requests, 1)
        await awaitRelaunch(model) { probe.complete(false) }
        XCTAssertEqual(model.state.generation, newGeneration)
        XCTAssertEqual(model.permissionPresentation, .relaunchRequired)
        XCTAssertNil(model.permissionActionFailure)
        XCTAssertFalse(model.isRelaunching)
        model.cancel()
    }

    func awaitRelaunch(_ model: AppModel, action: () -> Void) async {
        let finished = XCTestExpectation(description: "Relaunch completion")
        let observation = model.$isRelaunching.dropFirst().filter { !$0 }.sink { _ in finished.fulfill() }
        action()
        await waitFor(finished)
        withExtendedLifetime(observation) {}
    }

    private func waitFor(_ expectation: XCTestExpectation) async {
        let result = await XCTWaiter.fulfillment(of: [expectation], timeout: 5)
        XCTAssertEqual(result, .completed)
    }
}
