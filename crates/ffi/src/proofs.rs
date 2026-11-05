use super::*;

fn specified_phase(value: u32) -> Option<Phase> {
    match value {
        0 => Some(Phase::Idle),
        1 => Some(Phase::Scanning),
        2 => Some(Phase::Results),
        3 => Some(Phase::Opening),
        4 => Some(Phase::Empty),
        5 => Some(Phase::Failed),
        _ => None,
    }
}

fn specified_event(raw: WireEvent) -> Option<Event> {
    let id = ScanId::from_raw(raw.generation);
    match (raw.tag, raw.value) {
        (0, 0) => Some(Event::Begin),
        (1, 0) => Some(Event::Cancel),
        (2, 0) => Some(Event::Completed { id, found: false }),
        (2, 1) => Some(Event::Completed { id, found: true }),
        (3, 0) => Some(Event::Failed { id }),
        (4, 0) => Some(Event::Open {
            id,
            kind: PayloadKind::CopyOnly,
        }),
        (4, 1) => Some(Event::Open {
            id,
            kind: PayloadKind::WebUrl,
        }),
        (5, 0) => Some(Event::Copy { id }),
        (6, 0) => Some(Event::OpenFinished {
            id,
            succeeded: false,
        }),
        (6, 1) => Some(Event::OpenFinished {
            id,
            succeeded: true,
        }),
        _ => None,
    }
}

fn specified_phase_tag(phase: Phase) -> u32 {
    match phase {
        Phase::Idle => 0,
        Phase::Scanning => 1,
        Phase::Results => 2,
        Phase::Opening => 3,
        Phase::Empty => 4,
        Phase::Failed => 5,
    }
}

fn specified_effect_tag(effect: Effect) -> u32 {
    match effect {
        Effect::None => 0,
        Effect::Capture => 1,
        Effect::PresentResults => 2,
        Effect::PresentEmpty => 3,
        Effect::PresentFailure => 4,
        Effect::OpenSelected => 5,
        Effect::CopySelected => 6,
        Effect::Close => 7,
        Effect::PresentOpenFailure => 8,
        Effect::PresentExhausted => 9,
    }
}

#[kani::proof]
fn wire_decoders_match_the_documented_numeric_contract() {
    let phase = kani::any();
    assert_eq!(decode_phase(phase), specified_phase(phase));
    let event = WireEvent {
        generation: kani::any(),
        tag: kani::any(),
        value: kani::any(),
    };
    kani::assert(
        decode_event(event) == specified_event(event),
        "WIRE_EVENT_MAPPING",
    );
    kani::cover!(
        matches!(
            specified_event(event),
            Some(Event::Open {
                kind: PayloadKind::WebUrl,
                ..
            })
        ),
        "wire web event is admitted"
    );
}

#[kani::proof]
fn wire_transition_refines_the_core_for_every_raw_input() {
    let state = WireState {
        generation: kani::any(),
        phase: kani::any(),
    };
    let raw = WireEvent {
        generation: kani::any(),
        tag: kani::any(),
        value: kani::any(),
    };
    let actual = qr_transition(state, raw);
    match (specified_phase(state.phase), specified_event(raw)) {
        (Some(phase), Some(event)) => {
            let expected = transition(
                ScanState {
                    id: ScanId::from_raw(state.generation),
                    phase,
                },
                event,
            );
            assert_eq!(actual.state.generation, expected.state.id.raw());
            assert_eq!(
                actual.state.phase,
                specified_phase_tag(expected.state.phase)
            );
            assert_eq!(actual.effect, specified_effect_tag(expected.effect));
            assert_eq!(actual.status, 0);
        }
        _ => {
            assert_eq!(actual.state, state);
            assert_eq!(actual.effect, 0);
            assert_eq!(actual.status, 1);
        }
    }
    kani::cover!(actual.effect == 9, "wire exhaustion is reachable");
    kani::cover!(actual.status == 1, "malformed wire inputs are reachable");
}

#[kani::proof]
fn wire_inputs_are_validated_and_opening_is_confirmed() {
    let state = WireState {
        generation: kani::any(),
        phase: kani::any(),
    };
    let event = WireEvent {
        generation: kani::any(),
        tag: kani::any(),
        value: kani::any(),
    };
    let output = qr_transition(state, event);
    if decode_phase(state.phase).is_none() || decode_event(event).is_none() {
        assert_eq!(output.status, 1);
        assert_eq!(output.effect, 0);
        assert_eq!(output.state, state);
    } else {
        assert_eq!(output.status, 0);
        assert!(output.state.phase <= 5);
        if output.effect == 5 {
            assert_eq!(state.phase, 2);
            assert_eq!(event.tag, 4);
            assert_eq!(event.value, 1);
            assert_eq!(event.generation, state.generation);
        }
    }
    kani::cover!(output.effect == 5, "valid C ABI opening is reachable");
}
