use super::*;

#[kani::proof]
fn negative_controls_require_the_expected_assertion() {
    let facts = CounterexampleFacts {
        process_failed: kani::any(),
        named_assertion_failed: kani::any(),
        verification_failed: kani::any(),
        unwinding_failed: kani::any(),
    };
    assert_eq!(
        counterexample_rejected(facts),
        facts.process_failed
            && facts.named_assertion_failed
            && facts.verification_failed
            && !facts.unwinding_failed
    );
    kani::cover!(
        counterexample_rejected(facts),
        "named counterexample is reachable"
    );
}

#[kani::proof]
fn incomplete_verification_cannot_authorize_signing() {
    let facts = VerificationFacts {
        identities_match: kani::any(),
        checks_complete: kani::any(),
        commands_match: kani::any(),
        tools_match: kani::any(),
        artifacts_bound: kani::any(),
    };
    let expected = facts.identities_match
        && facts.checks_complete
        && facts.commands_match
        && facts.tools_match
        && facts.artifacts_bound;
    kani::assert(
        verification_complete(facts) == expected,
        "VERIFICATION_GATE",
    );
    kani::cover!(
        verification_complete(facts),
        "complete verification is reachable"
    );
}

#[kani::proof]
fn release_receipts_must_match_and_be_accepted() {
    let state = match kani::any::<u8>() {
        0 => NotaryState::Pending,
        1 => NotaryState::Accepted,
        _ => NotaryState::Rejected,
    };
    let matches: bool = kani::any();
    let expected = match (state, matches) {
        (NotaryState::Pending, true) => NextStep::Wait,
        (NotaryState::Accepted, true) => NextStep::Staple,
        (NotaryState::Rejected, true) | (_, false) => NextStep::Refuse,
    };
    assert_eq!(next_step(state, matches), expected);
    kani::cover!(
        next_step(state, matches) == NextStep::Staple,
        "finalization is reachable"
    );
}

#[kani::proof]
fn mutation_counts_are_complete_without_arithmetic_overflow() {
    let counts = MutationCounts {
        total: kani::any(),
        caught: kani::any(),
        unviable: kani::any(),
        missed: kani::any(),
        timed_out: kani::any(),
        unexpected_success: kani::any(),
        outcomes: kani::any(),
        baselines: kani::any(),
        caught_outcomes: kani::any(),
        unviable_outcomes: kani::any(),
    };
    let expected = counts.total != 0
        && counts.caught != 0
        && u128::from(counts.caught) + u128::from(counts.unviable) == u128::from(counts.total)
        && u128::from(counts.total) + 1 == u128::from(counts.outcomes)
        && counts.missed == 0
        && counts.timed_out == 0
        && counts.unexpected_success == 0
        && counts.baselines == 1
        && counts.caught_outcomes == counts.caught
        && counts.unviable_outcomes == counts.unviable;
    kani::assert(
        mutations_complete(counts) == expected,
        "MUTATION_COMPLETENESS",
    );
    kani::cover!(
        mutations_complete(counts) && counts.unviable > 0,
        "complete mixed campaign is reachable"
    );
}

#[kani::proof]
fn acceptance_requires_identity_and_arm64_session() {
    let facts = AcceptanceFacts {
        identities_match: kani::any(),
        arm64_session: kani::any(),
    };
    assert_eq!(
        acceptance_complete(facts),
        facts.identities_match && facts.arm64_session
    );
    kani::cover!(
        acceptance_complete(facts),
        "complete native acceptance is reachable"
    );
}

#[kani::proof]
#[kani::unwind(33)]
fn macos_version_parser_matches_numeric_grammar_up_to_32_bytes() {
    let bytes: [u8; 32] = kani::any();
    let length: usize = kani::any();
    kani::assume(length <= bytes.len());
    let Some(version) = bytes.get(..length) else {
        return;
    };
    let component = |value: &[u8]| !value.is_empty() && value.iter().all(u8::is_ascii_digit);
    let numeric =
        version
            .get(3..)
            .is_some_and(|tail| match tail.iter().position(|byte| *byte == b'.') {
                Some(index) => {
                    tail.get(..index).is_some_and(component)
                        && tail.get(index + 1..).is_some_and(component)
                }
                None => component(tail),
            });
    let specified = version.first() == Some(&b'2')
        && version.get(1) == Some(&b'7')
        && version.get(2) == Some(&b'.')
        && numeric;
    assert_eq!(MacOS27::parse(version).is_some(), specified);
    if let Some(parsed) = MacOS27::parse(version) {
        assert_eq!(parsed.as_bytes(), version);
        assert_eq!(MacOS27::parse(parsed.as_bytes()), Some(parsed));
    }
    kani::cover!(
        specified && length == 6,
        "minor and patch grammar is reachable"
    );
}
