use crate::pass_state::PerWidgetTooltipState;
use crate::{
    AreaState, Context, Id, InnerResponse, LayerId, Layout, Order, Popup, PopupAnchor, PopupKind,
    Response, Sense,
};
use emath::Vec2;

pub struct Tooltip<'a> {
    pub popup: Popup<'a>,

    /// The layer of the parent widget.
    parent_layer: LayerId,

    /// The id of the widget that owns this tooltip.
    parent_widget: Id,
}

impl Tooltip<'_> {
    /// Show a tooltip that is always open.
    #[deprecated = "Use `Tooltip::always_open` instead."]
    pub fn new(
        parent_widget: Id,
        ctx: Context,
        anchor: impl Into<PopupAnchor>,
        parent_layer: LayerId,
    ) -> Self {
        Self {
            popup: Popup::new(parent_widget, ctx, anchor.into(), parent_layer)
                .kind(PopupKind::Tooltip)
                .gap(4.0)
                .sense(Sense::hover()),
            parent_layer,
            parent_widget,
        }
    }

    /// Show a tooltip that is always open.
    pub fn always_open(
        ctx: Context,
        parent_layer: LayerId,
        parent_widget: Id,
        anchor: impl Into<PopupAnchor>,
    ) -> Self {
        let width = ctx.global_style().spacing.tooltip_width;
        Self {
            popup: Popup::new(parent_widget, ctx, anchor.into(), parent_layer)
                .kind(PopupKind::Tooltip)
                .gap(4.0)
                .width(width)
                .sense(Sense::hover()),
            parent_layer,
            parent_widget,
        }
    }

    /// Show a tooltip for a widget. Always open (as long as this function is called).
    pub fn for_widget(response: &Response) -> Self {
        let popup = Popup::from_response(response)
            .kind(PopupKind::Tooltip)
            .gap(4.0)
            .width(response.ctx.global_style().spacing.tooltip_width)
            .sense(Sense::hover());
        Self {
            popup,
            parent_layer: response.layer_id,
            parent_widget: response.id,
        }
    }

    /// Show a tooltip when hovering an enabled widget.
    pub fn for_enabled(response: &Response) -> Self {
        let mut tooltip = Self::for_widget(response);
        tooltip.popup = tooltip
            .popup
            .open(response.enabled() && Self::should_show_tooltip(response, true));
        tooltip
    }

    /// Show a tooltip when hovering a disabled widget.
    pub fn for_disabled(response: &Response) -> Self {
        let mut tooltip = Self::for_widget(response);
        tooltip.popup = tooltip
            .popup
            .open(!response.enabled() && Self::should_show_tooltip(response, true));
        tooltip
    }

    /// Show the tooltip at the pointer position.
    #[inline]
    pub fn at_pointer(mut self) -> Self {
        self.popup = self.popup.at_pointer();
        self
    }

    /// Set the gap between the tooltip and the anchor
    ///
    /// Default: 5.0
    #[inline]
    pub fn gap(mut self, gap: f32) -> Self {
        self.popup = self.popup.gap(gap);
        self
    }

    /// Set the layout of the tooltip
    #[inline]
    pub fn layout(mut self, layout: Layout) -> Self {
        self.popup = self.popup.layout(layout);
        self
    }

    /// Set the width of the tooltip
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.popup = self.popup.width(width);
        self
    }

    /// Show the tooltip
    pub fn show<R>(self, content: impl FnOnce(&mut crate::Ui) -> R) -> Option<InnerResponse<R>> {
        let Self {
            mut popup,
            parent_layer,
            parent_widget,
        } = self;

        if !popup.is_open() {
            return None;
        }

        let rect = popup.get_anchor_rect()?;

        let mut state = popup.ctx().pass_state_mut(|fs| {
            // Remember that this is the widget showing the tooltip:
            fs.layers
                .entry(parent_layer)
                .or_default()
                .widget_with_tooltip = Some(parent_widget);

            fs.tooltips
                .widget_tooltips
                .get(&parent_widget)
                .copied()
                .unwrap_or(PerWidgetTooltipState {
                    bounding_rect: rect,
                    tooltip_count: 0,
                })
        });

        let tooltip_area_id = Self::tooltip_id(parent_widget, state.tooltip_count);
        popup = popup.anchor(state.bounding_rect).id(tooltip_area_id);

        let response = popup.show(|ui| {
            // By default, the text in tooltips aren't selectable.
            // This means that most tooltips aren't interactable,
            // which also mean they won't stick around so you can click them.
            // Only tooltips that have actual interactive stuff (buttons, links, …)
            // will stick around when you try to click them.
            ui.style_mut().interaction.selectable_labels = false;

            content(ui)
        });

        // The popup might not be shown on at_pointer if there is no pointer.
        if let Some(response) = &response {
            state.tooltip_count += 1;
            state.bounding_rect |= response.response.rect;
            response
                .response
                .ctx
                .pass_state_mut(|fs| fs.tooltips.widget_tooltips.insert(parent_widget, state));
            Self::remember_that_tooltip_was_shown(&response.response.ctx);
        }

        response
    }

    fn when_was_a_toolip_last_shown_id() -> Id {
        Id::new("when_was_a_toolip_last_shown")
    }

    pub fn seconds_since_last_tooltip(ctx: &Context) -> f32 {
        let when_was_a_toolip_last_shown =
            ctx.data(|d| d.get_temp::<f64>(Self::when_was_a_toolip_last_shown_id()));

        if let Some(when_was_a_toolip_last_shown) = when_was_a_toolip_last_shown {
            let now = ctx.input(|i| i.time);
            (now - when_was_a_toolip_last_shown) as f32
        } else {
            f32::INFINITY
        }
    }

    fn remember_that_tooltip_was_shown(ctx: &Context) {
        let now = ctx.input(|i| i.time);
        ctx.data_mut(|data| data.insert_temp::<f64>(Self::when_was_a_toolip_last_shown_id(), now));
    }

    /// What is the id of the next tooltip for this widget?
    pub fn next_tooltip_id(ctx: &Context, widget_id: Id) -> Id {
        let tooltip_count = ctx.pass_state(|fs| {
            fs.tooltips
                .widget_tooltips
                .get(&widget_id)
                .map_or(0, |state| state.tooltip_count)
        });
        Self::tooltip_id(widget_id, tooltip_count)
    }

    pub fn tooltip_id(widget_id: Id, tooltip_count: usize) -> Id {
        widget_id.with(tooltip_count)
    }

    /// Should we show a tooltip for this response?
    ///
    /// Argument `allow_interactive_tooltip` controls whether mouse can interact with tooltip that
    /// contains interactive widgets
    pub fn should_show_tooltip(response: &Response, allow_interactive_tooltip: bool) -> bool {
        if response.ctx.memory(|mem| mem.everything_is_visible()) {
            return true;
        }

        let any_open_popups = response.ctx.prev_pass_state(|fs| {
            fs.layers
                .get(&response.layer_id)
                .is_some_and(|layer| !layer.open_popups.is_empty())
        });
        if any_open_popups {
            // Hide tooltips if the user opens a popup (menu, combo-box, etc.) in the same layer.
            return false;
        }

        let style = response.ctx.global_style();

        let tooltip_delay = style.interaction.tooltip_delay;
        let tooltip_grace_time = style.interaction.tooltip_grace_time;

        let (
            time_since_last_scroll,
            time_since_last_click,
            time_since_last_pointer_movement,
            pointer_pos,
            pointer_dir,
        ) = response.ctx.input(|i| {
            (
                i.time_since_last_scroll(),
                i.pointer.time_since_last_click(),
                i.pointer.time_since_last_movement(),
                i.pointer.hover_pos(),
                i.pointer.direction(),
            )
        });

        if time_since_last_scroll < tooltip_delay {
            // See https://github.com/emilk/egui/issues/4781
            // Note that this means we cannot have `ScrollArea`s in a tooltip.
            response
                .ctx
                .request_repaint_after_secs(tooltip_delay - time_since_last_scroll);
            return false;
        }

        let is_our_tooltip_open = response.is_tooltip_open();

        if is_our_tooltip_open {
            // Check if we should automatically stay open:

            let tooltip_id = Self::next_tooltip_id(&response.ctx, response.id);
            let tooltip_layer_id = LayerId::new(Order::Tooltip, tooltip_id);

            let tooltip_has_interactive_widget = allow_interactive_tooltip
                && response.ctx.viewport(|vp| {
                    vp.prev_pass
                        .widgets
                        .get_layer(tooltip_layer_id)
                        .any(|w| w.enabled && w.sense.interactive())
                });

            if tooltip_has_interactive_widget {
                // We keep the tooltip open if hovered,
                // or if the pointer is on its way to it,
                // so that the user can interact with the tooltip
                // (i.e. click links that are in it).
                if let Some(area) = AreaState::load(&response.ctx, tooltip_id) {
                    let rect = area.rect();

                    if let Some(pos) = pointer_pos {
                        if rect.contains(pos) {
                            return true; // hovering interactive tooltip
                        }
                        if pointer_dir != Vec2::ZERO
                            && rect.intersects_ray(pos, pointer_dir.normalized())
                        {
                            return true; // on the way to interactive tooltip
                        }
                    }
                }
            }
        }

        let clicked_more_recently_than_moved =
            time_since_last_click < time_since_last_pointer_movement + 0.1;
        if clicked_more_recently_than_moved {
            // It is common to click a widget and then rest the mouse there.
            // It would be annoying to then see a tooltip for it immediately.
            // Similarly, clicking should hide the existing tooltip.
            // Only hovering should lead to a tooltip, not clicking.
            // The offset is only to allow small movement just right after the click.
            return false;
        }

        if is_our_tooltip_open {
            // Check if we should automatically stay open:

            if pointer_pos.is_some_and(|pointer_pos| response.rect.contains(pointer_pos)) {
                // Handle the case of a big tooltip that covers the widget:
                return true;
            }
        }

        let is_other_tooltip_open = response.ctx.prev_pass_state(|fs| {
            if let Some(already_open_tooltip) = fs
                .layers
                .get(&response.layer_id)
                .and_then(|layer| layer.widget_with_tooltip)
            {
                already_open_tooltip != response.id
            } else {
                false
            }
        });
        if is_other_tooltip_open {
            // We only allow one tooltip per layer. First one wins. It is up to that tooltip to close itself.
            return false;
        }

        // Fast early-outs:
        if response.enabled() {
            if !response.hovered() || !response.ctx.input(|i| i.pointer.has_pointer()) {
                return false;
            }
        } else if !response
            .ctx
            .rect_contains_pointer(response.layer_id, response.rect)
        {
            return false;
        }

        // There is a tooltip_delay before showing the first tooltip,
        // but once one tooltip is show, moving the mouse cursor to
        // another widget should show the tooltip for that widget right away.

        // Let the user quickly move over some dead space to hover the next thing
        let tooltip_was_recently_shown =
            Self::seconds_since_last_tooltip(&response.ctx) < tooltip_grace_time;

        if !tooltip_was_recently_shown && !is_our_tooltip_open {
            if style.interaction.show_tooltips_only_when_still {
                // We only show the tooltip when the mouse pointer is still.
                if !response
                    .ctx
                    .input(|i| i.pointer.is_still() && !i.is_scrolling())
                {
                    // wait for mouse to stop
                    response.ctx.request_repaint();
                    return false;
                }
            }

            let time_since_last_interaction = time_since_last_scroll
                .min(time_since_last_pointer_movement)
                .min(time_since_last_click);
            let time_til_tooltip = tooltip_delay - time_since_last_interaction;

            if 0.0 < time_til_tooltip {
                // Wait until the mouse has been still for a while
                response.ctx.request_repaint_after_secs(time_til_tooltip);
                return false;
            }
        }

        // We don't want tooltips of things while we are dragging them,
        // but we do want tooltips while holding down on an item on a touch screen.
        if response
            .ctx
            .input(|i| i.pointer.any_down() && i.pointer.has_moved_too_much_for_a_click)
        {
            return false;
        }

        // All checks passed: show the tooltip!

        true
    }

    /// Was this tooltip visible last frame?
    pub fn was_tooltip_open_last_frame(ctx: &Context, widget_id: Id) -> bool {
        let primary_tooltip_area_id = Self::tooltip_id(widget_id, 0);
        ctx.memory(|mem| {
            mem.areas()
                .visible_last_frame(&LayerId::new(Order::Tooltip, primary_tooltip_area_id))
        })
    }
}
