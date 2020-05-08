//! A Floating is an Ui that has no parent, it floats on the background.
//! It is potentioally movable.
//! It has no frame or own size.
//! It is the foundation for a window

use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::*;

#[derive(Clone, Copy, Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub(crate) struct State {
    /// Last known pos
    pub pos: Pos2,

    /// Last know size. Used for catching clicks.
    pub size: Vec2,

    /// You can throw a Floating thing. It's fun.
    #[serde(skip)]
    pub vel: Vec2,
}

// TODO: rename Floating to something else. Area?
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
    pub fn show(self, ctx: &Arc<Context>, add_contents: impl FnOnce(&mut Ui)) {
        let default_pos = self.default_pos.unwrap_or_else(|| pos2(100.0, 100.0)); // TODO
        let id = ctx.register_unique_id(self.id, "Floating", default_pos);
        let layer = Layer::Window(id);

        let (mut state, _is_new) = match ctx.memory().get_floating(id) {
            Some(state) => (state, false),
            None => {
                let state = State {
                    pos: default_pos,
                    size: Vec2::zero(),
                    vel: Vec2::zero(),
                };
                (state, true)
            }
        };
        state.pos = state.pos.round();

        let mut ui = Ui::new(
            ctx.clone(),
            layer,
            id,
            Rect::from_min_size(state.pos, Vec2::infinity()),
        );
        add_contents(&mut ui);
        state.size = ui.bounding_size().ceil();

        let rect = Rect::from_min_size(state.pos, state.size);
        let clip_rect = Rect::everything();
        let move_interact = ctx.interact(layer, clip_rect, rect, Some(id.with("move")));

        let input = ctx.input();
        if move_interact.active {
            state.pos += input.mouse_move;
            state.vel = input.mouse_velocity;
        } else {
            let stop_speed = 20.0; // Pixels per second.
            let friction_coeff = 1000.0; // Pixels per second squared.

            let friction = friction_coeff * input.dt;
            if friction > state.vel.length() || state.vel.length() < stop_speed {
                state.vel = Vec2::zero();
            } else {
                state.vel -= friction * state.vel.normalized();
                state.pos += state.vel * input.dt;
            }
        }

        // Constrain to screen:
        let margin = 32.0;
        state.pos = state.pos.max(pos2(margin - state.size.x, 0.0));
        state.pos = state.pos.min(pos2(
            ctx.input().screen_size.x - margin,
            ctx.input().screen_size.y - margin,
        ));

        state.pos = state.pos.round();

        // ctx.debug_rect(
        //     Rect::from_min_size(state.pos, state.size),
        //     &format!("Floating size: {:?}", state.size),
        // );

        if move_interact.active || mouse_pressed_on_floating(ctx, id) {
            ctx.memory().move_floating_to_top(id);
        }
        ctx.memory().set_floating_state(id, state);
    }
}

fn mouse_pressed_on_floating(ctx: &Context, id: Id) -> bool {
    if let Some(mouse_pos) = ctx.input().mouse_pos {
        ctx.input().mouse_pressed && ctx.memory().layer_at(mouse_pos) == Layer::Window(id)
    } else {
        false
    }
}
