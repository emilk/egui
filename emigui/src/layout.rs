use std::{
    hash::Hash,
    sync::{Arc, Mutex},
};

use crate::{widgets::*, *};

// ----------------------------------------------------------------------------

// TODO: rename GuiResponse
pub struct GuiResponse {
    /// The mouse is hovering above this
    pub hovered: bool,

    /// The mouse went got pressed on this thing this frame
    pub clicked: bool,

    /// The mouse is interacting with this thing (e.g. dragging it)
    pub active: bool,

    /// The region of the screen we are talking about
    pub rect: Rect,

    /// Used for showing a popup (if any)
    pub data: Arc<Data>,
}

impl GuiResponse {
    /// Show some stuff if the item was hovered
    pub fn tooltip<F>(&mut self, add_contents: F) -> &mut Self
    where
        F: FnOnce(&mut Region),
    {
        if self.hovered {
            if let Some(mouse_pos) = self.data.input().mouse_pos {
                let window_pos = mouse_pos + vec2(16.0, 16.0);
                show_popup(&self.data, window_pos, add_contents);
            }
        }
        self
    }

    /// Show this text if the item was hovered
    pub fn tooltip_text<S: Into<String>>(&mut self, text: S) -> &mut Self {
        self.tooltip(|popup| {
            popup.add(Label::new(text));
        })
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Horizontal,
    Vertical,
}

impl Default for Direction {
    fn default() -> Direction {
        Direction::Vertical
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    /// Left/Top
    Min,

    /// Note: requires a bounded/known available_width.
    Center,

    /// Right/Bottom
    /// Note: requires a bounded/known available_width.
    Max,
}

impl Default for Align {
    fn default() -> Align {
        Align::Min
    }
}

// ----------------------------------------------------------------------------

// TODO: newtype
pub type Id = u64;

pub fn make_id<H: Hash>(source: &H) -> Id {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}

// ----------------------------------------------------------------------------

// TODO: give a better name. Context?
/// Contains the input, style and output of all GUI commands.
pub struct Data {
    /// The default style for new regions
    pub(crate) style: Mutex<Style>,
    pub(crate) fonts: Arc<Fonts>,
    pub(crate) input: GuiInput,
    pub(crate) memory: Mutex<Memory>,
    pub(crate) graphics: Mutex<GraphicLayers>,
}

impl Clone for Data {
    fn clone(&self) -> Self {
        Data {
            style: Mutex::new(self.style()),
            fonts: self.fonts.clone(),
            input: self.input,
            memory: Mutex::new(self.memory.lock().unwrap().clone()),
            graphics: Mutex::new(self.graphics.lock().unwrap().clone()),
        }
    }
}

impl Data {
    pub fn new(pixels_per_point: f32) -> Data {
        Data {
            style: Default::default(),
            fonts: Arc::new(Fonts::new(pixels_per_point)),
            input: Default::default(),
            memory: Default::default(),
            graphics: Default::default(),
        }
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn style(&self) -> Style {
        *self.style.lock().unwrap()
    }

    pub fn set_style(&self, style: Style) {
        *self.style.lock().unwrap() = style;
    }

    // TODO: move
    pub fn new_frame(&mut self, gui_input: GuiInput) {
        self.input = gui_input;
        if !gui_input.mouse_down || gui_input.mouse_pos.is_none() {
            self.memory.lock().unwrap().active_id = None;
        }
    }

    /// Is the user interacting with anything?
    pub fn any_active(&self) -> bool {
        self.memory.lock().unwrap().active_id.is_some()
    }

    pub fn interact(&self, layer: Layer, rect: Rect, interaction_id: Option<Id>) -> InteractInfo {
        let mut memory = self.memory.lock().unwrap();

        let hovered = if let Some(mouse_pos) = self.input.mouse_pos {
            if rect.contains(mouse_pos) {
                let is_something_else_active =
                    memory.active_id.is_some() && memory.active_id != interaction_id;

                !is_something_else_active && layer == memory.layer_at(mouse_pos)
            } else {
                false
            }
        } else {
            false
        };
        let active = if interaction_id.is_some() {
            if hovered && self.input.mouse_clicked {
                memory.active_id = interaction_id;
            }
            memory.active_id == interaction_id
        } else {
            false
        };

        let clicked = hovered && self.input.mouse_released;

        InteractInfo {
            rect,
            hovered,
            clicked,
            active,
        }
    }
}

impl Data {
    pub fn style_ui(&self, region: &mut Region) {
        let mut style = self.style();
        style.ui(region);
        self.set_style(style);
    }
}

/// Show a pop-over window
pub fn show_popup<F>(data: &Arc<Data>, window_pos: Vec2, add_contents: F)
where
    F: FnOnce(&mut Region),
{
    let layer = Layer::Popup;
    let where_to_put_background = data.graphics.lock().unwrap().layer(layer).len();

    let style = data.style();
    let window_padding = style.window_padding;

    let mut contents_region = Region {
        data: data.clone(),
        layer: Layer::Popup,
        style,
        id: Default::default(),
        dir: Direction::Vertical,
        align: Align::Min,
        cursor: window_pos + window_padding,
        bounding_size: vec2(0.0, 0.0),
        available_space: vec2(data.input.screen_size.x.min(350.0), std::f32::INFINITY), // TODO: popup/tooltip width
    };

    add_contents(&mut contents_region);

    // Now insert popup background:

    // TODO: handle the last item_spacing in a nicer way
    let inner_size = contents_region.bounding_size - style.item_spacing;
    let outer_size = inner_size + 2.0 * window_padding;

    let rect = Rect::from_min_size(window_pos, outer_size);

    let mut graphics = data.graphics.lock().unwrap();
    let graphics = graphics.layer(layer);
    graphics.insert(
        where_to_put_background,
        PaintCmd::Rect {
            corner_radius: 5.0,
            fill_color: Some(style.background_fill_color()),
            outline: Some(Outline {
                color: color::WHITE,
                width: 1.0,
            }),
            rect,
        },
    );
}
