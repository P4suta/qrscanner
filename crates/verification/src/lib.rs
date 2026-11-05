#![no_std]
#![deny(unsafe_code)]
#![deny(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotaryState {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NextStep {
    Wait,
    Staple,
    Refuse,
}

pub const fn next_step(state: NotaryState, identity_matches: bool) -> NextStep {
    if !identity_matches {
        return NextStep::Refuse;
    }
    match state {
        NotaryState::Pending => NextStep::Wait,
        NotaryState::Accepted => NextStep::Staple,
        NotaryState::Rejected => NextStep::Refuse,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VerificationFacts {
    pub identities_match: bool,
    pub checks_complete: bool,
    pub commands_match: bool,
    pub tools_match: bool,
    pub artifacts_bound: bool,
}

pub const fn verification_complete(facts: VerificationFacts) -> bool {
    facts.identities_match
        && facts.checks_complete
        && facts.commands_match
        && facts.tools_match
        && facts.artifacts_bound
}

#[derive(Clone, Copy, Debug)]
pub struct MutationCounts {
    pub total: u64,
    pub caught: u64,
    pub unviable: u64,
    pub missed: u64,
    pub timed_out: u64,
    pub unexpected_success: u64,
    pub outcomes: u64,
    pub baselines: u64,
    pub caught_outcomes: u64,
    pub unviable_outcomes: u64,
}

pub fn mutations_complete(counts: MutationCounts) -> bool {
    counts.caught > 0
        && counts.caught.checked_add(counts.unviable) == Some(counts.total)
        && counts.missed == 0
        && counts.timed_out == 0
        && counts.unexpected_success == 0
        && counts.total.checked_add(1) == Some(counts.outcomes)
        && counts.baselines == 1
        && counts.caught_outcomes == counts.caught
        && counts.unviable_outcomes == counts.unviable
}

#[derive(Clone, Copy, Debug)]
pub struct AcceptanceFacts {
    pub identities_match: bool,
    pub arm64_session: bool,
}

pub const fn acceptance_complete(facts: AcceptanceFacts) -> bool {
    facts.identities_match && facts.arm64_session
}

#[derive(Clone, Copy, Debug)]
pub struct CounterexampleFacts {
    pub process_failed: bool,
    pub named_assertion_failed: bool,
    pub verification_failed: bool,
    pub unwinding_failed: bool,
}

pub const fn counterexample_rejected(facts: CounterexampleFacts) -> bool {
    facts.process_failed
        && facts.named_assertion_failed
        && facts.verification_failed
        && !facts.unwinding_failed
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MacOS27<'a>(&'a [u8]);

impl<'a> MacOS27<'a> {
    pub fn parse(version: &'a [u8]) -> Option<Self> {
        let [b'2', b'7', b'.', tail @ ..] = version else {
            return None;
        };
        let mut has_digit = false;
        let mut has_patch = false;
        for byte in tail {
            match byte {
                b'0'..=b'9' => has_digit = true,
                b'.' if has_digit && !has_patch => {
                    has_patch = true;
                    has_digit = false;
                }
                _ => return None,
            }
        }
        if has_digit { Some(Self(version)) } else { None }
    }

    pub const fn as_bytes(self) -> &'a [u8] {
        self.0
    }
}

#[cfg(kani)]
mod proofs;
