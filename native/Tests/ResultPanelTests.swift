import AppKit
import XCTest

private struct PanelTestScanner: ScreenScanning {
    func scan(onCaptureComplete: @escaping @MainActor @Sendable () -> Void) async throws -> ScanOutput {
        throw CancellationError()
    }
}

@MainActor
final class ResultPanelTests: XCTestCase {
    func testWindowControlsKeepStandardSystemMetrics() throws {
        _ = NSApplication.shared
        let rectangle = NSRect(x: 0, y: 0, width: 480, height: 200)
        let panel = ResultPanel.make(contentRect: rectangle)
        let reference = NSWindow(contentRect: rectangle,
                                 styleMask: [.titled, .closable, .miniaturizable, .resizable],
                                 backing: .buffered, defer: false)
        panel.isReleasedWhenClosed = false
        reference.isReleasedWhenClosed = false
        defer { panel.close(); reference.close() }
        XCTAssertFalse(panel.styleMask.contains(.utilityWindow))
        for kind in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
            let actual = try XCTUnwrap(panel.standardWindowButton(kind))
            let expected = try XCTUnwrap(reference.standardWindowButton(kind))
            XCTAssertEqual(actual.frame.size, expected.frame.size)
            XCTAssertEqual(actual.frame.origin.y, expected.frame.origin.y)
        }
    }

    func testReturnInReadOnlyTextOpensTheSelectedLink() throws {
        var opened: [String] = []
        let value = "https://example.org/form?source=qr#details"
        let model = try resultModel(value, open: { opened.append($0.absoluteString); return false })
        let panel = makePanel(model)
        defer { panel.close(); model.cancel() }
        let editor = focusText(value, in: panel, copyPayload: model.copySelected)
        for keyCode in [UInt16(36), UInt16(76)] {
            panel.sendEvent(try keyEvent("\r", code: keyCode, in: panel))
        }
        XCTAssertEqual(opened, [value, value])
        XCTAssertEqual(editor.string, value)
        XCTAssertEqual(model.actionFailure, .open)
    }

    func testReturnInReadOnlyTextCopiesTheSelectedText() throws {
        var copied: [String] = []
        let value = "First line\n\(QRFixtures.greekGreeting) — 日本語\nLast line"
        let model = try resultModel(value, copy: { copied.append($0); return false })
        let panel = makePanel(model)
        defer { panel.close(); model.cancel() }
        let editor = focusText(value, in: panel, copyPayload: model.copySelected)
        panel.sendEvent(try keyEvent("\r", code: 36, in: panel))
        XCTAssertEqual(copied, [value])
        XCTAssertEqual(editor.string, value)
        XCTAssertEqual(model.actionFailure, .copy)
    }

    func testReturnUsesTheDefaultInsteadOfTheFocusedSecondaryAction() throws {
        var opened = 0
        var rescanned = 0
        let model = try resultModel("https://example.org", open: { _ in opened += 1; return false })
        let panel = makePanel(model)
        defer { panel.close(); model.cancel() }
        let target = NativeActionButton.Coordinator(action: { rescanned += 1; model.scan() })
        let button = ActionButton(title: "Scan Again", target: target, action: #selector(NativeActionButton.Coordinator.activate))
        button.bezelStyle = .rounded
        button.frame = NSRect(x: 20, y: 20, width: 120, height: 24)
        panel.contentView?.addSubview(button)
        XCTAssertTrue(panel.makeFirstResponder(button))
        XCTAssertTrue(panel.firstResponder === button)
        panel.sendEvent(try keyEvent("\r", code: 36, in: panel))
        XCTAssertEqual(opened, 1)
        XCTAssertEqual(rescanned, 0)
        XCTAssertEqual(model.state.phase, QR_PHASE_RESULTS)
    }

    func testCommandCWithCapsLockCopiesTheWholePayloadWithoutASelection() throws {
        var copied: [String] = []
        let value = "https://example.org/path?value=Full%20Value"
        let model = try resultModel(value, copy: { copied.append($0); return true })
        let panel = makePanel(model)
        defer { panel.close(); model.cancel() }
        _ = focusText(value, in: panel, copyPayload: model.copySelected)
        let event = try keyEvent("C", code: 8, modifiers: [.command, .capsLock], in: panel)
        XCTAssertTrue(panel.performKeyEquivalent(with: event))
        XCTAssertEqual(copied, [value])
        XCTAssertEqual(model.state.phase, QR_PHASE_IDLE)
        XCTAssertTrue(model.payloads.isEmpty)
    }

    func testCancelCannotDismissThePreAlertButCanDismissRecovery() throws {
        let model = AppModel(scanner: PanelTestScanner(), hasScreenAccess: { false }, requestScreenAccess: { false },
                             openScreenSettings: { false }, relaunch: { false }, defaults: try isolatedDefaults(),
                             openURL: { _ in false }, writeClipboard: { _ in false })
        let panel = makePanel(model)
        defer { panel.close(); model.cancel() }
        model.scan()
        panel.cancelOperation(nil)
        XCTAssertEqual(model.permissionPresentation, .explanation)
        XCTAssertEqual(model.state.phase, QR_PHASE_FAILED)
        model.requestScreenAccess()
        XCTAssertEqual(model.permissionPresentation, .recovery)
        panel.cancelOperation(nil)
        XCTAssertNil(model.permissionPresentation)
        XCTAssertEqual(model.state.phase, QR_PHASE_IDLE)
    }

    func testDefaultButtonCellTracksPrimaryChangesAndRemoval() throws {
        let model = try resultModel("text")
        let panel = makePanel(model)
        defer { panel.close(); model.cancel() }
        let first = ActionButton(title: "Copy Text", target: nil, action: nil)
        first.primary = true
        panel.contentView?.addSubview(first)
        XCTAssertTrue(panel.defaultButtonCell === first.cell)
        XCTAssertEqual(first.keyEquivalent, "\r")
        first.primary = false
        XCTAssertNil(panel.defaultButtonCell)
        XCTAssertEqual(first.keyEquivalent, "")
        first.primary = true
        let second = ActionButton(title: "Scan Again", target: nil, action: nil)
        second.primary = true
        panel.contentView?.addSubview(second)
        first.primary = false
        first.removeFromSuperview()
        XCTAssertTrue(panel.defaultButtonCell === second.cell)
        second.removeFromSuperview()
        XCTAssertNil(panel.defaultButtonCell)
    }

    private func isolatedDefaults() throws -> UserDefaults {
        let name = "QRScanner.ResultPanelTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        addTeardownBlock { UserDefaults.standard.removePersistentDomain(forName: name) }
        return defaults
    }

    private func resultModel(_ text: String, open: @escaping (URL) -> Bool = { _ in false },
                             copy: @escaping (String) -> Bool = { _ in false }) throws -> AppModel {
        let model = AppModel(scanner: PanelTestScanner(), hasScreenAccess: { true }, requestScreenAccess: { false },
                             openScreenSettings: { false }, relaunch: { false }, defaults: try isolatedDefaults(),
                             openURL: open, writeClipboard: copy)
        model.scan()
        model.receive(ScanOutput(payloads: [ScannedPayload(text)], unreadableCount: 0), generation: model.state.generation)
        return model
    }

    private func makePanel(_ model: AppModel) -> ResultPanel {
        _ = NSApplication.shared
        let panel = ResultPanel(contentRect: NSRect(x: 0, y: 0, width: 480, height: 200),
                                styleMask: [.titled, .closable], backing: .buffered, defer: false)
        panel.isReleasedWhenClosed = false
        panel.model = model
        panel.contentView = NSView(frame: NSRect(x: 0, y: 0, width: 480, height: 200))
        panel.makeKey()
        return panel
    }

    private func focusText(_ text: String, in panel: ResultPanel, copyPayload: @escaping () -> Void) -> PayloadTextEditor {
        let editor = PayloadTextEditor(frame: NSRect(x: 20, y: 60, width: 440, height: 100))
        editor.isEditable = false
        editor.isSelectable = true
        editor.string = text
        editor.copyPayload = copyPayload
        panel.contentView?.addSubview(editor)
        XCTAssertTrue(panel.makeFirstResponder(editor))
        XCTAssertTrue(panel.firstResponder === editor)
        editor.setSelectedRange(NSRange(location: 0, length: 0))
        return editor
    }

    private func keyEvent(_ characters: String, code: UInt16, modifiers: NSEvent.ModifierFlags = [],
                          in panel: ResultPanel) throws -> NSEvent {
        try XCTUnwrap(NSEvent.keyEvent(with: .keyDown, location: .zero, modifierFlags: modifiers,
                                      timestamp: 0, windowNumber: panel.windowNumber, context: nil,
                                      characters: characters, charactersIgnoringModifiers: characters.lowercased(),
                                      isARepeat: false, keyCode: code))
    }
}
