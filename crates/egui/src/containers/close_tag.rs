#[expect(unused_imports)]
use crate::{Ui, UiBuilder};
use std::sync::atomic::AtomicBool;

/// A tag to mark a container as closable.
///
/// Usually set via [`UiBuilder::closable`].
///
/// [`Ui::close`] will find the closest parent [`ClosableTag`] and set its `close` field to `true`.
/// Use [`Ui::should_close`] to check if close has been called.
#[derive(Debug, Default)]
pub struct ClosableTag {
    pub close: AtomicBool,
}

impl ClosableTag {
    pub const NAME: &'static str = "egui_close_tag";

    /// Set close to `true`
    pub fn set_close(&self) {
        self.close.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Returns `true` if [`ClosableTag::set_close`] has been called.
    pub fn should_close(&self) -> bool {
        self.close.load(std::sync::atomic::Ordering::Relaxed)
    }
}
