use std::{any::TypeId, sync::Arc};

use emath::Vec2;
use epaint::{Shadow, Stroke, mutex::Mutex, text::TextWrapMode};

use crate::{
    Frame, Id, Spacing, Style, TextStyle, Ui, Visuals,
    style::Widgets,
    util::IdTypeMap,
    widget_style::{
        BaseStyle, ButtonStyle, CheckboxStyle, Classes, HasClasses as _, LabelStyle,
        SELECTED_CLASS, SeparatorStyle, TextVisuals, WidgetState, WidgetStyle,
    },
};

/// A cache that can be implemented to reduce computation time of a `ThemeStyle`
#[derive(Debug, Default, Clone)]
pub struct ThemeCache {
    cache: IdTypeMap,
}

impl ThemeCache {
    /// Access the cache for the requested [`WidgetStyle`] based on the [`Classes`] and
    /// the [`WidgetState`]
    ///
    /// If no entry match the parameter then compute the fallback style and
    /// save the output for later.
    pub fn get<S: WidgetStyle + 'static>(
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

/// A Theme plugin that implement a style computation for a defined `WidgetStyle`
pub trait ThemeStyle<S> {
    /// The style according to the classes and state of the widget
    fn style(&mut self, themes: &Themes, classes: &Classes, state: WidgetState) -> S;

    /// Help to differ the different themes
    fn theme_type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}

#[derive(Debug, Clone)]
struct DefaultStyle;

impl ThemeStyle<BaseStyle> for DefaultStyle {
    fn style(&mut self, _themes: &Themes, _classes: &Classes, state: WidgetState) -> BaseStyle {
        let visuals = Widgets::dark();
        let spacing = Spacing::default();

        let visuals = match state {
            WidgetState::Noninteractive => visuals.noninteractive,
            WidgetState::Inactive => visuals.inactive,
            WidgetState::Hovered => visuals.hovered,
            WidgetState::Active => visuals.active,
        };

        BaseStyle {
            frame: Frame {
                fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
                corner_radius: visuals.corner_radius,
                inner_margin: spacing.button_padding.into(),
                ..Default::default()
            },
            stroke: visuals.fg_stroke,
            text: TextVisuals {
                color: visuals.text_color(),
                font_id: TextStyle::Body.resolve(&Style::default()),
                strikethrough: Stroke::NONE,
                underline: Stroke::NONE,
            },
        }
    }
}

impl ThemeStyle<ButtonStyle> for DefaultStyle {
    fn style(&mut self, themes: &Themes, classes: &Classes, state: WidgetState) -> ButtonStyle {
        let widget_visuals = Widgets::dark();
        let spacing = Spacing::default();

        let mut widget_visuals = match state {
            WidgetState::Noninteractive => widget_visuals.noninteractive,
            WidgetState::Inactive => widget_visuals.inactive,
            WidgetState::Hovered => widget_visuals.hovered,
            WidgetState::Active => widget_visuals.active,
        };

        let mut ws: BaseStyle = themes.get(classes, state);

        if classes.has(SELECTED_CLASS) {
            let visuals = Visuals::default();
            widget_visuals.weak_bg_fill = visuals.selection.bg_fill;
            widget_visuals.bg_fill = visuals.selection.bg_fill;
            widget_visuals.fg_stroke = visuals.selection.stroke;
            ws.text.color = visuals.selection.stroke.color;
        }

        ButtonStyle {
            frame: Frame {
                fill: widget_visuals.weak_bg_fill,
                stroke: widget_visuals.bg_stroke,
                corner_radius: widget_visuals.corner_radius,
                outer_margin: (-Vec2::splat(widget_visuals.expansion)).into(),
                inner_margin: (spacing.button_padding + Vec2::splat(widget_visuals.expansion)
                    - Vec2::splat(widget_visuals.bg_stroke.width))
                .into(),
                ..Default::default()
            },
            text_style: ws.text,
        }
    }
}

impl ThemeStyle<CheckboxStyle> for DefaultStyle {
    fn style(&mut self, themes: &Themes, classes: &Classes, state: WidgetState) -> CheckboxStyle {
        let widget_visuals = Widgets::dark();
        let spacing = Spacing::default();

        let widget_visuals = match state {
            WidgetState::Noninteractive => widget_visuals.noninteractive,
            WidgetState::Inactive => widget_visuals.inactive,
            WidgetState::Hovered => widget_visuals.hovered,
            WidgetState::Active => widget_visuals.active,
        };

        let ws: BaseStyle = themes.get(classes, state);

        CheckboxStyle {
            frame: Frame::new(),
            checkbox_size: spacing.icon_width,
            check_size: spacing.icon_width_inner,
            checkbox_frame: Frame {
                fill: widget_visuals.bg_fill,
                corner_radius: widget_visuals.corner_radius,
                stroke: widget_visuals.bg_stroke,
                ..Default::default()
            },
            text_style: ws.text,
            check_stroke: ws.stroke,
        }
    }
}

impl ThemeStyle<LabelStyle> for DefaultStyle {
    fn style(&mut self, themes: &Themes, classes: &Classes, state: WidgetState) -> LabelStyle {
        let ws: BaseStyle = themes.get(classes, state);

        LabelStyle {
            frame: Frame {
                fill: ws.frame.fill,
                inner_margin: 0.0.into(),
                outer_margin: 0.0.into(),
                stroke: Stroke::NONE,
                shadow: Shadow::NONE,
                corner_radius: 0.into(),
            },
            text: ws.text,
            wrap_mode: TextWrapMode::Wrap,
        }
    }
}

impl ThemeStyle<SeparatorStyle> for DefaultStyle {
    fn style(&mut self, themes: &Themes, classes: &Classes, state: WidgetState) -> SeparatorStyle {
        let ws: BaseStyle = themes.get(classes, state);

        SeparatorStyle {
            spacing: 6.0,
            stroke: ws.frame.stroke,
        }
    }
}

impl Ui {
    /// Access the register theme and fetch the requested [`WidgetStyle`].
    ///
    /// Requested widget style must implement [`WidgetStyle`].
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

        self.get_widget_style::<S>(classes, state)
    }
}

pub struct Themes {
    themes: IdTypeMap,
}

type ThemeWrap<S> = Arc<Mutex<Box<dyn ThemeStyle<S> + Send + Sync>>>;

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
    /// Register a [`ThemeStyle`] for the specified widget [`WidgetStyle`] `S`
    ///
    /// Existing themes are overwritten if `force` is `true` or the new theme differs.
    pub(crate) fn register<S: WidgetStyle + 'static>(
        &mut self,
        theme: impl ThemeStyle<S> + Send + Sync + 'static,
        force: bool,
    ) {
        if !force
            && self
                .themes
                .get_temp::<Arc<Mutex<Box<dyn ThemeStyle<S> + Send + Sync>>>>(Id::NULL)
                .is_some_and(|t| t.lock().theme_type_id() == theme.theme_type_id())
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
    pub fn get<S: WidgetStyle + 'static>(&self, classes: &Classes, state: WidgetState) -> S {
        let v = self
            .themes
            .get_temp::<Arc<Mutex<Box<dyn ThemeStyle<S> + Send + Sync>>>>(Id::NULL);

        v.unwrap_or_else(|| panic!("A style should be set for {:?}", std::any::type_name::<S>()))
            .lock()
            .style(self, classes, state)
    }
}
