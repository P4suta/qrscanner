---
date: 2026-11-09
status: proposed
---

# Sync proposal App credentials to the protected environment

## Context

Version proposals require stored GitHub App credentials.
Doppler's [identity limit](https://docs.doppler.com/docs/platform-limits) constrains fleet growth.
The release-please environment admits main and prevents administrator bypass.

## Decision

Use [native GitHub synchronization](https://docs.doppler.com/docs/github-actions) from a purpose-specific Doppler config into the existing environment.
Pass named secrets only to repository-scoped App token actions.
Preserve the proposal enablement variable and the prohibition on automatic tags and releases.
Keep native signing and publication separate.

## Alternatives

Doppler OIDC avoids GitHub copies and preserves workflow and event claims, but consumes identity capacity.
A service token adds another stored credential.

## Consequences

GitHub stores credential copies.
Protected main and environment policies replace Doppler claims.
The shared App key retains authority across its installations.
Reconsider when GitHub supports workload identity for the App.

## Confirmation

Verify synchronization and manual authentication from reviewed main before identity retirement.
Confirm repository identity and automatic token revocation.
Migration creates no release or tag.
