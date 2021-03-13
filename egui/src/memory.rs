use std::collections::{HashMap, HashSet};

use crate::{
    area, collapsing_header, menu, resize, scroll_area, util::Cache, widgets::text_edit, window,
    Id, InputState, LayerId, Pos2, Rect, Style,
};
use epaint::color::{Color32, Hsva};

// ----------------------------------------------------------------------------

/// The data that egui persists between frames.
///
/// This includes window positions and sizes,
/// how far the user has scrolled in a `ScrollArea` etc.
///
/// If you want this to persist when closing your app you should serialize `Memory` and store it.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Memory {
    pub(crate) options: Options,

    /// new scale that will be applied at the start of the next frame
    pub(crate) new_pixels_per_point: Option<f32>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) interaction: Interaction,

    // states of various types of widgets
    pub(crate) collapsing_headers: HashMap<Id, collapsing_header::State>,
    pub(crate) grid: HashMap<Id, crate::grid::State>,
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) menu_bar: HashMap<Id, menu::BarState>,
    pub(crate) resize: HashMap<Id, resize::State>,
    pub(crate) scroll_areas: HashMap<Id, scroll_area::State>,
    pub(crate) text_edit: HashMap<Id, text_edit::State>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) window_interaction: Option<window::WindowInteraction>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) drag_value: crate::widgets::drag_value::MonoState,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) tooltip: crate::containers::popup::MonoState,

    pub(crate) areas: Areas,

    /// Used by color picker
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) color_cache: Cache<Color32, Hsva>,

    /// Which popup-window is open (if any)?
    /// Could be a combo box, color picker, menu etc.
    #[cfg_attr(feature = "persistence", serde(skip))]
    popup: Option<Id>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    everything_is_visible: bool,
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub(crate) struct Options {
    /// The default style for new `Ui`:s.
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) style: std::sync::Arc<Style>,
    /// Controls the tessellator.
    pub(crate) tessellation_options: epaint::TessellationOptions,
    /// Font sizes etc.
    pub(crate) font_definitions: epaint::text::FontDefinitions,
}

// ----------------------------------------------------------------------------

/// Say there is a button in a scroll area.
/// If the user clicks the button, the button should click.
/// If the user drags the button we should scroll the scroll area.
/// So what we do is that when the mouse is pressed we register both the button
/// and the scroll area (as `click_id`/`drag_id`).
/// If the user releases the button without moving the mouse we register it as a click on `click_id`.
/// If the cursor moves too much we clear the `click_id` and start passing move events to `drag_id`.
#[derive(Clone, Debug, Default)]
pub(crate) struct Interaction {
    /// A widget interested in clicks that has a mouse press on it.
    pub click_id: Option<Id>,

    /// A widget interested in drags that has a mouse press on it.
    pub drag_id: Option<Id>,

    pub focus: Focus,

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

/// Keeps tracks of what widget has keyboard focus
#[derive(Clone, Debug, Default)]
pub(crate) struct Focus {
    /// The widget with keyboard focus (i.e. a text input field).
    id: Option<Id>,

    /// What had keyboard focus previous frame?
    id_previous_frame: Option<Id>,

    /// Give focus to this widget next frame
    id_next_frame: Option<Id>,

    /// If set, the next widget that is interested in focus will automatically get it.
    /// Probably because the user pressed Tab.
    give_to_next: bool,

    /// The last widget interested in focus.
    last_interested: Option<Id>,

    /// Set at the beginning of the frame, set to `false` when "used".
    pressed_tab: bool,
    /// Set at the beginning of the frame, set to `false` when "used".
    pressed_shift_tab: bool,
}

impl Interaction {
    /// Are we currently clicking or dragging an egui widget?
    pub fn is_using_pointer(&self) -> bool {
        self.click_id.is_some() || self.drag_id.is_some()
    }

    fn begin_frame(
        &mut self,
        prev_input: &crate::input_state::InputState,
        new_input: &crate::data::input::RawInput,
    ) {
        self.click_interest = false;
        self.drag_interest = false;

        if !prev_input.pointer.could_any_button_be_click() {
            self.click_id = None;
        }

        if !prev_input.pointer.any_down() || prev_input.pointer.latest_pos().is_none() {
            // pointer button was not down last frame
            self.click_id = None;
            self.drag_id = None;
        }

        self.focus.begin_frame(new_input);
    }
}

impl Focus {
    /// Which widget currently has keyboard focus?
    pub fn focused(&self) -> Option<Id> {
        self.id
    }

