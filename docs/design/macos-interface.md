# macOS interface

QRScanner scans visible QR codes, presents complete decoded content, and opens a web link only after an explicit action.
All interface copy is English and uses a String Catalog, including the result count's singular and plural forms.

## Surfaces and system behavior

Use a menu bar extra for Scan Screen, Settings, and Quit.
The configured global shortcut and reopening the app provide access when the system hides the extra.
Apple's [menu bar guidance](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar#Menu-bar-extras) recommends a menu for concise commands and warns that extras can disappear.
The menu-bar-only lifecycle is a project choice.

Use SwiftUI [Settings](https://developer.apple.com/documentation/swiftui/settings) and [SettingsLink](https://developer.apple.com/documentation/swiftui/settingslink) for the standard settings window and Command-Comma behavior.
Apple's [settings guidance](https://developer.apple.com/design/human-interface-guidelines/settings) places infrequent keyboard preferences in settings.
The settings view contains the shortcut recorder, recovery text, Restore Default, and app text enlargement.

Keep the result panel under AppKit ownership so scanning can hide it before capture.
Present recognition progress only after all display images have arrived.
The panel uses native window controls, focus, selection, and system appearance under Apple's [window guidance](https://developer.apple.com/design/human-interface-guidelines/windows).
Its initial width of 480 points and content inset of 20 points are project choices.
The panel supports widths down to 320 points, with content-driven height and vertical action layout when horizontal controls don't fit.

Use system text styles and native control colors and materials.
The [typography](https://developer.apple.com/design/human-interface-guidelines/typography) and [accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility) guidance support a readable hierarchy and text enlargement.
The app enlarges preferred system fonts up to 200% and keeps action buttons beside the result's bounded text viewport.
Native [flexible-height buttons](https://developer.apple.com/documentation/appkit/nsbutton/bezelstyle-swift.enum/flexiblepush) accommodate enlarged labels without clipping their text or bezel.

## Results and failure recovery

A web result shows its host and the full selectable destination.
Open Link requires a validated `HTTP` or `HTTPS` URL and an explicit action.
Copy Link preserves the entire decoded value.
Text, empty text, and other URL schemes remain copy-only content.
A result list maintains a valid selection as the window changes focus.
Changing selection updates the detail and available actions.
Open and Copy failures keep the selected value and show recovery text beside the action.

The permission explanation precedes the system-owned decision.
It explains on-device processing and provides Continue under Apple's [privacy guidance](https://developer.apple.com/design/human-interface-guidelines/privacy).
The system permission dialog owns the grant or denial.
Recovery opens System Settings or rechecks access without automatically starting capture on activation.
Offer relaunch only when a reported grant remains unusable to preflight.
A failed relaunch restores the shortcut and keeps recovery available.

Return invokes the current default action, and Space activates the focused button.
Escape cancels the current operation outside the permission explanation.
Command-C copies a selected text range through the native text view or copies the full payload when the selection is empty.
Tab and Shift-Tab follow the native responder chain.
Shortcut recording requires Command, Control, or Option and preserves the previous registered shortcut after a conflict.

## Verification

Inspect light and dark appearances, 320-point layouts, 200% text, long URLs, multiline Unicode, empty content, and result lists.
Automated tests check text preservation, button geometry, accessibility labels, native selection, default actions, and shortcut failure behavior.
Live acceptance checks VoiceOver, keyboard focus, Increase Contrast, Reduce Transparency, actual screen capture, and the sandboxed platform effects.
The [assurance contract](native-interface-assurance.md) distinguishes automated checks from live observations and proofs.
