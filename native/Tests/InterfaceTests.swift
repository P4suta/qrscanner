import AppKit
import SwiftUI
import XCTest

@MainActor
final class InterfaceTests: XCTestCase {
    func testShortResultListsShowEveryRowWithoutClipping() throws {
        _ = NSApplication.shared
        for appearance in InterfaceAppearance.allCases {
            for width: CGFloat in [320, 480] {
                let model = InterfaceFixture.twoResults.model(defaults: try isolatedDefaults())
                defer { model.cancel() }
                let window = try render(ResultView(model: model), width: width, appearance: appearance.native,
                                        scheme: appearance.scheme, name: "\(appearance.rawValue)/two-results-\(Int(width))")
                defer { window.close() }
                let host = try XCTUnwrap(window.contentView)
                let table = try XCTUnwrap(descendants(of: host).compactMap { $0 as? NSTableView }.first)
                XCTAssertEqual(table.numberOfRows, model.payloads.count)
                let clip = try XCTUnwrap(table.enclosingScrollView?.contentView)
                for row in 0..<table.numberOfRows {
                    let rect = table.convert(table.rect(ofRow: row), to: clip)
                    XCTAssertTrue(clip.bounds.insetBy(dx: -0.5, dy: -0.5).contains(rect), "Clipped result row \(row): \(rect), viewport \(clip.bounds)")
                }
            }
        }
    }

    func testNarrowWindowsKeepStandardActionsVisible() throws {
        _ = NSApplication.shared
        for fixture in [InterfaceFixture.link, .multiple, .recovery, .openFailure, .longLink] {
            let model = fixture.model(defaults: try isolatedDefaults())
            defer { model.cancel() }
            let window = try render(ResultView(model: model), width: 320,
                                    appearance: .aqua, scheme: .light, name: "narrow/\(fixture.rawValue)-320")
            defer { window.close() }
            let host = try XCTUnwrap(window.contentView)
            let buttons = try verifyButtons(in: host)
            for button in buttons {
                XCTAssertEqual(button.bounds.height, button.fittingSize.height, accuracy: 0.5)
            }
            for editor in descendants(of: host).compactMap({ $0 as? PayloadTextEditor }) {
                XCTAssertEqual(editor.font?.pointSize ?? 0, NSFont.preferredFont(forTextStyle: .body).pointSize, accuracy: 0.1)
            }
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
        let window = ResultPanel.make(contentRect: NSRect(x: 0, y: 0, width: width, height: 600))
        window.isReleasedWhenClosed = false
        window.title = "QRScanner"
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
