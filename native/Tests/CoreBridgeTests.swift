import XCTest

final class CoreBridgeTests: XCTestCase {
    func testNativeABIRequiresConfirmationAndRejectsStaleScan() {
        let initial = QRScanState(generation: 0, phase: QR_PHASE_IDLE)
        let started = qr_transition(initial, QRScanEvent(generation: 0, tag: QR_EVENT_BEGIN, value: 0))
        XCTAssertEqual(started.effect, QR_EFFECT_CAPTURE)
        let detected = qr_transition(started.state, QRScanEvent(
            generation: started.state.generation, tag: QR_EVENT_COMPLETED, value: 1))
        XCTAssertEqual(detected.effect, QR_EFFECT_RESULTS)
        let cancelled = qr_transition(detected.state, QRScanEvent(generation: 0, tag: QR_EVENT_CANCEL, value: 0))
        let late = qr_transition(cancelled.state, QRScanEvent(
            generation: started.state.generation, tag: QR_EVENT_COMPLETED, value: 1))
        XCTAssertEqual(late.effect, QR_EFFECT_NONE)
        XCTAssertEqual(late.state.phase, QR_PHASE_IDLE)
    }

    func testNativeABIRefusesUnknownValues() {
        let result = qr_transition(QRScanState(generation: 0, phase: UInt32.max),
                                   QRScanEvent(generation: 0, tag: QR_EVENT_OPEN, value: 1))
        XCTAssertEqual(result.status, 1)
        XCTAssertEqual(result.effect, QR_EFFECT_NONE)
    }
}
