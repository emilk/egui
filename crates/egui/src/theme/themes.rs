use std::sync::Arc;

use epaint::mutex::Mutex;

use crate::{Id, theme::StyleProvider, util::IdTypeMap, widget_style::WidgetStyle};

/// The registry of [`StyleProvider`]s, one per [`WidgetStyle`] type.
///
/// Each widget asks this registry for the provider of its style type
/// (e.g. [`crate::widget_style::ButtonStyle`]), and that provider computes the final style from the
/// widget's classes and state.
///
/// A default provider is registered for every built-in style. Register your
/// own with [`crate::Context::add_widget_theme`] or [`crate::Context::replace_widget_theme`].
#[derive(Default)]
pub struct Themes {
    themes: IdTypeMap,
}

type ThemeWrap<S> = Arc<Mutex<Box<dyn StyleProvider<S> + Send + Sync>>>;

impl Themes {
    /// Register a [`StyleProvider`] for the specified widget [`WidgetStyle`] `S`
    ///
    /// Existing themes are overwritten if `force` is `true` or the new theme differs.
    pub(crate) fn register<S: WidgetStyle + 'static>(
        &mut self,
        theme: impl StyleProvider<S> + Send + Sync + 'static,
        force: bool,
    ) {
        if !force
            && self
                .themes
                .get_temp::<ThemeWrap<S>>(Id::NULL)
                .is_some_and(|t| t.lock().type_id() == theme.type_id())
        {
            return;
        }

        self.themes
            .insert_temp::<ThemeWrap<S>>(Id::NULL, Arc::new(Mutex::new(Box::new(theme))));
    }

    /// Fetch the style of the current theme
    pub fn get<S: WidgetStyle + 'static>(&self) -> ThemeWrap<S> {
        let v = self.themes.get_temp::<ThemeWrap<S>>(Id::NULL);

        v.unwrap_or_else(|| {
            panic!(
                "A style should be set for {:?}",
                core::any::type_name::<S>()
            )
        })
    }
}
