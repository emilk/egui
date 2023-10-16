#![warn(missing_docs)] // Let's keep this file well-documented.` to memory.rs

use epaint::{emath::Rangef, vec2, Vec2};

use crate::{area, window, EventFilter, Id, IdMap, InputState, LayerId, Pos2, Rect, Style};

// ----------------------------------------------------------------------------

/// The data that egui persists between frames.
///
/// This includes window positions and sizes,
/// how far the user has scrolled in a [`ScrollArea`](crate::ScrollArea) etc.
///
/// If you want this to persist when closing your app you should serialize [`Memory`] and store it.
/// For this you need to enable the `persistence`.
///
/// If you want to store data for your widgets, you should look at [`Memory::data`]
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub struct Memory {
    /// Global egui options.
    pub options: Options,

    /// This map stores some superficial state for all widgets with custom [`Id`]s.
    ///
    /// This includes storing if a [`crate::CollapsingHeader`] is open, how far scrolled a
    /// [`crate::ScrollArea`] is, where the cursor in a [`crate::TextEdit`] is, etc.
    ///
    /// This is NOT meant to store any important data. Store that in your own structures!
    ///
    /// Each read clones the data, so keep your values cheap to clone.
    /// If you want to store a lot of data you should wrap it in `Arc<Mutex<…>>` so it is cheap to clone.
    ///
    /// This will be saved between different program runs if you use the `persistence` feature.
    ///
    /// To store a state common for all your widgets (a singleton), use [`Id::null`] as the key.
    pub data: crate::util::IdTypeMap,

    // ------------------------------------------
    /// Can be used to cache computations from one frame to another.
    ///
    /// This is for saving CPU when you have something that may take 1-100ms to compute.
    /// Things that are very slow (>100ms) should instead be done async (i.e. in another thread)
    /// so as not to lock the UI thread.
    ///
    /// ```
    /// use egui::util::cache::{ComputerMut, FrameCache};
    ///
    /// #[derive(Default)]
    /// struct CharCounter {}
    /// impl ComputerMut<&str, usize> for CharCounter {
    ///     fn compute(&mut self, s: &str) -> usize {
    ///         s.chars().count() // you probably want to cache something more expensive than this
    ///     }
    /// }
    /// type CharCountCache<'a> = FrameCache<usize, CharCounter>;
    ///
    /// # let mut ctx = egui::Context::default();
    /// ctx.memory_mut(|mem| {
    ///     let cache = mem.caches.cache::<CharCountCache<'_>>();
    ///     assert_eq!(cache.get("hello"), 5);
    /// });
    /// ```
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub caches: crate::util::cache::CacheStorage,

    // ------------------------------------------
    /// new scale that will be applied at the start of the next frame
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) new_pixels_per_point: Option<f32>,

    /// new fonts that will be applied at the start of the next frame
    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) new_font_definitions: Option<epaint::text::FontDefinitions>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) interaction: Interaction,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) window_interaction: Option<window::WindowInteraction>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    pub(crate) drag_value: crate::widgets::drag_value::MonoState,

    pub(crate) areas: Areas,

    /// Which popup-window is open (if any)?
    /// Could be a combo box, color picker, menu etc.
    #[cfg_attr(feature = "persistence", serde(skip))]
    popup: Option<Id>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    everything_is_visible: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FocusDirection {
    /// Select the widget closest above the current focused widget.
    Up,

    /// Select the widget to the right of the current focused widget.
    Right,

    /// Select the widget below the current focused widget.
    Down,

    /// Select the widget to the left of the the current focused widget.
    Left,

    /// Select the previous widget that had focus.
    Previous,

    /// Select the next widget that wants focus.
    Next,

    /// Don't change focus.
    #[default]
    None,
}

impl FocusDirection {
    fn is_cardinal(&self) -> bool {
        match self {
            FocusDirection::Up
            | FocusDirection::Right
            | FocusDirection::Down
            | FocusDirection::Left => true,

            FocusDirection::Previous | FocusDirection::Next | FocusDirection::None => false,
        }
    }
}

// ----------------------------------------------------------------------------

