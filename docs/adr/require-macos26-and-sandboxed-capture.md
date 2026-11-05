---
date: 2026-11-05
status: accepted
---

# Require macOS 26 and sandboxed capture

## Status

Accepted.
Partly superseded for deployment target and supported architecture by [Distribute macOS 27 on Apple silicon](distribute-macos27-on-apple-silicon.md).
Partly supersedes [Native macOS app with a verified decision core](native-macos-app-with-verified-core.md) for deployment target and sandbox adoption.

## Context

The existing macOS 14 target uses a streaming configuration for screenshots and manages settings through a custom panel.
The app needs explicit screenshot output, system settings behavior, preserved preferences, and a smaller resource-access boundary.
The Rust state machine and value-only `C ABI` already express confirmation and cancellation independently of Apple frameworks.

## Decision

Target macOS 26 and keep arm64 and x86_64 in one Universal app.
Use [captureScreenshot](https://developer.apple.com/documentation/screencapturekit/scscreenshotmanager/capturescreenshot(contentfilter:configuration:completionhandler:)) and [SCScreenshotConfiguration](https://developer.apple.com/documentation/screencapturekit/scscreenshotconfiguration), both introduced in macOS 26.
Request standard dynamic range images in memory with Retina pixel dimensions and exclude QRScanner on every display.
Keep the capture-aware result panel in AppKit with standard window chrome and button-driven actions.

Enable [App Sandbox](https://developer.apple.com/documentation/security/app-sandbox) without networking, audio, camera, or broad file-access entitlements.
Apple requires sandboxing for Mac App Store distribution.
Sandboxing for this Developer ID app is a project choice.
Use the [container migration manifest](https://developer.apple.com/documentation/security/migrating-your-app-s-files-to-its-app-sandbox-container) and an idempotent importer to preserve welcome and permission-request flags.

Bundle a no-collection privacy manifest as a project choice.
Apple's [privacy manifest scope](https://developer.apple.com/documentation/bundleresources/privacy-manifest-files) covers data-collection information on all platforms and names iOS, iPadOS, tvOS, visionOS, and watchOS for required-reason API declarations.
The macOS manifest makes no required-reason API claim.
Use [Logger and signposts](https://developer.apple.com/documentation/os/generating-log-messages-from-your-code) for fixed processing states, outcomes, and durations without images or decoded content.

## Consequences

Older macOS releases lose support in exchange for the explicit screenshot API contract.
An availability fallback to the previous API would preserve older systems but add a second capture contract and its verification cost.
The existing Rust state transitions and foreign interface remain unchanged.
Sandbox migration and system effects require native acceptance alongside pure-core proofs.
Signed distribution requires complete automated evidence and real macOS 26 login-session acceptance for both architecture slices.
