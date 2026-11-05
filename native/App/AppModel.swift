import AppKit
import CoreGraphics
import Combine

@MainActor
final class AppModel: ObservableObject {
    @Published private(set) var state = QRScanState(generation: 0, phase: QR_PHASE_IDLE)
    @Published private(set) var payloads: [ScannedPayload] = []
    @Published private(set) var unreadableCount = 0
    @Published private(set) var failure: FailureKind?
    @Published private(set) var permissionPresentation: PermissionPresentation?
    @Published private(set) var actionFailure: ActionFailure?
    @Published private(set) var permissionActionFailure: PermissionActionFailure?
    @Published private(set) var isRelaunching = false
    @Published private var selectedID: String?
    var presentPanel: (() -> Void)?
    var dismissPanel: (() -> Void)?

    private let scanner: any ScreenScanning
    private let hasScreenAccess: () -> Bool
    private let requestAccess: () -> Bool
    private let openSettings: () -> Bool
    private let relaunchApplication: @MainActor () async -> Bool
    private let defaults: UserDefaults
    private let openURL: (URL) -> Bool
    private let writeClipboard: (String) -> Bool
    private var task: Task<Void, Never>?
    private var isRequestingScreenAccess = false
    private var accessGrantedNeedsRelaunch = false
    private var capturePending = false

    var selection: String? {
        get { selectedID }
        set {
            let candidate = payloads.first { $0.id == newValue }
                ?? payloads.first { $0.id == selectedID }
                ?? payloads.first
            guard candidate?.id != selectedID else { return }
            selectedID = candidate?.id
            actionFailure = nil
        }
    }

    var selectedPayload: ScannedPayload? { payloads.first { $0.id == selection } }
    var actionError: String? { actionFailure?.message }
    var permissionError: String? { permissionActionFailure?.message }

