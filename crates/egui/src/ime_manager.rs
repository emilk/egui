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
    pub(crate) fn begin_pass(&mut self, interaction: &InteractionState, focus: Option<&Focus>) {
        if interaction.is_using_pointer() || focus.is_none_or(|focus| focus.is_focus_changed()) {
            self.ime_state.interrupt();
        }
    }

    pub(crate) fn end_pass(&mut self, platform_output: &mut PlatformOutput) {
        platform_output.ime = self.ime_state.take_ime_output();
    }

    /// See [`crate::Context::try_claim_ime_events_ownership`] for the
    /// documentation.
    pub(crate) fn try_claim_ime_events_ownership(&mut self, id: Id) -> bool {
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

    /// Used by [`crate::Context::try_set_ime_output`].
    pub(crate) fn can_set_ime_output(&self, id: Id) -> bool {
        matches!(
            self.ime_state,
            ImeState::Owned { owner, .. } if owner == id
        )
    }

    /// Used by [`crate::Context::try_set_ime_output`].
    ///
    /// Should only be called immediately after confirming ownership with
    /// [`Self::can_set_ime_output`].
    pub(crate) fn set_ime_output(&mut self, id: Id, ime_output: IMEOutput) {
        match &mut self.ime_state {
            ImeState::Owned {
                owner,
                ime_output: current_ime_output,
            } if *owner == id => {
                *current_ime_output = Some(ime_output);
            }
            _ => {
                debug_assert!(
                    false,
                    "Attempted to set the IME output from a widget that does not own IME events"
                );
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
enum ImeState {
    /// No widget currently owns or has claimed IME events.
    ///
    /// When a widget calls [`crate::Context::try_claim_ime_events_ownership`],
    /// the state transitions to [`ImeState::Claimed`] and the call returns
    /// `false` (ownership is not yet granted).
    ///
    /// This is also the state after an [`interrupt`](Self::interrupt) or when
    /// no widget reclaims ownership between frames.
    #[default]
    Idle,

    /// A widget has expressed interest in owning IME events this frame, but
    /// ownership has not yet been granted.
    ///
    /// At the end of the frame (in [`ImeManager::end_pass`]), the state
    /// transitions to [`ImeState::ClaimedLastFrame`], giving the widget the
    /// opportunity to confirm ownership on the next frame.
    Claimed { claimer: Id },

    /// A widget claimed (or previously owned) IME events last frame and may
    /// confirm ownership this frame.
    ///
    /// - If the same widget calls
    ///   [`crate::Context::try_claim_ime_events_ownership`] again, the state
    ///   transitions to [`ImeState::Owned`] and the call returns `true`.
    /// - Otherwise, the state transitions back to [`ImeState::Idle`] at the end
    ///   of the frame.
    ClaimedLastFrame { claimer: Id },

    /// A widget fully owns IME events for the current frame and may set
    /// [`IMEOutput`] via [`crate::Context::try_set_ime_output`].
    ///
    /// At the end of the frame the stored [`IMEOutput`] (if any) is forwarded
    /// to [`PlatformOutput`] and the state transitions to
    /// [`ImeState::ClaimedLastFrame`]. This means the owning widget will
    /// retain ownership on subsequent frames as long as it continues to call
    /// [`crate::Context::try_claim_ime_events_ownership`] and is not
    /// [`interrupted`](Self::interrupt).
    Owned {
        owner: Id,
        ime_output: Option<IMEOutput>,
    },
}

impl ImeState {
    /// Interrupts IME handling for the current frame, ensuring that no widget
    /// can claim ownership of IME events during this frame. This will result in
    /// the platform output's IME output being `None` for the current frame,
    /// which should cause the IME to be dismissed.
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
