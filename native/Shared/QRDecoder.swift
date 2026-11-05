import CoreGraphics
import Foundation
import Vision

struct DecodedFrame: Sendable {
    let texts: [String]
    let unreadableCount: Int
}

enum QRDecoder {
    struct RecoveryFailure: Error {
        let original: any Error
        let recovery: any Error
    }

    static func decode(_ image: CGImage) throws -> DecodedFrame {
        let results = try performWithCPURecovery(automatic: {
            try observations(in: image, usingCPU: false)
        }, cpu: {
            try observations(in: image, usingCPU: true)
        })
        let observations = results.sorted {
            if $0.boundingBox.midY != $1.boundingBox.midY {
                return $0.boundingBox.midY > $1.boundingBox.midY
            }
            return $0.boundingBox.minX < $1.boundingBox.minX
        }
        let texts = observations.compactMap(\.payloadStringValue)
        return DecodedFrame(texts: texts, unreadableCount: observations.count - texts.count)
    }

    static func performWithCPURecovery<Result>(automatic: () throws -> Result, cpu: () throws -> Result) throws -> Result {
        do {
            return try automatic()
        } catch {
            let original = error
            let failure = error as NSError
            guard failure.domain == VNErrorDomain, failure.code == VNErrorCode.internalError.rawValue else {
                throw original
            }
            do {
                return try cpu()
            } catch {
                throw RecoveryFailure(original: original, recovery: error)
            }
        }
    }

    private static func observations(in image: CGImage, usingCPU: Bool) throws -> [VNBarcodeObservation] {
        let request = VNDetectBarcodesRequest()
        request.symbologies = [.qr]
        if usingCPU {
            let devices = try request.supportedComputeStageDevices[.main] ?? []
            guard let cpu = devices.first(where: {
                if case .cpu = $0 { return true }
                return false
            }) else {
                throw NSError(domain: VNErrorDomain, code: VNErrorCode.unsupportedComputeDevice.rawValue)
            }
            request.setComputeDevice(cpu, for: .main)
        }
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        try handler.perform([request])
        return request.results ?? []
    }
}
