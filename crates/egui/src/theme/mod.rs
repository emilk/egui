//! Theming: pluggable [`StyleProvider`]s that compute the style of each widget.

// This module is only public with the `experimental_theme` feature,
// so without it a lot of it looks unused:
#![cfg_attr(not(feature = "experimental"), allow(dead_code, unused_imports))]

mod default_style;
mod style_provider;
mod themes;

pub use self::{default_style::DefaultStyle, style_provider::StyleProvider, themes::Themes};

use crate::{
    Ui,
    class::Classes,
    widget_style::{StyleArgs, WidgetState, WidgetStyle},
};

impl Ui {
    /// The style of the widget with the given [`crate::Id`] and `Classes`,
    /// as computed by the registered theme.
    ///
    /// The types you need to call this are only public with the
    /// `experimental_theme` feature.
    #[cfg_attr(not(feature = "experimental"), doc(hidden))]
    pub fn widget_style<S: WidgetStyle + Clone + 'static>(
        &self,
        id: crate::Id,
        classes: &Classes,
    ) -> S {
        // Fetch the state of the widget, as it was in the previous pass
        let state = if let Some(response) = self.read_response(id) {
            response.widget_state()
        } else {
            // We don't know the state of the widget yet, so we would style it wrong.
            // It will be styled correctly on next frame.
            self.ctx().request_repaint();
            WidgetState::default()
        };

        self.get_widget_style::<S>(&StyleArgs {
            classes,
            state,
            style: self.style(),
            stack: self.stack(),
            ctx: self,
        })
    }
}
