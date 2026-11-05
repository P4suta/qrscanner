# Distribution

QRScanner uses Developer ID distribution with App Sandbox and a macOS 26 deployment target.
Preparing a development app or running verification grants no authority to sign, notarize, or publish a product release.
The current implementation work prepares source and development artifacts only.

Run the complete automated gate before an authorized signing operation.

```sh
mise x -- just prepare-verification
mise x -- just verify-release
```

The release receipt binds the exact source, executable, bundle resources, compiler, SDK, OS, tool image, command logs, proof report, and mutation report.
Missing checks, duplicate checks, stale command sources, changed resources, and changed verification artifacts fail validation.
Changing the source or Release executable invalidates prior evidence.
Verification never signs or publishes as a side effect.
Mutation testing covers every runtime source file in the Rust core and C foreign-interface crates.
The complete Kani inventory and negative control check proof harness modules.
Unit-test mutation coverage excludes code compiled only for Kani.
The gate requires a successful baseline, a complete matching mutation inventory, caught executable mutants, and no survivors or timeouts.
The report identifies unbuildable mutants and caught mutants as distinct outcomes.

## Native acceptance

Signed distribution also requires `build/evidence/macos26-acceptance.json` from normal login sessions on macOS 26.
The record contains `source_sha256`, `binary_sha256`, and `sessions`.
Each session identifies `os_version`, `os_build`, `architecture`, `hardware`, and a `checks` object with Boolean results.
Both `arm64` and `x86_64` need a macOS 26 session with every required interaction passing.
The required check names live in `xtask/src/acceptance.rs`.

Exercise live screen capture, exclusion of QRScanner, permission denial, revocation and recovery, browser opening, clipboard copying, relaunch, shortcut conflict, and connected displays.
Exercise VoiceOver, keyboard focus and actions, 200% text, Increase Contrast, Reduce Transparency, and both appearances.
Keep screen images and decoded content out of diagnostic logs.
A newer macOS observation, a cross-compile, or a Rust proof supplies no macOS 26 acceptance.
The acceptance file records operator observations rather than proving their empirical truth.
Tests exercise the parser's refusal of missing, failing, stale, or wrong-OS records.

## Authorized signing

The signing command requires `QRSCANNER_SIGNING_IDENTITY`, `QRSCANNER_TEAM_ID`, and `QRSCANNER_NOTARY_PROFILE`.
The identity is the exact 40-character selector of an existing Developer ID Application certificate with its private key available to the signing service.
The team identifies the intended publisher, and the profile names an existing `notarytool` Keychain profile.
Keep credentials outside the repository and command arguments.

```sh
mise x -- just package
```

The command checks automated and native acceptance before accessing signing configuration.
It embeds the sandbox entitlement, signs with hardened runtime and a trusted timestamp, inspects the publisher and architecture slices, and preserves the signed notarization input.
A pending submission remains unfinished.
The same command resumes the preserved submission instead of rebuilding or replacing its archive.
Rejected or mismatched receipts stop processing.
After acceptance, packaging checks ticket code hashes, staples the app, creates and signs the disk image, and verifies its notarization, tickets, and Gatekeeper assessments.
Candidate directories hold checksums, a software inventory, the lockfile, and source-bound evidence.
Publishing remains a separate owner-authorized operation.

## Development artifact

```sh
mise x -- just package --development
```

Development packaging creates an ad hoc signed app and archives without release credentials.
The manifest records that the artifact has no notarized distribution status.
If disk-image creation fails, the command reports failure and preserves the development ZIP and its evidence.
