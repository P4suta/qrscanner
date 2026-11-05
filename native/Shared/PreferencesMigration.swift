import CoreFoundation
import Foundation

@MainActor
enum PreferencesMigration {
    private static let marker = "preferences.containerMigrationCompleted"

    static func load(defaults: UserDefaults, file: URL? = nil) {
        guard !defaults.bool(forKey: marker) else { return }
        let source = file ?? FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first?
            .appending(path: "QRScanner/ImportedPreferences.plist")
        guard let source, FileManager.default.fileExists(atPath: source.path) else { return }
        do {
            let data = try Data(contentsOf: source)
            guard let imported = try PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any] else {
                ProcessingLog.event(.migration, .failure)
                return
            }
            apply(imported, to: defaults)
        } catch {
            ProcessingLog.event(.migration, .failure)
        }
    }

    static func apply(_ imported: [String: Any], to defaults: UserDefaults) {
        guard !defaults.bool(forKey: marker) else { return }
        for key in ["hasShownWelcome", "screenAccess.requested"] where defaults.object(forKey: key) == nil {
            if let value = imported[key] as? NSNumber, CFGetTypeID(value) == CFBooleanGetTypeID() {
                defaults.set(value.boolValue, forKey: key)
            }
        }
        defaults.set(true, forKey: marker)
        ProcessingLog.event(.migration, .success)
    }
}
