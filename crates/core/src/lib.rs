#![no_std]
#![deny(unsafe_code)]
#![deny(
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanId(u64);

impl ScanId {
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Idle,
    Scanning,
    Results,
    Opening,
    Empty,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanState {
    pub id: ScanId,
    pub phase: Phase,
}

impl ScanState {
    pub const INITIAL: Self = Self {
        id: ScanId(0),
        phase: Phase::Idle,
    };
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PayloadKind {
    WebUrl,
    CopyOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Event {
    Begin,
    Cancel,
    Completed { id: ScanId, found: bool },
    Failed { id: ScanId },
    Open { id: ScanId, kind: PayloadKind },
    Copy { id: ScanId },
    OpenFinished { id: ScanId, succeeded: bool },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    None,
    Capture,
    PresentResults,
    PresentEmpty,
    PresentFailure,
    OpenSelected,
    CopySelected,
    Close,
    PresentOpenFailure,
    PresentExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Transition {
    pub state: ScanState,
    pub effect: Effect,
}

pub const fn transition(state: ScanState, event: Event) -> Transition {
    let unchanged = Transition {
        state,
        effect: Effect::None,
    };
    match event {
        Event::Begin => match state.phase {
            Phase::Scanning | Phase::Opening => unchanged,
            Phase::Idle | Phase::Results | Phase::Empty | Phase::Failed => match state.id.next() {
                Some(id) => Transition {
                    state: ScanState {
                        id,
                        phase: Phase::Scanning,
                    },
                    effect: Effect::Capture,
                },
                None => Transition {
                    state,
                    effect: Effect::PresentExhausted,
                },
            },
        },
        Event::Cancel => Transition {
            state: ScanState {
                phase: Phase::Idle,
                ..state
            },
            effect: Effect::Close,
        },
        Event::Completed { id, found } => {
            if id.raw() != state.id.raw() || !matches!(state.phase, Phase::Scanning) {
                unchanged
            } else if found {
                Transition {
                    state: ScanState {
                        phase: Phase::Results,
                        ..state
                    },
                    effect: Effect::PresentResults,
                }
            } else {
                Transition {
                    state: ScanState {
                        phase: Phase::Empty,
                        ..state
                    },
                    effect: Effect::PresentEmpty,
                }
            }
        }
        Event::Failed { id } => {
            if id.raw() == state.id.raw() && matches!(state.phase, Phase::Scanning) {
                Transition {
                    state: ScanState {
                        phase: Phase::Failed,
                        ..state
                    },
                    effect: Effect::PresentFailure,
                }
            } else {
                unchanged
            }
        }
        Event::Open { id, kind } => {
            if id.raw() == state.id.raw()
                && matches!(state.phase, Phase::Results)
                && matches!(kind, PayloadKind::WebUrl)
            {
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
        }
        Event::Copy { id } => {
            if id.raw() == state.id.raw() && matches!(state.phase, Phase::Results) {
                Transition {
                    state,
                    effect: Effect::CopySelected,
                }
            } else {
                unchanged
            }
        }
        Event::OpenFinished { id, succeeded } => {
            if id.raw() != state.id.raw() || !matches!(state.phase, Phase::Opening) {
                unchanged
            } else if succeeded {
                Transition {
                    state: ScanState {
                        phase: Phase::Idle,
                        ..state
                    },
                    effect: Effect::Close,
                }
            } else {
                Transition {
                    state: ScanState {
                        phase: Phase::Results,
                        ..state
                    },
                    effect: Effect::PresentOpenFailure,
                }
            }
        }
    }
}

#[cfg(kani)]
mod proofs;