/// Some global options that you can read and write.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Options {
    /// The default style for new [`Ui`](crate::Ui):s.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub(crate) style: std::sync::Arc<Style>,

    /// Controls the tessellator.
    pub tessellation_options: epaint::TessellationOptions,

    /// This is a signal to any backend that we want the [`crate::PlatformOutput::events`] read out loud.
    ///
    /// The only change to egui is that labels can be focused by pressing tab.
    ///
    /// Screen readers is an experimental feature of egui, and not supported on all platforms.
    ///
    /// `eframe` supports it only on web, using the `web_screen_reader` feature flag,
    /// but you should consider using [AccessKit](https://github.com/AccessKit/accesskit) instead,
    /// which `eframe` supports.
    pub screen_reader: bool,

    /// If true, the most common glyphs (ASCII) are pre-rendered to the texture atlas.
    ///
    /// Only the fonts in [`Style::text_styles`] will be pre-cached.
    ///
    /// This can lead to fewer texture operations, but may use up the texture atlas quicker
    /// if you are changing [`Style::text_styles`], of have a lot of text styles.
    pub preload_font_glyphs: bool,

    /// Check reusing of [`Id`]s, and show a visual warning on screen when one is found.
    ///
    /// By default this is `true` in debug builds.
    pub warn_on_id_clash: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            style: Default::default(),
            tessellation_options: Default::default(),
            screen_reader: false,
            preload_font_glyphs: true,
            warn_on_id_clash: cfg!(debug_assertions),
        }
    }
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
    focused_widget: Option<FocusWidget>,

    /// What had keyboard focus previous frame?
    id_previous_frame: Option<Id>,

    /// Give focus to this widget next frame
    id_next_frame: Option<Id>,

    #[cfg(feature = "accesskit")]
    id_requested_by_accesskit: Option<accesskit::NodeId>,

    /// If set, the next widget that is interested in focus will automatically get it.
    /// Probably because the user pressed Tab.
    give_to_next: bool,

    /// The last widget interested in focus.
    last_interested: Option<Id>,

    /// Set when looking for widget with navigational keys like arrows, tab, shift+tab
    focus_direction: FocusDirection,

    /// A cache of widget ids that are interested in focus with their corresponding rectangles.
    focus_widgets_cache: IdMap<Rect>,
}

/// The widget with focus.
#[derive(Clone, Copy, Debug)]
struct FocusWidget {
    pub id: Id,
    pub filter: EventFilter,
}

