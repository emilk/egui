use std::sync::{Arc, Mutex};

use crate::*;

/// Contains the input, style and output of all GUI commands.
pub struct Context {
    /// The default style for new regions
    pub(crate) style: Mutex<Style>,
    pub(crate) fonts: Arc<Fonts>,
    pub(crate) input: GuiInput,
    pub(crate) memory: Mutex<Memory>,
    pub(crate) graphics: Mutex<GraphicLayers>,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            style: Mutex::new(self.style()),
            fonts: self.fonts.clone(),
            input: self.input,
            memory: Mutex::new(self.memory.lock().unwrap().clone()),
            graphics: Mutex::new(self.graphics.lock().unwrap().clone()),
        }
    }
}

impl Context {
    pub fn new(pixels_per_point: f32) -> Context {
        Context {
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

impl Context {
    pub fn style_ui(&self, region: &mut Region) {
        let mut style = self.style();
        style.ui(region);
        self.set_style(style);
    }
}
