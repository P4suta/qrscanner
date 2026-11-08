---
doppler_automation_app: release_please
---

# Credential delivery

QRScanner uses the existing release-please GitHub App for version proposals.
The `qrscanner` Doppler project's `automation` config references `shared-automation.release_please`.
A purpose-specific config delivers the two selected fields through [native GitHub synchronization](https://docs.doppler.com/docs/github-actions).
The [manifest](../automation/release-app-sync.toml) binds the repository, environment, and field mapping.
The [delivery decision](adr/sync-proposal-app-credentials-to-the-protected-environment.md) records the boundary and alternatives.
RELEASE_APP_CLIENT_ID references GITHUB_APP_CLIENT_ID, and RELEASE_APP_PRIVATE_KEY references GITHUB_APP_PRIVATE_KEY.
The consumer receives only those two fields.

The protected `release-please` GitHub environment permits the main-branch proposal and read-only credential verification workflows.
The environment prevents administrator bypass.
GitHub stores credential copies, and protected main and environment policies replace Doppler workflow and event claims.
The App private key retains authority across every installation of the shared App.
Pass named secrets only to the App token action and scope each token to the current repository.
Run credential verification before enabling `RELEASE_PROPOSALS_ENABLED`.
The check confirms repository identity with a read-only token, which the action revokes during cleanup.
Keep the Doppler identity until replacement authentication succeeds for every consumer.
Rotate canonical values in Doppler and verify synchronization and authentication before retiring the previous key.
The proposal workflow creates no tag or release and receives no macOS signing or notarization credentials.

The expected macOS signing team is `XMBQLW82L9`.
The existing Developer ID Application certificate has public selector `336BAC3D05478E8A4BBAE509917D907A26C18037`.
Keep its private key and the notarization profile in the native Keychain.
Select the existing profile through `QRSCANNER_NOTARY_PROFILE` without putting private values in repository files or command arguments.

[Distribution](distribution.md) defines candidate verification, native acceptance, signing, notarization, and publication.
