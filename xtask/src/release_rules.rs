#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NotaryState {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum NextStep {
    Wait,
    Staple,
    Refuse,
}

pub(crate) const fn next_step(state: NotaryState, identity_matches: bool) -> NextStep {
    if !identity_matches {
        return NextStep::Refuse;
    }
    match state {
        NotaryState::Pending => NextStep::Wait,
        NotaryState::Accepted => NextStep::Staple,
        NotaryState::Rejected => NextStep::Refuse,
    }
}

#[derive(Clone, Copy)]
pub(crate) struct VerificationFacts {
    pub(crate) identities_match: bool,
    pub(crate) checks_complete: bool,
    pub(crate) commands_match: bool,
    pub(crate) tools_match: bool,
    pub(crate) artifacts_bound: bool,
}

pub(crate) const fn verification_complete(facts: VerificationFacts) -> bool {
    facts.identities_match
        && facts.checks_complete
        && facts.commands_match
        && facts.tools_match
        && facts.artifacts_bound
}

#[cfg(kani)]
#[kani::proof]
fn incomplete_verification_cannot_authorize_signing() {
    let facts = VerificationFacts {
        identities_match: kani::any(),
        checks_complete: kani::any(),
        commands_match: kani::any(),
        tools_match: kani::any(),
        artifacts_bound: kani::any(),
    };
    if verification_complete(facts) {
        assert!(facts.identities_match);
        assert!(facts.checks_complete);
        assert!(facts.commands_match);
        assert!(facts.tools_match);
        assert!(facts.artifacts_bound);
    }
    kani::cover!(
        verification_complete(facts),
        "complete verification is reachable"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_requires_every_fact() {
        for mask in 0_u8..32 {
            let facts = VerificationFacts {
                identities_match: mask & 1 != 0,
                checks_complete: mask & 2 != 0,
                commands_match: mask & 4 != 0,
                tools_match: mask & 8 != 0,
                artifacts_bound: mask & 16 != 0,
            };
            assert_eq!(verification_complete(facts), mask == 31);
        }
    }

    #[test]
    fn notarization_requires_acceptance_and_identity() {
        for state in [
            NotaryState::Pending,
            NotaryState::Accepted,
            NotaryState::Rejected,
        ] {
            assert_eq!(next_step(state, false), NextStep::Refuse);
        }
        assert_eq!(next_step(NotaryState::Pending, true), NextStep::Wait);
        assert_eq!(next_step(NotaryState::Accepted, true), NextStep::Staple);
        assert_eq!(next_step(NotaryState::Rejected, true), NextStep::Refuse);
    }
}

#[cfg(kani)]
#[kani::proof]
fn release_receipts_must_match_and_be_accepted() {
    let state = match kani::any::<u8>() {
        0 => NotaryState::Pending,
        1 => NotaryState::Accepted,
        _ => NotaryState::Rejected,
    };
    let matches: bool = kani::any();
    assert_eq!(
        next_step(state, matches) == NextStep::Staple,
        state == NotaryState::Accepted && matches
    );
    kani::cover!(
        next_step(state, matches) == NextStep::Staple,
        "finalization is reachable"
    );
}
