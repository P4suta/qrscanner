import AppKit
import Carbon
import SwiftUI
import XCTest

@MainActor
final class InterfaceTests: XCTestCase {
    func testNarrowWindowsKeepActionsVisibleAtTwoHundredPercentText() throws {
        _ = NSApplication.shared
        for fixture in [InterfaceFixture.link, .multiple, .recovery, .openFailure, .longLink] {
            let model = fixture.model(defaults: try isolatedDefaults())
            defer { model.cancel() }
            let window = try render(ResultView(model: model).environment(\.appTextScale, 2), width: 320,
                                    appearance: .aqua, scheme: .light, name: "large-text/\(fixture.rawValue)-320")
            defer { window.close() }
            let host = try XCTUnwrap(window.contentView)
            let buttons = try verifyButtons(in: host)
            XCTAssertTrue(buttons.allSatisfy { $0.bounds.height >= 28 })
            for button in buttons {
                let font = try XCTUnwrap(button.font)
                XCTAssertGreaterThanOrEqual(button.bounds.height, ceil(font.ascender - font.descender + font.leading) + 8)
            }
            for editor in descendants(of: host).compactMap({ $0 as? PayloadTextEditor }) {
                XCTAssertEqual(editor.font?.pointSize ?? 0, NSFont.preferredFont(forTextStyle: .body).pointSize * 2, accuracy: 0.1)
            }
        }
    }

    func testNarrowSettingsKeepRecorderAndActionsVisibleWithEnlargedText() throws {
        _ = NSApplication.shared
        for fixture in InterfaceSettingsFixture.allCases {
            let manager = fixture.manager(defaults: try isolatedDefaults())
            defer { manager.stop() }
            let window = try render(SettingsView(hotKey: manager).environment(\.appTextScale, 2), width: 320,
                                    appearance: .darkAqua, scheme: .dark, name: "large-text/settings-\(fixture.rawValue)-320")
            defer { window.close() }
            let host = try XCTUnwrap(window.contentView)
            _ = try verifyButtons(in: host)
        }
    }

    func testResultStatesRenderExactPayloadsWithinTheirWindows() throws {
        _ = NSApplication.shared
        for appearance in InterfaceAppearance.allCases {
            for fixture in InterfaceFixture.allCases {
                for width in fixture.widths {
                    let model = fixture.model(defaults: try isolatedDefaults())
                    defer { model.cancel() }
                    let window = try render(ResultView(model: model), width: width, appearance: appearance.native,
                                            scheme: appearance.scheme, name: "\(appearance.rawValue)/\(fixture.rawValue)-\(Int(width))",
                                            hidesWindowControls: model.permissionPresentation == .explanation)
                    defer { window.close() }
                    let host = try XCTUnwrap(window.contentView)
                    XCTAssertEqual(host.bounds.width, width, accuracy: 0.5)
                    let buttons = try verifyButtons(in: host)
                    let editors = descendants(of: host).compactMap { $0 as? PayloadTextEditor }
                    if let payload = model.selectedPayload, !payload.text.isEmpty {
                        let editor = try XCTUnwrap(editors.first)
                        XCTAssertEqual(editors.count, 1)
                        XCTAssertEqual(editor.string, payload.text)
                        XCTAssertTrue(editor.isSelectable)
                        XCTAssertFalse(editor.isEditable)
                        let scroll = try XCTUnwrap(editor.enclosingScrollView)
                        XCTAssertGreaterThan(scroll.contentSize.height, 0)
                        XCTAssertLessThanOrEqual(scroll.contentSize.height, 160.5)
                        XCTAssertTrue(host.bounds.contains(scroll.convert(scroll.bounds, to: host)))
                        let action = payload.webURL == nil ? "Copy Text" : "Open Link"
                        XCTAssertTrue(buttons.contains { buttonLabel($0) == action }, "Missing \(action) in \(fixture.rawValue)")
                        if fixture == .longLink || fixture == .unicode {
                            XCTAssertGreaterThan(editor.bounds.height, scroll.contentSize.height)
                        }
                    } else {
                        XCTAssertTrue(editors.isEmpty)
                    }
                }
            }
        }
    }

