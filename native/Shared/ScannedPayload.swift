import Foundation

struct ScannedPayload: Identifiable, Hashable, Sendable {
    let text: String
    let webURL: URL?

    var id: String { text }
    var host: String? { webURL?.host() }

    init(_ text: String) {
        self.text = text
        guard !text.contains(where: { $0.isWhitespace }),
              !text.unicodeScalars.contains(where: { CharacterSet.controlCharacters.contains($0) }),
              let url = URL(string: text, encodingInvalidCharacters: false),
              let scheme = url.scheme?.lowercased(),
              scheme == "https" || scheme == "http",
              let host = url.host(), !host.isEmpty,
              url.user() == nil, url.password() == nil else {
            webURL = nil
            return
        }
        webURL = url
    }

    static func unique(_ texts: [String]) -> [Self] {
        var seen = Set<String>()
        return texts.filter { seen.insert($0).inserted }.map(Self.init)
    }
}
