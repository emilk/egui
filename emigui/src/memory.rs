use std::collections::{HashMap, HashSet};

use crate::{
    containers::{area, collapsing_header, menu, resize, scroll_area, window},
    widgets::text_edit,
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
    pub(crate) resize: HashMap<Id, resize::State>,
    pub(crate) scroll_areas: HashMap<Id, scroll_area::State>,
    pub(crate) text_edit: HashMap<Id, text_edit::State>,

    #[serde(skip)]
    pub(crate) window_interaction: Option<window::WindowInteraction>,

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

    /// When an area want to be on top, it is put in here.
    /// At the end of the frame, this is used to reorder the layers.
    /// This means if several layers want to be on top, they will keep their relative order.
    /// So if you close three windows and then reopen them all in one frame,
    /// they will all be sent to the top, but keep their previous internal order.
    wants_to_be_on_top: HashSet<Layer>,
}

impl Memory {
    pub(crate) fn end_frame(&mut self) {
        self.areas.end_frame()
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

    pub fn visible_last_frame(&self, layer: &Layer) -> bool {
        self.visible_last_frame.contains(layer)
    }

    pub fn is_visible(&self, layer: &Layer) -> bool {
        self.visible_last_frame.contains(layer) || self.visible_current_frame.contains(layer)
    }

    pub fn move_to_top(&mut self, layer: Layer) {
        self.visible_current_frame.insert(layer);
        self.wants_to_be_on_top.insert(layer);

        if self.order.iter().find(|x| **x == layer).is_none() {
            self.order.push(layer);
        }
    }

    pub(crate) fn end_frame(&mut self) {
        let Self {
            visible_last_frame,
            visible_current_frame,
            order,
            wants_to_be_on_top,
            ..
        } = self;

        *visible_last_frame = std::mem::take(visible_current_frame);
        order.sort_by_key(|layer| (layer.order, wants_to_be_on_top.contains(layer)));
        wants_to_be_on_top.clear();
    }
}
