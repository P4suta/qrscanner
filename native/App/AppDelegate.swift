import AppKit
import Combine
import SwiftUI

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate, NSWindowDelegate {
    let model: AppModel
    private var panel: ResultPanel?
    private var observations = Set<AnyCancellable>()

    override init() {
        PreferencesMigration.load(defaults: .standard)
        let relaunch: @MainActor () async -> Bool = { await AppDelegate.relaunch() }
        #if DEBUG
        if ProcessInfo.processInfo.arguments.contains("--preview-results") {
            model = AppModel(scanner: PreviewScanner(), hasScreenAccess: { true }, relaunch: relaunch)
        } else {
            model = AppModel(relaunch: relaunch)
        }
        #else
        model = AppModel(relaunch: relaunch)
        #endif
        super.init()
    }

    static func relaunch(
        openApplication: @MainActor () async -> Bool = {
            let configuration = NSWorkspace.OpenConfiguration()
            configuration.createsNewApplicationInstance = true
            do {
                _ = try await NSWorkspace.shared.openApplication(at: Bundle.main.bundleURL, configuration: configuration)
                return true
            } catch {
                return false
            }
        },
        terminate: @MainActor () -> Void = { NSApplication.shared.terminate(nil) }
    ) async -> Bool {
        let interval = ProcessingLog.Interval(.relaunch)
        if await openApplication() {
            interval.finish(.success)
            terminate()
            return true
        }
        interval.finish(.failure)
        return false
    }

    func applicationDidFinishLaunching(_ notification: Notification) {
        model.presentPanel = { [weak self] in self?.showResults() }
        model.dismissPanel = { [weak self] in self?.panel?.orderOut(nil) }
        model.objectWillChange.sink { [weak self] in
            DispatchQueue.main.async { self?.updatePanelLayout() }
        }.store(in: &observations)
        if !UserDefaults.standard.bool(forKey: "hasShownWelcome") {
            showResults()
            UserDefaults.standard.set(true, forKey: "hasShownWelcome")
        }
        #if DEBUG
        if ProcessInfo.processInfo.arguments.contains("--preview-results") { model.scan() }
        #endif
    }

    func applicationDidBecomeActive(_ notification: Notification) {
        model.refreshScreenAccess()
    }

    func applicationWillTerminate(_ notification: Notification) {
        model.cancel()
    }

    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        model.presentCurrentState()
        return false
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        if sender === panel {
            guard model.permissionPresentation != .explanation else { return false }
            model.cancel()
            return false
        }
        return true
    }

    func windowDidResize(_ notification: Notification) {
        if notification.object as? NSWindow === panel { updatePanelLayout() }
    }

    private func showResults() {
        if panel == nil {
            let window = ResultPanel.make(contentRect: NSRect(x: 0, y: 0, width: 480, height: 155))
            window.model = model
            window.delegate = self
            window.title = "QRScanner"
            window.isFloatingPanel = true
            window.level = .floating
            window.hidesOnDeactivate = false
            window.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary]
            window.isReleasedWhenClosed = false
            window.contentMinSize = NSSize(width: 320, height: 80)
            window.contentMaxSize = NSSize(width: 900, height: 1200)
            let host = NSHostingView(rootView: ResultView(model: model))
            host.sizingOptions = [.intrinsicContentSize]
            window.contentView = host
            panel = window
        }
        guard let panel else { return }
        updatePanelLayout()
        let screen = NSScreen.screens.first { $0.frame.contains(NSEvent.mouseLocation) } ?? NSScreen.main
        if let frame = screen?.visibleFrame {
            panel.setFrameOrigin(NSPoint(x: frame.midX - panel.frame.width / 2,
                                         y: frame.midY - panel.frame.height / 2))
        }
        NSApplication.shared.activate(ignoringOtherApps: true)
        panel.makeKeyAndOrderFront(nil)
    }

    private func updatePanelLayout() {
        guard let panel, let host = panel.contentView else { return }
        let isPreAlert = model.permissionPresentation == .explanation
        panel.standardWindowButton(.closeButton)?.isHidden = isPreAlert
        panel.standardWindowButton(.miniaturizeButton)?.isHidden = isPreAlert
        panel.standardWindowButton(.zoomButton)?.isHidden = isPreAlert
        host.layoutSubtreeIfNeeded()
        let height = host.fittingSize.height
        guard height.isFinite, height > 0, abs(panel.contentLayoutRect.height - height) > 0.5 else { return }
        panel.setContentSize(NSSize(width: panel.contentLayoutRect.width, height: height))
    }

}

#if DEBUG
private struct PreviewScanner: ScreenScanning {
    func scan(onCaptureComplete: @escaping @MainActor @Sendable () -> Void) async throws -> ScanOutput {
        await onCaptureComplete()
        return ScanOutput(payloads: ScannedPayload.unique([
            "https://example.org/event/feedback?session=design&source=qr",
            "https://docs.example.org/session/slides",
            "Design review — Friday, 14:00\nRoom 3B\nBring your questions."
        ]), unreadableCount: 0)
    }
}
#endif
