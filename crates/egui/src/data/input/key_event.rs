use crate::{Modifiers, data::FullKey};

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct KeyEvent {
    /// The actual logical/physical key that was pressed/released, and modifier state.
    pub key: FullKey,

    /// Was it pressed or released?
    pub pressed: bool,

    /// If this is a `pressed` event, is it a key-repeat?
    ///
    /// On many platforms, holding down a key produces many repeated "pressed" events for it, so called key-repeats.
    /// Sometimes you will want to ignore such events, and this lets you do that.
    ///
    /// egui will automatically detect such repeat events and mark them as such here.
    /// Therefore, if you are writing an egui integration, you do not need to set this (just set it to `false`).
    pub repeat: bool,

    /// The state of the modifier keys at the time of the event.
    pub modifiers: Modifiers,
}
