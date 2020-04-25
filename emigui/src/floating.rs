//! A Floating is a region that has no parent, it floats on the background.
//! It is potentioally movable.
//! It has no frame or own size.
//! It is the foundation for a window
//!
use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::*;

// struct State {
//     pos: Pos2,
//     /// used for catching clickes
//     size: Vec2,
// }

type State = crate::window::State;

#[derive(Clone, Copy, Debug)]
pub struct Floating {
    movable: bool,
    default_pos: Option<Pos2>,
}

impl Floating {
    pub fn new() -> Self {
        Self {
            movable: true,
            default_pos: None,
        }
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self
    }
}

impl Floating {
    pub fn show(
        self,
        ctx: &Arc<Context>,
        id_source: impl Copy + Debug + Hash,
        add_contents: impl FnOnce(&mut Region),
    ) {
        let default_pos = self.default_pos.unwrap_or_else(|| pos2(100.0, 100.0)); // TODO
        let id = ctx.make_unique_id(id_source, default_pos);
        let layer = Layer::Window(id);

        let (mut state, _is_new) = match ctx.memory.lock().get_window(id) {
            Some(state) => (state, false),
            None => {
                let state = State {
                    outer_pos: default_pos,
                    inner_size: Vec2::zero(),
                    outer_rect: Rect::from_min_size(default_pos, Vec2::zero()),
                };
                (state, true)
            }
        };
        state.outer_pos = state.outer_pos.round();

        let mut region = Region::new(
            ctx.clone(),
            layer,
            id,
            Rect::from_min_size(state.outer_pos, Vec2::infinity()),
        );
        add_contents(&mut region);
        let size = region.bounding_size().ceil();

        state.outer_rect = Rect::from_min_size(state.outer_pos, size);
        let move_interact = ctx.interact(layer, &state.outer_rect, Some(id.with("move")));

        if move_interact.active {
            state.outer_pos += ctx.input().mouse_move;
        }

        // Constrain to screen:
        let margin = 32.0;
        state.outer_pos = state.outer_pos.max(pos2(margin - size.x, 0.0));
        state.outer_pos = state.outer_pos.min(pos2(
            ctx.input.screen_size.x - margin,
            ctx.input.screen_size.y - margin,
        ));
        state.outer_pos = state.outer_pos.round();

        state.inner_size = size;
        state.outer_rect = Rect::from_min_size(state.outer_pos, size);

        if move_interact.active || mouse_pressed_on_window(ctx, id) {
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
