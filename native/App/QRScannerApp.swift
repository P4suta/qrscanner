import SwiftUI

@main
struct QRScannerApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    var body: some Scene {
        MenuBarExtra("QRScanner", systemImage: "qrcode.viewfinder") {
            ScannerMenu(model: delegate.model)
        }
    }
}

private struct ScannerMenu: View {
    @ObservedObject var model: AppModel

    var body: some View {
        Button("Scan Screen", action: model.scan)
        Divider()
        Button("Quit QRScanner") { NSApplication.shared.terminate(nil) }.keyboardShortcut("q")
    }

}
