---
date: 2026-11-05
status: proposed
---

# Verify production contracts and proof sensitivity

## Status

Proposed.

## Context

Scan safety depends on correct transitions and a matching C interface.
Distribution also depends on the tool that accepts verification evidence.
A safety implication alone can pass when an implementation stops producing the required effect.
A false assertion control checks the solver's rejection path without checking whether production faults violate the intended contracts.

## Decision

Keep Kani as the verifier of executable Rust and add complete transition and wire-refinement contracts.
Keep deterministic evidence decisions in a dependency-free `no_std` crate with the same compiler restrictions as the scan core.
Include every runtime source file in those three crates in mutation testing.
Keep process, JSON, and filesystem adapters in `xtask` and test their actual consumers with altered evidence.

Require production fault controls in disposable copies after the unmodified proofs pass.
Accept a control only when the named contract assertion fails without an unwinding failure.
Keep bounded trace and parser proofs separate from all-input single-step and counter arithmetic proofs.
Add native record-layout checks and Swift memory and thread instrumentation on both architecture slices.
Replay the committed byte corpus and run a fixed, instrumented parser fuzz campaign with an independent string grammar and round-trip assertions.
The [assurance contract](../design/native-interface-assurance.md) records the domains and unsupported external boundaries.

## Consequences

The release gate checks test sensitivity, proof sensitivity, adapter refusal, and native execution through distinct mechanisms.
The gate takes longer than the fast contributor check and remains outside ordinary hosted CI.
Other provers would add value only for obligations that Kani doesn't establish or for independently specified semantics.
A detached theorem would leave the implementation correspondence unchecked.
Real macOS 26 login-session acceptance remains necessary before signed distribution.