    init(
        scanner: any ScreenScanning = ScreenCaptureService(),
        hasScreenAccess: @escaping () -> Bool = { CGPreflightScreenCaptureAccess() },
        requestScreenAccess: @escaping () -> Bool = { CGRequestScreenCaptureAccess() },
        openScreenSettings: @escaping () -> Bool = {
            guard let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture") else {
                return false
            }
            return NSWorkspace.shared.open(url)
        },
        relaunch: @escaping @MainActor () async -> Bool = { false },
        defaults: UserDefaults = .standard,
        openURL: @escaping (URL) -> Bool = { NSWorkspace.shared.open($0) },
        writeClipboard: @escaping (String) -> Bool = {
            NSPasteboard.general.clearContents()
            return NSPasteboard.general.setString($0, forType: .string)
        }
    ) {
        self.scanner = scanner
        self.hasScreenAccess = hasScreenAccess
        self.requestAccess = requestScreenAccess
        self.openSettings = openScreenSettings
        self.relaunchApplication = relaunch
        self.defaults = defaults
        self.openURL = openURL
        self.writeClipboard = writeClipboard
    }

    func scan() {
        guard advance(tag: QR_EVENT_BEGIN) == QR_EFFECT_CAPTURE else { return }
        let generation = state.generation
        dismissPanel?()
        clearTransientContents()
        guard hasScreenAccess() else {
            ProcessingLog.event(.permission, .denied, generation: generation)
            failure = .permission
            if accessGrantedNeedsRelaunch {
                permissionPresentation = .relaunchRequired
            } else {
                permissionPresentation = defaults.bool(forKey: "screenAccess.requested") ? .recovery : .explanation
            }
            _ = advance(tag: QR_EVENT_FAILED, generation: generation)
            presentPanel?()
            return
        }
        accessGrantedNeedsRelaunch = false
        defaults.set(true, forKey: "screenAccess.requested")
        capturePending = true
        let scanner = scanner
        task = Task { [weak self] in
            do {
                try Task.checkCancellation()
                let output = try await scanner.scan { [weak self] in
                    self?.captureCompleted(generation: generation)
                }
                guard !Task.isCancelled else { return }
                self?.receive(output, generation: generation)
            } catch is CancellationError {
                return
            } catch {
                self?.receiveFailure(generation: generation, error: error as? CaptureError)
            }
        }
    }

    func captureCompleted(generation: UInt64) {
        guard generation == state.generation, state.phase == QR_PHASE_SCANNING else { return }
        capturePending = false
        presentPanel?()
    }

    func presentCurrentState() {
        guard !capturePending else { return }
        presentPanel?()
    }

    func receive(_ output: ScanOutput, generation: UInt64) {
        guard generation == state.generation, state.phase == QR_PHASE_SCANNING else {
            ProcessingLog.event(.recognition, .stale, generation: generation)
            return
        }
        guard hasScreenAccess() else {
            receiveFailure(generation: generation)
            return
        }
        let effect = advance(tag: QR_EVENT_COMPLETED, value: output.payloads.isEmpty ? 0 : 1, generation: generation)
        guard effect == QR_EFFECT_RESULTS || effect == QR_EFFECT_EMPTY else { return }
        capturePending = false
        payloads = output.payloads
        unreadableCount = output.unreadableCount
        selection = output.payloads.first?.id
        task = nil
        presentPanel?()
    }

    func receiveFailure(generation: UInt64, error: CaptureError? = nil) {
        guard advance(tag: QR_EVENT_FAILED, generation: generation) == QR_EFFECT_FAILURE else { return }
        capturePending = false
        if hasScreenAccess() {
            switch error {
            case .noDisplays: failure = .noDisplays
            case .unavailableImage: failure = .unavailableImage
            case .recognition: failure = .recognition
            case .invalidDimensions, nil: failure = .capture
            }
        } else {
            failure = .permission
            permissionPresentation = .recovery
        }
        task = nil
        presentPanel?()
    }

    func openSelected() {
        guard let payload = selectedPayload, let url = payload.webURL,
              advance(tag: QR_EVENT_OPEN, value: 1) == QR_EFFECT_OPEN else { return }
        let succeeded = openURL(url)
        ProcessingLog.event(.open, succeeded ? .success : .failure, generation: state.generation)
        let effect = advance(tag: QR_EVENT_OPEN_FINISHED, value: succeeded ? 1 : 0)
        if effect == QR_EFFECT_CLOSE {
            clearAndDismiss()
        } else if effect == QR_EFFECT_OPEN_FAILURE {
            actionFailure = .open
        }
    }

    func copySelected() {
        guard let payload = selectedPayload, advance(tag: QR_EVENT_COPY) == QR_EFFECT_COPY else { return }
        guard writeClipboard(payload.text) else {
            ProcessingLog.event(.copy, .failure, generation: state.generation)
            actionFailure = .copy
            return
        }
        ProcessingLog.event(.copy, .success, generation: state.generation)
        cancel()
    }

    func cancel() {
        if state.phase == QR_PHASE_SCANNING { ProcessingLog.event(.scan, .cancelled, generation: state.generation) }
        task?.cancel()
        task = nil
        _ = advance(tag: QR_EVENT_CANCEL)
        clearAndDismiss()
    }

    func requestScreenAccess() {
        guard permissionPresentation == .explanation, !isRequestingScreenAccess else { return }
        defaults.set(true, forKey: "screenAccess.requested")
        isRequestingScreenAccess = true
        let granted = requestAccess()
        ProcessingLog.event(.permission, granted ? .success : .denied, generation: state.generation)
        isRequestingScreenAccess = false
        accessGrantedNeedsRelaunch = granted
        if hasScreenAccess() {
            scan()
        } else {
            permissionPresentation = granted ? .relaunchRequired : .recovery
            presentPanel?()
        }
    }

    func refreshScreenAccess() {
        guard !isRequestingScreenAccess, let permissionPresentation, permissionPresentation != .explanation else { return }
        permissionActionFailure = nil
        if hasScreenAccess() {
            self.permissionPresentation = .ready
            accessGrantedNeedsRelaunch = false
        } else {
            self.permissionPresentation = accessGrantedNeedsRelaunch ? .relaunchRequired : .recovery
        }
    }

    func retryScreenAccess() {
        guard permissionPresentation != .explanation else { return }
        refreshScreenAccess()
        if permissionPresentation == .ready { scan() }
    }

    func openScreenSettings() {
        permissionActionFailure = openSettings() ? nil : .settings
    }

    func relaunch() {
        guard permissionPresentation == .relaunchRequired, !isRelaunching else { return }
        isRelaunching = true
        permissionActionFailure = nil
        let generation = state.generation
        let relaunchApplication = relaunchApplication
        Task { [weak self] in
            let succeeded = await relaunchApplication()
            guard let self else { return }
            if self.state.generation == generation, self.permissionPresentation == .relaunchRequired {
                self.permissionActionFailure = succeeded ? nil : .relaunch
            }
            self.isRelaunching = false
        }
    }

    private func clearTransientContents() {
        capturePending = false
        payloads = []
        selection = nil
        failure = nil
        unreadableCount = 0
        actionFailure = nil
        permissionPresentation = nil
        permissionActionFailure = nil
    }

    private func clearAndDismiss() {
        clearTransientContents()
        dismissPanel?()
    }

    private func advance(tag: UInt32, value: UInt32 = 0, generation: UInt64? = nil) -> UInt32 {
        let result = qr_transition(state, QRScanEvent(generation: generation ?? state.generation, tag: tag, value: value))
        guard result.status == 0 else {
            state = QRScanState(generation: state.generation, phase: QR_PHASE_FAILED)
            failure = .invalidState
            return QR_EFFECT_FAILURE
        }
        state = result.state
        if result.effect == QR_EFFECT_EXHAUSTED { failure = .exhausted }
        return result.effect
    }
}

extension AppModel {
    enum FailureKind {
        case permission
        case capture
        case noDisplays
        case unavailableImage
        case recognition
        case invalidState
        case exhausted
    }

    func performDefaultAction() {
        if let permissionPresentation {
            switch permissionPresentation {
            case .explanation: requestScreenAccess()
            case .recovery: openScreenSettings()
            case .ready: retryScreenAccess()
            case .relaunchRequired: relaunch()
            }
        } else if failure == .exhausted || state.phase == QR_PHASE_SCANNING {
            cancel()
        } else if let payload = selectedPayload {
            if payload.webURL == nil { copySelected() } else { openSelected() }
        } else {
            scan()
        }
    }

    enum PermissionPresentation {
        case explanation
        case recovery
        case ready
        case relaunchRequired
    }

    enum ActionFailure {
        case open
        case copy

        var message: String {
            switch self {
            case .open: String(localized: "Couldn’t open the link. Try again or copy it.")
            case .copy: String(localized: "Couldn’t copy. Try again or select the text to copy it.")
            }
        }
    }

    enum PermissionActionFailure {
        case settings
        case relaunch

        var message: String {
            switch self {
            case .settings: String(localized: "Couldn’t open System Settings. Open it from the Apple menu, then enable screen access.")
            case .relaunch: String(localized: "Quit QRScanner and open it again to finish granting screen access.")
            }
        }
    }
}
