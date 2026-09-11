//! When has the user finished changing the text selection?
//!
//! egui reports settled selections through
//! [`crate::OutputCommand::TextSelectionSettled`], which on X11 and Wayland is
//! what fills the PRIMARY selection: the "select here, middle-click paste
//! there" channel, separate from the Ctrl+C/Ctrl+V clipboard.
//!
//! Reporting only when the selection settles is not just about cost. Claiming
//! PRIMARY makes the process its owner, so re-claiming it on every frame of a
//! drag would keep stealing the selection back from whatever application the
//! user selected in most recently. Any integration acting on this command wants
//! the same thing: one report per selection, not one per frame.

use crate::Ui;

/// Is anyone listening for settled text selections?
///
/// Assembling the selected text costs an allocation and can span several
/// widgets, so it is only worth doing when the integration will use it.
pub(crate) fn is_reported(ui: &Ui) -> bool {
    ui.ctx().options(|options| options.report_text_selection)
}

/// Has the user just finished selecting, so that a pending selection should be
/// reported now?
///
/// `selection_is_dirty` says that the selection has changed since it was last
/// reported. Without it a plain click, which releases the pointer without
/// touching the selection, would report an unchanged selection again.
pub(crate) fn should_report(ui: &Ui, selection_is_dirty: bool) -> bool {
    selection_is_dirty && is_reported(ui) && ui.input(|i| !i.pointer.any_down())
}
