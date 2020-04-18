use std::collections::{HashMap, HashSet};

use crate::{window::WindowState, *};

#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    pub(crate) active_id: Option<Id>,

    /// Which foldable regions are open.
    pub(crate) open_foldables: HashSet<Id>,

    windows: HashMap<Id, WindowState>,

    /// Top is last
    pub window_order: Vec<Id>,
}

impl Memory {
    /// default_rect: where to put it if it does NOT exist
    pub fn get_or_create_window(&mut self, id: Id, default_rect: Rect) -> WindowState {
        if let Some(state) = self.windows.get(&id) {
            *state
        } else {
            let state = WindowState { rect: default_rect };
            self.windows.insert(id, state);
            self.window_order.push(id);
            state
        }
    }

    pub fn set_window_state(&mut self, id: Id, state: WindowState) {
        self.windows.insert(id, state);
    }

    pub fn layer_at(&self, pos: Pos2) -> Layer {
        for window_id in self.window_order.iter().rev() {
            if let Some(state) = self.windows.get(window_id) {
                if state.rect.contains(pos) {
                    return Layer::Window(*window_id);
                }
            }
        }
        Layer::Background
    }

    pub fn move_window_to_top(&mut self, id: Id) {
        if self.window_order.last() == Some(&id) {
            return; // common case early-out
        }
        if let Some(index) = self.window_order.iter().position(|x| *x == id) {
            self.window_order.remove(index);
        }
        self.window_order.push(id);
    }
}
