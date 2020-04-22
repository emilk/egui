use std::sync::Arc;

use crate::{mesher::Path, widgets::*, *};

#[derive(Clone, Copy, Debug)]
pub struct State {
    /// Last known pos
    pub outer_pos: Pos2,
    pub inner_size: Vec2,

    /// used for catching clicks:
    pub outer_rect: Rect,
}

// TODO: separate out resizing into a contained and reusable Resize-region.
#[derive(Clone, Debug)]
pub struct Window {
    /// The title of the window and by default the source of its identity.
    title: String,
    /// Put the window here the first time
    default_pos: Option<Pos2>,

    /// Size of the window first time
    default_size: Option<Vec2>,

    resizeable: bool,

    // If true, won't allow you to make window so big that it creates spacing
    shrink_to_fit_content: bool,

    // If true, won't allow you to resize smaller than that everything fits.
    expand_to_fit_content: bool,

    min_size: Vec2,
    max_size: Option<Vec2>,
}

impl Default for Window {
    fn default() -> Self {
        Self {
            title: "".to_owned(),
            default_pos: None,
            default_size: None,
            resizeable: true,
            shrink_to_fit_content: false, // Normally you want this when resizable = false
            expand_to_fit_content: true,
            min_size: Vec2::splat(16.0),
            max_size: None,
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

    pub fn default_size(mut self, default_size: Vec2) -> Self {
        self.default_size = Some(default_size);
        self
    }

    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    pub fn max_size(mut self, max_size: Vec2) -> Self {
        self.max_size = Some(max_size);
        self
    }

    pub fn fixed_size(mut self, size: Vec2) -> Self {
        self.shrink_to_fit_content = false;
        self.shrink_to_fit_content = false;
        self.expand_to_fit_content = false;
        self.default_size = Some(size);
        self.min_size = size;
        self.max_size = Some(size);
        self
    }

    /// Can you resize it with the mouse?
    /// Note that a window can still auto-resize
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
}

impl Window {
    pub fn show<F>(self, ctx: &Arc<Context>, add_contents: F)
    where
        F: FnOnce(&mut Region),
    {
        let style = ctx.style();
        let window_padding = style.window_padding;

        let min_inner_size = self.min_size;
        let max_inner_size = self
            .max_size
            .unwrap_or(ctx.input.screen_size - 2.0 * window_padding);

        let default_pos = self.default_pos.unwrap_or(pos2(100.0, 100.0)); // TODO
        let default_inner_size = self.default_size.unwrap_or(vec2(250.0, 250.0));

        let id = ctx.make_unique_id(&self.title, default_pos);

        let (mut state, is_new_window) = match ctx.memory.lock().get_window(id) {
            Some(state) => (state, false),
            None => {
                let state = State {
                    outer_pos: default_pos,
                    inner_size: default_inner_size,
                    outer_rect: Rect::from_min_size(
                        default_pos,
                        default_inner_size + 2.0 * window_padding,
                    ),
                };
                (state, true)
            }
        };

        let layer = Layer::Window(id);
        let where_to_put_background = ctx.graphics.lock().layer(layer).len();

        let inner_rect = Rect::from_min_size(state.outer_pos + window_padding, state.inner_size);
        let mut contents_region = Region::new(ctx.clone(), layer, id, inner_rect);
        // TODO: handle contents_region.clip_rect while resizing

        // Show top bar:
        contents_region.add(Label::new(self.title).text_style(TextStyle::Heading));
        contents_region.add(Separator::new().line_width(1.0).extra(window_padding.x)); // TODO: nicer way to split window title from contents

        add_contents(&mut contents_region);

        // TODO: handle the last item_spacing in a nicer way
        let desired_inner_size = contents_region.bounding_size - style.item_spacing;
        let desired_inner_size = desired_inner_size.ceil(); // Avoid rounding errors in math

        let mut new_inner_size = state.inner_size;
        if self.shrink_to_fit_content {
            new_inner_size = new_inner_size.min(desired_inner_size);
        }
        if self.expand_to_fit_content || is_new_window {
            new_inner_size = new_inner_size.max(desired_inner_size);
        }
        new_inner_size = new_inner_size.max(min_inner_size);
        new_inner_size = new_inner_size.min(max_inner_size);

        let new_outer_size = new_inner_size + 2.0 * window_padding;

        let outer_rect = Rect::from_min_size(state.outer_pos, new_outer_size);

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
                    rect: outer_rect,
                },
            ),
        );

        let corner_interact = if self.resizeable {
            // Resize-corner:
            let corner_center = outer_rect.max() - Vec2::splat(corner_radius);
            let corner_rect = Rect::from_min_size(corner_center, Vec2::splat(corner_radius));

            let corner_interact = ctx.interact(layer, corner_rect, Some(id.with("corner")));

            graphics.layer(layer).push((
                Rect::everything(),
                paint_resize_corner(corner_center, corner_radius, &style, &corner_interact),
            ));
            corner_interact
        } else {
            InteractInfo::default()
        };

        let win_interact = ctx.interact(layer, outer_rect, Some(id.with("window")));

        if corner_interact.active {
            if let Some(mouse_pos) = ctx.input().mouse_pos {
                let new_outer_size =
                    mouse_pos - state.outer_pos + 0.5 * corner_interact.rect.size();
                new_inner_size = new_outer_size - 2.0 * window_padding;
                new_inner_size = new_inner_size.max(min_inner_size);
                new_inner_size = new_inner_size.min(max_inner_size);
            }
        } else if win_interact.active {
            state.outer_pos += ctx.input().mouse_move;
        }

        state = State {
            outer_pos: state.outer_pos,
            inner_size: new_inner_size,
            outer_rect: outer_rect,
        };

        if corner_interact.hovered || corner_interact.active {
            *ctx.cursor_icon.lock() = CursorIcon::ResizeNorthWestSouthEast;
        }

        if win_interact.active || corner_interact.active || mouse_pressed_on_window(ctx, id) {
            ctx.memory.lock().move_window_to_top(id);
        }
        ctx.memory.lock().set_window_state(id, state);
    }
}

fn mouse_pressed_on_window(ctx: &Context, id: Id) -> bool {
    if let Some(mouse_pos) = ctx.input.mouse_pos {
        ctx.input.mouse_pressed && ctx.memory.lock().layer_at(mouse_pos) == Layer::Window(id)
    } else {
        false
    }
}

fn paint_resize_corner(
    center: Pos2,
    radius: f32,
    style: &Style,
    interact: &InteractInfo,
) -> PaintCmd {
    // TODO: Path::circle_sector() or something
    let quadrant = 0.0; // Bottom-right
    let mut path = Path::default();
    path.add_point(center, vec2(0.0, -1.0));
    path.add_point(center + vec2(radius, 0.0), vec2(0.0, -1.0));
    path.add_circle_quadrant(center, radius, quadrant);
    path.add_point(center + vec2(0.0, radius), vec2(-1.0, 0.0));
    path.add_point(center, vec2(-1.0, 0.0));
    PaintCmd::Path {
        path,
        closed: true,
        fill_color: style.interact_fill_color(&interact),
        outline: style.interact_outline(&interact),
    }
}
