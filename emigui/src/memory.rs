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
    area_order: Vec<Layer>,
    area_visible_last_frame: HashSet<Layer>,
    area_visible_current_frame: HashSet<Layer>,
}

impl Memory {
    pub(crate) fn get_area(&mut self, id: Id) -> Option<area::State> {
        self.area.get(&id).cloned()
    }

    pub(crate) fn area_order(&self) -> &[Layer] {
        &self.area_order
    }

    pub(crate) fn set_area_state(&mut self, layer: Layer, state: area::State) {
        self.area_visible_current_frame.insert(layer);
        let did_insert = self.area.insert(layer.id, state).is_none();
        if did_insert {
            self.area_order.push(layer);
            self.area_order.sort_by_key(|layer| layer.order);
        }
    }

    /// TODO: call once at the start of the frame for the current mouse pos
    pub fn layer_at(&self, pos: Pos2) -> Option<Layer> {
        for layer in self.area_order.iter().rev() {
            if self.is_area_visible(layer) {
                if let Some(state) = self.area.get(&layer.id) {
                    if state.interactable {
                        let rect = Rect::from_min_size(state.pos, state.size);
                        if rect.contains(pos) {
                            return Some(*layer);
                        }
                    }
                }
            }
        }
        None
    }

    pub fn is_area_visible(&self, layer: &Layer) -> bool {
        self.area_visible_last_frame.contains(layer)
            || self.area_visible_current_frame.contains(layer)
    }

    pub fn move_area_to_top(&mut self, layer: Layer) {
        self.area_visible_current_frame.insert(layer);

        if self.area_order.last() == Some(&layer) {
            return; // common case early-out
        }
        if let Some(index) = self.area_order.iter().position(|x| *x == layer) {
            self.area_order.remove(index);
        }
        self.area_order.push(layer);

        self.area_order.sort_by_key(|layer| layer.order);
    }

    pub(crate) fn begin_frame(&mut self) {
        self.area_visible_last_frame = std::mem::take(&mut self.area_visible_current_frame);
    }
}
