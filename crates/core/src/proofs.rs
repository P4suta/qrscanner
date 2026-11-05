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
    kani::cover!(id != state.id, "stale generations are admitted");
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

#[kani::proof]
fn begin_is_total_and_makes_progress() {
    let state = arbitrary_state();
    let result = transition(state, Event::Begin);
    let expected = if matches!(state.phase, Phase::Scanning | Phase::Opening) {
        Transition {
            state,
            effect: Effect::None,
        }
    } else if state.id.raw() == u64::MAX {
        Transition {
            state,
            effect: Effect::PresentExhausted,
        }
    } else {
        Transition {
            state: ScanState {
                id: ScanId::from_raw(state.id.raw() + 1),
                phase: Phase::Scanning,
            },
            effect: Effect::Capture,
        }
    };
    kani::assert(result == expected, "BEGIN_PROGRESS");
    kani::cover!(result.effect == Effect::Capture, "capture makes progress");
    kani::cover!(
        result.effect == Effect::PresentExhausted,
        "exhaustion is reachable"
    );
}

#[kani::proof]
fn completion_and_failure_have_exact_outcomes() {
    let state = arbitrary_state();
    let id = ScanId::from_raw(kani::any());
    let found: bool = kani::any();
    let completed = transition(state, Event::Completed { id, found });
    let failed = transition(state, Event::Failed { id });
    let current = state.id == id && state.phase == Phase::Scanning;
    let expected = if current {
        Transition {
            state: ScanState {
                phase: if found { Phase::Results } else { Phase::Empty },
                ..state
            },
            effect: if found {
                Effect::PresentResults
            } else {
                Effect::PresentEmpty
            },
        }
    } else {
        Transition {
            state,
            effect: Effect::None,
        }
    };
    kani::assert(completed == expected, "COMPLETION_CONTRACT");
    assert_eq!(
        failed,
        if current {
            Transition {
                state: ScanState {
                    phase: Phase::Failed,
                    ..state
                },
                effect: Effect::PresentFailure,
            }
        } else {
            Transition {
                state,
                effect: Effect::None,
            }
        }
    );
    kani::cover!(
        completed.effect == Effect::PresentResults,
        "results are reachable"
    );
    kani::cover!(
        completed.effect == Effect::PresentEmpty,
        "empty results are reachable"
    );
    kani::cover!(
        failed.effect == Effect::PresentFailure,
        "scan failure is reachable"
    );
}

#[kani::proof]
fn cancellation_always_closes_and_preserves_generation() {
    let state = arbitrary_state();
    let result = transition(state, Event::Cancel);
    assert_eq!(
        result,
        Transition {
            state: ScanState {
                phase: Phase::Idle,
                ..state
            },
            effect: Effect::Close
        }
    );
}

#[kani::proof]
fn result_actions_and_open_completion_have_exact_outcomes() {
    let state = arbitrary_state();
    let id = ScanId::from_raw(kani::any());
    let web: bool = kani::any();
    let succeeded: bool = kani::any();
    let opened = transition(
        state,
        Event::Open {
            id,
            kind: if web {
                PayloadKind::WebUrl
            } else {
                PayloadKind::CopyOnly
            },
        },
    );
    let copied = transition(state, Event::Copy { id });
    let finished = transition(state, Event::OpenFinished { id, succeeded });
    let unchanged = Transition {
        state,
        effect: Effect::None,
    };
    assert_eq!(
        opened,
        if id == state.id && state.phase == Phase::Results && web {
            Transition {
                state: ScanState {
                    phase: Phase::Opening,
                    ..state
                },
                effect: Effect::OpenSelected,
            }
        } else {
            unchanged
        }
    );
    assert_eq!(
        copied,
        if id == state.id && state.phase == Phase::Results {
            Transition {
                state,
                effect: Effect::CopySelected,
            }
        } else {
            unchanged
        }
    );
    assert_eq!(
        finished,
        if id == state.id && state.phase == Phase::Opening {
            Transition {
                state: ScanState {
                    phase: if succeeded {
                        Phase::Idle
                    } else {
                        Phase::Results
                    },
                    ..state
                },
                effect: if succeeded {
                    Effect::Close
                } else {
                    Effect::PresentOpenFailure
                },
            }
        } else {
            unchanged
        }
    );
    kani::cover!(
        opened.effect == Effect::OpenSelected,
        "confirmed opening makes progress"
    );
    kani::cover!(copied.effect == Effect::CopySelected, "copy makes progress");
    kani::cover!(
        finished.effect == Effect::PresentOpenFailure,
        "failed opening preserves results"
    );
    kani::cover!(
        finished.effect == Effect::Close,
        "successful opening closes"
    );
}

#[kani::proof]
#[kani::unwind(9)]
fn eight_event_traces_preserve_confirmation_and_generations() {
    let mut state = arbitrary_state();
    let mut previous_id = state.id;
    for _ in 0..8 {
        let event = match kani::any::<u8>() {
            0 => Event::Begin,
            1 => Event::Cancel,
            choice => {
                let id = if kani::any() { state.id } else { previous_id };
                match choice {
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
        };
        let result = transition(state, event);
        assert!(result.state.id.raw() >= state.id.raw());
        if result.state.id != state.id {
            assert_eq!(event, Event::Begin);
            assert_eq!(result.effect, Effect::Capture);
        }
        if result.effect == Effect::OpenSelected {
            assert_eq!(state.phase, Phase::Results);
            assert_eq!(
                event,
                Event::Open {
                    id: state.id,
                    kind: PayloadKind::WebUrl
                }
            );
        }
        if event == Event::Cancel {
            assert_eq!(result.state.phase, Phase::Idle);
            assert_eq!(result.state.id, state.id);
        }
        previous_id = state.id;
        state = result.state;
    }
    kani::cover!(state.phase == Phase::Results, "trace can preserve results");
    kani::cover!(
        state.phase == Phase::Opening,
        "trace can reach confirmed opening"
    );
}
