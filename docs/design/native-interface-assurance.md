# Native interface assurance

## Contract and production boundaries

The [interface design](macos-interface.md) keeps screenshots and results in memory.
Capture covers every reported display, excludes QRScanner, uses the filter's pixel scale, requests standard dynamic range, and leaves the screenshot file destination unset.
The result panel stays hidden until capture completes.
Cancellation and obsolete operation generations reject late content and failures.
Revocation before result publication discards captured results and offers permission recovery.

The Rust core admits every generation, domain phase, and event.
The foreign interface validates wire tags and Boolean fields before constructing domain events.
The value-only `C ABI` preserves its existing layout and transition behavior.
Kani imports production functions to check complete single-step outcomes, confirmed opening, stale events, cancellation, generation overflow, wire validation, icon lengths, notarization identity, and the release gate's required facts.
The wire refinement uses a numeric specification independent of the production encoder and decoder.
Native tests check the header constants, record sizes, alignments, field offsets, and transition matrix on arm64.

Single-step scan and wire proofs quantify every admitted phase, event, 64-bit generation, and raw 32-bit wire field.
A bounded proof explores eight events with current and previous generations and checks monotonic generations and confirmed opening at every step.
The OS-version parser proof compares production parsing with an independent numeric grammar for inputs up to 32 bytes.
Successful unwinding assertions establish those finite bounds rather than an unbounded trace or string theorem.
The parser returns a validated view of the original bytes, and the bounded proof checks byte preservation and parsing round trips.
[Kani's harness and unwinding documentation](https://model-checking.github.io/kani/reference/attributes.html) defines the verification boundary.

The dependency-free `no_std` verification crate contains release authorization, native acceptance, counterexample acceptance, and mutation-count decisions.
Its proofs cover every Boolean fact combination and every 64-bit mutation count, including arithmetic overflow.
The filesystem adapter rehashes each required command log and compares its saved command record when issuing or consuming a receipt.
Adapter tests reject changed or missing logs and command records, stale identities, and malformed native acceptance metadata.
The predicate proofs don't establish the empirical truth of supplied facts, filesystem atomicity, or Apple's acceptance results.

Swift adapters own permission queries, capture, Vision, window presentation, preferences, browser opening, clipboard writes, and relaunch.
These boundaries depend on Swift actors and Apple's framework object graphs, which Kani doesn't execute.
Strict Swift concurrency checking, injected failures, real framework fixtures, and native interaction tests provide a scoped alternative.
They aren't an all-input proof of Swift or Apple framework behavior.
Reassess the alternative when a maintained verifier can execute those boundaries or when a decision moves into the Rust core.

ScreenCaptureKit must return the requested image on a successful call, CoreGraphics must report current access, and system browser, clipboard, and launch APIs must report their outcomes accurately.
Known-answer Vision images and a blank control test recognition without establishing accuracy on arbitrary screens.
One retry on a Vision internal error uses a supported CPU device and preserves both causes if recovery fails.

Container migration imports only Boolean welcome and permission-request flags.
Existing container preferences take precedence, and repeated import changes nothing.
Native tests exercise the importer and relaunch ordering without modifying a person's real container or launching another app instance.
The signed app's migration and sandbox behavior still need live acceptance.

## Verification and evidence

`mise x -- just check` runs the fast contributor gate.
`mise x -- just verify-release` runs optimized arm64 native suites, AddressSanitizer and ThreadSanitizer suites, every production Kani harness and its rejecting controls, and the complete scan, foreign-interface, and verification-policy mutation inventory.
Missing, empty, failed, timed-out, and incomplete required work creates no completed release receipt.
The runner records source, tools, target semantics, logs, executable, and resource identities.
Tests reject missing checks, duplicate checks, changed sources, changed binaries, and incorrect acceptance records.

Seven production fault controls change disposable copies of the actual implementation.
They suppress capture, wrap generations, open without confirmation, accept stale completion, change an event tag, drop artifact binding, and accept a mutation timeout.
Each control must fail its named assertion after the unmodified harness inventory passes.
A compiler error, an unrelated failure, or an unwinding failure supplies no accepted counterexample.
The false automatic-open assertion remains a separate solver control.

Mutation testing checks tests' sensitivity to the selected tool's generated operators.
It supplies no proof that every defect has an operator.
The gate compares every outcome with the complete inventory and requires work in every selected production file.
[cargo-mutants documents its report inventory and outcome categories](https://mutants.rs/mutants-out.html).

The fast gate replays every committed parser corpus case through an independent string grammar and byte-preserving round-trip assertions.
The corpus stores byte arrays as JSON so valid inputs need no trailing newline and invalid UTF-8 remains representable.
The release gate decodes those arrays into a disposable corpus and runs 4096 libFuzzer inputs with seed 1 and an input limit of 256 bytes.
The pinned Linux runner instruments the parser and oracle with AddressSanitizer.
The fixed campaign adds sampled execution beyond the parser proof's 32-byte bound without establishing a theorem for longer inputs.
[The Rust Fuzz Book](https://rust-fuzz.github.io/book/cargo-fuzz/tutorial.html) describes target execution, and [the libFuzzer reference documents the run count and input bounds](https://llvm.org/docs/LibFuzzer.html#options).

The native runner prohibits app activation and keeps fixture windows offscreen.
The sanitizer variants instrument owned Swift production and test code and execute the same nonempty suites on arm64.
The linked Rust archive, Apple frameworks, and system services remain outside that instrumentation.
Leak detection isn't part of the AddressSanitizer contract.
[Swift's sanitizer guidance](https://www.swift.org/documentation/server/guides/llvm-sanitizers.html) describes the compiler instrumentation.
Passing suites detect no memory or thread error in the exercised executions.
They don't prove freedom from every race or invalid access.

Native tests preserve empty values, Unicode, long links, and result selection after action failure.
They cover permission denial, cancellation, revocation, recovery, stale completion, opening System Settings, and relaunch failures.
Production views render in both appearances and in narrow windows using standard text styles and native control metrics.
Two-result fixtures verify that every native row fits inside the list viewport.
Generated PNGs remain under `build/ui-review/` for visual inspection.

Screen Recording consent, display fidelity, protected content, app exclusion, and full-screen Spaces require a normal login session.
The same applies to browser launching, clipboard access, sandbox relaunch, VoiceOver, and system accessibility preferences.
The release acceptance record requires macOS 27 arm64 observations.
Evidence from a newer OS remains evidence for that OS.
No release or notarization follows from a passing development test.
