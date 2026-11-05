use qrscanner_core::{Effect, Event, PayloadKind, Phase, ScanId, ScanState, transition};

const QR_PHASE_IDLE: u32 = 0;
const QR_PHASE_SCANNING: u32 = 1;
const QR_PHASE_RESULTS: u32 = 2;
const QR_PHASE_OPENING: u32 = 3;
const QR_PHASE_EMPTY: u32 = 4;
const QR_PHASE_FAILED: u32 = 5;
const QR_EVENT_BEGIN: u32 = 0;
const QR_EVENT_CANCEL: u32 = 1;
const QR_EVENT_COMPLETED: u32 = 2;
const QR_EVENT_FAILED: u32 = 3;
const QR_EVENT_OPEN: u32 = 4;
const QR_EVENT_COPY: u32 = 5;
const QR_EVENT_OPEN_FINISHED: u32 = 6;
const QR_EFFECT_NONE: u32 = 0;
const QR_EFFECT_CAPTURE: u32 = 1;
const QR_EFFECT_RESULTS: u32 = 2;
const QR_EFFECT_EMPTY: u32 = 3;
const QR_EFFECT_FAILURE: u32 = 4;
const QR_EFFECT_OPEN: u32 = 5;
const QR_EFFECT_COPY: u32 = 6;
const QR_EFFECT_CLOSE: u32 = 7;
const QR_EFFECT_OPEN_FAILURE: u32 = 8;
const QR_EFFECT_EXHAUSTED: u32 = 9;
const QR_VALUE_FALSE: u32 = 0;
const QR_VALUE_TRUE: u32 = 1;
const QR_STATUS_OK: u32 = 0;
const QR_STATUS_INVALID: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WireState {
    pub generation: u64,
    pub phase: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WireEvent {
    pub generation: u64,
    pub tag: u32,
    pub value: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct WireTransition {
    pub state: WireState,
    pub effect: u32,
    pub status: u32,
}

fn decode_phase(value: u32) -> Option<Phase> {
    match value {
        QR_PHASE_IDLE => Some(Phase::Idle),
        QR_PHASE_SCANNING => Some(Phase::Scanning),
        QR_PHASE_RESULTS => Some(Phase::Results),
        QR_PHASE_OPENING => Some(Phase::Opening),
        QR_PHASE_EMPTY => Some(Phase::Empty),
        QR_PHASE_FAILED => Some(Phase::Failed),
        _ => None,
    }
}

fn encode_phase(phase: Phase) -> u32 {
    match phase {
        Phase::Idle => QR_PHASE_IDLE,
        Phase::Scanning => QR_PHASE_SCANNING,
        Phase::Results => QR_PHASE_RESULTS,
        Phase::Opening => QR_PHASE_OPENING,
        Phase::Empty => QR_PHASE_EMPTY,
        Phase::Failed => QR_PHASE_FAILED,
    }
}

fn decode_event(event: WireEvent) -> Option<Event> {
    let id = ScanId::from_raw(event.generation);
    match (event.tag, event.value) {
        (QR_EVENT_BEGIN, QR_VALUE_FALSE) => Some(Event::Begin),
        (QR_EVENT_CANCEL, QR_VALUE_FALSE) => Some(Event::Cancel),
        (QR_EVENT_COMPLETED, QR_VALUE_FALSE | QR_VALUE_TRUE) => Some(Event::Completed {
            id,
            found: event.value == QR_VALUE_TRUE,
        }),
        (QR_EVENT_FAILED, QR_VALUE_FALSE) => Some(Event::Failed { id }),
        (QR_EVENT_OPEN, QR_VALUE_FALSE) => Some(Event::Open {
            id,
            kind: PayloadKind::CopyOnly,
        }),
        (QR_EVENT_OPEN, QR_VALUE_TRUE) => Some(Event::Open {
            id,
            kind: PayloadKind::WebUrl,
        }),
        (QR_EVENT_COPY, QR_VALUE_FALSE) => Some(Event::Copy { id }),
        (QR_EVENT_OPEN_FINISHED, QR_VALUE_FALSE | QR_VALUE_TRUE) => Some(Event::OpenFinished {
            id,
            succeeded: event.value == QR_VALUE_TRUE,
        }),
        _ => None,
    }
}

fn encode_effect(effect: Effect) -> u32 {
    match effect {
        Effect::None => QR_EFFECT_NONE,
        Effect::Capture => QR_EFFECT_CAPTURE,
        Effect::PresentResults => QR_EFFECT_RESULTS,
        Effect::PresentEmpty => QR_EFFECT_EMPTY,
        Effect::PresentFailure => QR_EFFECT_FAILURE,
        Effect::OpenSelected => QR_EFFECT_OPEN,
        Effect::CopySelected => QR_EFFECT_COPY,
        Effect::Close => QR_EFFECT_CLOSE,
        Effect::PresentOpenFailure => QR_EFFECT_OPEN_FAILURE,
        Effect::PresentExhausted => QR_EFFECT_EXHAUSTED,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn qr_transition(state: WireState, event: WireEvent) -> WireTransition {
    let (Some(phase), Some(event)) = (decode_phase(state.phase), decode_event(event)) else {
        return WireTransition {
            state,
            effect: QR_EFFECT_NONE,
            status: QR_STATUS_INVALID,
        };
    };
    let result = transition(
        ScanState {
            id: ScanId::from_raw(state.generation),
            phase,
        },
        event,
    );
    WireTransition {
        state: WireState {
            generation: result.state.id.raw(),
            phase: encode_phase(result.state.phase),
        },
        effect: encode_effect(result.effect),
        status: QR_STATUS_OK,
    }
}

#[cfg(kani)]
mod proofs;
