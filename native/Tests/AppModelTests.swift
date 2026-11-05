import XCTest

struct FixtureScanner: ScreenScanning {
    func scan(onCaptureComplete: @escaping @MainActor @Sendable () -> Void) async throws -> ScanOutput {
        await onCaptureComplete()
        return ScanOutput(payloads: [], unreadableCount: 0)
    }
}

@MainActor
final class AppModelTests: XCTestCase {
    func isolatedDefaults() throws -> UserDefaults {
        let name = "QRScanner.AppModelTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        addTeardownBlock { UserDefaults.standard.removePersistentDomain(forName: name) }
        return defaults
    }

    private func readyModel(open: @escaping (URL) -> Bool = { _ in true },
                            copy: @escaping (String) -> Bool = { _ in true }) throws -> AppModel {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { true },
                             defaults: try isolatedDefaults(), openURL: open, writeClipboard: copy)
        model.scan()
        model.receive(ScanOutput(payloads: [ScannedPayload("https://example.org/form")], unreadableCount: 0),
                      generation: model.state.generation)
        return model
    }

    func testReceivingURLDoesNotOpenBrowser() throws {
        var opened: [URL] = []
        let model = try readyModel(open: { opened.append($0); return true })
        XCTAssertTrue(opened.isEmpty)
        model.openSelected()
        XCTAssertEqual(opened.map(\.absoluteString), ["https://example.org/form"])
        XCTAssertTrue(model.payloads.isEmpty)
    }

    func testOpenFailureAllowsRetryAndCopy() throws {
        var attempts = 0
        var copied = ""
        let model = try readyModel(open: { _ in attempts += 1; return false }, copy: { copied = $0; return true })
        model.openSelected()
        XCTAssertEqual(model.state.phase, QR_PHASE_RESULTS)
        XCTAssertEqual(model.actionFailure, .open)
        model.openSelected()
        XCTAssertEqual(attempts, 2)
        model.copySelected()
        XCTAssertEqual(copied, "https://example.org/form")
        XCTAssertEqual(model.state.phase, QR_PHASE_IDLE)
    }

    func testClipboardFailureKeepsResultAvailable() throws {
        let model = try readyModel(copy: { _ in false })
        model.copySelected()
        XCTAssertEqual(model.state.phase, QR_PHASE_RESULTS)
        XCTAssertEqual(model.actionFailure, .copy)
        XCTAssertEqual(model.selectedPayload?.text, "https://example.org/form")
    }

    func testCancellationDiscardsLatePayloadsAndFailure() throws {
        let model = try readyModel()
        model.scan()
        let cancelledGeneration = model.state.generation
        model.cancel()
        model.scan()
        model.receive(ScanOutput(payloads: [ScannedPayload("https://example.org/stale")], unreadableCount: 0),
                      generation: cancelledGeneration)
        model.receiveFailure(generation: cancelledGeneration)
        XCTAssertTrue(model.payloads.isEmpty)
        XCTAssertNil(model.failure)
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        model.cancel()
    }

    func testDeniedPermissionDoesNotStartCapture() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, defaults: try isolatedDefaults())
        model.scan()
        XCTAssertEqual(model.failure, .permission)
        XCTAssertEqual(model.permissionPresentation, .explanation)
        XCTAssertEqual(model.state.phase, QR_PHASE_FAILED)
        model.cancel()
    }

    func testBusyShortcutDoesNotChangeGeneration() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { true }, defaults: try isolatedDefaults())
        model.scan()
        let generation = model.state.generation
        model.scan()
        XCTAssertEqual(model.state.generation, generation)
        model.cancel()
    }

    func testScanHidesExistingUIBeforeCapture() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { true }, defaults: try isolatedDefaults())
        var presented = 0
        var dismissed = 0
        model.presentPanel = { presented += 1 }
        model.dismissPanel = { dismissed += 1 }
        model.scan()
        XCTAssertEqual(dismissed, 1)
        XCTAssertEqual(presented, 0)
        model.cancel()
    }

    func testCaptureCompletionPresentsOnlyTheActiveGeneration() throws {
        let model = try readyModel()
        var presented = 0
        model.presentPanel = { presented += 1 }
        model.scan()
        let cancelledGeneration = model.state.generation
        model.cancel()
        model.captureCompleted(generation: cancelledGeneration)
        XCTAssertEqual(presented, 0)
        model.scan()
        model.captureCompleted(generation: cancelledGeneration)
        XCTAssertEqual(presented, 0)
        model.captureCompleted(generation: model.state.generation)
        XCTAssertEqual(presented, 1)
        model.cancel()
    }

    func testDeniedRequestAndRetryDoNotRestartCaptureOrPrompt() throws {
        var requests = 0
        let defaults = try isolatedDefaults()
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false },
                             requestScreenAccess: { requests += 1; return false }, defaults: defaults)
        model.scan()
        let generation = model.state.generation
        model.requestScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .recovery)
        XCTAssertEqual(requests, 1)
        model.retryScreenAccess()
        model.requestScreenAccess()
        XCTAssertEqual(model.state.generation, generation)
        XCTAssertEqual(model.state.phase, QR_PHASE_FAILED)
        XCTAssertEqual(requests, 1)
        let restarted = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, defaults: defaults)
        restarted.scan()
        XCTAssertEqual(restarted.permissionPresentation, .recovery)
        model.cancel()
        restarted.cancel()
    }

    func testNativeGrantStartsCaptureOnlyAfterPreflightSucceeds() throws {
        var allowed = false
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { allowed },
                             requestScreenAccess: { allowed = true; return true }, defaults: try isolatedDefaults())
        model.scan()
        var presented = 0
        model.presentPanel = { presented += 1 }
        model.requestScreenAccess()
        XCTAssertNil(model.permissionPresentation)
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        XCTAssertEqual(presented, 0)
        model.cancel()
    }

    func testGrantWithoutUsableAccessShowsConditionalRelaunch() async throws {
        var relaunches = 0
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, requestScreenAccess: { true },
                             relaunch: { relaunches += 1; return false }, defaults: try isolatedDefaults())
        model.scan()
        model.relaunch()
        XCTAssertEqual(relaunches, 0)
        model.requestScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .relaunchRequired)
        XCTAssertEqual(model.state.phase, QR_PHASE_FAILED)
        model.retryScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .relaunchRequired)
        await awaitRelaunch(model) { model.relaunch() }
        XCTAssertEqual(relaunches, 1)
        XCTAssertEqual(model.permissionActionFailure, .relaunch)
        model.cancel()
        XCTAssertNil(model.permissionActionFailure)
        XCTAssertNil(model.permissionPresentation)
    }

    func testSettingsReturnRechecksAccessWithoutCapturingUntilRetry() throws {
        var allowed = false
        var settingsOpened = 0
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { allowed }, requestScreenAccess: { false },
                             openScreenSettings: { settingsOpened += 1; return true }, defaults: try isolatedDefaults())
        model.scan()
        model.requestScreenAccess()
        let generation = model.state.generation
        model.openScreenSettings()
        XCTAssertEqual(settingsOpened, 1)
        model.refreshScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .recovery)
        allowed = true
        model.refreshScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .ready)
        XCTAssertEqual(model.state.generation, generation)
        XCTAssertEqual(model.state.phase, QR_PHASE_FAILED)
        model.retryScreenAccess()
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        XCTAssertNil(model.permissionPresentation)
        model.cancel()
    }

    func testSettingsFailureKeepsRecoveryAvailable() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, requestScreenAccess: { false },
                             openScreenSettings: { false }, defaults: try isolatedDefaults())
        model.scan()
        model.requestScreenAccess()
        model.openScreenSettings()
        XCTAssertEqual(model.permissionActionFailure, .settings)
        XCTAssertEqual(model.permissionPresentation, .recovery)
        XCTAssertNil(model.actionFailure)
        model.cancel()
    }

    func testChangingSelectionClearsFailureWithoutChangingPayloads() throws {
        let model = try readyModel(open: { _ in false })
        model.scan()
        let values = [ScannedPayload("https://example.org/form"), ScannedPayload("Line 1\n\(QRFixtures.greekGreeting)\nLine 3")]
        model.receive(ScanOutput(payloads: values, unreadableCount: 0), generation: model.state.generation)
        model.openSelected()
        XCTAssertEqual(model.actionFailure, .open)
        model.selection = values[0].id
        XCTAssertEqual(model.actionFailure, .open)
        model.selection = values[1].id
        XCTAssertNil(model.actionFailure)
        XCTAssertEqual(model.payloads, values)
        XCTAssertEqual(model.selectedPayload?.text, values[1].text)
        model.cancel()
    }

    func testEmptyTextCopiesZeroLengthAndClearsTransientContents() throws {
        var copied: [String] = []
        let model = try readyModel(copy: { copied.append($0); return true })
        model.scan()
        model.receive(ScanOutput(payloads: [ScannedPayload("")], unreadableCount: 2), generation: model.state.generation)
        model.copySelected()
        XCTAssertEqual(copied, [""])
        XCTAssertEqual(model.state.phase, QR_PHASE_IDLE)
        XCTAssertTrue(model.payloads.isEmpty)
        XCTAssertNil(model.selection)
        XCTAssertEqual(model.unreadableCount, 0)
        XCTAssertNil(model.failure)
        XCTAssertNil(model.actionFailure)
    }

    func testCopyOnlyContentCannotOpenAndFailureRetainsExactValue() throws {
        var opened = 0
        let model = try readyModel(open: { _ in opened += 1; return true }, copy: { _ in false })
        let value = "mailto:hello@example.org?subject=Feedback\n\(QRFixtures.greekGreeting)"
        model.scan()
        model.receive(ScanOutput(payloads: [ScannedPayload(value)], unreadableCount: 0), generation: model.state.generation)
        model.openSelected()
        XCTAssertEqual(opened, 0)
        model.copySelected()
        XCTAssertEqual(model.actionFailure, .copy)
        XCTAssertEqual(model.selectedPayload?.text, value)
        model.cancel()
        XCTAssertNil(model.actionFailure)
        XCTAssertTrue(model.payloads.isEmpty)
    }

}

