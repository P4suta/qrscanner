import Foundation

enum AppStrings {
    static func resultCount(_ count: Int, bundle: Bundle = .main) -> String {
        String(localized: "\(count) QR codes found", bundle: bundle)
    }
}
