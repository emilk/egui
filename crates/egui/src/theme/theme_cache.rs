use crate::{
    Id,
    theme::StyleProvider,
    util::IdTypeMap,
    widget_style::{StyleArgs, WidgetStyle},
};

/// A cache that can be implemented to reduce computation time of a `StyleProvider`
#[derive(Debug, Default, Clone)]
pub struct ThemeCache<Theme> {
    cache: IdTypeMap,
    inner: Theme,
}

impl<Theme> ThemeCache<Theme> {
    pub fn new(theme: Theme) -> Self {
        Self {
            cache: IdTypeMap::default(),
            inner: theme,
        }
    }
}

impl<Theme: StyleProvider<S>, S: WidgetStyle> StyleProvider<S> for ThemeCache<Theme> {
    /// Access the cache for the requested [`WidgetStyle`] based on the [`Classes`](crate::widget_style::Classes) and
    /// the [`WidgetState`](crate::widget_style::WidgetState)
    ///
    /// If no entry match the parameter then compute the fallback style and
    /// save the output for later.
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> S {
        let StyleArgs { classes, state, .. } = modifiers;
        let style_id = Id::new((classes, state));
        if let Some(style) = self.cache.get_temp::<S>(style_id) {
            style
        } else {
            let style = self.inner.style(modifiers);
            self.cache.insert_temp(style_id, style.clone());
            style
        }
    }
}