extension AppModelTests {
    func testRevokedPermissionDiscardsCapturedResultsAndOffersRecovery() throws {
        var allowed = true
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { allowed }, defaults: try isolatedDefaults())
        model.scan()
        let generation = model.state.generation
        allowed = false
        model.receive(ScanOutput(payloads: [ScannedPayload("private value")], unreadableCount: 0), generation: generation)
        XCTAssertEqual(model.state.phase, QR_PHASE_FAILED)
        XCTAssertEqual(model.failure, .permission)
        XCTAssertEqual(model.permissionPresentation, .recovery)
        XCTAssertTrue(model.payloads.isEmpty)
        allowed = true
        model.refreshScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .ready)
        model.cancel()
    }

    func testInvalidSelectionKeepsCurrentPayloadAndActionFailure() throws {
        let model = try readyModel(open: { _ in false })
        model.openSelected()
        model.selection = nil
        XCTAssertEqual(model.selectedPayload?.text, "https://example.org/form")
        XCTAssertEqual(model.actionFailure, .open)
        model.selection = "missing-result"
        XCTAssertEqual(model.selectedPayload?.text, "https://example.org/form")
        XCTAssertEqual(model.actionFailure, .open)
        model.cancel()
        XCTAssertNil(model.selection)
        model.selection = "missing-result"
        XCTAssertNil(model.selection)
        model.scan()
        XCTAssertNil(model.selection)
        model.cancel()
    }

    func testReopenWaitsForCaptureButCanShowRecognitionProgress() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { true }, defaults: try isolatedDefaults())
        var presented = 0
        model.presentPanel = { presented += 1 }
        model.scan()
        model.presentCurrentState()
        XCTAssertEqual(presented, 0)
        presented = 0
        model.captureCompleted(generation: model.state.generation)
        XCTAssertEqual(presented, 1)
        model.presentCurrentState()
        XCTAssertEqual(presented, 2)
        model.cancel()
        model.presentCurrentState()
        XCTAssertEqual(presented, 3)
    }

    func testCaptureFailureReleasesReopenGate() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { true }, defaults: try isolatedDefaults())
        var presented = 0
        model.presentPanel = { presented += 1 }
        model.scan()
        model.receiveFailure(generation: model.state.generation)
        XCTAssertEqual(presented, 1)
        model.presentCurrentState()
        XCTAssertEqual(presented, 2)
        model.cancel()
    }

    func testDefaultActionFollowsPermissionState() throws {
        var allowed = false
        var requests = 0
        var settingsOpened = 0
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { allowed },
                             requestScreenAccess: { requests += 1; return false },
                             openScreenSettings: { settingsOpened += 1; return true }, defaults: try isolatedDefaults())
        model.scan()
        model.performDefaultAction()
        XCTAssertEqual(requests, 1)
        XCTAssertEqual(model.permissionPresentation, .recovery)
        model.performDefaultAction()
        XCTAssertEqual(settingsOpened, 1)
        allowed = true
        model.refreshScreenAccess()
        model.performDefaultAction()
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        model.cancel()
    }

    func testDefaultActionRelaunchesOnlyWhenRequired() async throws {
        var relaunched = 0
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { false }, requestScreenAccess: { true },
                             relaunch: { relaunched += 1; return false }, defaults: try isolatedDefaults())
        model.scan()
        model.requestScreenAccess()
        await awaitRelaunch(model) { model.performDefaultAction() }
        XCTAssertEqual(relaunched, 1)
        model.cancel()
    }

    func testDefaultActionOpensWebAndCopiesText() throws {
        var opened = 0
        var copied: [String] = []
        let model = try readyModel(open: { _ in opened += 1; return true }, copy: { copied.append($0); return true })
        model.performDefaultAction()
        XCTAssertEqual(opened, 1)
        model.scan()
        model.receive(ScanOutput(payloads: [ScannedPayload("Plain text")], unreadableCount: 0), generation: model.state.generation)
        model.performDefaultAction()
        XCTAssertEqual(copied, ["Plain text"])
        XCTAssertEqual(opened, 1)
        model.cancel()
    }

    func testDefaultActionScansWelcomeEmptyFailureAndCancelsScanning() throws {
        let model = AppModel(scanner: FixtureScanner(), hasScreenAccess: { true }, defaults: try isolatedDefaults())
        model.performDefaultAction()
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        model.performDefaultAction()
        XCTAssertEqual(model.state.phase, QR_PHASE_IDLE)
        model.scan()
        model.receive(ScanOutput(payloads: [], unreadableCount: 0), generation: model.state.generation)
        model.performDefaultAction()
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        model.receiveFailure(generation: model.state.generation)
        model.performDefaultAction()
        XCTAssertEqual(model.state.phase, QR_PHASE_SCANNING)
        model.cancel()
    }

}
