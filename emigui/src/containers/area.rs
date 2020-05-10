//! Area is a `Ui` that has no parent, it floats on the background.
//! It has no frame or own size. It is potentioally movable.
//! It is the foundation for windows and popups.

use std::{fmt::Debug, hash::Hash, sync::Arc};

use crate::*;

#[derive(Clone, Copy, Debug, serde_derive::Deserialize, serde_derive::Serialize)]
pub(crate) struct State {
    /// Last known pos
    pub pos: Pos2,

    /// Last know size. Used for catching clicks.
    pub size: Vec2,

    /// If false, clicks goes stright throught to what is behind us.
    /// Good for tooltips etc.
    pub interactable: bool,

    /// You can throw a moveable Area. It's fun.
    /// TODO: separate out moveable to container?
    #[serde(skip)]
    pub vel: Vec2,
}

#[derive(Clone, Copy, Debug)]
pub struct Area {
    id: Id,
    movable: bool,
    interactable: bool,
    order: Order,
    default_pos: Option<Pos2>,
    fixed_pos: Option<Pos2>,
}

impl Area {
    pub fn new(id_source: impl Hash) -> Self {
        Self {
            id: Id::new(id_source),
            movable: true,
            interactable: true,
            order: Order::Middle,
            default_pos: None,
            fixed_pos: None,
        }
    }

    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self.interactable |= movable;
        self
    }

    /// If false, clicks goes stright throught to what is behind us.
    /// Good for tooltips etc.
    pub fn interactable(mut self, interactable: bool) -> Self {
        self.interactable = interactable;
        self.movable &= interactable;
        self
    }

    /// `order(Order::Foreground)` for an Area that should always be on top
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    pub fn default_pos(mut self, default_pos: Pos2) -> Self {
        self.default_pos = Some(default_pos);
        self
    }

    pub fn fixed_pos(mut self, fixed_pos: Pos2) -> Self {
        self.default_pos = Some(fixed_pos);
        self.fixed_pos = Some(fixed_pos);
        self.movable = false;
        self
    }
}

impl Area {
    // TODO
    // pub fn show(self, ui: &Ui, add_contents: impl FnOnce(&mut Ui)) {
    //     let default_pos = self.default_pos.unwrap_or_else(|| ui.top_left() + pos2(100.0, 100.0)); // TODO
    // }

    pub fn show(self, ctx: &Arc<Context>, add_contents: impl FnOnce(&mut Ui)) -> InteractInfo {
        let Area {
            id,
            movable,
            order,
            interactable,
            default_pos,
            fixed_pos,
        } = self;

        let default_pos = default_pos.unwrap_or_else(|| pos2(100.0, 100.0)); // TODO
        let id = ctx.register_unique_id(id, "Area", default_pos);
        let layer = Layer { order, id };

        let (mut state, _is_new) = match ctx.memory().get_area(id) {
            Some(state) => (state, false),
            None => {
                let state = State {
                    pos: default_pos,
                    size: Vec2::zero(),
                    interactable,
                    vel: Vec2::zero(),
                };
                (state, true)
            }
        };
        state.pos = fixed_pos.unwrap_or(state.pos);
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
        let clip_rect = Rect::everything(); // TODO: get from context

        let interact_id = if movable { Some(id.with("move")) } else { None };
        let move_interact = ctx.interact(layer, clip_rect, rect, interact_id);

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
        //     &format!("Area size: {:?}", state.size),
        // );

        if move_interact.active || mouse_pressed_on_area(ctx, layer) {
            ctx.memory().move_area_to_top(layer);
        }
        ctx.memory().set_area_state(layer, state);

        move_interact
    }
}

fn mouse_pressed_on_area(ctx: &Context, layer: Layer) -> bool {
    if let Some(mouse_pos) = ctx.input().mouse_pos {
        ctx.input().mouse_pressed && ctx.memory().layer_at(mouse_pos) == Some(layer)
    } else {
        false
    }
}
