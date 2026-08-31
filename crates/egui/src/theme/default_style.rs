use emath::Vec2;
use epaint::Margin;

use crate::{
    Button, Context, Frame, TextStyle,
    theme::StyleProvider,
    widget_style::{
        ButtonStyle, CheckboxStyle, HasClasses as _, SeparatorStyle, StyleArgs, TextVisuals,
        WidgetState,
    },
};

/// The default [`StyleProvider`], implementing the default egui look based on
/// [`crate::style::WidgetVisuals`].
#[derive(Debug, Clone)]
pub struct DefaultStyle;

impl DefaultStyle {
    /// Register `Self` as the [`StyleProvider`] of every built-in widget style.
    ///
    /// [`Context::default`] does this. Any theme you register yourself
    /// replaces the default one for that widget style.
    pub fn register(ctx: &Context) {
        ctx.add_widget_theme::<ButtonStyle>(Self);
        ctx.add_widget_theme::<SeparatorStyle>(Self);
        ctx.add_widget_theme::<CheckboxStyle>(Self);
    }
}

impl StyleProvider<ButtonStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> ButtonStyle {
        let StyleArgs {
            classes,
            style,
            state,
            ..
        } = modifiers;
        let spacing = &style.spacing;
        let mut widget_visuals = *style.visuals.widgets.state(*state);

        if classes.has_class(&Button::CLASS_SELECTED) {
            let visuals = &style.visuals;
            widget_visuals.weak_bg_fill = visuals.selection.bg_fill;
            widget_visuals.bg_fill = visuals.selection.bg_fill;
            widget_visuals.fg_stroke = visuals.selection.stroke;
        }

        let mut inner_margin: Margin = spacing.button_padding.into();

        // A small button as high as regular text
        if classes.has_class(&Button::CLASS_SMALL) {
            inner_margin.top = 0;
            inner_margin.bottom = 0;
        }

        let painted_frame = Frame {
            fill: widget_visuals.weak_bg_fill,
            stroke: widget_visuals.bg_stroke,
            corner_radius: widget_visuals.corner_radius,
            inner_margin,
            ..Default::default()
        }
        // Ensure changing expansion and stroke don't affect layout:
        .expand_in_place(widget_visuals.expansion);

        let has_frame = classes.has_class(&Button::CLASS_FRAME)
            || (!classes.has_class(&Button::CLASS_NO_FRAME) && style.visuals.button_frame);

        let frame = if !has_frame {
            // No frame at all: the button takes up no more room than its contents.
            Frame::new()
        } else if classes.has_class(&Button::CLASS_HIDE_FRAME_WHEN_INACTIVE)
            && *state == WidgetState::Inactive
        {
            // Hide the frame, but keep its spacing
            painted_frame.invisible()
        } else {
            painted_frame
        };

        ButtonStyle {
            min_size: if classes.has_class(&Button::CLASS_SMALL) {
                Vec2::ZERO
            } else {
                Vec2::new(0.0, spacing.interact_size.y)
            },
            frame,
            text_style: TextVisuals::from_widget_visuals(style, TextStyle::Body, &widget_visuals),
        }
    }
}

impl StyleProvider<CheckboxStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> CheckboxStyle {
        let StyleArgs { style, state, .. } = modifiers;
        let spacing = &style.spacing;
        let widget_visuals = *style.visuals.widgets.state(*state);

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
            text_style: TextVisuals::from_widget_visuals(style, TextStyle::Body, &widget_visuals),
            check_stroke: widget_visuals.fg_stroke,
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