impl FocusWidget {
    pub fn new(id: Id) -> Self {
        Self {
            id,
            filter: Default::default(),
        }
    }
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
        self.focused_widget.as_ref().map(|w| w.id)
    }

    fn begin_frame(&mut self, new_input: &crate::data::input::RawInput) {
        self.id_previous_frame = self.focused();
        if let Some(id) = self.id_next_frame.take() {
            self.focused_widget = Some(FocusWidget::new(id));
        }
        let event_filter = self.focused_widget.map(|w| w.filter).unwrap_or_default();

        #[cfg(feature = "accesskit")]
        {
            self.id_requested_by_accesskit = None;
        }

        self.focus_direction = FocusDirection::None;

        for event in &new_input.events {
            if !event_filter.matches(event) {
                if let crate::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = event
                {
                    if let Some(cardinality) = match key {
                        crate::Key::ArrowUp => Some(FocusDirection::Up),
                        crate::Key::ArrowRight => Some(FocusDirection::Right),
                        crate::Key::ArrowDown => Some(FocusDirection::Down),
                        crate::Key::ArrowLeft => Some(FocusDirection::Left),

                        crate::Key::Tab => {
                            if modifiers.shift {
                                Some(FocusDirection::Previous)
                            } else {
                                Some(FocusDirection::Next)
                            }
                        }
                        crate::Key::Escape => {
                            self.focused_widget = None;
                            Some(FocusDirection::None)
                        }
                        _ => None,
                    } {
                        self.focus_direction = cardinality;
                    }
                }
            }

            #[cfg(feature = "accesskit")]
            {
                if let crate::Event::AccessKitActionRequest(accesskit::ActionRequest {
                    action: accesskit::Action::Focus,
                    target,
                    data: None,
                }) = event
                {
                    self.id_requested_by_accesskit = Some(*target);
                }
            }
        }
    }

    pub(crate) fn end_frame(&mut self, used_ids: &IdMap<Rect>) {
        if self.focus_direction.is_cardinal() {
            if let Some(found_widget) = self.find_widget_in_direction(used_ids) {
                self.focused_widget = Some(FocusWidget::new(found_widget));
            }
        }

        if let Some(focused_widget) = self.focused_widget {
            // Allow calling `request_focus` one frame and not using it until next frame
            let recently_gained_focus = self.id_previous_frame != Some(focused_widget.id);

            if !recently_gained_focus && !used_ids.contains_key(&focused_widget.id) {
                // Dead-mans-switch: the widget with focus has disappeared!
                self.focused_widget = None;
            }
        }
    }

    pub(crate) fn had_focus_last_frame(&self, id: Id) -> bool {
        self.id_previous_frame == Some(id)
    }

    fn interested_in_focus(&mut self, id: Id) {
        #[cfg(feature = "accesskit")]
        {
            if self.id_requested_by_accesskit == Some(id.accesskit_id()) {
                self.focused_widget = Some(FocusWidget::new(id));
                self.id_requested_by_accesskit = None;
                self.give_to_next = false;
                self.reset_focus();
            }
        }

        // The rect is updated at the end of the frame.
        self.focus_widgets_cache
            .entry(id)
            .or_insert(Rect::EVERYTHING);

        if self.give_to_next && !self.had_focus_last_frame(id) {
            self.focused_widget = Some(FocusWidget::new(id));
            self.give_to_next = false;
        } else if self.focused() == Some(id) {
            if self.focus_direction == FocusDirection::Next {
                self.focused_widget = None;
                self.give_to_next = true;
                self.reset_focus();
            } else if self.focus_direction == FocusDirection::Previous {
                self.id_next_frame = self.last_interested; // frame-delay so gained_focus works
                self.reset_focus();
            }
        } else if self.focus_direction == FocusDirection::Next
            && self.focused_widget.is_none()
            && !self.give_to_next
        {
            // nothing has focus and the user pressed tab - give focus to the first widgets that wants it:
            self.focused_widget = Some(FocusWidget::new(id));
            self.reset_focus();
        }

        self.last_interested = Some(id);
    }

    fn reset_focus(&mut self) {
        self.focus_direction = FocusDirection::None;
    }

    fn find_widget_in_direction(&mut self, new_rects: &IdMap<Rect>) -> Option<Id> {
        // NOTE: `new_rects` here include some widgets _not_ interested in focus.

        /// * negative if `a` is left of `b`
        /// * positive if `a` is right of `b`
        /// * zero if the ranges overlap significantly
        fn range_diff(a: Rangef, b: Rangef) -> f32 {
            let has_significant_overlap = a.intersection(b).span() >= 0.5 * b.span().min(a.span());
            if has_significant_overlap {
                0.0
            } else {
                a.center() - b.center()
            }
        }

        let Some(current_focused) = self.focused_widget else {
            return None;
        };

        // In what direction we are looking for the next widget.
        let search_direction = match self.focus_direction {
            FocusDirection::Up => Vec2::UP,
            FocusDirection::Right => Vec2::RIGHT,
            FocusDirection::Down => Vec2::DOWN,
            FocusDirection::Left => Vec2::LEFT,
            _ => {
                return None;
            }
        };

        // Update cache with new rects
        self.focus_widgets_cache.retain(|id, old_rect| {
            if let Some(new_rect) = new_rects.get(id) {
                *old_rect = *new_rect;
                true // Keep the item
            } else {
                false // Remove the item
            }
        });

        let Some(current_rect) = self.focus_widgets_cache.get(&current_focused.id) else {
            return None;
        };

        let mut best_score = std::f32::INFINITY;
        let mut best_id = None;

        for (candidate_id, candidate_rect) in &self.focus_widgets_cache {
            if *candidate_id == current_focused.id {
                continue;
            }

            // There is a lot of room for improvement here.
            let to_candidate = vec2(
                range_diff(candidate_rect.x_range(), current_rect.x_range()),
                range_diff(candidate_rect.y_range(), current_rect.y_range()),
            );

            let acos_angle = to_candidate.normalized().dot(search_direction);

            // Only interested in widgets that fall in a 90° cone (±45°)
            // of the search direction.
            let is_in_search_cone = 0.5_f32.sqrt() <= acos_angle;
            if is_in_search_cone {
                let distance = to_candidate.length();

                // There is a lot of room for improvement here.
                let score = distance / (acos_angle * acos_angle);

                if score < best_score {
                    best_score = score;
                    best_id = Some(*candidate_id);
                }
            }
        }

        best_id
    }
}

