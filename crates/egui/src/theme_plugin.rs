use std::{any::TypeId, sync::Arc};

use emath::Vec2;
use epaint::{Shadow, Stroke, mutex::Mutex, text::TextWrapMode};

use crate::{
    Frame, Id, TextStyle, Ui,
    util::IdTypeMap,
    widget_style::{
        BaseStyle, ButtonStyle, CheckboxStyle, Classes, HasClasses as _, LabelStyle,
        SELECTED_CLASS, SeparatorStyle, StyleArgs, TextVisuals, WidgetState, WidgetStyle,
    },
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
    /// Access the cache for the requested [`WidgetStyle`] based on the [`Classes`] and
    /// the [`WidgetState`]
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

/// A Theme plugin that implement a style computation for a defined `WidgetStyle`
pub trait StyleProvider<S> {
    /// The style according to the classes and state of the widget
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> S;

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

impl StyleProvider<BaseStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> BaseStyle {
        let StyleArgs { style, state, .. } = modifiers;
        let spacing = &style.spacing;
        let widget_visuals = match state {
            WidgetState::Noninteractive => style.visuals.widgets.noninteractive,
            WidgetState::Inactive => style.visuals.widgets.inactive,
            WidgetState::Hovered => style.visuals.widgets.hovered,
            WidgetState::Active => style.visuals.widgets.active,
        };

        BaseStyle {
            frame: Frame {
                fill: widget_visuals.bg_fill,
                stroke: widget_visuals.bg_stroke,
                corner_radius: widget_visuals.corner_radius,
                inner_margin: spacing.button_padding.into(),
                ..Default::default()
            },
            stroke: widget_visuals.fg_stroke,
            text: TextVisuals {
                color: widget_visuals.text_color(),
                font_id: modifiers
                    .style
                    .override_font_id
                    .clone()
                    .unwrap_or_else(|| TextStyle::Body.resolve(style)),
                strikethrough: Stroke::NONE,
                underline: Stroke::NONE,
            },
        }
    }
}

impl StyleProvider<ButtonStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> ButtonStyle {
        let StyleArgs {
            ctx,
            classes,
            style,
            state,
            ..
        } = modifiers;
        let spacing = &style.spacing;
        let mut widget_visuals = match state {
            WidgetState::Noninteractive => style.visuals.widgets.noninteractive,
            WidgetState::Inactive => style.visuals.widgets.inactive,
            WidgetState::Hovered => style.visuals.widgets.hovered,
            WidgetState::Active => style.visuals.widgets.active,
        };

        let mut ws: BaseStyle = ctx.get_widget_style(modifiers);

        if classes.has(SELECTED_CLASS) {
            let visuals = &style.visuals;
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

impl StyleProvider<CheckboxStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> CheckboxStyle {
        let StyleArgs {
            ctx, style, state, ..
        } = modifiers;
        let spacing = &style.spacing;
        let widget_visuals = match state {
            WidgetState::Noninteractive => style.visuals.widgets.noninteractive,
            WidgetState::Inactive => style.visuals.widgets.inactive,
            WidgetState::Hovered => style.visuals.widgets.hovered,
            WidgetState::Active => style.visuals.widgets.active,
        };

        let ws: BaseStyle = ctx.get_widget_style(modifiers);

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

impl StyleProvider<LabelStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> LabelStyle {
        let StyleArgs { ctx, .. } = modifiers;
        let ws: BaseStyle = ctx.get_widget_style(modifiers);

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

impl StyleProvider<SeparatorStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> SeparatorStyle {
        let StyleArgs { ctx, .. } = modifiers;
        let ws: BaseStyle = ctx.get_widget_style(modifiers);

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

        self.get_widget_style::<S>(&StyleArgs {
            classes,
            state,
            style: self.style(),
            stack: self.stack(),
            ctx: self,
        })
    }
}

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
                .get_temp::<Arc<Mutex<Box<dyn StyleProvider<S> + Send + Sync>>>>(Id::NULL)
                .is_some_and(|t| t.lock().theme_type_id() == theme.theme_type_id())
        {
            return;
        }

        self.themes
            .insert_temp::<Arc<Mutex<Box<dyn StyleProvider<S> + Send + Sync>>>>(
                Id::NULL,
                Arc::new(Mutex::new(Box::new(theme))),
            );
    }

    /// Fetch the style of the current theme
    pub fn get<S: WidgetStyle + 'static>(
        &self,
    ) -> Arc<Mutex<Box<dyn StyleProvider<S> + Send + Sync>>> {
        let v = self
            .themes
            .get_temp::<Arc<Mutex<Box<dyn StyleProvider<S> + Send + Sync>>>>(Id::NULL);

        v.unwrap_or_else(|| panic!("A style should be set for {:?}", std::any::type_name::<S>()))
    }
}
