use std::collections::HashMap;

use crate::{window::WindowState, *};

// TODO: move together with foldable code into own file
#[derive(Clone, Copy, Debug)]
pub struct FoldableState {
    pub open: bool,
    pub toggle_time: f64,
}

impl Default for FoldableState {
    fn default() -> Self {
        Self {
            open: false,
            toggle_time: -std::f64::INFINITY,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Memory {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    pub(crate) active_id: Option<Id>,

    /// Which foldable regions are open.
    pub(crate) foldables: HashMap<Id, FoldableState>,

    windows: HashMap<Id, WindowState>,

    /// Top is last
    pub window_order: Vec<Id>,
}

impl Memory {
    pub fn get_window(&mut self, id: Id) -> Option<WindowState> {
        self.windows.get(&id).cloned()
    }

    pub fn set_window_state(&mut self, id: Id, state: WindowState) {
        let did_insert = self.windows.insert(id, state).is_none();
        if did_insert {
            self.window_order.push(id);
        }
    }

    /// TODO: call once at the start of the frame for the current mouse pos
    pub fn layer_at(&self, pos: Pos2) -> Layer {
        for window_id in self.window_order.iter().rev() {
            if let Some(state) = self.windows.get(window_id) {
                if state.outer_rect.contains(pos) {
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
