use super::*;

fn arbitrary_phase() -> Phase {
    match kani::any::<u8>() {
        0 => Phase::Idle,
        1 => Phase::Scanning,
        2 => Phase::Results,
        3 => Phase::Opening,
        4 => Phase::Empty,
        _ => Phase::Failed,
    }
}

fn arbitrary_state() -> ScanState {
    ScanState {
        id: ScanId::from_raw(kani::any()),
        phase: arbitrary_phase(),
    }
}

fn arbitrary_event() -> Event {
    let id = ScanId::from_raw(kani::any());
    match kani::any::<u8>() {
        0 => Event::Begin,
        1 => Event::Cancel,
        2 => Event::Completed {
            id,
            found: kani::any(),
        },
        3 => Event::Failed { id },
        4 => Event::Open {
            id,
            kind: if kani::any() {
                PayloadKind::WebUrl
            } else {
                PayloadKind::CopyOnly
            },
        },
        5 => Event::Copy { id },
        _ => Event::OpenFinished {
            id,
            succeeded: kani::any(),
        },
    }
}

#[kani::proof]
fn opening_requires_current_confirmed_web_result() {
    let state = arbitrary_state();
    let event = arbitrary_event();
    let result = transition(state, event);
    if result.effect == Effect::OpenSelected {
        assert_eq!(state.phase, Phase::Results);
        assert_eq!(
            event,
            Event::Open {
                id: state.id,
                kind: PayloadKind::WebUrl
            }
        );
        assert_eq!(result.state.phase, Phase::Opening);
    }
    kani::cover!(
        result.effect == Effect::OpenSelected,
        "opening is reachable"
    );
}

#[kani::proof]
fn cancellation_rejects_every_late_completion() {
    let state = arbitrary_state();
    let cancelled = transition(state, Event::Cancel).state;
    let result = transition(
        cancelled,
        Event::Completed {
            id: ScanId::from_raw(kani::any()),
            found: kani::any(),
        },
    );
    assert_eq!(result.state, cancelled);
    assert_eq!(result.effect, Effect::None);
}

#[kani::proof]
fn stale_events_have_no_effect() {
    let state = arbitrary_state();
    let id = ScanId::from_raw(kani::any());
    kani::assume(id != state.id);
    let event = match kani::any::<u8>() {
        0 => Event::Completed {
            id,
            found: kani::any(),
        },
        1 => Event::Failed { id },
        2 => Event::Open {
            id,
            kind: PayloadKind::WebUrl,
        },
        3 => Event::Copy { id },
        _ => Event::OpenFinished {
            id,
            succeeded: kani::any(),
        },
    };
    let result = transition(state, event);
    assert_eq!(result.state, state);
    assert_eq!(result.effect, Effect::None);
}

#[kani::proof]
fn generation_is_monotonic_and_never_wraps() {
    let state = arbitrary_state();
    let result = transition(state, Event::Begin);
    assert!(result.state.id.raw() >= state.id.raw());
    if result.effect == Effect::Capture {
        assert!(result.state.id.raw() > state.id.raw());
        assert_eq!(result.state.phase, Phase::Scanning);
    }
    if state.id.raw() == u64::MAX {
        assert_eq!(result.state.id, state.id);
        assert_ne!(result.effect, Effect::Capture);
    }
}

#[kani::proof]
fn busy_scans_cannot_capture_again() {
    let id = ScanId::from_raw(kani::any());
    let state = ScanState {
        id,
        phase: if kani::any() {
            Phase::Scanning
        } else {
            Phase::Opening
        },
    };
    let result = transition(state, Event::Begin);
    assert_eq!(result.state, state);
    assert_eq!(result.effect, Effect::None);
}

#[kani::proof]
fn asynchronous_effects_require_the_current_expected_phase() {
    let state = arbitrary_state();
    let event = arbitrary_event();
    let result = transition(state, event);
    match result.effect {
        Effect::PresentResults | Effect::PresentEmpty => {
            assert_eq!(state.phase, Phase::Scanning);
            assert!(matches!(event, Event::Completed { id, .. } if id == state.id));
        }
        Effect::PresentFailure => {
            assert_eq!(state.phase, Phase::Scanning);
            assert_eq!(event, Event::Failed { id: state.id });
        }
        Effect::CopySelected => {
            assert_eq!(state.phase, Phase::Results);
            assert_eq!(event, Event::Copy { id: state.id });
        }
        Effect::PresentOpenFailure => {
            assert_eq!(state.phase, Phase::Opening);
            assert_eq!(
                event,
                Event::OpenFinished {
                    id: state.id,
                    succeeded: false
                }
            );
        }
        Effect::Close => {
            if event != Event::Cancel {
                assert_eq!(state.phase, Phase::Opening);
                assert_eq!(
                    event,
                    Event::OpenFinished {
                        id: state.id,
                        succeeded: true
                    }
                );
            }
        }
        Effect::None | Effect::Capture | Effect::OpenSelected | Effect::PresentExhausted => {}
    }
}

#[cfg(feature = "negative-probe")]
#[kani::proof]
fn reject_automatic_open_probe() {
    let state = ScanState {
        id: ScanId::from_raw(1),
        phase: Phase::Scanning,
    };
    let result = transition(
        state,
        Event::Completed {
            id: state.id,
            found: true,
        },
    );
    kani::assert(
        result.effect == Effect::OpenSelected,
        "NEGATIVE_CONTROL_AUTOMATIC_OPEN",
    );
}