    func testSettingsStatesRenderTheActualRecorderAndAvailableActions() throws {
        _ = NSApplication.shared
        for appearance in InterfaceAppearance.allCases {
            for fixture in InterfaceSettingsFixture.allCases {
                let manager = fixture.manager(defaults: try isolatedDefaults())
                defer { manager.stop() }
                let window = try render(SettingsView(hotKey: manager), width: 480, appearance: appearance.native,
                                        scheme: appearance.scheme, name: "\(appearance.rawValue)/settings-\(fixture.rawValue)")
                defer { window.close() }
                let host = try XCTUnwrap(window.contentView)
                let buttons = try verifyButtons(in: host)
                let recorder = try XCTUnwrap(descendants(of: host).compactMap { $0 as? NSButton }
                    .first { $0.accessibilityIdentifier() == "shortcutRecorder" })
                XCTAssertEqual(recorder.title, manager.recorderTitle)
                XCTAssertEqual(recorder.accessibilityValue() as? String, manager.recorderTitle)
                XCTAssertEqual(recorder.accessibilityHelp(), manager.helperText)
                XCTAssertEqual(recorder.bounds.width, 144, accuracy: 0.5)
                if fixture == .idle || fixture == .custom {
                    let restore = try XCTUnwrap(buttons.first { buttonLabel($0) == "Restore Default" })
                    XCTAssertEqual(restore.isAccessibilityEnabled(), fixture == .custom)
                } else {
                    XCTAssertTrue(buttons.contains { buttonLabel($0) == "Cancel" })
                }
            }
        }
    }

    func testPayloadEditorPreservesUnicodeAndUsesPayloadCopyWithoutSelection() throws {
        _ = NSApplication.shared
        let value = "First line\n\(QRFixtures.greekGreeting) — 日本語 — 👩🏽\u{200D}💻\nLast line"
        var copies = 0
        let window = try render(PayloadTextView(text: value, copyPayload: { copies += 1 }), width: 440,
                                appearance: .aqua, scheme: .light, name: "light/payload-selection")
        defer { window.close() }
        let host = try XCTUnwrap(window.contentView)
        let editor = try XCTUnwrap(descendants(of: host).compactMap { $0 as? PayloadTextEditor }.first)
        XCTAssertEqual(editor.string, value)
        let selected = (value as NSString).range(of: QRFixtures.greekGreeting)
        editor.setSelectedRange(selected)
        XCTAssertEqual((editor.string as NSString).substring(with: editor.selectedRange()), QRFixtures.greekGreeting)
        XCTAssertEqual(copies, 0)
        editor.setSelectedRange(NSRange(location: 0, length: 0))
        editor.copy(nil)
        XCTAssertEqual(copies, 1)
        XCTAssertEqual(editor.string, value)
    }

    func testLiveSettingsHostGrowsForFailureAndShrinksAfterCancellation() throws {
        _ = NSApplication.shared
        for appearance in InterfaceAppearance.allCases {
            let manager = InterfaceSettingsFixture.idle.manager(defaults: try isolatedDefaults())
            defer { manager.stop() }
            let window = try render(SettingsView(hotKey: manager), width: 480, appearance: appearance.native,
                                    scheme: appearance.scheme, name: "\(appearance.rawValue)/settings-live-idle")
            defer { window.close() }
            let host = try XCTUnwrap(window.contentView)
            let initialHeight = host.bounds.height
            let liveRecorder = try XCTUnwrap(descendants(of: host).compactMap { $0 as? NSButton }
                .first { $0.accessibilityIdentifier() == "shortcutRecorder" })
            liveRecorder.performClick(nil)
            XCTAssertEqual(liveRecorder.title, manager.recorderTitle)
            XCTAssertEqual(liveRecorder.accessibilityHelp(), manager.helperText)
            XCTAssertFalse(manager.update(KeyboardShortcut(keyCode: UInt32(kVK_Space),
                                                           modifiers: KeyboardShortcut.defaultValue.modifiers, keyLabel: "Space")))
            let failureHeight = try fit(window)
            XCTAssertTrue(window.contentView === host)
            XCTAssertGreaterThan(failureHeight, initialHeight)
            let failureButtons = try verifyButtons(in: host)
            XCTAssertTrue(failureButtons.contains { buttonLabel($0) == "Try Another Shortcut" })
            let recorder = try XCTUnwrap(failureButtons.first { $0.accessibilityIdentifier() == "shortcutRecorder" })
            XCTAssertEqual(recorder.bounds.width, 144, accuracy: 0.5)
            XCTAssertEqual(recorder.title, manager.recorderTitle)
            XCTAssertEqual(recorder.accessibilityValue() as? String, manager.recorderTitle)
            XCTAssertEqual(recorder.accessibilityHelp(), manager.helperText)
            try writeImage(of: window, name: "\(appearance.rawValue)/settings-live-unavailable")
            manager.cancelRecording()
            XCTAssertEqual(try fit(window), initialHeight, accuracy: 0.5)
            XCTAssertTrue(window.contentView === host)
            let restore = try XCTUnwrap(verifyButtons(in: host).first { buttonLabel($0) == "Restore Default" })
            XCTAssertFalse(restore.isAccessibilityEnabled())
            XCTAssertEqual(recorder.title, manager.recorderTitle)
            XCTAssertEqual(recorder.accessibilityValue() as? String, manager.recorderTitle)
            XCTAssertEqual(recorder.accessibilityHelp(), manager.helperText)
            try writeImage(of: window, name: "\(appearance.rawValue)/settings-live-restored")
        }
    }

