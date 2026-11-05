import XCTest

final class CoreBridgeTests: XCTestCase {
    func testHeaderConstantsMatchTheNumericWireContract() {
        XCTAssertEqual([QR_PHASE_IDLE, QR_PHASE_SCANNING, QR_PHASE_RESULTS, QR_PHASE_OPENING,
                        QR_PHASE_EMPTY, QR_PHASE_FAILED], Array(UInt32(0)...5))
        XCTAssertEqual([QR_EVENT_BEGIN, QR_EVENT_CANCEL, QR_EVENT_COMPLETED, QR_EVENT_FAILED,
                        QR_EVENT_OPEN, QR_EVENT_COPY, QR_EVENT_OPEN_FINISHED], Array(UInt32(0)...6))
        XCTAssertEqual([QR_EFFECT_NONE, QR_EFFECT_CAPTURE, QR_EFFECT_RESULTS, QR_EFFECT_EMPTY,
                        QR_EFFECT_FAILURE, QR_EFFECT_OPEN, QR_EFFECT_COPY, QR_EFFECT_CLOSE,
                        QR_EFFECT_OPEN_FAILURE, QR_EFFECT_EXHAUSTED], Array(UInt32(0)...9))
    }

    func testNativeRecordSizesAlignmentsAndFieldOffsets() {
        XCTAssertEqual(MemoryLayout<QRScanState>.size, 16)
        XCTAssertEqual(MemoryLayout<QRScanEvent>.size, 16)
        XCTAssertEqual(MemoryLayout<QRScanTransition>.size, 24)
        XCTAssertEqual(MemoryLayout<QRScanState>.alignment, 8)
        XCTAssertEqual(MemoryLayout<QRScanEvent>.alignment, 8)
        XCTAssertEqual(MemoryLayout<QRScanTransition>.alignment, 8)
        XCTAssertEqual(MemoryLayout<QRScanState>.offset(of: \.generation), 0)
        XCTAssertEqual(MemoryLayout<QRScanState>.offset(of: \.phase), 8)
        XCTAssertEqual(MemoryLayout<QRScanEvent>.offset(of: \.generation), 0)
        XCTAssertEqual(MemoryLayout<QRScanEvent>.offset(of: \.tag), 8)
        XCTAssertEqual(MemoryLayout<QRScanEvent>.offset(of: \.value), 12)
        XCTAssertEqual(MemoryLayout<QRScanTransition>.offset(of: \.state), 0)
        XCTAssertEqual(MemoryLayout<QRScanTransition>.offset(of: \.effect), 16)
        XCTAssertEqual(MemoryLayout<QRScanTransition>.offset(of: \.status), 20)
    }

    func testEveryWireEventAcrossPhasesAndGenerationBoundaries() {
        for phase in UInt32(0)...6 {
            for generation in [UInt64(0), 1, UInt64.max] {
                for tag in [UInt32(0), 1, 2, 3, 4, 5, 6, UInt32.max] {
                    for value in [UInt32(0), 1, 2, UInt32.max] {
                        for eventGeneration in [generation, generation &+ 1] {
                            let state = QRScanState(generation: generation, phase: phase)
                            let event = QRScanEvent(generation: eventGeneration, tag: tag, value: value)
                            let actual = qr_transition(state, event)
                            let expected = specifiedTransition(state, event)
                            XCTAssertEqual(actual.state.generation, expected.state.generation)
                            XCTAssertEqual(actual.state.phase, expected.state.phase)
                            XCTAssertEqual(actual.effect, expected.effect)
                            XCTAssertEqual(actual.status, expected.status)
                        }
                    }
                }
            }
        }
    }

    private func specifiedTransition(_ state: QRScanState, _ event: QRScanEvent) -> QRScanTransition {
        let valid = state.phase <= 5 && event.tag <= 6
            && (event.tag == 2 || event.tag == 4 || event.tag == 6 ? event.value <= 1 : event.value == 0)
        guard valid else { return QRScanTransition(state: state, effect: 0, status: 1) }
        var result = QRScanTransition(state: state, effect: 0, status: 0)
        if event.tag == 1 {
            result.state.phase = 0
            result.effect = 7
        } else if event.tag == 0 && state.phase != 1 && state.phase != 3 {
            if state.generation == UInt64.max {
                result.effect = 9
            } else {
                result.state = QRScanState(generation: state.generation + 1, phase: 1)
                result.effect = 1
            }
        } else if event.generation == state.generation, let outcome = specifiedCurrentEvent(state, event) {
            result.state.phase = outcome.phase
            result.effect = outcome.effect
        }
        return result
    }

    private func specifiedCurrentEvent(_ state: QRScanState, _ event: QRScanEvent) -> (phase: UInt32, effect: UInt32)? {
        switch (state.phase, event.tag, event.value) {
        case (1, 2, 0): return (4, 3)
        case (1, 2, 1): return (2, 2)
        case (1, 3, 0): return (5, 4)
        case (2, 4, 1): return (3, 5)
        case (2, 5, 0): return (2, 6)
        case (3, 6, 0): return (2, 8)
        case (3, 6, 1): return (0, 7)
        default: return nil
        }
    }

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
