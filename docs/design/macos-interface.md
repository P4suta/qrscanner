# macOS interface

QRScanner scans visible QR codes, presents complete decoded content, and opens a web link only after an explicit action.
All interface copy is English and uses a String Catalog, including the result count's singular and plural forms.

## Surfaces and system behavior

Use a menu bar extra for Scan Screen and Quit.
Reopening the app provides access when the system hides the extra.
Apple's [menu bar guidance](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar#Menu-bar-extras) recommends a menu for concise commands and warns that extras can disappear.
The menu-bar-only lifecycle is a project choice.

The primary task uses action buttons and system defaults.

Keep the result panel under AppKit ownership so scanning can hide it before capture.
Present recognition progress only after all display images have arrived.
The panel uses native window controls, focus, selection, and system appearance under Apple's [window guidance](https://developer.apple.com/design/human-interface-guidelines/windows).
Use the normal titled panel style and keep system window buttons at their standard sizes and positions.
Floating presentation preserves the system window chrome.
Its initial width of 480 points and content inset of 20 points are project choices.
The panel supports widths down to 320 points, with content-driven height and vertical action layout when horizontal controls don't fit.

Use system text styles and native control colors and materials.
The [typography](https://developer.apple.com/design/human-interface-guidelines/typography) and [accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility) guidance support a readable hierarchy and text enlargement.
Use standard SwiftUI fonts and the system's preferred text font in the selectable text view.
Keep action buttons at their native fitting sizes beside the result's bounded text viewport.

## Results and failure recovery

A web result shows its host and the full selectable destination.
Open Link requires a validated `HTTP` or `HTTPS` URL and an explicit action.
Copy Link preserves the entire decoded value.
Text, empty text, and other URL schemes remain copy-only content.
A result list maintains a valid selection as the window changes focus.
Size the list from its [reported content size and insets](https://developer.apple.com/documentation/swiftui/scrollgeometry/contentinsets), with a bounded viewport for longer result lists.
Changing selection updates the detail and available actions.
Open and Copy failures keep the selected value and show recovery text beside the action.

The permission explanation precedes the system-owned decision.
It explains on-device processing and provides Continue under Apple's [privacy guidance](https://developer.apple.com/design/human-interface-guidelines/privacy).
The system permission dialog owns the grant or denial.
Recovery opens System Settings or rechecks access without automatically starting capture on activation.
Offer relaunch only when a reported grant remains unusable to preflight.
A failed relaunch keeps the current app and recovery available.

Return invokes the current default action, and Space activates the focused button.
Escape cancels the current operation outside the permission explanation.
Command-C copies a selected text range through the native text view or copies the full payload when the selection is empty.
Tab and Shift-Tab follow the native responder chain.

## Verification

Inspect light and dark appearances, 320-point layouts, system text preferences, long URLs, multiline Unicode, empty content, and result lists.
Automated tests check text preservation, standard window controls, button geometry, accessibility labels, native selection, and default actions.
Live acceptance checks VoiceOver, keyboard focus, Increase Contrast, Reduce Transparency, actual screen capture, and the sandboxed platform effects.
The [assurance contract](native-interface-assurance.md) distinguishes automated checks from live observations and proofs.
