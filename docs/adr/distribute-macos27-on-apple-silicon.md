---
date: 2026-11-05
status: accepted
---

# Distribute macOS 27 on Apple silicon

## Status

Accepted.
Partly supersedes [Require macOS 26 and sandboxed capture](require-macos26-and-sandboxed-capture.md) for the deployment target and supported architecture.
Partly supersedes [Verify production contracts and proof sensitivity](verify-production-contracts-and-proof-sensitivity.md) for native target coverage.

## Context

The available acceptance platform runs macOS 27 on Apple silicon.
Native execution on that platform doesn't establish macOS 26 or physical Intel compatibility.
The public support contract must match the platform that receives native acceptance.

## Decision

Target macOS 27.0 and ship only arm64.
Require a macOS 27 arm64 login-session acceptance record bound to the verified source and executable before signing.
Keep App Sandbox, native window controls, and the existing capture contract.
Keep full Rust proofs, fault controls, mutation testing, parser fuzzing, and native sanitizer suites for the supported architecture.

Use [release-please](https://github.com/googleapis/release-please-action) to prepare version and CHANGELOG PRs from Conventional Commits.
Use a repository-scoped GitHub App token in a separate proposal environment.
The proposal workflow creates no tag or GitHub Release.
Keep signing, notarization, and publication separate from automatic proposals.

## Consequences

macOS 26 and Intel users need another supported build.
The smaller support contract permits native acceptance on the available machine without claiming unmeasured compatibility.
Reintroducing an OS version or architecture requires its native acceptance and a new support decision.
Rust workspace versions, the lockfile, and app metadata must change together in release PRs.
