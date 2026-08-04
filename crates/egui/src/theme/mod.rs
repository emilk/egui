//! Theming: pluggable [`StyleProvider`]s that compute the style of each widget.

mod default_style;
mod style_provider;
mod themes;

pub use self::{style_provider::StyleProvider, themes::Themes};

use crate::{
    Ui,
    widget_style::{Classes, StyleArgs, WidgetState, WidgetStyle},
};

impl Ui {
    /// The style of the widget with the given [`crate::Id`] and [`Classes`],
    /// as computed by the registered theme.
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
            // Discard this pass and style it correctly in the next one.
            self.ctx()
                .request_discard("Widget style depends on a widget response we don't have yet");
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
