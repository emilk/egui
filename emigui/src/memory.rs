use std::collections::{HashMap, HashSet};

use crate::{
    containers::{area, collapsing_header, menu, resize, scroll_area},
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
    pub(crate) menu_bar: HashMap<Id, menu::BarState>,
    pub(crate) scroll_areas: HashMap<Id, scroll_area::State>,
    pub(crate) resize: HashMap<Id, resize::State>,

    pub(crate) areas: Areas,
}

#[derive(Clone, Debug, Default, serde_derive::Deserialize, serde_derive::Serialize)]
#[serde(default)]
pub struct Areas {
    areas: HashMap<Id, area::State>,
    /// Top is last
    order: Vec<Layer>,
    visible_last_frame: HashSet<Layer>,
    visible_current_frame: HashSet<Layer>,
}

impl Memory {
    pub(crate) fn begin_frame(&mut self) {
        self.areas.begin_frame()
    }

    /// TODO: call once at the start of the frame for the current mouse pos
    pub fn layer_at(&self, pos: Pos2) -> Option<Layer> {
        self.areas.layer_at(pos)
    }
}

impl Areas {
    pub(crate) fn count(&self) -> usize {
        self.areas.len()
    }

    pub(crate) fn get(&mut self, id: Id) -> Option<area::State> {
        self.areas.get(&id).cloned()
    }

    pub(crate) fn order(&self) -> &[Layer] {
        &self.order
    }

    pub(crate) fn set_state(&mut self, layer: Layer, state: area::State) {
        self.visible_current_frame.insert(layer);
        let did_insert = self.areas.insert(layer.id, state).is_none();
        if did_insert {
            self.order.push(layer);
            self.order.sort_by_key(|layer| layer.order);
        }
    }

    /// TODO: call once at the start of the frame for the current mouse pos
    pub fn layer_at(&self, pos: Pos2) -> Option<Layer> {
        for layer in self.order.iter().rev() {
            if self.is_visible(layer) {
                if let Some(state) = self.areas.get(&layer.id) {
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

    pub fn is_visible(&self, layer: &Layer) -> bool {
        self.visible_last_frame.contains(layer) || self.visible_current_frame.contains(layer)
    }

    pub fn move_to_top(&mut self, layer: Layer) {
        self.visible_current_frame.insert(layer);

        if self.order.last() == Some(&layer) {
            return; // common case early-out
        }
        if let Some(index) = self.order.iter().position(|x| *x == layer) {
            self.order.remove(index);
        }
        self.order.push(layer);

        self.order.sort_by_key(|layer| layer.order);
    }

    pub(crate) fn begin_frame(&mut self) {
        self.visible_last_frame = std::mem::take(&mut self.visible_current_frame);
    }
}
