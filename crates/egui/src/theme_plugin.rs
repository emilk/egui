use std::sync::Arc;

use epaint::mutex::Mutex;

use crate::{
    Id, Style, Ui,
    util::IdTypeMap,
    widget_style::{Classes, StyleStruct, WidgetState},
};

/// A cache that can be implemented to reduce computation time of a `ThemeStyle`
#[derive(Debug, Default, Clone)]
pub struct ThemeCache {
    cache: IdTypeMap,
}

impl ThemeCache {
    /// Access the cache for the requested [`StyleStruct`] based on the [`Classes`] and
    /// the [`WidgetState`]
    ///
    /// If no entry match the parameter then compute the fallback style and
    /// save the output for later.
    pub fn get<S: StyleStruct + 'static>(
        &mut self,
        classes: &Classes,
        state: WidgetState,
        fallback: impl FnOnce() -> S,
    ) -> S {
        let style_id = Id::new(classes).with(state);
        if let Some(style) = self.cache.get_temp::<S>(style_id) {
            style
        } else {
            let style = fallback();
            self.cache.insert_temp(style_id, style.clone());
            style
        }
    }
}

/// A Theme plugin that implement a style computation for a defined `StyleStruct`
pub trait ThemeStyle<S> {
    /// The style according to the classes and state of the widget
    fn style(&mut self, classes: &Classes, state: WidgetState, base: &Style) -> S;
}

impl Ui {
    /// Access the installed theme plugin if there is one and fetch the requested widget style if it exist.
    /// Fallback to the default style if not found.
    ///
    /// Requested widget style must implement [`StyleStruct`].
    pub fn widget_style<S: StyleStruct + Clone + 'static>(
        &self,
        id: crate::Id,
        classes: &Classes,
    ) -> S {
        // If the requested `StyleStruct` is cached, return it without computing.
        // Otherwise proceed to compute the style from the widget information.

        // Fetch the current state of the widget
        let state = self
            .ctx()
            .read_response(id)
            .map(|r| r.widget_state())
            .unwrap_or_default();

        if let Some(style) = self.get_style::<S>(classes, state, self.style()) {
            style
        } else {
            S::default_style(classes, state, self.style())
        }
    }
}

#[derive(Default)]
pub(crate) struct Themes {
    themes: IdTypeMap,
}

impl Themes {
    /// Register a theme and the style associated
    pub(crate) fn register<S: StyleStruct + 'static>(
        &mut self,
        theme: impl ThemeStyle<S> + Send + Sync + 'static,
        force: bool,
    ) {
        if !force
            && self
                .themes
                .get_temp::<Arc<Mutex<Box<dyn ThemeStyle<S> + Send + Sync>>>>(Id::NULL)
                .is_some()
        {
            return;
        }
        self.themes
            .insert_temp::<Arc<Mutex<Box<dyn ThemeStyle<S> + Send + Sync>>>>(
                Id::NULL,
                Arc::new(Mutex::new(Box::new(theme))),
            );
    }

    /// Fetch the style of the current theme
    pub(crate) fn get<S: StyleStruct + 'static>(
        &self,
        classes: &Classes,
        state: WidgetState,
        base: &Style,
    ) -> Option<S> {
        let v = self
            .themes
            .get_temp::<Arc<Mutex<Box<dyn ThemeStyle<S> + Send + Sync>>>>(Id::NULL);
        v.map(|engine| engine.lock().style(classes, state, base))
    }
}
