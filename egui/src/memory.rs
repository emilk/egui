use std::collections::{HashMap, HashSet};

use crate::{
    area, collapsing_header, menu, resize, scroll_area, widgets::text_edit, window, Id, Layer,
    Pos2, Rect,
};

/// The data that Egui persists between frames.
///
/// This includes window positions and sizes,
/// how far the user has scrolled in a `ScrollArea` etc.
///
/// If you want this to persist when closing your app you should serialize `Memory` and store it.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Memory {
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) interaction: Interaction,

    // states of various types of widgets
    pub(crate) collapsing_headers: HashMap<Id, collapsing_header::State>,
    pub(crate) menu_bar: HashMap<Id, menu::BarState>,
    pub(crate) resize: HashMap<Id, resize::State>,
    pub(crate) scroll_areas: HashMap<Id, scroll_area::State>,
    pub(crate) text_edit: HashMap<Id, text_edit::State>,

    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) window_interaction: Option<window::WindowInteraction>,

    pub(crate) areas: Areas,
}

/// Say there is a button in a scroll area.
/// If the user clicks the button, the button should click.
/// If the user drags the button we should scroll the scroll area.
/// So what we do is that when the mouse is pressed we register both the button
/// and the scroll area (as `click_id`/`drag_id`).
/// If the user releases the button without moving the mouse we register it as a click on `click_id`.
/// If the cursor moves too much we clear the `click_id` and start passing move events to `drag_id`.
#[derive(Clone, Debug, Default)]
pub struct Interaction {
    /// A widget interested in clicks that has a mouse press on it.
    pub click_id: Option<Id>,

    /// A widget interested in drags that has a mouse press on it.
    pub drag_id: Option<Id>,

    /// The widget with keyboard focus (i.e. a text input field).
    pub kb_focus_id: Option<Id>,

    /// HACK: windows have low priority on dragging.
    /// This is so that if you drag a slider in a window,
    /// the slider will steal the drag away from the window.
    /// This is needed because we do window interaction first (to prevent frame delay),
    /// and then do content layout.
    pub drag_is_window: bool,

    /// Any interest in catching clicks this frame?
    /// Cleared to false at start of each frame.
    pub click_interest: bool,

    /// Any interest in catching clicks this frame?
    /// Cleared to false at start of each frame.
    pub drag_interest: bool,
}

impl Interaction {
    pub fn is_using_mouse(&self) -> bool {
        self.click_id.is_some() || self.drag_id.is_some()
    }

    fn begin_frame(&mut self, prev_input: &crate::input::InputState) {
        self.click_interest = false;
        self.drag_interest = false;

        if !prev_input.mouse.could_be_click {
            self.click_id = None;
        }

        if !prev_input.mouse.down || prev_input.mouse.pos.is_none() {
            // mouse was not down last frame
            self.click_id = None;
            self.drag_id = None;
        }
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
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
    pub(crate) fn begin_frame(&mut self, prev_input: &crate::input::InputState) {
        self.interaction.begin_frame(prev_input);

        if !prev_input.mouse.down || prev_input.mouse.pos.is_none() {
            // mouse was not down last frame

            let window_interaction = self.window_interaction.take();
            if let Some(window_interaction) = window_interaction {
                if window_interaction.is_pure_move() {
                    // Throw windows because it is fun:
                    let area_layer = window_interaction.area_layer;
                    let area_state = self.areas.get(area_layer.id).cloned();
                    if let Some(mut area_state) = area_state {
                        area_state.vel = prev_input.mouse.velocity;
                        self.areas.set_state(area_layer, area_state);
                    }
                }
            }
        }
    }

    pub(crate) fn end_frame(&mut self) {
        self.areas.end_frame();
    }

    pub fn layer_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<Layer> {
        self.areas.layer_at(pos, resize_interact_radius_side)
    }

    pub fn has_kb_focus(&self, id: Id) -> bool {
        self.interaction.kb_focus_id == Some(id)
    }

    pub fn request_kb_focus(&mut self, id: Id) {
        self.interaction.kb_focus_id = Some(id);
    }

    pub fn surrender_kb_focus(&mut self, id: Id) {
        if self.interaction.kb_focus_id == Some(id) {
            self.interaction.kb_focus_id = None;
        }
    }
}

impl Areas {
    pub(crate) fn count(&self) -> usize {
        self.areas.len()
    }

    pub(crate) fn get(&self, id: Id) -> Option<&area::State> {
        self.areas.get(&id)
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

    pub fn layer_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<Layer> {
        for layer in self.order.iter().rev() {
            if self.is_visible(layer) {
                if let Some(state) = self.areas.get(&layer.id) {
                    if state.interactable {
                        let rect = Rect::from_min_size(state.pos, state.size);
                        // Allow us to resize by dragging just outside the window:
                        let rect = rect.expand(resize_interact_radius_side);
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

    pub fn visible_layers(&self) -> HashSet<Layer> {
        self.visible_last_frame
            .iter()
            .cloned()
            .chain(self.visible_current_frame.iter().cloned())
            .collect()
    }

    pub(crate) fn visible_windows(&self) -> Vec<&area::State> {
        self.visible_layers()
            .iter()
            .filter(|layer| layer.order == crate::layers::Order::Middle)
            .filter_map(|layer| self.get(layer.id))
            .collect()
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
