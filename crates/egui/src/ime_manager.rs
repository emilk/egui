use crate::{
    Id, PlatformOutput,
    memory::{Focus, InteractionState},
    output::IMEOutput,
};

#[derive(Debug, Default)]
pub(crate) struct ImeManager {
    ime_state: ImeState,
}

impl ImeManager {
    pub fn begin_pass(&mut self, interaction: &InteractionState, focus: Option<&Focus>) {
        if interaction.is_using_pointer() || focus.is_none_or(|focus| focus.is_focus_changed()) {
            self.ime_state.interrupt();
        }
    }

    pub fn end_pass(&mut self, platform_output: &mut PlatformOutput) {
        platform_output.ime = self.ime_state.take_ime_output();
    }

    pub fn try_claim_ime_events_ownership(&mut self, id: Id) -> bool {
        match &self.ime_state {
            ImeState::Idle => {
                self.ime_state = ImeState::Claimed { claimer: id };
                false
            }
            ImeState::ClaimedLastFrame { claimer } if *claimer == id => {
                self.ime_state = ImeState::Owned {
                    owner: id,
                    ime_output: None,
                };
                true
            }
            _ => false,
        }
    }

    pub fn try_set_ime_output(&mut self, id: Id, ime_output: impl FnOnce() -> IMEOutput) {
        match &mut self.ime_state {
            ImeState::Owned {
                owner,
                ime_output: current_ime_output,
            } if *owner == id => {
                *current_ime_output = Some(ime_output());
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, Default)]
enum ImeState {
    #[default]
    Idle,
    Owned {
        owner: Id,
        ime_output: Option<IMEOutput>,
    },
    Claimed {
        claimer: Id,
    },
    ClaimedLastFrame {
        claimer: Id,
    },
}

impl ImeState {
    fn interrupt(&mut self) {
        *self = Self::Idle;
    }

    fn take_ime_output(&mut self) -> Option<IMEOutput> {
        match self {
            Self::Owned { owner, ime_output } => {
                let ime_output = *ime_output;
                *self = Self::ClaimedLastFrame { claimer: *owner };
                ime_output
            }
            Self::Claimed { claimer } => {
                *self = Self::ClaimedLastFrame { claimer: *claimer };
                None
            }
            _ => {
                *self = Self::Idle;
                None
            }
        }
    }
}
