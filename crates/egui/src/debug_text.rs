//! This is an example of how to create a plugin for egui.
//!
//! A plugin is a struct that implements the [`Plugin`] trait and holds some state.
//! The plugin is registered with the [`Context`] using [`Context::add_plugin`]
//! to get callbacks on certain events ([`Plugin::on_begin_pass`], [`Plugin::on_end_pass`]).

use crate::{
    Align, Align2, Color32, Context, FontFamily, FontId, Plugin, Rect, Shape, Ui, Vec2, WidgetText,
    text,
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

    let location = std::panic::Location::caller();
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
        let entries = std::mem::take(&mut self.entries);
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

        for Entry { location, text } in entries {
            {
                // Paint location to left of `pos`:
                let location_galley =
                    ctx.fonts_mut(|f| f.layout(location, font_id.clone(), color, f32::INFINITY));
                let location_rect =
                    Align2::RIGHT_TOP.anchor_size(pos - 4.0 * Vec2::X, location_galley.size());
                painter.galley(location_rect.min, location_galley, color);
                bounding_rect |= location_rect;
            }

            {
                // Paint `text` to right of `pos`:
                let available_width = ctx.content_rect().max.x - pos.x;
                let galley = text.into_galley_impl(
                    ctx,
                    &ctx.global_style(),
                    text::TextWrapping::wrap_at_width(available_width),
                    font_id.clone().into(),
                    Align::TOP,
                );
                let rect = Align2::LEFT_TOP.anchor_size(pos, galley.size());
                painter.galley(rect.min, galley, color);
                bounding_rect |= rect;
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
