use std::collections::{HashMap, HashSet};

use crate::{
    area,
    cache::Cache,
    collapsing_header, menu,
    paint::color::{Hsva, Srgba},
    resize, scroll_area,
    widgets::text_edit,
    window, Id, LayerId, Pos2, Rect,
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
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) menu_bar: HashMap<Id, menu::BarState>,
    pub(crate) resize: HashMap<Id, resize::State>,
    pub(crate) scroll_areas: HashMap<Id, scroll_area::State>,
    pub(crate) text_edit: HashMap<Id, text_edit::State>,

    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) window_interaction: Option<window::WindowInteraction>,

    /// For temporary edit of e.g. a slider value.
    /// Couples with `kb_focus_id`.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) temp_edit_string: Option<String>,

    pub(crate) areas: Areas,

    /// Used by color picker
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) color_cache: Cache<Srgba, Hsva>,

    /// Which popup-window is open (if any)?
    /// Could be a combo box, color picker, menu etc.
    #[cfg_attr(feature = "serde", serde(skip))]
    popup: Option<Id>,

    /// Useful for debugging, benchmarking etc.
    pub all_collpasing_are_open: bool,
    /// Useful for debugging, benchmarking etc.
    pub all_menues_are_open: bool,
    /// Useful for debugging, benchmarking etc.
    pub all_windows_are_open: bool,
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

impl Memory {
    pub(crate) fn begin_frame(&mut self, prev_input: &crate::input::InputState) {
        self.interaction.begin_frame(prev_input);

        if !prev_input.mouse.down {
            self.window_interaction = None;
        }
    }

    pub(crate) fn end_frame(&mut self) {
        self.areas.end_frame();
    }

    pub fn layer_id_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<LayerId> {
        self.areas.layer_id_at(pos, resize_interact_radius_side)
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

    /// Stop editing of active `TextEdit` (if any).
    pub fn stop_text_input(&mut self) {
        self.interaction.kb_focus_id = None;
    }

    pub fn is_anything_being_dragged(&self) -> bool {
        self.interaction.drag_id.is_some()
    }

    pub fn is_being_dragged(&self, id: Id) -> bool {
        self.interaction.drag_id == Some(id)
    }

    /// Forget window positions, sizes etc.
    /// Can be used to auto-layout windows.
    pub fn reset_areas(&mut self) {
        self.areas = Default::default();
    }
}

/// ## Popups
/// Popups are things like combo-boxes, color pickers, menus etc.
/// Only one can be be open at a time.
impl Memory {
    pub fn is_popup_open(&mut self, popup_id: Id) -> bool {
        self.popup == Some(popup_id)
    }

    pub fn open_popup(&mut self, popup_id: Id) {
        self.popup = Some(popup_id);
    }

    pub fn close_popup(&mut self) {
        self.popup = None;
    }

    pub fn toggle_popup(&mut self, popup_id: Id) {
        if self.is_popup_open(popup_id) {
            self.close_popup();
        } else {
            self.open_popup(popup_id);
        }
    }
}

// ----------------------------------------------------------------------------

/// Keeps track of `Area`s, which are free-floating `Ui`s.
/// These `Area`s can be in any `Order`.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Areas {
    areas: HashMap<Id, area::State>,
    /// Top is last
    order: Vec<LayerId>,
    visible_last_frame: HashSet<LayerId>,
    visible_current_frame: HashSet<LayerId>,

    /// When an area want to be on top, it is put in here.
    /// At the end of the frame, this is used to reorder the layers.
    /// This means if several layers want to be on top, they will keep their relative order.
    /// So if you close three windows and then reopen them all in one frame,
    /// they will all be sent to the top, but keep their previous internal order.
    wants_to_be_on_top: HashSet<LayerId>,
}

impl Areas {
    pub(crate) fn count(&self) -> usize {
        self.areas.len()
    }

    pub(crate) fn get(&self, id: Id) -> Option<&area::State> {
        self.areas.get(&id)
    }

    pub(crate) fn order(&self) -> &[LayerId] {
        &self.order
    }

    pub(crate) fn set_state(&mut self, layer_id: LayerId, state: area::State) {
        self.visible_current_frame.insert(layer_id);
        self.areas.insert(layer_id.id, state);
        if self.order.iter().find(|x| **x == layer_id).is_none() {
            self.order.push(layer_id);
        }
    }

    pub fn layer_id_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<LayerId> {
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

    pub fn visible_last_frame(&self, layer_id: &LayerId) -> bool {
        self.visible_last_frame.contains(layer_id)
    }

    pub fn is_visible(&self, layer_id: &LayerId) -> bool {
        self.visible_last_frame.contains(layer_id) || self.visible_current_frame.contains(layer_id)
    }

    pub fn visible_layer_ids(&self) -> HashSet<LayerId> {
        self.visible_last_frame
            .iter()
            .cloned()
            .chain(self.visible_current_frame.iter().cloned())
            .collect()
    }

    pub(crate) fn visible_windows(&self) -> Vec<&area::State> {
        self.visible_layer_ids()
            .iter()
            .filter(|layer| layer.order == crate::layers::Order::Middle)
            .filter_map(|layer| self.get(layer.id))
            .collect()
    }

    pub fn move_to_top(&mut self, layer_id: LayerId) {
        self.visible_current_frame.insert(layer_id);
        self.wants_to_be_on_top.insert(layer_id);

        if self.order.iter().find(|x| **x == layer_id).is_none() {
            self.order.push(layer_id);
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
