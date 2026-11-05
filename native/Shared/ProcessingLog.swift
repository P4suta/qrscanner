import Foundation
import os

enum ProcessingLog {
    enum Stage: String {
        case scan, capture, recognition, permission, open, copy, relaunch, migration
    }

    enum Outcome: String {
        case success, failure, cancelled, stale, denied
    }

    private static let logger = Logger(subsystem: "com.p4suta.qrscanner", category: "processing")
    private static let signposter = OSSignposter(subsystem: "com.p4suta.qrscanner", category: "processing")

    static func event(_ stage: Stage, _ outcome: Outcome, generation: UInt64 = 0) {
        logger.info("stage=\(stage.rawValue, privacy: .public) outcome=\(outcome.rawValue, privacy: .public) generation=\(generation)")
    }

    struct Interval {
        private let stage: Stage
        private let started = ContinuousClock.now
        private let state: OSSignpostIntervalState

        init(_ stage: Stage) {
            self.stage = stage
            state = signposter.beginInterval("Processing", id: signposter.makeSignpostID(), "stage=\(stage.rawValue, privacy: .public)")
        }

        func finish(_ outcome: Outcome) {
            let duration = started.duration(to: .now).components
            let milliseconds = Double(duration.seconds) * 1000 + Double(duration.attoseconds) / 1e15
            signposter.endInterval("Processing", state, "outcome=\(outcome.rawValue, privacy: .public)")
            logger.info("stage=\(stage.rawValue, privacy: .public) outcome=\(outcome.rawValue, privacy: .public) duration_ms=\(milliseconds)")
        }
    }
}
