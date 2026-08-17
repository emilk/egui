//! The X11/Wayland PRIMARY selection: "select here, middle-click paste there".
//!
//! PRIMARY is a second, implicit clipboard that every X11 and Wayland
//! application is expected to fill in whenever the user selects text, without
//! the user pressing Ctrl+C. It is served by the process that owns it, so egui
//! only has to hand the text to the integration, which keeps it available for
//! as long as the app runs.
//!
//! Claiming it is not free: whoever claims it last owns it, so re-claiming on
//! every frame would keep stealing the selection back from whatever application
//! the user selected in most recently. egui therefore publishes only when the
//! user finishes a selection: on pointer release after a drag or a
//! double/triple click, and when the keyboard changes the selection while no
//! pointer button is held.

use crate::{Ui, os::OperatingSystem};

/// Is there a PRIMARY selection on this platform at all?
///
/// Only X11 and Wayland have one. Everywhere else the integration would just
/// throw the text away, so we do not even assemble it.
pub(crate) fn is_supported(ui: &Ui) -> bool {
    ui.ctx().os() == OperatingSystem::Nix
}

/// Has the user just finished selecting, so that a pending selection should be
/// published to PRIMARY now?
///
/// `selection_is_dirty` says that the selection has changed since it was last
/// published. Without it a plain click, which releases the pointer without
/// touching the selection, would re-claim PRIMARY and steal it from another
/// application.
pub(crate) fn should_publish(ui: &Ui, selection_is_dirty: bool) -> bool {
    selection_is_dirty && is_supported(ui) && ui.input(|i| !i.pointer.any_down())
}
