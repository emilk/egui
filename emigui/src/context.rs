use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use crate::{layout::align_rect, *};

/// Contains the input, style and output of all GUI commands.
pub struct Context {
    /// The default style for new regions
    pub(crate) style: Mutex<Style>,
    pub(crate) fonts: Arc<Fonts>,
    pub(crate) input: GuiInput,
    pub(crate) memory: Mutex<Memory>,
    pub(crate) graphics: Mutex<GraphicLayers>,

    pub output: Mutex<Output>,

    /// Used to debug name clashes of e.g. windows
    used_ids: Mutex<HashMap<Id, Pos2>>,
}

// TODO: remove this impl.
impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            style: Mutex::new(self.style()),
            fonts: self.fonts.clone(),
            input: self.input,
            memory: Mutex::new(self.memory.lock().clone()),
            graphics: Mutex::new(self.graphics.lock().clone()),
            output: Mutex::new(self.output.lock().clone()),
            used_ids: Mutex::new(self.used_ids.lock().clone()),
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
            output: Default::default(),
            used_ids: Default::default(),
        }
    }

    /// Useful for pixel-perfect rendering
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.input.pixels_per_point).round() / self.input.pixels_per_point
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    pub fn style(&self) -> Style {
        *self.style.lock()
    }

    pub fn set_style(&self, style: Style) {
        *self.style.lock() = style;
    }

    // TODO: move
    pub fn begin_frame(&mut self, gui_input: GuiInput) {
        self.used_ids.lock().clear();
        self.input = gui_input;
    }

    pub fn end_frame(&self) -> Output {
        std::mem::take(&mut self.output.lock())
    }

    pub fn drain_paint_lists(&self) -> Vec<(Rect, PaintCmd)> {
        let memory = self.memory.lock();
        self.graphics.lock().drain(&memory.window_order).collect()
    }

    /// Is the user interacting with anything?
    pub fn any_active(&self) -> bool {
        self.memory.lock().active_id.is_some()
    }

    /// Generate a id from the given source.
    /// If it is not unique, an error will be printed at the given position.
    pub fn make_unique_id<IdSource>(&self, source: &IdSource, pos: Pos2) -> Id
    where
        IdSource: std::hash::Hash + std::fmt::Debug,
    {
        self.register_unique_id(Id::new(source), source, pos)
    }

    /// If the given Id is not unique, an error will be printed at the given position.
    pub fn register_unique_id(&self, id: Id, source_name: &impl std::fmt::Debug, pos: Pos2) -> Id {
        if let Some(clash_pos) = self.used_ids.lock().insert(id, pos) {
            if clash_pos.dist(pos) < 4.0 {
                self.show_error(
                    pos,
                    &format!("use of non-unique ID {:?} (name clash?)", source_name),
                );
            } else {
                self.show_error(
                    clash_pos,
                    &format!("first use of non-unique ID {:?} (name clash?)", source_name),
                );
                self.show_error(
                    pos,
                    &format!(
                        "second use of non-unique ID {:?} (name clash?)",
                        source_name
                    ),
                );
            }
            id
        } else {
            id
        }
    }

    pub fn contains_mouse_pos(&self, layer: Layer, rect: &Rect) -> bool {
        if let Some(mouse_pos) = self.input.mouse_pos {
            rect.contains(mouse_pos) && layer == self.memory.lock().layer_at(mouse_pos)
        } else {
            false
        }
    }

    pub fn interact(&self, layer: Layer, rect: &Rect, interaction_id: Option<Id>) -> InteractInfo {
        let hovered = self.contains_mouse_pos(layer, &rect);

        let mut memory = self.memory.lock();
        let active = interaction_id.is_some() && memory.active_id == interaction_id;

        if self.input.mouse_pressed {
            if hovered && interaction_id.is_some() {
                if memory.active_id.is_some() {
                    // Already clicked something else this frame
                    InteractInfo {
                        rect: *rect,
                        hovered,
                        clicked: false,
                        active: false,
                    }
                } else {
                    memory.active_id = interaction_id;
                    InteractInfo {
                        rect: *rect,
                        hovered,
                        clicked: false,
                        active: true,
                    }
                }
            } else {
                InteractInfo {
                    rect: *rect,
                    hovered,
                    clicked: false,
                    active: false,
                }
            }
        } else if self.input.mouse_released {
            InteractInfo {
                rect: *rect,
                hovered,
                clicked: hovered && active,
                active,
            }
        } else if self.input.mouse_down {
            InteractInfo {
                rect: *rect,
                hovered: hovered && active,
                clicked: false,
                active,
            }
        } else {
            InteractInfo {
                rect: *rect,
                hovered,
                clicked: false,
                active,
            }
        }
    }

    pub fn show_error(&self, pos: Pos2, text: &str) {
        let align = (Align::Min, Align::Min);
        let layer = Layer::Popup; // TODO: Layer::Debug
        let text_style = TextStyle::Monospace;
        let font = &self.fonts[text_style];
        let (text, size) = font.layout_multiline(text, f32::INFINITY);
        let rect = align_rect(&Rect::from_min_size(pos, size), align);
        self.add_paint_cmd(
            layer,
            PaintCmd::Rect {
                corner_radius: 0.0,
                fill_color: Some(color::gray(0, 240)),
                outline: Some(Outline::new(1.0, color::RED)),
                rect: rect.expand(2.0),
            },
        );
        self.add_text(layer, rect.min(), text_style, text, Some(color::RED));
    }

    pub fn debug_text(&self, pos: Pos2, text: &str) {
        let layer = Layer::Popup; // TODO: Layer::Debug
        let align = (Align::Min, Align::Min);
        self.floating_text(
            layer,
            pos,
            text,
            TextStyle::Monospace,
            align,
            Some(color::YELLOW),
        );
    }

    /// Show some text anywhere on screen.
    /// To center the text at the given position, use `align: (Center, Center)`.
    pub fn floating_text(
        &self,
        layer: Layer,
        pos: Pos2,
        text: &str,
        text_style: TextStyle,
        align: (Align, Align),
        text_color: Option<Color>,
    ) -> Vec2 {
        let font = &self.fonts[text_style];
        let (text, size) = font.layout_multiline(text, f32::INFINITY);
        let rect = align_rect(&Rect::from_min_size(pos, size), align);
        self.add_text(layer, rect.min(), text_style, text, text_color);
        size
    }

    /// Already layed out text.
    pub fn add_text(
        &self,
        layer: Layer,
        pos: Pos2,
        text_style: TextStyle,
        text: Vec<font::TextFragment>,
        color: Option<Color>,
    ) {
        let color = color.unwrap_or_else(|| self.style().text_color());
        for fragment in text {
            self.add_paint_cmd(
                layer,
                PaintCmd::Text {
                    color,
                    pos: pos + vec2(0.0, fragment.y_offset),
                    text: fragment.text,
                    text_style,
                    x_offsets: fragment.x_offsets,
                },
            );
        }
    }

    pub fn add_paint_cmd(&self, layer: Layer, paint_cmd: PaintCmd) {
        self.graphics
            .lock()
            .layer(layer)
            .push((Rect::everything(), paint_cmd))
    }
}

impl Context {
    pub fn style_ui(&self, region: &mut Region) {
        let mut style = self.style();
        style.ui(region);
        self.set_style(style);
    }
}