    fn begin_frame(&mut self, new_input: &crate::data::input::RawInput) {
        self.id_previous_frame = self.id;
        if let Some(id) = self.id_next_frame.take() {
            self.id = Some(id);
        }

        self.pressed_tab = false;
        self.pressed_shift_tab = false;
        for event in &new_input.events {
            if matches!(
                event,
                crate::Event::Key {
                    key: crate::Key::Escape,
                    pressed: true,
                    modifiers: _,
                }
            ) {
                self.id = None;
                break;
            }

            if let crate::Event::Key {
                key: crate::Key::Tab,
                pressed: true,
                modifiers,
            } = event
            {
                if modifiers.shift {
                    self.pressed_shift_tab = true;
                } else {
                    self.pressed_tab = true;
                }
            }
        }
    }

    pub(crate) fn end_frame(&mut self, used_ids: &epaint::ahash::AHashMap<Id, Pos2>) {
        if let Some(id) = self.id {
            // Allow calling `request_focus` one frame and not using it until next frame
            let recently_gained_focus = self.id_previous_frame != Some(id);

            if !recently_gained_focus && !used_ids.contains_key(&id) {
                // Dead-mans-switch: the widget with focus has disappeared!
                self.id = None;
            }
        }
    }

    pub(crate) fn had_focus_last_frame(&self, id: Id) -> bool {
        self.id_previous_frame == Some(id)
    }

    fn interested_in_focus(&mut self, id: Id) {
        if self.give_to_next && !self.had_focus_last_frame(id) {
            self.id = Some(id);
            self.give_to_next = false;
        } else if self.id == Some(id) {
            if self.pressed_tab {
                self.id = None;
                self.give_to_next = true;
                self.pressed_tab = false;
            } else if self.pressed_shift_tab {
                self.id_next_frame = self.last_interested; // frame-delay so gained_focus works
                self.pressed_shift_tab = false;
            }
        } else if self.pressed_tab && self.id == None && !self.give_to_next {
            // nothing has focus and the user pressed tab - give focus to the first widgets that wants it:
            self.id = Some(id);
        }

        self.last_interested = Some(id);
    }
}

impl Memory {
    pub(crate) fn begin_frame(
        &mut self,
        prev_input: &crate::input_state::InputState,
        new_input: &crate::data::input::RawInput,
    ) {
        self.interaction.begin_frame(prev_input, new_input);

        if !prev_input.pointer.any_down() {
            self.window_interaction = None;
        }
    }

    pub(crate) fn end_frame(
        &mut self,
        input: &InputState,
        used_ids: &epaint::ahash::AHashMap<Id, Pos2>,
    ) {
        self.areas.end_frame();
        self.interaction.focus.end_frame(used_ids);
        self.drag_value.end_frame(input);
    }

    pub fn layer_id_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<LayerId> {
        self.areas.layer_id_at(pos, resize_interact_radius_side)
    }

    pub(crate) fn had_focus_last_frame(&self, id: Id) -> bool {
        self.interaction.focus.id_previous_frame == Some(id)
    }

    /// True if the given widget had keyboard focus last frame, but not this one.
    pub(crate) fn lost_focus(&self, id: Id) -> bool {
        self.had_focus_last_frame(id) && !self.has_focus(id)
    }

    /// True if the given widget has keyboard focus this frame, but didn't last frame.
    pub(crate) fn gained_focus(&self, id: Id) -> bool {
        !self.had_focus_last_frame(id) && self.has_focus(id)
    }

    pub(crate) fn has_focus(&self, id: Id) -> bool {
        self.interaction.focus.id == Some(id)
    }

    /// Give keyboard focus to a specific widget
    pub fn request_focus(&mut self, id: Id) {
        self.interaction.focus.id = Some(id);
    }

    pub fn surrender_focus(&mut self, id: Id) {
        if self.interaction.focus.id == Some(id) {
            self.interaction.focus.id = None;
        }
    }

    /// Register this widget as being interested in getting keyboard focus.
    /// This will allow the user to select it with tab and shift-tab.
    pub(crate) fn interested_in_focus(&mut self, id: Id) {
        self.interaction.focus.interested_in_focus(id);
    }

    /// Stop editing of active `TextEdit` (if any).
    pub fn stop_text_input(&mut self) {
        self.interaction.focus.id = None;
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
        self.popup == Some(popup_id) || self.everything_is_visible()
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

    /// If true, all windows, menus, tooltips etc are to be visible at once.
    ///
    /// This is useful for testing, benchmarking, pre-caching, etc.
    ///
    /// Experimental feature!
    pub fn everything_is_visible(&self) -> bool {
        self.everything_is_visible
    }

    /// If true, all windows, menus, tooltips etc are to be visible at once.
    ///
    /// This is useful for testing, benchmarking, pre-caching, etc.
    ///
    /// Experimental feature!
    pub fn set_everything_is_visible(&mut self, value: bool) {
        self.everything_is_visible = value;
    }
}

// ----------------------------------------------------------------------------

/// Keeps track of `Area`s, which are free-floating `Ui`s.
/// These `Area`s can be in any `Order`.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
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
            .filter(|layer| layer.order == crate::Order::Middle)
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