impl Memory {
    pub(crate) fn begin_frame(
        &mut self,
        prev_input: &crate::input_state::InputState,
        new_input: &crate::data::input::RawInput,
    ) {
        crate::profile_function!();
        self.interaction.begin_frame(prev_input, new_input);

        if !prev_input.pointer.any_down() {
            self.window_interaction = None;
        }
    }

    pub(crate) fn end_frame(&mut self, input: &InputState, used_ids: &IdMap<Rect>) {
        self.caches.update();
        self.areas.end_frame();
        self.interaction.focus.end_frame(used_ids);
        self.drag_value.end_frame(input);
    }

    /// Top-most layer at the given position.
    pub fn layer_id_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<LayerId> {
        self.areas.layer_id_at(pos, resize_interact_radius_side)
    }

    /// An iterator over all layers. Back-to-front. Top is last.
    pub fn layer_ids(&self) -> impl ExactSizeIterator<Item = LayerId> + '_ {
        self.areas.order().iter().copied()
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

    /// Does this widget have keyboard focus?
    ///
    /// This function does not consider whether the UI as a whole (e.g. window)
    /// has the keyboard focus. That makes this function suitable for deciding
    /// widget state that should not be disrupted if the user moves away
    /// from the window and back.
    #[inline(always)]
    pub fn has_focus(&self, id: Id) -> bool {
        self.interaction.focus.focused() == Some(id)
    }

    /// Which widget has keyboard focus?
    pub fn focus(&self) -> Option<Id> {
        self.interaction.focus.focused()
    }

    /// Set an event filter for a widget.
    ///
    /// This allows you to control whether the widget will loose focus
    /// when the user presses tab, arrow keys, or escape.
    ///
    /// You must first give focus to the widget before calling this.
    pub fn set_focus_lock_filter(&mut self, id: Id, event_filter: EventFilter) {
        if self.had_focus_last_frame(id) && self.has_focus(id) {
            if let Some(focused) = &mut self.interaction.focus.focused_widget {
                if focused.id == id {
                    focused.filter = event_filter;
                }
            }
        }
    }

    /// Set an event filter for a widget.
    ///
    /// You must first give focus to the widget before calling this.
    #[deprecated = "Use set_focus_lock_filter instead"]
    pub fn lock_focus(&mut self, id: Id, lock_focus: bool) {
        self.set_focus_lock_filter(
            id,
            EventFilter {
                tab: lock_focus,
                arrows: lock_focus,
                escape: false,
            },
        );
    }

    /// Give keyboard focus to a specific widget.
    /// See also [`crate::Response::request_focus`].
    #[inline(always)]
    pub fn request_focus(&mut self, id: Id) {
        self.interaction.focus.focused_widget = Some(FocusWidget::new(id));
    }

    /// Surrender keyboard focus for a specific widget.
    /// See also [`crate::Response::surrender_focus`].
    #[inline(always)]
    pub fn surrender_focus(&mut self, id: Id) {
        if self.interaction.focus.focused() == Some(id) {
            self.interaction.focus.focused_widget = None;
        }
    }

    /// Register this widget as being interested in getting keyboard focus.
    /// This will allow the user to select it with tab and shift-tab.
    /// This is normally done automatically when handling interactions,
    /// but it is sometimes useful to pre-register interest in focus,
    /// e.g. before deciding which type of underlying widget to use,
    /// as in the [`crate::DragValue`] widget, so a widget can be focused
    /// and rendered correctly in a single frame.
    #[inline(always)]
    pub fn interested_in_focus(&mut self, id: Id) {
        self.interaction.focus.interested_in_focus(id);
    }

    /// Stop editing of active [`TextEdit`](crate::TextEdit) (if any).
    #[inline(always)]
    pub fn stop_text_input(&mut self) {
        self.interaction.focus.focused_widget = None;
    }

    /// Is any widget being dragged?
    #[inline(always)]
    pub fn is_anything_being_dragged(&self) -> bool {
        self.interaction.drag_id.is_some()
    }

