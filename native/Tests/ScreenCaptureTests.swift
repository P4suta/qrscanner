import CoreGraphics
import ScreenCaptureKit
import XCTest

final class ScreenCaptureTests: XCTestCase {
    func testScreenshotConfigurationUsesRetinaPixelsSDRAndMemoryOnly() throws {
        let configuration = try ScreenCaptureService.configuration(rect: CGRect(x: -100, y: 0, width: 1512, height: 982), scale: 2)
        XCTAssertEqual(configuration.width, 3024)
        XCTAssertEqual(configuration.height, 1964)
        XCTAssertEqual(configuration.dynamicRange, .sdr)
        XCTAssertFalse(configuration.showsCursor)
        XCTAssertNil(configuration.fileURL)
    }

    func testInvalidDisplayDimensionsAreRejectedWithoutIntegerTraps() {
        for size in [CGFloat.zero, -1, .infinity, .nan, .greatestFiniteMagnitude] {
            XCTAssertThrowsError(try ScreenCaptureService.configuration(rect: CGRect(x: 0, y: 0, width: size, height: 100), scale: 2))
        }
        XCTAssertThrowsError(try ScreenCaptureService.configuration(rect: CGRect(x: 0, y: 0, width: 100, height: 100), scale: 0))
    }

    @MainActor
    func testEveryCapturedDisplayIsDecodedAfterCaptureCompletion() async throws {
        let first = try QRFixtures.canvas(width: 232, height: 232)
        let second = try QRFixtures.canvas(width: 232, height: 232)
        QRFixtures.draw(QRFixtures.alphabet, on: first, scale: 8, origin: .zero)
        QRFixtures.draw(QRFixtures.hello, on: second, scale: 8, origin: .zero)
        let images = try [XCTUnwrap(first.makeImage()), XCTUnwrap(second.makeImage())]
        let scanner = ScreenCaptureService(capture: { images })
        var captured = false
        let output = try await scanner.scan { captured = true }
        XCTAssertTrue(captured)
        XCTAssertEqual(output.payloads.map(\.text), ["ABCDEF", "hello"])
        XCTAssertEqual(output.unreadableCount, 0)
    }

    @MainActor
    func testNoDisplaysNeverPublishesCaptureCompletion() async {
        let scanner = ScreenCaptureService(capture: { [] })
        var captured = false
        do {
            _ = try await scanner.scan { captured = true }
            XCTFail("An empty display inventory must fail")
        } catch {
            XCTAssertEqual(error as? CaptureError, .noDisplays)
        }
        XCTAssertFalse(captured)
    }
}
