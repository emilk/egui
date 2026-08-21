use emath::Vec2;
use epaint::{Color32, Margin};

use crate::{
    Button, Context, Frame, TextEdit, TextStyle,
    class::HasClasses as _,
    theme::StyleProvider,
    widget_style::{
        AtomLayoutStyle, ButtonStyle, CheckboxStyle, PopupStyle, SeparatorStyle, StyleArgs,
        TextEditStyle, TextVisuals, WidgetState,
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
        ctx.add_widget_theme::<TextEditStyle>(Self);
        ctx.add_widget_theme::<PopupStyle>(Self);
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
            corner_radius: widget_visuals.corner_radius,
            inner_margin,
            ..Default::default()
        }
        // Ensure changing expansion and stroke don't affect layout:
        .apply_stroke_and_expansion_without_layout_shift(
            widget_visuals.bg_stroke,
            widget_visuals.expansion,
        );

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

        let text_style = TextVisuals::from_widget_visuals(style, TextStyle::Body, &widget_visuals);
        let image_tint = if classes.has_class(&Button::CLASS_IMAGE_TINT_FOLLOWS_TEXT_COLOR) {
            text_style.color
        } else {
            Color32::WHITE
        };

        ButtonStyle {
            atom_layout: AtomLayoutStyle {
                min_size: if classes.has_class(&Button::CLASS_SMALL) {
                    Vec2::ZERO
                } else {
                    Vec2::new(0.0, spacing.interact_size.y)
                },
                gap: spacing.icon_spacing,
                frame,
                text_style,
                image_tint,
                ..Default::default()
            },
        }
    }
}

impl StyleProvider<PopupStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> PopupStyle {
        let StyleArgs { style, .. } = modifiers;

        PopupStyle {
            frame: Frame::popup(style),
            item_spacing: style.spacing.item_spacing,
        }
    }
}

impl StyleProvider<TextEditStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> TextEditStyle {
        let StyleArgs {
            classes,
            style,
            state,
            ..
        } = modifiers;

        let widget_visuals = style.visuals.widgets.state(*state);

        // A text edit over an immutable buffer is painted without a background.
        let read_only = classes.has_class(&TextEdit::CLASS_READ_ONLY);

        let fill = if read_only {
            Color32::TRANSPARENT
        } else {
            style.visuals.text_edit_bg_color()
        };

        let stroke = if read_only {
            style.visuals.widgets.inactive.bg_stroke
        } else if *state == WidgetState::Active {
            // While focused, the frame is outlined in the selection color.
            style.visuals.selection.stroke
        } else {
            widget_visuals.bg_stroke
        };

        // The text of a text edit doesn't brighten on hover — that would be distracting while
        // typing — so it keeps the inactive color no matter the state.
        let text = TextVisuals::from_widget_visuals(
            style,
            TextStyle::Body,
            &style.visuals.widgets.inactive,
        );

        TextEditStyle {
            atom_layout: AtomLayoutStyle {
                frame: Frame {
                    fill,
                    corner_radius: widget_visuals.corner_radius,
                    inner_margin: Margin::symmetric(4, 2),
                    ..Default::default()
                }
                .apply_stroke_and_expansion_without_layout_shift(stroke, widget_visuals.expansion),
                gap: style.spacing.icon_spacing,
                text_style: text,
                ..Default::default()
            },
            hint_text_color: style.visuals.weak_text_color(),
            prefix_suffix_color: style.visuals.text_color(),
        }
    }
}

impl StyleProvider<CheckboxStyle> for DefaultStyle {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> CheckboxStyle {
        let StyleArgs { style, state, .. } = modifiers;
        let spacing = &style.spacing;
        let widget_visuals = *style.visuals.widgets.state(*state);

        CheckboxStyle {
            atom_layout: AtomLayoutStyle {
                min_size: Vec2::splat(spacing.interact_size.y),
                gap: spacing.icon_spacing,
                frame: Frame::new(),
                text_style: TextVisuals::from_widget_visuals(
                    style,
                    TextStyle::Body,
                    &widget_visuals,
                ),
                ..Default::default()
            },
            checkbox_size: spacing.icon_width,
            check_size: spacing.icon_width_inner,
            checkbox_frame: Frame {
                fill: widget_visuals.bg_fill,
                corner_radius: widget_visuals.corner_radius,
                stroke: widget_visuals.bg_stroke,
                ..Default::default()
            },
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
