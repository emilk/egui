use std::sync::Arc;

use crate::{mesher::Path, widgets::*, *};

#[derive(Clone, Copy, Debug)]
pub struct WindowState {
    /// Last known pos/size
    pub rect: Rect,
}

#[derive(Clone, Debug)]
pub struct Window {
    /// The title of the window and by default the source of its identity.
    title: String,
    /// Put the window here the first time
    default_pos: Option<Pos2>,

    resizeable: bool,

    // If true, won't allow you to make window so big that it creates spacing
    shrink_to_fit_content: bool,

    // If true, won't allow you to resize smaller than that everything fits.
    expand_to_fit_content: bool,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: "".to_owned(),
            default_pos: None,
            resizeable: true,
            shrink_to_fit_content: false, // Normally you want this when resizable = false
            expand_to_fit_content: true,
        }
    }
}

impl Window {
    pub fn new<S: Into<String>>(title: S) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    pub fn default_pos(mut self, default_pos: Pos2) -> Self {
        self.default_pos = Some(default_pos);
        self
    }

    pub fn resizeable(mut self, resizeable: bool) -> Self {
        self.resizeable = resizeable;
        self
    }

    pub fn shrink_to_fit_content(mut self, shrink_to_fit_content: bool) -> Self {
        self.shrink_to_fit_content = shrink_to_fit_content;
        self
    }

    pub fn expand_to_fit_content(mut self, expand_to_fit_content: bool) -> Self {
        self.expand_to_fit_content = expand_to_fit_content;
        self
    }

    pub fn show<F>(self, ctx: &Arc<Context>, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let default_pos = self.default_pos.unwrap_or(pos2(100.0, 100.0)); // TODO
        let default_size = vec2(200.0, 50.0); // TODO
        let default_rect = Rect::from_min_size(default_pos, default_size);

        let id = ctx.make_unique_id(&self.title, default_pos);

        let (mut state, is_new_window) = match ctx.memory.lock().get_window(id) {
            Some(state) => (state, false),
            None => {
                let state = WindowState { rect: default_rect };
                (state, true)
            }
        };

        let layer = Layer::Window(id);
        let where_to_put_background = ctx.graphics.lock().layer(layer).len();

        let style = ctx.style();
        let window_padding = style.window_padding;

        let inner_rect = Rect::from_min_size(
            state.rect.min() + window_padding,
            state.rect.size() - 2.0 * window_padding,
        );
        let mut contents_region = Region::new(ctx.clone(), layer, id, inner_rect);

        // Show top bar:
        contents_region.add(Label::new(self.title).text_style(TextStyle::Heading));
        contents_region.add(Separator::new().line_width(1.0).extra(window_padding.x)); // TODO: nicer way to split window title from contents

        add_contents(&mut contents_region);

        // Now insert window background:

        // TODO: handle the last item_spacing in a nicer way
        let inner_size = contents_region.bounding_size - style.item_spacing;
        let inner_size = inner_size.ceil(); // Avoid rounding errors in math
        let desired_outer_size = inner_size + 2.0 * window_padding;
        let mut new_outer_size = state.rect.size();

        if self.shrink_to_fit_content {
            new_outer_size = new_outer_size.min(desired_outer_size);
        }

        if self.expand_to_fit_content || is_new_window {
            new_outer_size = new_outer_size.max(desired_outer_size);
        }

        state.rect = Rect::from_min_size(state.rect.min(), new_outer_size);

        let mut graphics = ctx.graphics.lock();

        let corner_radius = style.window.corner_radius;
        graphics.layer(layer).insert(
            where_to_put_background,
            (
                Rect::everything(),
                PaintCmd::Rect {
                    corner_radius,
                    fill_color: Some(style.background_fill_color()),
                    outline: Some(Outline::new(1.0, color::WHITE)),
                    rect: state.rect,
                },
            ),
        );

        let corner_interact = if self.resizeable {
            // Resize-corner:
            let mut path = Path::default();
            let quadrant = 0.0; // Bottom-right
            let corner_center = state.rect.max() - Vec2::splat(corner_radius);
            let corner_rect = Rect::from_min_size(corner_center, Vec2::splat(corner_radius));

            let corner_interact = ctx.interact(layer, corner_rect, Some(id.with(&"corner")));

            // TODO: Path::circle_sector() or something
            path.add_point(corner_center, vec2(0.0, -1.0));
            path.add_point(corner_center + vec2(corner_radius, 0.0), vec2(0.0, -1.0));
            path.add_circle_quadrant(corner_center, corner_radius, quadrant);
            path.add_point(corner_center + vec2(0.0, corner_radius), vec2(-1.0, 0.0));
            path.add_point(corner_center, vec2(-1.0, 0.0));
            graphics.layer(layer).insert(
                where_to_put_background + 1,
                (
                    Rect::everything(),
                    PaintCmd::Path {
                        path,
                        closed: true,
                        fill_color: style.interact_fill_color(&corner_interact),
                        outline: style.interact_outline(&corner_interact),
                    },
                ),
            );
            corner_interact
        } else {
            InteractInfo::default()
        };

        let win_interact = ctx.interact(layer, state.rect, Some(id.with(&"window")));

        if corner_interact.active {
            if let Some(mouse_pos) = ctx.input().mouse_pos {
                let new_size = mouse_pos - state.rect.min() + 0.5 * corner_interact.rect.size();
                let new_size = new_size.max(Vec2::splat(0.0));
                state.rect = Rect::from_min_size(state.rect.min(), new_size);
            }
        } else if win_interact.active {
            state.rect = state.rect.translate(ctx.input().mouse_move);
        }

        if corner_interact.hovered || corner_interact.active {
            *ctx.cursor_icon.lock() = CursorIcon::ResizeNorthWestSouthEast;
        }

        let mut memory = ctx.memory.lock();
        if win_interact.active || corner_interact.active {
            memory.move_window_to_top(id);
        }
        memory.set_window_state(id, state);
    }
}
