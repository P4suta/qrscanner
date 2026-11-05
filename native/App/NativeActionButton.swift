import AppKit
import SwiftUI

struct NativeActionButton: NSViewRepresentable {
    let title: String
    let primary: Bool
    let action: () -> Void
    @Environment(\.isEnabled) private var isEnabled

    init(_ title: LocalizedStringResource, primary: Bool = false, action: @escaping () -> Void) {
        self.title = String(localized: title)
        self.primary = primary
        self.action = action
    }

    func makeCoordinator() -> Coordinator { Coordinator(action: action) }

    func makeNSView(context: Context) -> ActionButton {
        let button = ActionButton(title: title, target: context.coordinator, action: #selector(Coordinator.activate))
        button.bezelStyle = .rounded
        button.controlSize = .regular
        button.font = .preferredFont(forTextStyle: .body)
        button.focusRingType = .exterior
        button.setAccessibilityElement(true)
        button.setAccessibilityRole(.button)
        updateNSView(button, context: context)
        return button
    }

    func updateNSView(_ nsView: ActionButton, context: Context) {
        context.coordinator.action = action
        nsView.title = title
        nsView.isEnabled = isEnabled
        nsView.primary = primary
        nsView.setAccessibilityLabel(title)
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: ActionButton, context: Context) -> CGSize? {
        nsView.fittingSize
    }

    @MainActor
    final class Coordinator: NSObject {
        var action: () -> Void
        init(action: @escaping () -> Void) { self.action = action }
        @objc func activate() { action() }
    }
}

struct NativeActions<Content: View>: View {
    let content: Content

    init(@ViewBuilder content: () -> Content) { self.content = content() }

    var body: some View {
        ViewThatFits(in: .horizontal) {
            HStack(spacing: 8) { Spacer(minLength: 0); content.fixedSize() }
            VStack(alignment: .trailing, spacing: 8) { content.fixedSize(horizontal: false, vertical: true) }
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
    }
}

@MainActor
final class ActionButton: NSButton {
    var primary = false { didSet { updateDefaultButton() } }
    override var acceptsFirstResponder: Bool { true }

    override func viewWillMove(toWindow newWindow: NSWindow?) {
        if window !== newWindow, window?.defaultButtonCell === cell { window?.defaultButtonCell = nil }
        super.viewWillMove(toWindow: newWindow)
    }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        updateDefaultButton()
    }

    private func updateDefaultButton() {
        keyEquivalent = primary ? "\r" : ""
        keyEquivalentModifierMask = []
        if primary {
            window?.defaultButtonCell = cell as? NSButtonCell
        } else if window?.defaultButtonCell === cell {
            window?.defaultButtonCell = nil
        }
    }
}
