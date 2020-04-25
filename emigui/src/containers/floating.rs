//! A Floating is a region that has no parent, it floats on the background.
//! It is potentioally movable.
//! It has no frame or own size.
//! It is the foundation for a window

use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct State {
    /// Last known pos
    pub pos: Pos2,

    /// Last know size. Used for catching clicks.
    pub size: Vec2,
}

// TODO: rename Floating to something else.
#[derive(Clone, Copy, Debug)]
pub struct Floating {
    id: Id,
    movable: bool,
    default_pos: Option<Pos2>,
}

impl Floating {
    pub fn new(id_source: impl Hash) -> Self {
        Self {
            id: Id::new(id_source),
            movable: true,
            default_pos: None,
        }
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }

    pub fn default_pos(mut self, default_pos: Pos2) -> Self {
        self.default_pos = Some(default_pos);
        self
    }
}

impl Floating {
    pub fn show(self, ctx: &Arc<Context>, add_contents: impl FnOnce(&mut Region)) {
        let default_pos = self.default_pos.unwrap_or_else(|| pos2(100.0, 100.0)); // TODO
        let id = ctx.register_unique_id(self.id, "Floating", default_pos);
        let layer = Layer::Window(id);

        let (mut state, _is_new) = match ctx.memory.lock().get_floating(id) {
            Some(state) => (state, false),
            None => {
                let state = State {
                    pos: default_pos,
                    size: Vec2::zero(),
                };
                (state, true)
            }
        };
        state.pos = state.pos.round();

        let mut region = Region::new(
            ctx.clone(),
            layer,
            id,
            Rect::from_min_size(state.pos, Vec2::infinity()),
        );
        add_contents(&mut region);
        state.size = region.bounding_size().ceil();

        let rect = Rect::from_min_size(state.pos, state.size);
        let move_interact = ctx.interact(layer, &rect, Some(id.with("move")));

        if move_interact.active {
            state.pos += ctx.input().mouse_move;
        }

        // Constrain to screen:
        let margin = 32.0;
        state.pos = state.pos.max(pos2(margin - state.size.x, 0.0));
        state.pos = state.pos.min(pos2(
            ctx.input.screen_size.x - margin,
            ctx.input.screen_size.y - margin,
        ));

        state.pos = state.pos.round();

        if move_interact.active || mouse_pressed_on_floating(ctx, id) {
            ctx.memory.lock().move_floating_to_top(id);
        }
        ctx.memory.lock().set_floating_state(id, state);
    }
}

fn mouse_pressed_on_floating(ctx: &Context, id: Id) -> bool {
    if let Some(mouse_pos) = ctx.input.mouse_pos {
        ctx.input.mouse_pressed && ctx.memory.lock().layer_at(mouse_pos) == Layer::Window(id)
    } else {
        false
    }
}
