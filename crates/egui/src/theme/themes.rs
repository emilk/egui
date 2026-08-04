use std::sync::Arc;

use epaint::mutex::Mutex;

use crate::{
    Id,
    theme::{StyleProvider, default_style::DefaultStyle},
    util::IdTypeMap,
    widget_style::{
        BaseStyle, ButtonStyle, CheckboxStyle, LabelStyle, SeparatorStyle, WidgetStyle,
    },
};

pub struct Themes {
    themes: IdTypeMap,
}

type ThemeWrap<S> = Arc<Mutex<Box<dyn StyleProvider<S> + Send + Sync>>>;

impl Default for Themes {
    /// Register the default egui theme
    fn default() -> Self {
        let mut themes = IdTypeMap::default();

        themes.insert_temp::<ThemeWrap<BaseStyle>>(
            Id::NULL,
            Arc::new(Mutex::new(Box::new(DefaultStyle))),
        );

        themes.insert_temp::<ThemeWrap<ButtonStyle>>(
            Id::NULL,
            Arc::new(Mutex::new(Box::new(DefaultStyle))),
        );

        themes.insert_temp::<ThemeWrap<SeparatorStyle>>(
            Id::NULL,
            Arc::new(Mutex::new(Box::new(DefaultStyle))),
        );

        themes.insert_temp::<ThemeWrap<CheckboxStyle>>(
            Id::NULL,
            Arc::new(Mutex::new(Box::new(DefaultStyle))),
        );

        themes.insert_temp::<ThemeWrap<LabelStyle>>(
            Id::NULL,
            Arc::new(Mutex::new(Box::new(DefaultStyle))),
        );

        Self { themes }
    }
}

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
                .is_some_and(|t| t.lock().theme_type_id() == theme.theme_type_id())
        {
            return;
        }

        self.themes
            .insert_temp::<ThemeWrap<S>>(Id::NULL, Arc::new(Mutex::new(Box::new(theme))));
    }

    /// Fetch the style of the current theme
    pub fn get<S: WidgetStyle + 'static>(&self) -> ThemeWrap<S> {
        let v = self.themes.get_temp::<ThemeWrap<S>>(Id::NULL);

        v.unwrap_or_else(|| panic!("A style should be set for {:?}", std::any::type_name::<S>()))
    }
}
