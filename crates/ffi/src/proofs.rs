use super::*;

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
