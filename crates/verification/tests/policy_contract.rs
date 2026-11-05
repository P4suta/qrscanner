use qrscanner_verification::{
    AcceptanceFacts, MacOS27, MutationCounts, NextStep, NotaryState, VerificationFacts,
    acceptance_complete, mutations_complete, next_step, verification_complete,
};

#[test]
fn a_negative_control_requires_a_named_assertion_failure() {
    use qrscanner_verification::{CounterexampleFacts, counterexample_rejected};
    for mask in 0_u8..16 {
        assert_eq!(
            counterexample_rejected(CounterexampleFacts {
                process_failed: mask & 1 != 0,
                named_assertion_failed: mask & 2 != 0,
                verification_failed: mask & 4 != 0,
                unwinding_failed: mask & 8 != 0,
            }),
            mask == 7
        );
    }
}

#[test]
fn verification_and_acceptance_require_every_fact() {
    for mask in 0_u8..32 {
        assert_eq!(
            verification_complete(VerificationFacts {
                identities_match: mask & 1 != 0,
                checks_complete: mask & 2 != 0,
                commands_match: mask & 4 != 0,
                tools_match: mask & 8 != 0,
                artifacts_bound: mask & 16 != 0,
            }),
            mask == 31
        );
    }
    for mask in 0_u8..4 {
        assert_eq!(
            acceptance_complete(AcceptanceFacts {
                identities_match: mask & 1 != 0,
                arm64_session: mask & 2 != 0,
            }),
            mask == 3
        );
    }
}

#[test]
fn notarization_requires_acceptance_and_identity() {
    for (state, expected) in [
        (NotaryState::Pending, NextStep::Wait),
        (NotaryState::Accepted, NextStep::Staple),
        (NotaryState::Rejected, NextStep::Refuse),
    ] {
        assert_eq!(next_step(state, true), expected);
        assert_eq!(next_step(state, false), NextStep::Refuse);
    }
}

#[test]
fn mutation_counts_require_complete_nonempty_outcomes_without_overflow() {
    let complete = MutationCounts {
        total: 2,
        caught: 1,
        unviable: 1,
        missed: 0,
        timed_out: 0,
        unexpected_success: 0,
        outcomes: 3,
        baselines: 1,
        caught_outcomes: 1,
        unviable_outcomes: 1,
    };
    assert!(mutations_complete(complete));
    assert!(mutations_complete(MutationCounts {
        total: 1,
        unviable: 0,
        unviable_outcomes: 0,
        outcomes: 2,
        ..complete
    }));
    for invalid in [
        MutationCounts {
            total: 0,
            ..complete
        },
        MutationCounts {
            caught: 0,
            unviable: 2,
            caught_outcomes: 0,
            unviable_outcomes: 2,
            ..complete
        },
        MutationCounts {
            total: 3,
            outcomes: 4,
            ..complete
        },
        MutationCounts {
            missed: 1,
            ..complete
        },
        MutationCounts {
            timed_out: 1,
            ..complete
        },
        MutationCounts {
            unexpected_success: 1,
            ..complete
        },
        MutationCounts {
            outcomes: 2,
            ..complete
        },
        MutationCounts {
            baselines: 0,
            ..complete
        },
        MutationCounts {
            baselines: 2,
            ..complete
        },
        MutationCounts {
            caught_outcomes: 2,
            ..complete
        },
        MutationCounts {
            unviable_outcomes: 2,
            ..complete
        },
        MutationCounts {
            total: u64::MAX,
            caught: u64::MAX,
            unviable: 1,
            outcomes: 0,
            ..complete
        },
        MutationCounts {
            total: u64::MAX,
            caught: u64::MAX,
            unviable: 0,
            outcomes: 0,
            ..complete
        },
    ] {
        assert!(!mutations_complete(invalid), "accepted {invalid:?}");
    }
}

#[test]
fn os_versions_require_numeric_minor_and_optional_patch_components() {
    for version in ["27.0", "27.1", "27.10.2", "27.1234567890.1234567890"] {
        assert!(MacOS27::parse(version.as_bytes()).is_some());
        if let Some(parsed) = MacOS27::parse(version.as_bytes()) {
            assert_eq!(parsed.as_bytes(), version.as_bytes());
            assert_eq!(MacOS27::parse(parsed.as_bytes()), Some(parsed));
        }
    }
    for version in [
        "", "27", "27.", "27.1.", "27..1", "27.1.2.3", "27.1a", "27.0\n", "27.０", "26.0", "127.0",
        "027.0",
    ] {
        assert!(
            MacOS27::parse(version.as_bytes()).is_none(),
            "accepted {version:?}"
        );
    }
    for byte in 0..=u8::MAX {
        assert_eq!(
            MacOS27::parse(&[b'2', b'7', b'.', byte]).is_some(),
            byte.is_ascii_digit()
        );
        assert_eq!(
            MacOS27::parse(&[b'2', b'7', b'.', b'0', b'.', byte]).is_some(),
            byte.is_ascii_digit()
        );
    }
}

#[test]
fn distribution_version_requires_macos27() {
    assert!(MacOS27::parse(b"27.0.1").is_some());
    assert!(MacOS27::parse(b"26.0").is_none());
    assert!(MacOS27::parse(b"28.0").is_none());
}
