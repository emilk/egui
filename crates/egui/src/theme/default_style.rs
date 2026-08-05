use emath::Vec2;
use epaint::{Shadow, Stroke, text::TextWrapMode};

use crate::{
    Frame, TextStyle,
    theme::StyleProvider,
    widget_style::{
        BaseStyle, ButtonStyle, CheckboxStyle, HasClasses as _, LabelStyle, SELECTED_CLASS,
        SeparatorStyle, StyleArgs, TextVisuals, WidgetState,
    },
};

#[derive(Debug, Clone)]
pub(super) struct DefaultStyle;

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
        let StyleArgs { style, .. } = modifiers;

        SeparatorStyle {
            spacing: 6.0,
            // A separator is never interactive, so its stroke doesn't depend on the widget state:
            stroke: style.visuals.widgets.noninteractive.bg_stroke,
        }
    }
}
