# Native interface assurance

## Contract and production boundaries

The [interface design](macos-interface.md) keeps screenshots and results in memory.
Capture covers every reported display, excludes QRScanner, uses the filter's pixel scale, requests standard dynamic range, and leaves the screenshot file destination unset.
The result panel stays hidden until capture completes.
Cancellation and obsolete operation generations reject late content and failures.
Revocation before result publication discards captured results and offers permission recovery.

The Rust core admits every generation, domain phase, and event.
The foreign interface validates wire tags and Boolean fields before constructing domain events.
The value-only `C ABI` preserves its existing layout and transition behavior.
Kani imports production functions to check confirmed opening, stale events, cancellation, generation overflow, wire validation, icon lengths, notarization identity, and the release gate's required facts.
The release predicate proof covers its declared Boolean facts rather than the empirical truth of those facts or the complete parser and filesystem adapter.

Swift adapters own permission queries, capture, Vision, window presentation, preferences, browser opening, clipboard writes, shortcuts, and relaunch.
These boundaries depend on Swift actors and Apple's framework object graphs, which Kani doesn't execute.
Strict Swift concurrency checking, injected failures, real framework fixtures, and native interaction tests provide a scoped alternative.
They aren't an all-input proof of Swift or Apple framework behavior.
Reassess the alternative when a maintained verifier can execute those boundaries or when a decision moves into the Rust core.

ScreenCaptureKit must return the requested image on a successful call, CoreGraphics must report current access, and system browser, clipboard, and launch APIs must report their outcomes accurately.
Carbon cleanup assumes removal of a valid registration succeeds.
Known-answer Vision images and a blank control test recognition without establishing accuracy on arbitrary screens.
One retry on a Vision internal error uses a supported CPU device and preserves both causes if recovery fails.

Container migration imports only valid shortcut preferences and Boolean welcome and permission-request flags.
Existing container preferences take precedence, and repeated import changes nothing.
Native tests exercise the importer and relaunch ordering without modifying a person's real container or launching another app instance.
The signed app's migration and sandbox behavior still need live acceptance.

## Verification and evidence

`mise x -- just check` runs the fast contributor gate.
`mise x -- just verify-release` runs optimized native suites on both architectures, every production Kani harness and its rejecting control, and the complete core and foreign-interface mutation inventory.
Missing, empty, failed, timed-out, and incomplete required work creates no completed release receipt.
The runner records source, tools, target semantics, logs, executable, and resource identities.
Tests reject missing checks, duplicate checks, changed sources, changed binaries, and incorrect acceptance records.

Native tests preserve empty values, Unicode, long links, and result selection after action failure.
They cover permission denial, cancellation, revocation, recovery, stale completion, shortcut conflict, settings failures, and relaunch failures.
Production views render in both appearances and at 200% text in narrow windows.
Generated PNGs remain under `build/ui-review/` for visual inspection.

Screen Recording consent, display fidelity, protected content, app exclusion, and full-screen Spaces require a normal login session.
The same applies to global shortcut delivery, browser launching, clipboard access, sandbox relaunch, VoiceOver, and system accessibility preferences.
The release acceptance record requires macOS 26 observations for both architecture slices.
Evidence from a newer OS remains evidence for that OS.
No release or notarization follows from a passing development test.
