import AppKit

@MainActor
final class ResultPanel: NSPanel {
    weak var model: AppModel?

    override func sendEvent(_ event: NSEvent) {
        let flags = event.modifierFlags.intersection([.command, .control, .option, .shift])
        if event.type == .keyDown, flags.isEmpty, event.keyCode == 36 || event.keyCode == 76 {
            model?.performDefaultAction()
            return
        }
        super.sendEvent(event)
    }

    override func cancelOperation(_ sender: Any?) {
        if model?.permissionPresentation != .explanation { model?.cancel() }
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        let flags = event.modifierFlags.intersection([.command, .control, .option, .shift])
        if flags == .command, event.charactersIgnoringModifiers?.lowercased() == "c", model?.selectedPayload != nil {
            if let text = firstResponder as? NSTextView, text.selectedRange().length > 0 {
                text.copy(nil)
            } else {
                model?.copySelected()
            }
            return true
        }
        return super.performKeyEquivalent(with: event)
    }
}