    /// Is this specific widget being dragged?
    #[inline(always)]
    pub fn is_being_dragged(&self, id: Id) -> bool {
        self.interaction.drag_id == Some(id)
    }

    /// Set which widget is being dragged.
    #[inline(always)]
    pub fn set_dragged_id(&mut self, id: Id) {
        self.interaction.drag_id = Some(id);
    }

    /// Stop dragging any widget.
    #[inline(always)]
    pub fn stop_dragging(&mut self) {
        self.interaction.drag_id = None;
    }

    /// Forget window positions, sizes etc.
    /// Can be used to auto-layout windows.
    pub fn reset_areas(&mut self) {
        self.areas = Default::default();
    }

    /// Obtain the previous rectangle of an area.
    pub fn area_rect(&self, id: impl Into<Id>) -> Option<Rect> {
        self.areas.get(id.into()).map(|state| state.rect())
    }
}

/// ## Popups
/// Popups are things like combo-boxes, color pickers, menus etc.
/// Only one can be be open at a time.
impl Memory {
    /// Is the given popup open?
    pub fn is_popup_open(&self, popup_id: Id) -> bool {
        self.popup == Some(popup_id) || self.everything_is_visible()
    }

    /// Is any popup open?
    pub fn any_popup_open(&self) -> bool {
        self.popup.is_some() || self.everything_is_visible()
    }

    /// Open the given popup, and close all other.
    pub fn open_popup(&mut self, popup_id: Id) {
        self.popup = Some(popup_id);
    }

    /// Close the open popup, if any.
    pub fn close_popup(&mut self) {
        self.popup = None;
    }

    /// Toggle the given popup between closed and open.
    ///
    /// Note: at most one popup can be open at one time.
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
    #[inline(always)]
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

/// Keeps track of [`Area`](crate::containers::area::Area)s, which are free-floating [`Ui`](crate::Ui)s.
/// These [`Area`](crate::containers::area::Area)s can be in any [`Order`](crate::Order).
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Areas {
    areas: IdMap<area::State>,

    /// Back-to-front. Top is last.
    order: Vec<LayerId>,

    visible_last_frame: ahash::HashSet<LayerId>,
    visible_current_frame: ahash::HashSet<LayerId>,

    /// When an area want to be on top, it is put in here.
    /// At the end of the frame, this is used to reorder the layers.
    /// This means if several layers want to be on top, they will keep their relative order.
    /// So if you close three windows and then reopen them all in one frame,
    /// they will all be sent to the top, but keep their previous internal order.
    wants_to_be_on_top: ahash::HashSet<LayerId>,
}

impl Areas {
    pub(crate) fn count(&self) -> usize {
        self.areas.len()
    }

    pub(crate) fn get(&self, id: Id) -> Option<&area::State> {
        self.areas.get(&id)
    }

    /// Back-to-front. Top is last.
    pub(crate) fn order(&self) -> &[LayerId] {
        &self.order
    }

    pub(crate) fn set_state(&mut self, layer_id: LayerId, state: area::State) {
        self.visible_current_frame.insert(layer_id);
        self.areas.insert(layer_id.id, state);
        if !self.order.iter().any(|x| *x == layer_id) {
            self.order.push(layer_id);
        }
    }

    /// Top-most layer at the given position.
    pub fn layer_id_at(&self, pos: Pos2, resize_interact_radius_side: f32) -> Option<LayerId> {
        for layer in self.order.iter().rev() {
            if self.is_visible(layer) {
                if let Some(state) = self.areas.get(&layer.id) {
                    let mut rect = state.rect();
                    if state.interactable {
                        // Allow us to resize by dragging just outside the window:
                        rect = rect.expand(resize_interact_radius_side);
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

    pub fn visible_layer_ids(&self) -> ahash::HashSet<LayerId> {
        self.visible_last_frame
            .iter()
            .copied()
            .chain(self.visible_current_frame.iter().copied())
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

        if !self.order.iter().any(|x| *x == layer_id) {
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

        std::mem::swap(visible_last_frame, visible_current_frame);
        visible_current_frame.clear();
        order.sort_by_key(|layer| (layer.order, wants_to_be_on_top.contains(layer)));
        wants_to_be_on_top.clear();
    }
}

// ----------------------------------------------------------------------------

#[test]
fn memory_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Memory>();
}