    private func isolatedDefaults() throws -> UserDefaults {
        let name = "QRScanner.InterfaceTests.\(UUID().uuidString)"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: name))
        addTeardownBlock { UserDefaults.standard.removePersistentDomain(forName: name) }
        return defaults
    }

    private func descendants(of view: NSView) -> [NSView] {
        [view] + view.subviews.flatMap { descendants(of: $0) }
    }

    private func buttonLabel(_ element: any NSAccessibilityProtocol) -> String {
        element.accessibilityLabel() ?? element.accessibilityTitle() ?? ""
    }

    private func verifyButtons(in host: NSView) throws -> [NSButton] {
        let buttons = descendants(of: host).compactMap { $0 as? NSButton }.filter { !$0.isHiddenOrHasHiddenAncestor }
        XCTAssertFalse(buttons.isEmpty, "The production view must expose accessible action buttons")
        let window = try XCTUnwrap(host.window)
        let bounds = window.convertToScreen(host.convert(host.bounds, to: nil))
        for button in buttons {
            let label = buttonLabel(button)
            let rect = button.accessibilityFrame()
            let nativeRect = window.convertToScreen(button.convert(button.bounds, to: nil))
            XCTAssertEqual(button.accessibilityRole(), .button)
            XCTAssertEqual(button.isAccessibilityEnabled(), button.isEnabled)
            XCTAssertFalse(label.isEmpty)
            XCTAssertGreaterThan(rect.width, 0, label)
            XCTAssertGreaterThan(rect.height, 0, label)
            XCTAssertGreaterThan(nativeRect.width, 0, label)
            XCTAssertGreaterThan(nativeRect.height, 0, label)
            XCTAssertTrue(bounds.insetBy(dx: -0.5, dy: -0.5).contains(nativeRect), "Clipped native button: \(label), \(nativeRect)")
            XCTAssertTrue(bounds.insetBy(dx: -0.5, dy: -0.5).contains(rect), "Clipped action: \(label), \(rect)")
        }
        return buttons
    }

    private func render<Content: View>(_ content: Content, width: CGFloat, appearance: NSAppearance.Name,
                                       scheme: ColorScheme, name: String, hidesWindowControls: Bool = false) throws -> NSWindow {
        let window = NSPanel(contentRect: NSRect(x: 0, y: 0, width: width, height: 600),
                             styleMask: [.titled, .closable, .miniaturizable, .resizable, .utilityWindow], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        window.title = name.contains("settings") ? "Settings" : "QRScanner"
        window.appearance = try XCTUnwrap(NSAppearance(named: appearance))
        let host = NSHostingView(rootView: content.environment(\.colorScheme, scheme).frame(width: width))
        host.sizingOptions = [.intrinsicContentSize]
        window.contentView = host
        for control in [NSWindow.ButtonType.closeButton, .miniaturizeButton, .zoomButton] {
            window.standardWindowButton(control)?.isHidden = hidesWindowControls
        }
        _ = try fit(window)
        window.makeKey()
        host.layoutSubtreeIfNeeded()
        try writeImage(of: window, name: name)
        return window
    }

    private func fit(_ window: NSWindow) throws -> CGFloat {
        let host = try XCTUnwrap(window.contentView)
        host.needsLayout = true
        host.layoutSubtreeIfNeeded()
        let height = host.fittingSize.height
        XCTAssertTrue(height.isFinite && height > 0)
        XCTAssertLessThan(height, 800)
        window.setContentSize(NSSize(width: host.bounds.width, height: height))
        host.layoutSubtreeIfNeeded()
        return height
    }

    private func writeImage(of window: NSWindow, name: String) throws {
        let host = try XCTUnwrap(window.contentView)
        let canvas = try XCTUnwrap(host.superview)
        canvas.layoutSubtreeIfNeeded()
        let bitmap = try XCTUnwrap(canvas.bitmapImageRepForCachingDisplay(in: canvas.bounds))
        canvas.cacheDisplay(in: canvas.bounds, to: bitmap)
        let png = try XCTUnwrap(bitmap.representation(using: .png, properties: [:]))
        XCTAssertGreaterThan(png.count, 0)
        #if arch(arm64)
        let architecture = "arm64"
        #else
        let architecture = "x86_64"
        #endif
        let path = URL(fileURLWithPath: "build/ui-review/\(architecture)/\(name).png")
        try FileManager.default.createDirectory(at: path.deletingLastPathComponent(), withIntermediateDirectories: true)
        try png.write(to: path)
    }
}
