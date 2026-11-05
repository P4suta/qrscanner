import CoreGraphics
import Foundation
import ScreenCaptureKit

struct ScanOutput: Sendable {
    let payloads: [ScannedPayload]
    let unreadableCount: Int
}

protocol ScreenScanning: Sendable {
    func scan(onCaptureComplete: @escaping @MainActor @Sendable () -> Void) async throws -> ScanOutput
}

actor ScreenCaptureService: ScreenScanning {
    private let capture: @Sendable () async throws -> [CGImage]

    init(capture: @escaping @Sendable () async throws -> [CGImage] = ScreenCaptureService.captureDisplays) {
        self.capture = capture
    }

    func scan(onCaptureComplete: @escaping @MainActor @Sendable () -> Void) async throws -> ScanOutput {
        let captureInterval = ProcessingLog.Interval(.capture)
        let images: [CGImage]
        do {
            try Task.checkCancellation()
            images = try await capture()
            try Task.checkCancellation()
            guard !images.isEmpty else { throw CaptureError.noDisplays }
            captureInterval.finish(.success)
        } catch {
            captureInterval.finish(error is CancellationError ? .cancelled : .failure)
            throw error
        }
        await onCaptureComplete()
        let recognitionInterval = ProcessingLog.Interval(.recognition)
        do {
            var texts: [String] = []
            var unreadableCount = 0
            for image in images {
                try Task.checkCancellation()
                let frame = try QRDecoder.decode(image)
                texts.append(contentsOf: frame.texts)
                unreadableCount += frame.unreadableCount
            }
            try Task.checkCancellation()
            recognitionInterval.finish(.success)
            return ScanOutput(payloads: ScannedPayload.unique(texts), unreadableCount: unreadableCount)
        } catch is CancellationError {
            recognitionInterval.finish(.cancelled)
            throw CancellationError()
        } catch {
            recognitionInterval.finish(.failure)
            throw CaptureError.recognition
        }
    }

    static func configuration(rect: CGRect, scale: CGFloat) throws -> SCScreenshotConfiguration {
        guard scale.isFinite, scale > 0, rect.size.width > 0, rect.size.height > 0,
              let width = Int(exactly: (rect.width * scale).rounded(.up)),
              let height = Int(exactly: (rect.height * scale).rounded(.up)), width > 0, height > 0 else {
            throw CaptureError.invalidDimensions
        }
        let configuration = SCScreenshotConfiguration()
        configuration.width = width
        configuration.height = height
        configuration.showsCursor = false
        configuration.dynamicRange = .sdr
        configuration.fileURL = nil
        return configuration
    }

    private static func captureDisplays() async throws -> [CGImage] {
        try Task.checkCancellation()
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        let ownApplication = content.applications.filter { $0.processID == ProcessInfo.processInfo.processIdentifier }
        guard !content.displays.isEmpty else { throw CaptureError.noDisplays }
        var images: [CGImage] = []
        for display in content.displays.sorted(by: { $0.displayID < $1.displayID }) {
            try Task.checkCancellation()
            let filter = SCContentFilter(display: display, excludingApplications: ownApplication, exceptingWindows: [])
            let configuration = try configuration(rect: filter.contentRect, scale: CGFloat(filter.pointPixelScale))
            let output = try await SCScreenshotManager.captureScreenshot(contentFilter: filter, configuration: configuration)
            guard let image = output.sdrImage else { throw CaptureError.unavailableImage }
            images.append(image)
        }
        try Task.checkCancellation()
        return images
    }
}

enum CaptureError: Error, Equatable {
    case noDisplays
    case invalidDimensions
    case unavailableImage
    case recognition
}
