use std::collections::{HashMap, HashSet};

use crate::{
    containers::{area, collapsing_header, resize, scroll_area},
    Id, Layer, Pos2, Rect,
};

#[derive(Clone, Debug, Default, serde_derive::Deserialize, serde_derive::Serialize)]
#[serde(default)]
pub struct Memory {
    /// The widget being interacted with (e.g. dragged, in case of a slider).
    #[serde(skip)]
    pub(crate) active_id: Option<Id>,

    /// The widget with keyboard focus (i.e. a text input field).
    #[serde(skip)]
    pub(crate) kb_focus_id: Option<Id>,

    // states of various types of widgets
    pub(crate) collapsing_headers: HashMap<Id, collapsing_header::State>,
    pub(crate) scroll_areas: HashMap<Id, scroll_area::State>,
    pub(crate) resize: HashMap<Id, resize::State>,

    area: HashMap<Id, area::State>,
    /// Top is last
    area_order: Vec<Id>,
    area_visible_last_frame: HashSet<Id>,
    area_visible_current_frame: HashSet<Id>,
}

impl Memory {
    pub(crate) fn get_area(&mut self, id: Id) -> Option<area::State> {
        self.area.get(&id).cloned()
    }

    pub(crate) fn area_order(&self) -> &[Id] {
        &self.area_order
    }

    pub(crate) fn set_area_state(&mut self, id: Id, state: area::State) {
        self.area_visible_current_frame.insert(id);
        let did_insert = self.area.insert(id, state).is_none();
        if did_insert {
            self.area_order.push(id);
        }
    }

    /// TODO: call once at the start of the frame for the current mouse pos
    pub fn layer_at(&self, pos: Pos2) -> Layer {
        for area_id in self.area_order.iter().rev() {
            if self.area_visible_last_frame.contains(area_id)
                || self.area_visible_current_frame.contains(area_id)
            {
                if let Some(state) = self.area.get(area_id) {
                    let rect = Rect::from_min_size(state.pos, state.size);
                    if rect.contains(pos) {
                        return Layer::Window(*area_id);
                    }
                }
            }
        }
        Layer::Background
    }

    pub fn move_area_to_top(&mut self, id: Id) {
        if self.area_order.last() == Some(&id) {
            return; // common case early-out
        }
        if let Some(index) = self.area_order.iter().position(|x| *x == id) {
            self.area_order.remove(index);
        }
        self.area_order.push(id);
        self.area_visible_current_frame.insert(id);
    }

    pub(crate) fn begin_frame(&mut self) {
        self.area_visible_last_frame = std::mem::take(&mut self.area_visible_current_frame);
    }
}
