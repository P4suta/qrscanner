import AppKit
import SwiftUI

private struct TextScaleKey: EnvironmentKey {
    static let defaultValue: CGFloat = 1
}

extension EnvironmentValues {
    var appTextScale: CGFloat {
        get { self[TextScaleKey.self] }
        set { self[TextScaleKey.self] = newValue.isFinite ? min(2, max(1, newValue)) : 1 }
    }
}

extension NSFont {
    static func appFont(_ style: NSFont.TextStyle, scale: CGFloat) -> NSFont {
        let preferred = NSFont.preferredFont(forTextStyle: style)
        return NSFont(descriptor: preferred.fontDescriptor, size: preferred.pointSize * scale) ?? preferred
    }
}

private struct SystemTextStyle: ViewModifier {
    let style: NSFont.TextStyle
    @Environment(\.appTextScale) private var scale

    func body(content: Content) -> some View {
        content.font(Font(NSFont.appFont(style, scale: scale)))
    }
}

extension View {
    func appFont(_ style: NSFont.TextStyle) -> some View { modifier(SystemTextStyle(style: style)) }
}
