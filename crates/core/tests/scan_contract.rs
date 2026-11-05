use qrscanner_core::{Effect, Event, PayloadKind, Phase, ScanId, ScanState, transition};

fn scanning() -> ScanState {
    transition(ScanState::INITIAL, Event::Begin).state
}

fn results() -> ScanState {
    let state = scanning();
    transition(
        state,
        Event::Completed {
            id: state.id,
            found: true,
        },
    )
    .state
}

#[test]
fn begins_capture_without_opening() {
    let result = transition(ScanState::INITIAL, Event::Begin);
    assert_eq!(result.state.phase, Phase::Scanning);
    assert_eq!(result.effect, Effect::Capture);
    assert_eq!(result.state.id, ScanId::from_raw(1));
}

#[test]
fn detection_requires_explicit_confirmation() {
    let state = scanning();
    let result = transition(
        state,
        Event::Completed {
            id: state.id,
            found: true,
        },
    );
    assert_eq!(result.state.phase, Phase::Results);
    assert_eq!(result.effect, Effect::PresentResults);
}

#[test]
fn web_url_can_only_open_from_current_results() {
    let state = results();
    let event = Event::Open {
        id: state.id,
        kind: PayloadKind::WebUrl,
    };
    let opened = transition(state, event);
    assert_eq!(opened.effect, Effect::OpenSelected);
    assert_eq!(opened.state.phase, Phase::Opening);
    assert_eq!(transition(opened.state, event).effect, Effect::None);
}

#[test]
fn text_and_non_web_schemes_are_copy_only() {
    let state = results();
    assert_eq!(
        transition(
            state,
            Event::Open {
                id: state.id,
                kind: PayloadKind::CopyOnly
            }
        )
        .effect,
        Effect::None
    );
    let copied = transition(state, Event::Copy { id: state.id });
    assert_eq!(copied.effect, Effect::CopySelected);
    assert_eq!(copied.state.phase, Phase::Results);
}

#[test]
fn cancellation_discards_a_late_completion() {
    let state = scanning();
    let cancelled = transition(state, Event::Cancel).state;
    let late = transition(
        cancelled,
        Event::Completed {
            id: state.id,
            found: true,
        },
    );
    assert_eq!(late.state.phase, Phase::Idle);
    assert_eq!(late.effect, Effect::None);
}

#[test]
fn restarted_scan_rejects_previous_generation() {
    let old = scanning();
    let cancelled = transition(old, Event::Cancel).state;
    let new = transition(cancelled, Event::Begin).state;
    let stale = transition(
        new,
        Event::Completed {
            id: old.id,
            found: true,
        },
    );
    assert_eq!(stale.state, new);
    assert_eq!(stale.effect, Effect::None);
}

#[test]
fn repeated_shortcuts_do_not_start_parallel_scans() {
    let state = scanning();
    let result = transition(state, Event::Begin);
    assert_eq!(result.state, state);
    assert_eq!(result.effect, Effect::None);
}

#[test]
fn failed_open_preserves_results_for_retry() {
    let state = results();
    let opening = transition(
        state,
        Event::Open {
            id: state.id,
            kind: PayloadKind::WebUrl,
        },
    )
    .state;
    let failed = transition(
        opening,
        Event::OpenFinished {
            id: state.id,
            succeeded: false,
        },
    );
    assert_eq!(failed.state.phase, Phase::Results);
    assert_eq!(failed.effect, Effect::PresentOpenFailure);
}

#[test]
fn successful_open_closes_the_panel() {
    let state = results();
    let opening = transition(
        state,
        Event::Open {
            id: state.id,
            kind: PayloadKind::WebUrl,
        },
    )
    .state;
    let result = transition(
        opening,
        Event::OpenFinished {
            id: state.id,
            succeeded: true,
        },
    );
    assert_eq!(result.state.phase, Phase::Idle);
    assert_eq!(result.effect, Effect::Close);
}

#[test]
fn empty_and_failed_scans_are_distinct() {
    let state = scanning();
    assert_eq!(
        transition(
            state,
            Event::Completed {
                id: state.id,
                found: false
            }
        )
        .state
        .phase,
        Phase::Empty
    );
    assert_eq!(
        transition(state, Event::Failed { id: state.id })
            .state
            .phase,
        Phase::Failed
    );
    assert_eq!(
        transition(
            state,
            Event::Completed {
                id: state.id,
                found: false
            }
        )
        .effect,
        Effect::PresentEmpty
    );
    assert_eq!(
        transition(state, Event::Failed { id: state.id }).effect,
        Effect::PresentFailure
    );
}

#[test]
fn generation_never_wraps() {
    let state = ScanState {
        id: ScanId::from_raw(u64::MAX),
        phase: Phase::Idle,
    };
    let result = transition(state, Event::Begin);
    assert_eq!(result.state, state);
    assert_eq!(result.effect, Effect::PresentExhausted);
}

#[test]
fn asynchronous_events_require_both_the_current_generation_and_expected_phase() {
    let current = ScanId::from_raw(9);
    let stale = ScanId::from_raw(8);
    for phase in [
        Phase::Idle,
        Phase::Scanning,
        Phase::Results,
        Phase::Opening,
        Phase::Empty,
        Phase::Failed,
    ] {
        let state = ScanState { id: current, phase };
        for id in [current, stale] {
            for (event, required, next, effect) in [
                (
                    Event::Failed { id },
                    Phase::Scanning,
                    Phase::Failed,
                    Effect::PresentFailure,
                ),
                (
                    Event::Copy { id },
                    Phase::Results,
                    Phase::Results,
                    Effect::CopySelected,
                ),
                (
                    Event::OpenFinished {
                        id,
                        succeeded: true,
                    },
                    Phase::Opening,
                    Phase::Idle,
                    Effect::Close,
                ),
                (
                    Event::OpenFinished {
                        id,
                        succeeded: false,
                    },
                    Phase::Opening,
                    Phase::Results,
                    Effect::PresentOpenFailure,
                ),
            ] {
                let actual = transition(state, event);
                if id == current && phase == required {
                    assert_eq!(
                        actual.state,
                        ScanState {
                            id: current,
                            phase: next
                        }
                    );
                    assert_eq!(actual.effect, effect);
                } else {
                    assert_eq!(actual.state, state);
                    assert_eq!(actual.effect, Effect::None);
                }
            }
        }
    }
}
