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

    /// The area of the screen we are talking about
    pub rect: Rect,

    /// Used for optionally showing a tooltip
    pub ctx: Arc<Context>,
}

impl GuiResponse {
    /// Show some stuff if the item was hovered
    pub fn tooltip(&mut self, add_contents: impl FnOnce(&mut Ui)) -> &mut Self {
        if self.hovered {
            show_tooltip(&self.ctx, add_contents);
        }
        self
    }

    /// Show this text if the item was hovered
    pub fn tooltip_text(&mut self, text: impl Into<String>) -> &mut Self {
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

    /// Full width/height.
    /// Use this when you want
    Justified,
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

/// Give a position within the rect, specified by the aligns
pub fn align_rect(rect: Rect, align: (Align, Align)) -> Rect {
    let x = match align.0 {
        Align::Min | Align::Justified => rect.left(),
        Align::Center => rect.left() - 0.5 * rect.width(),
        Align::Max => rect.left() - rect.width(),
    };
    let y = match align.1 {
        Align::Min | Align::Justified => rect.top(),
        Align::Center => rect.top() - 0.5 * rect.height(),
        Align::Max => rect.top() - rect.height(),
    };
    Rect::from_min_size(pos2(x, y), rect.size())
}

// ----------------------------------------------------------------------------

pub fn show_tooltip(ctx: &Arc<Context>, add_contents: impl FnOnce(&mut Ui)) {
    if let Some(mouse_pos) = ctx.input().mouse_pos {
        //  TODO: default size
        let id = Id::tooltip();
        let window_pos = mouse_pos + vec2(16.0, 16.0);
        show_popup(ctx, id, window_pos, add_contents);
    }
}

/// Show a pop-over window
pub fn show_popup(
    ctx: &Arc<Context>,
    id: Id,
    window_pos: Pos2,
    add_contents: impl FnOnce(&mut Ui),
) -> InteractInfo {
    use containers::*;
    Area::new(id)
        .order(Order::Foreground)
        .fixed_pos(window_pos)
        .interactable(false)
        .show(ctx, |ui| Frame::popup(&ctx.style()).show(ui, add_contents))
}
