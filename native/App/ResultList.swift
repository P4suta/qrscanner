import SwiftUI

struct ResultList: View {
    let payloads: [ScannedPayload]
    @Binding var selection: String?
    @Environment(\.appTextScale) private var scale

    var body: some View {
        List(payloads, selection: $selection) { payload in
            HStack(spacing: 10) {
                Image(systemName: payload.webURL == nil ? "text.alignleft" : "link")
                    .appFont(.title2).frame(width: 20 * scale).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    Text(payload.host ?? String(localized: "Text")).appFont(.headline)
                    Text(preview(payload)).appFont(.callout).foregroundStyle(.secondary).lineLimit(1)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .frame(minHeight: 49 * scale)
            .listRowInsets(EdgeInsets(top: 0, leading: 12, bottom: 0, trailing: 12))
            .listRowSeparator(.hidden)
            .tag(payload.id)
        }
        .listStyle(.inset(alternatesRowBackgrounds: false))
        .scrollContentBackground(.hidden)
        .environment(\.defaultMinListRowHeight, 49 * scale)
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
