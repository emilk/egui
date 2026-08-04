//! Theming: pluggable [`StyleProvider`]s that compute the style of each widget.

mod default_style;
mod style_provider;
mod themes;

pub use self::{style_provider::StyleProvider, themes::Themes};

use crate::{
    Ui,
    widget_style::{Classes, StyleArgs, WidgetStyle},
};

impl Ui {
    /// The style of the widget with the given [`crate::Id`] and [`Classes`],
    /// as computed by the registered theme.
    pub fn widget_style<S: WidgetStyle + Clone + 'static>(
        &self,
        id: crate::Id,
        classes: &Classes,
    ) -> S {
        // Fetch the current state of the widget
        let state = self
            .read_response(id)
            .map(|r| r.widget_state())
            .unwrap_or_default();

        self.get_widget_style::<S>(&StyleArgs {
            classes,
            state,
            style: self.style(),
            stack: self.stack(),
            ctx: self,
        })
    }
}
