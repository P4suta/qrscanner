import AppKit
import SwiftUI

struct PayloadTextView: NSViewRepresentable {
    let text: String
    let copyPayload: () -> Void

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.borderType = .noBorder
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true
        let value = PayloadTextEditor()
        value.isEditable = false
        value.isSelectable = true
        value.isRichText = false
        value.drawsBackground = false
        value.font = .preferredFont(forTextStyle: .body)
        value.textColor = .labelColor
        value.textContainerInset = .zero
        value.textContainer?.lineFragmentPadding = 0
        value.textContainer?.widthTracksTextView = true
        value.isHorizontallyResizable = false
        value.isVerticallyResizable = true
        value.autoresizingMask = [.width]
        value.setAccessibilityLabel(String(localized: "Decoded value"))
        scroll.documentView = value
        updateNSView(scroll, context: context)
        return scroll
    }

    func updateNSView(_ nsView: NSScrollView, context: Context) {
        guard let value = nsView.documentView as? PayloadTextEditor else { return }
        value.copyPayload = copyPayload
        value.font = .preferredFont(forTextStyle: .body)
        if value.string != text {
            value.string = text
            value.setSelectedRange(NSRange(location: 0, length: 0))
            nsView.contentView.scroll(to: .zero)
            nsView.reflectScrolledClipView(nsView.contentView)
        }
    }

    func sizeThatFits(_ proposal: ProposedViewSize, nsView: NSScrollView, context: Context) -> CGSize? {
        guard let value = nsView.documentView as? NSTextView,
              let container = value.textContainer, let layout = value.layoutManager else { return nil }
        let width = proposal.width ?? 440
        let gutter = nsView.scrollerStyle == .legacy
            ? NSScroller.scrollerWidth(for: .regular, scrollerStyle: .legacy) : 0
        let textWidth = max(1, width - gutter)
        container.containerSize = NSSize(width: textWidth, height: .greatestFiniteMagnitude)
        layout.ensureLayout(for: container)
        let height = max(16, ceil(layout.usedRect(for: container).height))
        value.setFrameSize(NSSize(width: textWidth, height: height))
        return CGSize(width: width, height: min(height, 160))
    }
}

@MainActor
final class PayloadTextEditor: NSTextView {
    var copyPayload: (() -> Void)?

    override func copy(_ sender: Any?) {
        if selectedRange().length > 0 {
            super.copy(sender)
        } else {
            copyPayload?()
        }
    }
}
