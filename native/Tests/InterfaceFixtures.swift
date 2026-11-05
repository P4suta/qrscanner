import AppKit
import SwiftUI

enum InterfaceAppearance: String, CaseIterable {
    case light, dark

    var native: NSAppearance.Name { self == .light ? .aqua : .darkAqua }
    var scheme: ColorScheme { self == .light ? .light : .dark }
}

private struct InterfaceScanner: ScreenScanning {
    func scan(onCaptureComplete: @escaping @MainActor @Sendable () -> Void) async throws -> ScanOutput {
        throw CancellationError()
    }
}

enum InterfaceFixture: String, CaseIterable {
    case welcome, permission, recovery, scanning, link, multiple, text, empty
    case noResults, unreadable, scanFailure, openFailure, copyFailure
    case longLink, unicode, manyResults, mixedText, twoResults, permissionReady, relaunchRequired

    static let exampleURL = "https://example.org/event/feedback?session=design&source=qr"
    static let notes = "Design review — Friday, 14:00\nRoom 3B\nBring your questions."

    var widths: [CGFloat] { self == .longLink ? [440, 480, 560] : [480] }

    @MainActor
    func model(defaults: UserDefaults) -> AppModel {
        var allowed = ![Self.permission, .recovery, .permissionReady, .relaunchRequired].contains(self)
        let model = AppModel(
            scanner: InterfaceScanner(), hasScreenAccess: { allowed },
            requestScreenAccess: { self == .relaunchRequired }, openScreenSettings: { false }, relaunch: { false },
            defaults: defaults, openURL: { _ in false }, writeClipboard: { _ in false })
        guard self != .welcome else { return model }
        model.scan()
        switch self {
        case .permission:
            return model
        case .recovery, .relaunchRequired:
            model.requestScreenAccess()
            return model
        case .permissionReady:
            model.requestScreenAccess()
            allowed = true
            model.refreshScreenAccess()
            return model
        case .scanning:
            model.captureCompleted(generation: model.state.generation)
            return model
        case .scanFailure:
            model.receiveFailure(generation: model.state.generation)
            return model
        default:
            let values = payloads.map(ScannedPayload.init)
            model.receive(ScanOutput(payloads: values, unreadableCount: self == .unreadable ? 1 : 0),
                          generation: model.state.generation)
            if self == .mixedText || self == .twoResults { model.selection = values.last?.id }
            if self == .openFailure { model.openSelected() }
            if self == .copyFailure { model.copySelected() }
            return model
        }
    }

    private var payloads: [String] {
        switch self {
        case .link, .openFailure:
            [Self.exampleURL]
        case .multiple, .mixedText:
            [Self.exampleURL, "https://docs.example.org/session/slides", Self.notes]
        case .twoResults:
            ["https://example.org/", "hello"]
        case .text, .copyFailure:
            [Self.notes]
        case .empty:
            [""]
        case .longLink:
            ["https://a-very-long-subdomain-for-conference-feedback.example.org/" + String(repeating: "0123456789abcdef", count: 100)]
        case .unicode:
            ["Notes — \(QRFixtures.greekGreeting) — \u{041F}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} — 日本語\n\n" +
             (1...20).map { "Paragraph \($0) — Original line breaks are retained." }.joined(separator: "\n")]
        case .manyResults:
            (1...14).map { "https://example.org/session/\($0)" }
        default:
            []
        }
    }
}
