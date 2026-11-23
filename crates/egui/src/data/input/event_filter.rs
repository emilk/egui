use super::Event;

// TODO(emilk): generalize this to a proper event filter.
/// Controls which events that a focused widget will have exclusive access to.
///
/// Currently this only controls a few special keyboard events,
/// but in the future this `struct` should be extended into a full callback thing.
///
/// Any events not covered by the filter are given to the widget, but are not exclusive.
#[derive(Clone, Copy, Debug)]
pub struct EventFilter {
    /// If `true`, pressing tab will act on the widget,
    /// and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub tab: bool,

    /// If `true`, pressing horizontal arrows will act on the
    /// widget, and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub horizontal_arrows: bool,

    /// If `true`, pressing vertical arrows will act on the
    /// widget, and NOT move focus away from the focused widget.
    ///
    /// Default: `false`
    pub vertical_arrows: bool,

    /// If `true`, pressing escape will act on the widget,
    /// and NOT surrender focus from the focused widget.
    ///
    /// Default: `false`
    pub escape: bool,
}

#[expect(clippy::derivable_impls)] // let's be explicit
impl Default for EventFilter {
    fn default() -> Self {
        Self {
            tab: false,
            horizontal_arrows: false,
            vertical_arrows: false,
            escape: false,
        }
    }
}

impl EventFilter {
    pub fn matches(&self, event: &Event) -> bool {
        if let Event::Key(crate::KeyEvent { key, .. }) = event {
            match key.logical {
                crate::Key::Named(crate::NamedKey::Tab) => self.tab,
                crate::Key::Named(crate::NamedKey::ArrowUp | crate::NamedKey::ArrowDown) => {
                    self.vertical_arrows
                }
                crate::Key::Named(crate::NamedKey::ArrowRight | crate::NamedKey::ArrowLeft) => {
                    self.horizontal_arrows
                }
                crate::Key::Named(crate::NamedKey::Escape) => self.escape,
                _ => true,
            }
        } else {
            true
        }
    }
}
