---
doppler_automation_app: release_please
---

# Credential delivery

QRScanner uses the existing release-please GitHub App for version proposals.
The `qrscanner` Doppler project's `automation` config references `shared-automation.release_please`.
`GITHUB_APP_CLIENT_ID` identifies that App, and `GITHUB_APP_PRIVATE_KEY` authenticates its short-lived repository tokens.
The consumer receives only those two fields.

The protected `release-please` GitHub environment permits the main-branch proposal and read-only credential verification workflows.
Run credential verification before enabling `RELEASE_PROPOSALS_ENABLED`.
The proposal workflow creates no tag or release and receives no macOS signing or notarization credentials.

The expected macOS signing team is `XMBQLW82L9`.
The existing Developer ID Application certificate has public selector `336BAC3D05478E8A4BBAE509917D907A26C18037`.
Keep its private key and the notarization profile in the native Keychain.
Select the existing profile through `QRSCANNER_NOTARY_PROFILE` without putting private values in repository files or command arguments.

[Distribution](distribution.md) defines candidate verification, native acceptance, signing, notarization, and publication.
