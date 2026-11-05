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
        let keys = ["shortcut.code", "shortcut.modifiers", "shortcut.label"]
        if keys.allSatisfy({ defaults.object(forKey: $0) == nil }),
           let code = imported["shortcut.code"] as? NSNumber,
           let modifiers = imported["shortcut.modifiers"] as? NSNumber,
           CFGetTypeID(code) != CFBooleanGetTypeID(), CFGetTypeID(modifiers) != CFBooleanGetTypeID(),
           code.int64Value >= 0, modifiers.int64Value >= 0,
           let exactCode = UInt32(exactly: code.doubleValue), let exactModifiers = UInt(exactly: modifiers.doubleValue),
           let label = imported["shortcut.label"] as? String,
           KeyboardShortcut(keyCode: exactCode, modifiers: exactModifiers, keyLabel: label).isValid {
            defaults.set(exactCode, forKey: keys[0])
            defaults.set(exactModifiers, forKey: keys[1])
            defaults.set(label, forKey: keys[2])
        }
        for key in ["hasShownWelcome", "screenAccess.requested"] where defaults.object(forKey: key) == nil {
            if let value = imported[key] as? NSNumber, CFGetTypeID(value) == CFBooleanGetTypeID() {
                defaults.set(value.boolValue, forKey: key)
            }
        }
        defaults.set(true, forKey: marker)
        ProcessingLog.event(.migration, .success)
    }
}
