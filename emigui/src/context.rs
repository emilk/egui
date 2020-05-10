use std::{collections::HashMap, sync::Arc};

use parking_lot::Mutex;

use crate::{layout::align_rect, *};

#[derive(Clone, Copy, Default)]
struct PaintStats {
    num_batches: usize,
    num_primitives: usize,
    num_vertices: usize,
    num_triangles: usize,
}

/// Contains the input, style and output of all GUI commands.
/// `Ui`:s keep an Arc pointer to this.
/// This allows us to create several child `Ui`:s at once,
/// all working against the same shared Context.
pub struct Context {
    /// The default style for new `Ui`:s
    style: Mutex<Style>,
    mesher_options: Mutex<mesher::MesherOptions>,
    fonts: Arc<Fonts>,
    /// HACK: set a new font next frame
    new_fonts: Mutex<Option<Arc<Fonts>>>,
    memory: Arc<Mutex<Memory>>,

    // Input releated stuff:
    /// Raw input from last frame. Use `input()` instead.
    last_raw_input: RawInput,
    input: GuiInput,
    mouse_tracker: MovementTracker<Pos2>,

    // The output of a frame:
    graphics: Mutex<GraphicLayers>,
    output: Mutex<Output>,
    /// Used to debug name clashes of e.g. windows
    used_ids: Mutex<HashMap<Id, Pos2>>,

    paint_stats: Mutex<PaintStats>,
}

// TODO: remove this impl.
impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            style: Mutex::new(self.style()),
            mesher_options: Mutex::new(*self.mesher_options.lock()),
            fonts: self.fonts.clone(),
            new_fonts: Mutex::new(self.new_fonts.lock().clone()),
            memory: self.memory.clone(),
            last_raw_input: self.last_raw_input.clone(),
            input: self.input.clone(),
            mouse_tracker: self.mouse_tracker.clone(),
            graphics: Mutex::new(self.graphics.lock().clone()),
            output: Mutex::new(self.output.lock().clone()),
            used_ids: Mutex::new(self.used_ids.lock().clone()),
            paint_stats: Mutex::new(*self.paint_stats.lock()),
        }
    }
}

impl Context {
    pub fn new(pixels_per_point: f32) -> Arc<Context> {
        Arc::new(Context {
            style: Default::default(),
            mesher_options: Default::default(),
            fonts: Arc::new(Fonts::new(pixels_per_point)),
            new_fonts: Default::default(),
            memory: Default::default(),

            last_raw_input: Default::default(),
            input: Default::default(),
            mouse_tracker: MovementTracker::new(1000, 0.1),

            graphics: Default::default(),
            output: Default::default(),
            used_ids: Default::default(),
            paint_stats: Default::default(),
        })
    }

