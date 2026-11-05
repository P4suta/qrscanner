---
date: 2026-10-20
status: accepted
---

# Native macOS app with a verified decision core

## Status

Accepted.
Partly superseded for deployment target and sandbox adoption by [Require macOS 26 and sandboxed capture](require-macos26-and-sandboxed-capture.md).

## Context

Reading a QR code displayed on a computer should not require another device.
The app must capture visible screens on demand, show the decoded destination, and open a web URL only after confirmation.
The first distribution targets macOS 14 and later on Apple Silicon and Intel.
The interaction must remain responsive during capture and recognition.

## Decision

Use SwiftUI and AppKit for the menu bar, result panel, and shortcut recorder.
Use ScreenCaptureKit for screenshots and Vision for QR recognition.
Keep images and results in memory, and persist only shortcut preferences and the welcome-screen preference.

A dependency-free `no_std` Rust core returns state transitions and named effects.
Kani imports the production functions and checks confirmation, cancellation, stale generations, and overflow behavior.
The native adapter alone owns screen access, browser opening, and clipboard writes.

Connect Swift and Rust through a C ABI of fixed-width, by-value records.
Validate every wire tag and Boolean before constructing a domain event.
Pass no pointers, strings, ownership, callbacks, or asynchronous operations through the ABI.
Compile layout assertions in the header and exercise it from native tests.

UniFFI was considered, but its dependencies were unavailable in the offline implementation environment.
The limited value-only interface does not require generated bindings.
A Swift-only core would lack the selected implementation-bound verifier.
A web-based UI would introduce a browser runtime for a small native interaction.

## Consequences

Building requires the pinned Rust toolchain and Xcode.
The macOS permission system, screenshot fidelity, Vision recognition, and platform effects remain native integration boundaries.
Their tests cannot be replaced by the Rust proofs.
Distribution requires completed native verification, Developer ID signing, matching notarization receipts, and Gatekeeper checks.
