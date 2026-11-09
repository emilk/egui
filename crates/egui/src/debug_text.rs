//! This is an example of how to create a plugin for egui.
//!
//! A plugin is a struct that implements the [`Plugin`] trait and holds some state.
//! The plugin is registered with the [`Context`] using [`Context::add_plugin`]
//! to get callbacks on certain events ([`Plugin::on_begin_pass`], [`Plugin::on_end_pass`]).

use crate::{
    Align, Align2, Color32, Context, FontFamily, FontId, Plugin, Rect, Shape, Ui, Vec2, WidgetText,
    text, vec2,
};

/// Print this text next to the cursor at the end of the pass.
///
/// If you call this multiple times, the text will be appended.
///
/// This only works if compiled with `debug_assertions`.
///
/// ```
/// # let ctx = &egui::Context::default();
/// # let state = true;
/// egui::debug_text::print(ctx, format!("State: {state:?}"));
/// ```
#[track_caller]
pub fn print(ctx: &Context, text: impl Into<WidgetText>) {
    if !cfg!(debug_assertions) {
        return;
    }

    let location = core::panic::Location::caller();
    let location = format!("{}:{}", location.file(), location.line());

    let plugin = ctx.plugin::<DebugTextPlugin>();
    let mut state = plugin.lock();
    state.entries.push(Entry {
        location,
        text: text.into(),
    });
}

#[derive(Clone)]
struct Entry {
    location: String,
    text: WidgetText,
}

/// A plugin for easily showing debug-text on-screen.
///
/// This is a built-in plugin in egui, automatically registered during [`Context`] creation.
#[derive(Clone, Default)]
pub struct DebugTextPlugin {
    // This gets re-filled every pass.
    entries: Vec<Entry>,
}

impl Plugin for DebugTextPlugin {
    fn debug_name(&self) -> &'static str {
        "DebugTextPlugin"
    }

    fn on_end_pass(&mut self, ui: &mut Ui) {
        let entries = core::mem::take(&mut self.entries);
        Self::paint_entries(ui, entries);
    }
}

impl DebugTextPlugin {
    fn paint_entries(ctx: &Context, entries: Vec<Entry>) {
        if entries.is_empty() {
            return;
        }

        // Show debug-text next to the cursor.
        let mut pos = ctx
            .input(|i| i.pointer.latest_pos())
            .unwrap_or_else(|| ctx.content_rect().center())
            + 8.0 * Vec2::Y;

        let painter = ctx.debug_painter();
        let where_to_put_background = painter.add(Shape::Noop);

        let mut bounding_rect = Rect::from_points(&[pos]);

        let color = Color32::GRAY;
        let font_id = FontId::new(10.0, FontFamily::Proportional);

        // The room the two sides of the pointer offer, floored at 0.0 so a pointer at
        // or beyond an edge can never hand a negative width to `wrap_at_width`.
        let content_rect = ctx.content_rect();
        let room_right = (content_rect.max.x - pos.x).max(0.0);
        let room_left = (pos.x - 4.0 - content_rect.min.x).max(0.0);

        for Entry { location, text } in entries {
            // Probe the entry text's natural (unwrapped) width: the same layout as the
            // painted one below, just with unlimited wrapping, so the gate cannot
            // mis-measure.
            let probe = text.clone().into_galley_impl(
                ctx,
                &ctx.global_style(),
                text::TextWrapping::wrap_at_width(f32::INFINITY),
                font_id.clone().into(),
                Align::TOP,
                ctx.language(),
            );
            // Flip sides only when the entry does not fit the room right of the pointer
            // AND the left side has strictly more room. An overflow-only test alone
            // would also flip a left-half pointer's very long entry into the narrow
            // left strip, even though its old right-side wrap was readable rather than
            // clipped — so the flip must also be strictly better. Whenever the old
            // output was neither clipped nor squashed, the sides stay as they were.
            let flip = probe.size().x > room_right && room_left > room_right;

            if flip {
                // Entry text hangs leftward from the pointer; the 4.0 px gap between
                // pointer and text comes off the wrap width, so the galley's right
                // edge cannot overhang the content rect's left edge.
                let galley = text.into_galley_impl(
                    ctx,
                    &ctx.global_style(),
                    text::TextWrapping::wrap_at_width(room_left),
                    font_id.clone().into(),
                    Align::TOP,
                    ctx.language(),
                );
                let rect = Align2::RIGHT_TOP.anchor_size(pos - 4.0 * Vec2::X, galley.size());
                painter.galley(rect.min - vec2(galley.rect.left(), 0.0), galley, color);
                bounding_rect |= rect;

                // The location is metadata: near the edge one of the two elements must
                // give, so the entry (the primary content, kept to a single line) gets
                // the room and the location wraps inside the strip that remains right
                // of the pointer. The bounded wrap width keeps a long `file:line` from
                // growing rightward off the screen.
                let location_galley = ctx.fonts_mut(|f| {
                    f.layout(
                        location,
                        font_id.clone(),
                        color,
                        (content_rect.max.x - pos.x - 4.0).max(0.0),
                    )
                });
                let location_rect =
                    Align2::LEFT_TOP.anchor_size(pos + 4.0 * Vec2::X, location_galley.size());
                painter.galley(
                    location_rect.min - vec2(location_galley.rect.left(), 0.0),
                    location_galley,
                    color,
                );
                bounding_rect |= location_rect;
            } else {
                {
                    // Paint location to left of `pos`:
                    let location_galley = ctx
                        .fonts_mut(|f| f.layout(location, font_id.clone(), color, f32::INFINITY));
                    let location_rect =
                        Align2::RIGHT_TOP.anchor_size(pos - 4.0 * Vec2::X, location_galley.size());
                    painter.galley(
                        location_rect.min - vec2(location_galley.rect.left(), 0.0),
                        location_galley,
                        color,
                    );
                    bounding_rect |= location_rect;
                }

                {
                    // Paint `text` to right of `pos`:
                    let galley = text.into_galley_impl(
                        ctx,
                        &ctx.global_style(),
                        text::TextWrapping::wrap_at_width(room_right),
                        font_id.clone().into(),
                        Align::TOP,
                        ctx.language(),
                    );
                    let rect = Align2::LEFT_TOP.anchor_size(pos, galley.size());
                    painter.galley(rect.min - vec2(galley.rect.left(), 0.0), galley, color);
                    bounding_rect |= rect;
                }
            }

            pos.y = bounding_rect.max.y + 4.0;
        }

        painter.set(
            where_to_put_background,
            Shape::rect_filled(
                bounding_rect.expand(4.0),
                2.0,
                Color32::from_black_alpha(192),
            ),
        );
    }
}
