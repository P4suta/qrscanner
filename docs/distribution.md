# Distribution

QRScanner uses Developer ID distribution with App Sandbox and a macOS 27 deployment target on Apple silicon.
Preparing a development app or running verification grants no authority to sign, notarize, or publish a product release.

Run the complete automated gate before an authorized signing operation.

```sh
mise x -- just prepare-verification
mise x -- just verify-release
```

The release receipt binds the exact source, executable, bundle resources, compiler, SDK, OS, tool image, command logs, proof report, mutation report, and fuzz report.
Missing checks, duplicate checks, stale command sources, changed or missing logs, altered command records, changed resources, and changed verification artifacts fail validation.
Changing the source or Release executable invalidates prior evidence.
Verification never signs or publishes as a side effect.
Mutation testing covers every runtime source file in the Rust scan, C foreign-interface, and verification-policy crates.
The complete Kani inventory and production fault controls check proof harness sensitivity.
Unit-test mutation coverage excludes code compiled only for Kani.
The gate requires a successful baseline, a complete matching mutation inventory, caught executable mutants, and no survivors or timeouts.
The report identifies unbuildable mutants and caught mutants as distinct outcomes.
The fixed parser fuzz campaign must complete its declared run count after corpus replay.
The fuzz runtime's `NCSA` license allowance applies only to the pinned `libfuzzer-sys` tool dependency.
The app doesn't link the fuzz runtime.

## Native acceptance

Signed distribution also requires `build/evidence/macos27-acceptance.json` from a normal login session on macOS 27 arm64.
The record contains `source_sha256`, `binary_sha256`, and `sessions`.
Each session identifies `os_version`, `os_build`, `architecture`, `hardware`, and a `checks` object with Boolean results.
An `arm64` session must pass every required interaction.
The required check names live in `xtask/src/acceptance.rs`.

Exercise live screen capture, exclusion of QRScanner, permission denial, revocation and recovery, browser opening, clipboard copying, relaunch, and connected displays.
Exercise VoiceOver, keyboard focus and actions, system text preferences, Increase Contrast, Reduce Transparency, and both appearances.
Keep screen images and decoded content out of diagnostic logs.
A different OS, a cross-compile, or a Rust proof supplies no macOS 27 acceptance.
The acceptance file records operator observations rather than proving their empirical truth.
Tests exercise the parser's refusal of missing, failing, stale, or wrong-OS records.
OS versions need numeric minor and optional patch components after `27.`.
Blank build and hardware fields fail validation.

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
On the supported macOS 27 host, packaging also runs `syspolicy_check distribution` for the app, following [Apple's native distribution test guidance](https://developer.apple.com/forums/thread/130560).
Candidate directories hold checksums, a software inventory, the lockfile, and source-bound evidence.
Publishing remains a separate owner-authorized operation.

## Version proposals

The release-please workflow maintains a version and changelog PR from Conventional Commits on `main`.
It uses the existing release-please GitHub App through the `qrscanner` Doppler project's `automation` config.
The `release-please` GitHub environment permits only `main` and has no reviewer gate for automatic proposals.
Its repository variable `DOPPLER_AUTOMATION_IDENTITY_ID` identifies the matching read-only Doppler identity.
Run the read-only `Verify release credentials` workflow before setting `RELEASE_PROPOSALS_ENABLED` to `true`.
The proposal workflow never creates tags or GitHub Releases.
Merging a proposal doesn't sign or publish an artifact.

The `simple` strategy updates `version.txt` and `CHANGELOG.md`.
Typed extra-file updaters synchronize the Rust workspace version, owned lockfile packages, and app version metadata.
Extra-file annotations update the exact private dependency constraints.
The initial manifest records `0.0.0` as bootstrap state before the first public release.
The first feature release proposes `0.1.0`.

After merging a release proposal, verify that exact protected source, complete native acceptance, and run the authorized package command.
Create a draft GitHub Release for the same version and source commit.
Upload the final disk image, checksums, software inventory, and bound verification records before publication.
Publish only after native signature, notarization, Gatekeeper, and installation checks pass.
Keep existing published artifacts and tags immutable.

## Development artifact

```sh
mise x -- just package --development
```

Development packaging creates an ad hoc signed app and archives without release credentials.
The package command keeps the interactive Apple Development app unchanged.
The manifest records that the artifact has no notarized distribution status.
If disk-image creation fails, the command reports failure and preserves the development ZIP and its evidence.
