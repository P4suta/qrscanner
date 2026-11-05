import AppKit
import SwiftUI

struct ResultView: View {
    @ObservedObject var model: AppModel
    var shortcut: String? = KeyboardShortcut.defaultValue.display

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            if let permission = model.permissionPresentation {
                permissionView(permission)
            } else if let failure = model.failure {
                failureView(failure)
            } else {
                scanContent
            }
        }
        .appFont(.body)
        .padding(20)
        .frame(minWidth: 320, idealWidth: 480, maxWidth: .infinity, alignment: .leading)
        .fixedSize(horizontal: false, vertical: true)
        .background(Color(nsColor: .windowBackgroundColor))
        .onExitCommand {
            if model.permissionPresentation != .explanation { model.cancel() }
        }
    }

    @ViewBuilder private var scanContent: some View {
        if model.state.phase == QR_PHASE_IDLE {
            status("Scan QR codes on your screen", symbol: "qrcode.viewfinder",
                   message: "Show a QR code on any display, then scan to open its link or copy its text.")
            Text(shortcut.map { String(localized: "Scan from anywhere with \($0).") } ?? String(localized: "Use Scan Screen in the menu bar."))
                .appFont(.callout).foregroundStyle(.secondary)
            actions { NativeActionButton("Scan Screen", primary: true, action: model.scan) }
        } else if model.state.phase == QR_PHASE_SCANNING {
            HStack(spacing: 12) {
                ProgressView().controlSize(.small)
                Text("Looking for QR codes…")
            }
            actions { NativeActionButton("Cancel", primary: true, action: model.cancel) }
        } else if model.payloads.isEmpty {
            noResults
        } else {
            results
        }
    }

    private var noResults: some View {
        Group {
            if model.unreadableCount > 0 {
                status("No readable text", symbol: "doc.questionmark",
                       message: "This QR code doesn’t contain readable text. QRScanner can open web links and copy text.")
            } else {
                status("No QR codes found", symbol: "qrcode.viewfinder",
                       message: "Make sure the entire QR code is visible, then scan again.")
            }
            actions {
                NativeActionButton("Close", action: model.cancel)
                NativeActionButton("Scan Again", primary: true, action: model.scan)
            }
        }
    }

    @ViewBuilder private var results: some View {
        if model.payloads.count > 1 {
            Text(AppStrings.resultCount(model.payloads.count))
                .appFont(.headline)
            ResultList(payloads: model.payloads, selection: $model.selection)
                .frame(height: min(CGFloat(model.payloads.count) * 49, 196))
        }
        if let payload = model.selectedPayload {
            if payload.text.isEmpty {
                status("Empty text", symbol: "text.alignleft", message: "This QR code contains an empty text value.")
            } else {
                payloadDetail(payload)
            }
            if let message = model.actionError { errorMessage(message) }
            actions {
                if model.actionFailure == .copy {
                    NativeActionButton("Close", action: model.cancel)
                } else if model.actionFailure == nil {
                    NativeActionButton("Scan Again", action: model.scan)
                }
                if payload.webURL != nil {
                    NativeActionButton("Copy Link", action: model.copySelected)
                    NativeActionButton("Open Link", primary: true, action: model.openSelected)
                } else {
                    NativeActionButton("Copy Text", primary: true, action: model.copySelected)
                }
            }
        }
    }

    private func payloadDetail(_ payload: ScannedPayload) -> some View {
        VStack(alignment: .leading, spacing: 16) {
            HStack(spacing: 12) {
                Image(systemName: payload.webURL == nil ? "text.alignleft" : "link")
                    .appFont(.title2).foregroundStyle(.tint)
                    .frame(width: 24).accessibilityHidden(true)
                VStack(alignment: .leading, spacing: 2) {
                    if payload.webURL != nil {
                        Text(model.payloads.count > 1 ? String(localized: "Selected link") : String(localized: "Link"))
                            .appFont(.callout).foregroundStyle(.secondary)
                    } else if URLComponents(string: payload.text)?.scheme != nil {
                        Text("QR Code Content").appFont(.callout).foregroundStyle(.secondary)
                    }
                    Text(payload.host ?? String(localized: "Text"))
                        .appFont(.title3).fontWeight(.bold)
                        .fixedSize(horizontal: false, vertical: true)
                        .textSelection(.enabled)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            PayloadTextView(text: payload.text, copyPayload: model.copySelected)
                .accessibilityLabel(payload.webURL == nil ? String(localized: "Decoded text") : String(localized: "Full link"))
        }
    }

    @ViewBuilder private func permissionView(_ permission: AppModel.PermissionPresentation) -> some View {
        switch permission {
        case .explanation:
            status("Allow Screen Recording", symbol: "rectangle.inset.filled.and.person.filled",
                   message: "QRScanner needs screen access to read QR codes. Screenshots are processed on your Mac and aren’t saved.")
            actions { NativeActionButton("Continue", primary: true, action: model.requestScreenAccess) }
        case .recovery:
            status("Turn on screen access", symbol: "lock.rectangle",
                   message: "In System Settings, open Privacy & Security → Screen & System Audio Recording, then enable QRScanner.")
            Text("If macOS asks you to reopen the app, choose Quit & Reopen.")
                .appFont(.callout).foregroundStyle(.secondary)
            if let message = model.permissionError { errorMessage(message) }
            actions {
                NativeActionButton("Try Again", action: model.retryScreenAccess)
                NativeActionButton("Open System Settings…", primary: true, action: model.openScreenSettings)
            }
        case .ready:
            status("Screen access is ready", symbol: "checkmark.circle",
                   message: "Make sure the QR code is visible, then scan your screen.")
            actions {
                NativeActionButton("Close", action: model.cancel)
                NativeActionButton("Scan Screen", primary: true, action: model.retryScreenAccess)
            }
        case .relaunchRequired:
            status("Reopen QRScanner", symbol: "arrow.clockwise",
                   message: "Screen access is enabled. Reopen QRScanner to use it.")
            if let message = model.permissionError { errorMessage(message) }
            actions {
                NativeActionButton("Close", action: model.cancel)
                NativeActionButton("Relaunch QRScanner", primary: true, action: model.relaunch)
                    .disabled(model.isRelaunching)
            }
        }
    }

    private func failureView(_ failure: AppModel.FailureKind) -> some View {
        Group {
            status("Couldn’t scan your screen", symbol: "exclamationmark.triangle",
                   message: failureMessage(failure))
            actions {
                NativeActionButton("Close", primary: failure == .exhausted, action: model.cancel)
                if failure != .exhausted {
                    NativeActionButton("Try Again", primary: true, action: model.scan)
                }
            }
        }
    }

    private func failureMessage(_ failure: AppModel.FailureKind) -> LocalizedStringResource {
        switch failure {
        case .exhausted: "Quit and reopen QRScanner to scan again."
        case .noDisplays: "No displays are available. Connect or wake a display, then try again."
        case .unavailableImage: "A display image wasn’t available. Make sure the QR code is visible, then try again."
        case .recognition: "Couldn’t read the captured image. Try scanning again."
        case .permission, .capture, .invalidState: "Try scanning again. If the problem continues, quit and reopen QRScanner."
        }
    }

    private func status(_ title: LocalizedStringResource, symbol: String, message: LocalizedStringResource) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Image(systemName: symbol).appFont(.largeTitle).foregroundStyle(.tint).accessibilityHidden(true)
            Text(title).appFont(.title3).fontWeight(.bold)
            Text(message).foregroundStyle(.secondary).fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func errorMessage(_ message: String) -> some View {
        Label(message, systemImage: "exclamationmark.circle")
            .appFont(.body).foregroundStyle(.primary)
            .fixedSize(horizontal: false, vertical: true)
    }

    private func actions<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        NativeActions(content: content)
    }
}