    pub fn memory(&self) -> parking_lot::MutexGuard<'_, Memory> {
        self.memory.lock()
    }

    pub fn graphics(&self) -> parking_lot::MutexGuard<'_, GraphicLayers> {
        self.graphics.lock()
    }

    pub fn output(&self) -> parking_lot::MutexGuard<'_, Output> {
        self.output.lock()
    }

    pub fn input(&self) -> &GuiInput {
        &self.input
    }

    /// Smoothed mouse velocity, in points per second
    pub fn mouse_vel(&self) -> Vec2 {
        self.mouse_tracker.velocity().unwrap_or_default()
    }

    /// Raw input from last frame. Use `input()` instead.
    pub fn last_raw_input(&self) -> &RawInput {
        &self.last_raw_input
    }

    pub fn fonts(&self) -> &Fonts {
        &*self.fonts
    }

    pub fn texture(&self) -> &Texture {
        self.fonts().texture()
    }

    /// Will become active next frame
    pub fn set_fonts(&self, fonts: Fonts) {
        *self.new_fonts.lock() = Some(Arc::new(fonts));
    }

    // TODO: return MutexGuard
    pub fn style(&self) -> Style {
        *self.style.lock()
    }

    pub fn set_style(&self, style: Style) {
        *self.style.lock() = style;
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.input.pixels_per_point
    }

    /// Useful for pixel-perfect rendering
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.input.pixels_per_point).round() / self.input.pixels_per_point
    }

    pub fn round_pos_to_pixels(&self, pos: Pos2) -> Pos2 {
        pos2(self.round_to_pixel(pos.x), self.round_to_pixel(pos.y))
    }

    pub fn round_vec_to_pixels(&self, vec: Vec2) -> Vec2 {
        vec2(self.round_to_pixel(vec.x), self.round_to_pixel(vec.y))
    }

    // ---------------------------------------------------------------------

    pub fn begin_frame(self: &mut Arc<Self>, new_input: RawInput) {
        let mut self_: Self = (**self).clone();
        self_.begin_frame_mut(new_input);
        *self = Arc::new(self_);
    }

    fn begin_frame_mut(&mut self, new_input: RawInput) {
        if !self.last_raw_input.mouse_down || self.last_raw_input.mouse_pos.is_none() {
            self.memory().active_id = None;
        }
        self.memory().begin_frame();

        self.used_ids.lock().clear();

        if let Some(new_fonts) = self.new_fonts.lock().take() {
            self.fonts = new_fonts;
        }

        if let Some(mouse_pos) = new_input.mouse_pos {
            self.mouse_tracker.add(new_input.time, mouse_pos);
        } else {
            self.mouse_tracker.clear();
        }
        self.input = GuiInput::from_last_and_new(&self.last_raw_input, &new_input);
        self.input.mouse_velocity = self.mouse_vel();
        self.last_raw_input = new_input;
    }

    pub fn end_frame(&self) -> (Output, PaintBatches) {
        let output: Output = std::mem::take(&mut self.output());
        let paint_batches = self.paint();
        (output, paint_batches)
    }

    fn drain_paint_lists(&self) -> Vec<(Rect, PaintCmd)> {
        let memory = self.memory();
        self.graphics().drain(memory.areas.order()).collect()
    }

    fn paint(&self) -> PaintBatches {
        let mut mesher_options = *self.mesher_options.lock();
        mesher_options.aa_size = 1.0 / self.pixels_per_point();
        let paint_commands = self.drain_paint_lists();
        let num_primitives = paint_commands.len();
        let batches = mesher::mesh_paint_commands(mesher_options, self.fonts(), paint_commands);

        {
            let mut stats = PaintStats::default();
            stats.num_batches = batches.len();
            stats.num_primitives = num_primitives;
            for (_, mesh) in &batches {
                stats.num_vertices += mesh.vertices.len();
                stats.num_triangles += mesh.indices.len() / 3;
            }
            *self.paint_stats.lock() = stats;
        }

        batches
    }

    // ---------------------------------------------------------------------

    /// A `Ui` for the entire screen, behind any windows.
    pub fn fullscreen_ui(self: &Arc<Self>) -> Ui {
        let rect = Rect::from_min_size(Default::default(), self.input().screen_size);
        let id = Id::background();
        let layer = Layer {
            order: Order::Background,
            id,
        };
        // Ensure we register the background area so it is painted:
        self.memory().areas.set_state(
            layer,
            containers::area::State {
                pos: rect.min,
                size: rect.size(),
                interactable: true,
                vel: Default::default(),
            },
        );
        Ui::new(self.clone(), layer, id, rect)
    }

    // ---------------------------------------------------------------------

    /// Is the user interacting with anything?
    pub fn any_active(&self) -> bool {
        self.memory().active_id.is_some()
    }

    /// Generate a id from the given source.
    /// If it is not unique, an error will be printed at the given position.
    pub fn make_unique_id<IdSource>(&self, source: IdSource, pos: Pos2) -> Id
    where
        IdSource: std::hash::Hash + std::fmt::Debug + Copy,
    {
        self.register_unique_id(Id::new(source), source, pos)
    }

    /// If the given Id is not unique, an error will be printed at the given position.
    pub fn register_unique_id(&self, id: Id, source_name: impl std::fmt::Debug, pos: Pos2) -> Id {
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

    pub fn contains_mouse(&self, layer: Layer, clip_rect: Rect, rect: Rect) -> bool {
        let rect = rect.intersect(clip_rect);
        if let Some(mouse_pos) = self.input.mouse_pos {
            rect.contains(mouse_pos) && self.memory().layer_at(mouse_pos) == Some(layer)
        } else {
            false
        }
    }

    pub fn interact(
        &self,
        layer: Layer,
        clip_rect: Rect,
        rect: Rect,
        interaction_id: Option<Id>,
    ) -> InteractInfo {
        let hovered = self.contains_mouse(layer, clip_rect, rect);

        let mut memory = self.memory();
        let active = interaction_id.is_some() && memory.active_id == interaction_id;

        if self.input.mouse_pressed {
            if hovered && interaction_id.is_some() {
                if memory.active_id.is_some() {
                    // Already clicked something else this frame
                    InteractInfo {
                        rect,
                        hovered,
                        clicked: false,
                        active: false,
                    }
                } else {
                    memory.active_id = interaction_id;
                    InteractInfo {
                        rect,
                        hovered,
                        clicked: false,
                        active: true,
                    }
                }
            } else {
                InteractInfo {
                    rect,
                    hovered,
                    clicked: false,
                    active: false,
                }
            }
        } else if self.input.mouse_released {
            InteractInfo {
                rect,
                hovered,
                clicked: hovered && active,
                active,
            }
        } else if self.input.mouse_down {
            InteractInfo {
                rect,
                hovered: hovered && active,
                clicked: false,
                active,
            }
        } else {
            InteractInfo {
                rect,
                hovered,
                clicked: false,
                active,
            }
        }
    }

    // ---------------------------------------------------------------------

    pub fn show_error(&self, pos: Pos2, text: &str) {
        let align = (Align::Min, Align::Min);
        let layer = Layer::debug();
        let text_style = TextStyle::Monospace;
        let font = &self.fonts[text_style];
        let (text, size) = font.layout_multiline(text, f32::INFINITY);
        let rect = align_rect(Rect::from_min_size(pos, size), align);
        self.add_paint_cmd(
            layer,
            PaintCmd::Rect {
                corner_radius: 0.0,
                fill_color: Some(color::gray(0, 240)),
                outline: Some(Outline::new(1.0, color::RED)),
                rect: rect.expand(2.0),
            },
        );
        self.add_text(layer, rect.min, text_style, text, Some(color::RED));
    }

    pub fn debug_text(&self, pos: Pos2, text: &str) {
        let layer = Layer::debug();
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

    pub fn debug_rect(&self, rect: Rect, text: &str) {
        let layer = Layer::debug();
        self.add_paint_cmd(
            layer,
            PaintCmd::Rect {
                corner_radius: 0.0,
                fill_color: None,
                outline: Some(Outline::new(1.0, color::RED)),
                rect,
            },
        );
        let align = (Align::Min, Align::Min);
        let text_style = TextStyle::Monospace;
        self.floating_text(layer, rect.min, text, text_style, align, Some(color::RED));
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
        let rect = align_rect(Rect::from_min_size(pos, size), align);
        self.add_text(layer, rect.min, text_style, text, text_color);
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
        self.graphics()
            .layer(layer)
            .push((Rect::everything(), paint_cmd))
    }
}

impl Context {
    pub fn settings_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        CollapsingHeader::new("Style")
            // .default_open()
            .show(ui, |ui| {
                self.mesher_options.lock().ui(ui);
                self.style_ui(ui);
            });

        CollapsingHeader::new("Fonts")
            // .default_open()
            .show(ui, |ui| {
                let old_font_definitions = self.fonts().definitions();
                let mut new_font_definitions = old_font_definitions.clone();
                font_definitions_ui(&mut new_font_definitions, ui);
                self.fonts().texture().ui(ui);
                if *old_font_definitions != new_font_definitions {
                    let fonts = Fonts::from_definitions(
                        new_font_definitions,
                        self.input().pixels_per_point,
                    );
                    self.set_fonts(fonts);
                }
            });
    }

    pub fn inspection_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        ui.collapsing("Input", |ui| {
            CollapsingHeader::new("Raw Input")
                .default_open()
                .show(ui, |ui| ui.ctx().last_raw_input().clone().ui(ui));
            CollapsingHeader::new("Input")
                .default_open()
                .show(ui, |ui| ui.input().clone().ui(ui));
        });

        ui.collapsing("Stats", |ui| {
            ui.add(label!(
                "Screen size: {} x {} points, pixels_per_point: {}",
                ui.input().screen_size.x,
                ui.input().screen_size.y,
                ui.input().pixels_per_point,
            ));

            ui.add(label!("Painting:").text_style(TextStyle::Heading));
            self.paint_stats.lock().ui(ui);
        });
    }

    pub fn memory_ui(&self, ui: &mut crate::Ui) {
        use crate::widgets::*;

        if ui
            .add(Button::new("Reset all"))
            .tooltip_text("Reset all Emigui state")
            .clicked
        {
            *self.memory() = Default::default();
        }

        ui.horizontal(|ui| {
            ui.add(label!(
                "{} areas (window positions)",
                self.memory().areas.count()
            ));
            if ui.add(Button::new("Reset")).clicked {
                self.memory().areas = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.add(label!(
                "{} collapsing headers",
                self.memory().collapsing_headers.len()
            ));
            if ui.add(Button::new("Reset")).clicked {
                self.memory().collapsing_headers = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.add(label!("{} menu bars", self.memory().menu_bar.len()));
            if ui.add(Button::new("Reset")).clicked {
                self.memory().menu_bar = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.add(label!("{} scroll areas", self.memory().scroll_areas.len()));
            if ui.add(Button::new("Reset")).clicked {
                self.memory().scroll_areas = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.add(label!("{} resize areas", self.memory().resize.len()));
            if ui.add(Button::new("Reset")).clicked {
                self.memory().resize = Default::default();
            }
        });

        ui.add(
            label!("NOTE: the position of this window cannot be reset from within itself.")
                .auto_shrink(),
        );
    }
}

fn font_definitions_ui(font_definitions: &mut FontDefinitions, ui: &mut Ui) {
    use crate::widgets::*;
    for (text_style, (_family, size)) in font_definitions.iter_mut() {
        // TODO: radiobutton for family
        ui.add(
            Slider::f32(size, 4.0..=40.0)
                .precision(0)
                .text(format!("{:?}", text_style)),
        );
    }
    if ui.add(Button::new("Reset fonts")).clicked {
        *font_definitions = crate::fonts::default_font_definitions();
    }
}

impl Context {
    pub fn style_ui(&self, ui: &mut Ui) {
        let mut style = self.style();
        style.ui(ui);
        self.set_style(style);
    }
}

impl mesher::MesherOptions {
    pub fn ui(&mut self, ui: &mut Ui) {
        use crate::widgets::*;
        ui.add(Checkbox::new(&mut self.anti_alias, "Antialias"));
        ui.add(Checkbox::new(
            &mut self.debug_paint_clip_rects,
            "Paint Clip Rects (debug)",
        ));
    }
}

impl PaintStats {
    pub fn ui(&self, ui: &mut Ui) {
        ui.add(label!("Batches: {}", self.num_batches))
            .tooltip_text("Number of separate clip rectanlges");
        ui.add(label!("Primitives: {}", self.num_primitives))
            .tooltip_text("Boxes, circles, text areas etc");
        ui.add(label!("Vertices: {}", self.num_vertices));
        ui.add(label!("Triangles: {}", self.num_triangles));
    }
}
