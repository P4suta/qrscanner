use qrscanner_ffi::{WireEvent, WireState, qr_transition};

#[test]
fn invalid_wire_values_are_refused_without_effects() {
    for phase in [6, 7, u32::MAX] {
        let output = qr_transition(
            WireState {
                generation: 1,
                phase,
            },
            WireEvent {
                generation: 1,
                tag: 0,
                value: 0,
            },
        );
        assert_eq!(output.status, 1);
        assert_eq!(output.effect, 0);
    }
    for (tag, value) in [(99, 0), (2, 2), (4, 2), (6, u32::MAX)] {
        let output = qr_transition(
            WireState {
                generation: 1,
                phase: 2,
            },
            WireEvent {
                generation: 1,
                tag,
                value,
            },
        );
        assert_eq!(output.status, 1);
        assert_eq!(output.effect, 0);
    }
}

#[test]
fn wire_sequence_requires_confirmation_and_finishes_opening() {
    let started = qr_transition(
        WireState {
            generation: 0,
            phase: 0,
        },
        WireEvent {
            generation: 0,
            tag: 0,
            value: 0,
        },
    );
    assert_eq!(started.effect, 1);
    let detected = qr_transition(
        started.state,
        WireEvent {
            generation: started.state.generation,
            tag: 2,
            value: 1,
        },
    );
    assert_eq!(detected.effect, 2);
    let opened = qr_transition(
        detected.state,
        WireEvent {
            generation: detected.state.generation,
            tag: 4,
            value: 1,
        },
    );
    assert_eq!(opened.effect, 5);
    let finished = qr_transition(
        opened.state,
        WireEvent {
            generation: opened.state.generation,
            tag: 6,
            value: 1,
        },
    );
    assert_eq!(finished.effect, 7);
    assert_eq!(finished.state.phase, 0);
}

#[test]
fn c_layout_matches_the_native_header() {
    assert_eq!(size_of::<WireState>(), 16);
    assert_eq!(size_of::<WireEvent>(), 16);
    assert_eq!(size_of::<qrscanner_ffi::WireTransition>(), 24);
    assert_eq!(align_of::<WireState>(), 8);
}

#[test]
fn every_wire_phase_supports_cancellation() {
    for phase in 0..=5 {
        let output = qr_transition(
            WireState {
                generation: 9,
                phase,
            },
            WireEvent {
                generation: 9,
                tag: 1,
                value: 0,
            },
        );
        assert_eq!(output.status, 0);
        assert_eq!(
            output.state,
            WireState {
                generation: 9,
                phase: 0
            }
        );
        assert_eq!(output.effect, 7);
    }
}

#[test]
fn every_documented_event_preserves_its_wire_contract() {
    for (phase, tag, value, next_phase, effect) in [
        (0, 0, 0, 1, 1),
        (1, 1, 0, 0, 7),
        (1, 2, 0, 4, 3),
        (1, 2, 1, 2, 2),
        (1, 3, 0, 5, 4),
        (2, 4, 0, 2, 0),
        (2, 4, 1, 3, 5),
        (2, 5, 0, 2, 6),
        (3, 6, 0, 2, 8),
        (3, 6, 1, 0, 7),
    ] {
        let output = qr_transition(
            WireState {
                generation: 9,
                phase,
            },
            WireEvent {
                generation: 9,
                tag,
                value,
            },
        );
        assert_eq!(output.status, 0, "event {tag}/{value}");
        assert_eq!(output.state.phase, next_phase, "event {tag}/{value}");
        assert_eq!(output.state.generation, if tag == 0 { 10 } else { 9 });
        assert_eq!(output.effect, effect, "event {tag}/{value}");
    }
}
