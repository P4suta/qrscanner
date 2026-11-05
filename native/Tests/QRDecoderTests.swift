import CoreGraphics
import Vision
import XCTest

final class QRDecoderTests: XCTestCase {
    func testSuccessfulAutomaticDetectionDoesNotAttemptCPURecovery() throws {
        var recoveryAttempts = 0
        let result = try QRDecoder.performWithCPURecovery(automatic: { 0 }, cpu: {
            recoveryAttempts += 1
            return 1
        })
        XCTAssertEqual(result, 0)
        XCTAssertEqual(recoveryAttempts, 0)
    }

    func testOnlyVisionInternalErrorsPermitCPURecovery() {
        let failures = [
            NSError(domain: "OtherFramework", code: VNErrorCode.internalError.rawValue),
            NSError(domain: VNErrorDomain, code: VNErrorCode.requestCancelled.rawValue),
            NSError(domain: VNErrorDomain, code: VNErrorCode.invalidImage.rawValue),
            NSError(domain: VNErrorDomain, code: VNErrorCode.outOfMemory.rawValue),
            NSError(domain: VNErrorDomain, code: VNErrorCode.unsupportedComputeDevice.rawValue)
        ]
        for failure in failures {
            var recoveryAttempts = 0
            XCTAssertThrowsError(try QRDecoder.performWithCPURecovery(automatic: { throw failure }, cpu: {
                recoveryAttempts += 1
            })) { error in
                XCTAssertTrue(error as NSError === failure)
            }
            XCTAssertEqual(recoveryAttempts, 0)
        }
    }

    func testVisionInternalErrorAttemptsCPURecoveryOnce() throws {
        var automaticAttempts = 0
        var recoveryAttempts = 0
        let result: String = try QRDecoder.performWithCPURecovery(automatic: {
            automaticAttempts += 1
            throw NSError(domain: VNErrorDomain, code: VNErrorCode.internalError.rawValue)
        }, cpu: {
            recoveryAttempts += 1
            return "decoded"
        })
        XCTAssertEqual(result, "decoded")
        XCTAssertEqual(automaticAttempts, 1)
        XCTAssertEqual(recoveryAttempts, 1)
    }

    func testCPURecoveryFailuresPreserveBothCausesWithoutRetrying() {
        let original = NSError(domain: VNErrorDomain, code: VNErrorCode.internalError.rawValue)
        let failures = [
            NSError(domain: VNErrorDomain, code: VNErrorCode.unsupportedComputeDevice.rawValue),
            NSError(domain: "ComputeDeviceQuery", code: 1),
            NSError(domain: VNErrorDomain, code: VNErrorCode.internalError.rawValue)
        ]
        for failure in failures {
            var recoveryAttempts = 0
            XCTAssertThrowsError(try QRDecoder.performWithCPURecovery(automatic: { throw original }, cpu: {
                recoveryAttempts += 1
                throw failure
            })) { error in
                guard let recovery = error as? QRDecoder.RecoveryFailure else {
                    return XCTFail("Expected both Vision recovery causes, received \(error)")
                }
                XCTAssertTrue(recovery.original as NSError === original)
                XCTAssertTrue(recovery.recovery as NSError === failure)
            }
            XCTAssertEqual(recoveryAttempts, 1)
        }
    }

    func testGoldenQRFixturesDecodeWithoutAnImageGenerator() throws {
        for (matrix, expected) in [(QRFixtures.alphabet, "ABCDEF"), (QRFixtures.hello, "hello")] {
            let canvas = try QRFixtures.canvas(width: 232, height: 232)
            QRFixtures.draw(matrix, on: canvas, scale: 8, origin: .zero)
            let result = try QRDecoder.decode(XCTUnwrap(canvas.makeImage()))
            XCTAssertEqual(result.texts, [expected])
            XCTAssertEqual(result.unreadableCount, 0)
        }
    }

    func testMultipleSmallAndRotatedCodesOnRetinaSizedScreen() throws {
        let canvas = try QRFixtures.canvas(width: 3840, height: 2160)
        QRFixtures.draw(QRFixtures.alphabet, on: canvas, scale: 4, origin: CGPoint(x: 200, y: 400))
        QRFixtures.draw(QRFixtures.hello, on: canvas, scale: 6, origin: CGPoint(x: 2500, y: 1200), rotated: true)
        let result = try QRDecoder.decode(XCTUnwrap(canvas.makeImage()))
        XCTAssertEqual(Set(result.texts), Set(["ABCDEF", "hello"]))
    }

    func testBlankScreenIsNegativeControl() throws {
        let canvas = try QRFixtures.canvas(width: 1920, height: 1080)
        XCTAssertTrue(try QRDecoder.decode(XCTUnwrap(canvas.makeImage())).texts.isEmpty)
    }
}
