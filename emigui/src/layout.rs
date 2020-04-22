use std::sync::Arc;

use crate::{widgets::*, *};

// ----------------------------------------------------------------------------

// TODO: rename GuiResponse
pub struct GuiResponse {
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse clicked this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,

    /// Used for optionally showing a tooltip
    pub ctx: Arc<Context>,
}

impl GuiResponse {
    /// Show some stuff if the item was hovered
    pub fn tooltip<F>(&mut self, add_contents: F) -> &mut Self
    where
        F: FnOnce(&mut Region),
    {
        if self.hovered {
            if let Some(mouse_pos) = self.ctx.input().mouse_pos {
                let window_pos = mouse_pos + vec2(16.0, 16.0);
                show_popup(&self.ctx, window_pos, add_contents);
            }
        }
        self
    }

    /// Show this text if the item was hovered
    pub fn tooltip_text<S: Into<String>>(&mut self, text: S) -> &mut Self {
        self.tooltip(|popup| {
            popup.add(Label::new(text));
        })
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Vertical
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    /// Left/Top
    Min,

    /// Note: requires a bounded/known available_width.
    Center,

    /// Right/Bottom
    /// Note: requires a bounded/known available_width.
    Max,
    // TODO: Justified
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

pub fn align_rect(rect: Rect, align: (Align, Align)) -> Rect {
    let x = match align.0 {
        Align::Min => rect.left(),
        Align::Center => rect.left() - 0.5 * rect.size().x,
        Align::Max => rect.left() - rect.size().x,
    };
    let y = match align.1 {
        Align::Min => rect.top(),
        Align::Center => rect.top() - 0.5 * rect.size().y,
        Align::Max => rect.top() - rect.size().y,
    };
    Rect::from_min_size(pos2(x, y), rect.size())
}

// ----------------------------------------------------------------------------

// TODO: move show_popup, and expand its features (default size, autosize, etc)
/// Show a pop-over window
pub fn show_popup<F>(ctx: &Arc<Context>, window_pos: Pos2, add_contents: F)
where
    F: FnOnce(&mut Region),
{
    let layer = Layer::Popup;
    let where_to_put_background = ctx.graphics.lock().layer(layer).len();

    let style = ctx.style();
    let window_padding = style.window_padding;

    let size = vec2(ctx.input.screen_size.x.min(350.0), std::f32::INFINITY); // TODO: popup/tooltip width
    let inner_rect = Rect::from_min_size(window_pos + window_padding, size);
    let mut contents_region = Region::new(ctx.clone(), layer, Id::popup(), inner_rect);

    add_contents(&mut contents_region);

    // Now insert popup background:

    // TODO: handle the last item_spacing in a nicer way
    let inner_size = contents_region.bounding_size - style.item_spacing;
    let outer_size = inner_size + 2.0 * window_padding;

    let rect = Rect::from_min_size(window_pos, outer_size);

    let mut graphics = ctx.graphics.lock();
    graphics.layer(layer).insert(
        where_to_put_background,
        (
            Rect::everything(),
            PaintCmd::Rect {
                corner_radius: 5.0,
                fill_color: Some(style.background_fill_color()),
                outline: Some(Outline::new(1.0, color::WHITE)),
                rect,
            },
        ),
    );
}
