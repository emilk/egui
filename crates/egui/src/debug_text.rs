//! This is an example of how to create a plugin for egui.
//!
//! A plugin usually consist of a struct that holds some state,
//! which is stored using [`Context::data_mut`].
//! The plugin registers itself onto a specific [`Context`]
//! to get callbacks on certain events ([`Context::on_begin_frame`], [`Context::on_end_frame`]).

use crate::*;

/// Register this plugin on the given egui context,
/// so that it will be called every frame.
///
/// This is a built-in plugin in egui,
/// meaning [`Context`] calls this from its `Default` implementation,
/// so this i marked as `pub(crate)`.
pub(crate) fn register(ctx: &Context) {
    ctx.on_end_frame("debug_text", std::sync::Arc::new(State::end_frame));
}

/// Print this text next to the cursor at the end of the frame.
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
    ctx.data_mut(|data| {
        // We use `Id::NULL` as the id, since we only have one instance of this plugin.
        // We use the `temp` version instead of `persisted` since we don't want to
        // persist state on disk when the egui app is closed.
        let state = data.get_temp_mut_or_default::<State>(Id::NULL);
        state.entries.push(Entry {
            location,
            text: text.into(),
        });
    });
}

#[derive(Clone)]
struct Entry {
    location: String,
    text: WidgetText,
}

/// A plugin for easily showing debug-text on-screen.
///
/// This is a built-in plugin in egui.
#[derive(Clone, Default)]
struct State {
    // This gets re-filled every frame.
    entries: Vec<Entry>,
}

impl State {
    fn end_frame(ctx: &Context) {
        let state = ctx.data_mut(|data| data.remove_temp::<Self>(Id::NULL));
        if let Some(state) = state {
            state.paint(ctx);
        }
    }

    fn paint(self, ctx: &Context) {
        let Self { entries } = self;

        if entries.is_empty() {
            return;
        }

        // Show debug-text next to the cursor.
        let mut pos = ctx
            .input(|i| i.pointer.latest_pos())
            .unwrap_or_else(|| ctx.screen_rect().center())
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
                    ctx.fonts(|f| f.layout(location, font_id.clone(), color, f32::INFINITY));
                let location_rect =
                    Align2::RIGHT_TOP.anchor_size(pos - 4.0 * Vec2::X, location_galley.size());
                painter.galley(location_rect.min, location_galley, color);
                bounding_rect = bounding_rect.union(location_rect);
            }

            {
                // Paint `text` to right of `pos`:
                let wrap = true;
                let available_width = ctx.screen_rect().max.x - pos.x;
                let galley = text.into_galley_impl(
                    ctx,
                    &ctx.style(),
                    wrap,
                    available_width,
                    font_id.clone().into(),
                    Align::TOP,
                );
                let rect = Align2::LEFT_TOP.anchor_size(pos, galley.size());
                painter.galley(rect.min, galley, color);
                bounding_rect = bounding_rect.union(rect);
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
