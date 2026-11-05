import SwiftUI

struct ResultList: View {
    private static let maximumHeight: CGFloat = 196
    let payloads: [ScannedPayload]
    @Binding var selection: String?
    @State private var contentHeight: CGFloat?

    var body: some View {
        List(payloads, selection: $selection) { payload in
            HStack(spacing: 10) {
                Image(systemName: payload.webURL == nil ? "text.alignleft" : "link")
                    .font(.title2).frame(width: 20).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text(payload.host ?? String(localized: "Text")).font(.headline)
                    Text(preview(payload)).font(.callout).foregroundStyle(.secondary).lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .listRowSeparator(.hidden)
            .tag(payload.id)
        }
        .listStyle(.inset(alternatesRowBackgrounds: false))
        .scrollContentBackground(.hidden)
        .onScrollGeometryChange(for: CGFloat.self) { geometry in
            geometry.contentSize.height + geometry.contentInsets.top + geometry.contentInsets.bottom
        } action: { _, height in
            if height.isFinite, height > 0 { contentHeight = height }
        }
        .frame(height: min(contentHeight ?? Self.maximumHeight, Self.maximumHeight))
        .accessibilityLabel("QR codes")
    }

    private func preview(_ payload: ScannedPayload) -> String {
        guard let url = payload.webURL, let host = payload.host,
              let range = payload.text.range(of: host) else {
            return payload.text.components(separatedBy: .newlines).first ?? payload.text
        }
        let suffix = String(payload.text[range.upperBound...])
        return suffix.isEmpty ? url.path : suffix
    }
}
