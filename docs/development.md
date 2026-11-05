# Development

Build on macOS 26 or later with Xcode and the tools in `mise.toml`.
The app targets macOS 26 and contains arm64 and x86_64 slices.

```sh
mise trust
mise install
mise x -- rustup component add rustfmt clippy
mise x -- rustup target add aarch64-apple-darwin x86_64-apple-darwin
mise x -- cargo fetch --locked
QRSCANNER_DEVELOPMENT_IDENTITY=YOUR_CERTIFICATE_SELECTOR mise x -- just app
mise x -- just check
```

`app` creates a sandboxed Universal app at `build/QRScanner.app` using the selected existing Apple Development identity.
The identity is its exact public certificate selector, supplied through `QRSCANNER_DEVELOPMENT_IDENTITY`.
Apple's [code identity requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements) explain why a stable development signature preserves privacy authorization across rebuilds.
Use `mise x -- just app --ad-hoc` only for an ephemeral app whose rebuilt signature can require fresh Screen Recording authorization.
It inspects the signature, entitlements, compiled plural rules, privacy manifest, migration resource, and architecture slices.
Open the app and choose Scan Screen from the menu bar or the result window.
The interface uses system text styles and standard window controls.

`check` runs Rust formatting, strict Clippy, Rust tests, dependency policy, the pinned advisory snapshot, and secret detection.
On a Mac it also runs SwiftLint, a strict Universal Debug build, and native XCTest suites on the host architecture.
Checks inspect an isolated ad hoc bundle at `build/check/QRScanner.app` and refuse any change to the interactive app's contents or signature.
The fast gate needs no private signing identity.
The gate compares the String Catalog with compiler-extracted UI text and refuses missing or stale keys.
Run `mise x -- just localize` after changing interface text to update the catalog through Apple's `xcstringstool`.
Linux CI runs the same command without the Mac-only steps.
Owner-only document generation and PR preparation remain separate from contributor checks.
Use `dotfiles repository check` for the generated README and licenses when the owner tool is available.

The native runner compiles production adapters, views, AppDelegate, and tests with Swift 6 strict concurrency and warnings as errors.
Known-answer QR images and a blank image exercise Vision.
The UI tests inspect actual selectable text, standard window controls, button geometry, focus behavior, and light and dark renderings.
Narrow-window tests keep action buttons within a 320-point content area.
Images appear under `build/ui-review/`, and command logs appear under `build/logs/`.

## Full verification

Prepare the pinned Linux verification image with Docker, then run the release gate on a logged-in Mac.

```sh
mise x -- just prepare-verification
mise x -- just verify-release
```

The Dockerfile pins its Rust base image by digest and installs the declared Kani and cargo-mutants releases.
An existing tools image can serve as an optional cache by passing `IMAGE_TAG@sha256:IMAGE_ID` to `prepare-verification`.
The helper checks that tag's exact local image ID before importing tools, keeps the project's pinned Rust compiler, and records the resulting immutable image ID.
The cache changes preparation cost without replacing the project runner or its version checks.
Proof and mutation containers have two CPUs, 2048 MiB of memory, no swap, 256 processes, and a 900-second safety stop.
A checkout-specific Docker volume keeps Linux compiler caches apart from native Mac caches and shared-directory filesystem behavior.
A safety stop is inconclusive and creates no passing receipt.

The release gate runs fast checks, a Universal Release build, and optimized native tests on both architectures.
It runs every declared Kani harness, the rejecting automatic-open control, and the full core and foreign-interface mutation inventory.
An Apple silicon Mac executes the Intel runner through Rosetta.
The Intel runner records execution of Intel code without claiming physical Intel hardware acceptance.
Use `just proofs` or `just mutants` for the same bounded campaigns on demand.
The source must remain unchanged throughout release verification.

Successful automated verification writes `build/evidence/verification.json` with source, executable, resource, command-log, proof, mutation, toolchain, SDK, OS, and container identities.
Fast checks create no release receipt.
Signing additionally requires the login-session acceptance described in [Distribution](distribution.md).
The pinned RustSec revision makes contributor checks reproducible.
The signing path also runs an online advisory audit for current freshness.

`QRSCANNER_RUST_TOOLCHAIN` can select an existing toolchain whose complete `rustc -vV` output matches the project compiler.
The selected compiler must match the pinned version.
