use std::sync::Arc;

use {ahash::AHashMap, parking_lot::Mutex};

use crate::{paint::*, *};

#[derive(Clone, Copy, Default)]
struct PaintStats {
    num_jobs: usize,
    num_primitives: usize,
    num_vertices: usize,
    num_triangles: usize,
}

/// Contains the input, style and output of all GUI commands.
/// `Ui`:s keep an Arc pointer to this.
/// This allows us to create several child `Ui`:s at once,
/// all working against the same shared Context.
#[derive(Default)]
pub struct Context {
    /// The default style for new `Ui`:s
    style: Mutex<Style>,
    paint_options: Mutex<paint::PaintOptions>,
    /// None until first call to `begin_frame`.
    fonts: Option<Arc<Fonts>>,
    font_definitions: Mutex<FontDefinitions>,
    memory: Arc<Mutex<Memory>>,

    input: InputState,

    // The output of a frame:
    graphics: Mutex<GraphicLayers>,
    output: Mutex<Output>,
    /// Used to debug name clashes of e.g. windows
    used_ids: Mutex<AHashMap<Id, Pos2>>,

    paint_stats: Mutex<PaintStats>,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            style: Mutex::new(self.style()),
            paint_options: Mutex::new(*self.paint_options.lock()),
            fonts: self.fonts.clone(),
            font_definitions: Mutex::new(self.font_definitions.lock().clone()),
            memory: self.memory.clone(),
            input: self.input.clone(),
            graphics: Mutex::new(self.graphics.lock().clone()),
            output: Mutex::new(self.output.lock().clone()),
            used_ids: Mutex::new(self.used_ids.lock().clone()),
            paint_stats: Mutex::new(*self.paint_stats.lock()),
        }
    }
}

impl Context {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), self.input.screen_size)
    }

    pub fn memory(&self) -> parking_lot::MutexGuard<'_, Memory> {
        self.memory.try_lock().expect("memory already locked")
    }

    pub fn graphics(&self) -> parking_lot::MutexGuard<'_, GraphicLayers> {
        self.graphics.try_lock().expect("graphics already locked")
    }

    pub fn output(&self) -> parking_lot::MutexGuard<'_, Output> {
        self.output.try_lock().expect("output already locked")
    }

    /// Call this if there is need to repaint the UI, i.e. if you are showing an animation.
    /// If this is called at least once in a frame, then there will be another frame right after this.
    /// Call as many times as you wish, only one repaint will be issued.
    pub fn request_repaint(&self) {
        self.output().needs_repaint = true;
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Not valid until first call to `begin_frame()`
    /// That's because since we don't know the proper `pixels_per_point` until then.
    pub fn fonts(&self) -> &Fonts {
        &*self
            .fonts
            .as_ref()
            .expect("No fonts available until first call to Contex::begin_frame()`")
    }

    /// Not valid until first call to `begin_frame()`
    /// That's because since we don't know the proper `pixels_per_point` until then.
    pub fn texture(&self) -> &paint::Texture {
        self.fonts().texture()
    }

    /// Will become active at the start of the next frame.
    /// `pixels_per_point` will be ignored (overwitten at start of each frame with the contents of input)
    pub fn set_fonts(&self, font_definitions: FontDefinitions) {
        *self.font_definitions.lock() = font_definitions;
    }

    // TODO: return MutexGuard
    pub fn style(&self) -> Style {
        self.style.try_lock().expect("style already locked").clone()
    }

    pub fn set_style(&self, style: Style) {
        *self.style.try_lock().expect("style already locked") = style;
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.input.pixels_per_point
    }

    /// Useful for pixel-perfect rendering
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.input.pixels_per_point).round() / self.input.pixels_per_point
    }

    /// Useful for pixel-perfect rendering
    pub fn round_pos_to_pixels(&self, pos: Pos2) -> Pos2 {
        pos2(self.round_to_pixel(pos.x), self.round_to_pixel(pos.y))
    }

    /// Useful for pixel-perfect rendering
    pub fn round_vec_to_pixels(&self, vec: Vec2) -> Vec2 {
        vec2(self.round_to_pixel(vec.x), self.round_to_pixel(vec.y))
    }

    /// Useful for pixel-perfect rendering
    pub fn round_rect_to_pixels(&self, rect: Rect) -> Rect {
        Rect {
            min: self.round_pos_to_pixels(rect.min),
            max: self.round_pos_to_pixels(rect.max),
        }
    }

    // ---------------------------------------------------------------------

    /// Call at the start of every frame.
    /// Returns a master fullscreen UI, covering the entire screen.
    pub fn begin_frame(self: &mut Arc<Self>, new_input: RawInput) -> Ui {
        let mut self_: Self = (**self).clone();
        self_.begin_frame_mut(new_input);
        *self = Arc::new(self_);
        self.fullscreen_ui()
    }

    fn begin_frame_mut(&mut self, new_raw_input: RawInput) {
        self.memory().begin_frame(&self.input);

        self.used_ids.lock().clear();

        self.input = std::mem::take(&mut self.input).begin_frame(new_raw_input);
        let mut font_definitions = self.font_definitions.lock();
        font_definitions.pixels_per_point = self.input.pixels_per_point;
        if self.fonts.is_none() || *self.fonts.as_ref().unwrap().definitions() != *font_definitions
        {
            self.fonts = Some(Arc::new(Fonts::from_definitions(font_definitions.clone())));
        }
    }

    /// Call at the end of each frame.
    /// Returns what has happened this frame (`Output`) as well as what you need to paint.
    #[must_use]
    pub fn end_frame(&self) -> (Output, PaintJobs) {
        if self.input.wants_repaint() {
            self.request_repaint();
        }

        self.memory().end_frame();
        let output: Output = std::mem::take(&mut self.output());
        let paint_jobs = self.paint();
        (output, paint_jobs)
    }

    fn drain_paint_lists(&self) -> Vec<(Rect, PaintCmd)> {
        let memory = self.memory();
        self.graphics().drain(memory.areas.order()).collect()
    }

    fn paint(&self) -> PaintJobs {
        let mut paint_options = *self.paint_options.lock();
        paint_options.aa_size = 1.0 / self.pixels_per_point();
        paint_options.aa_size *= 1.5; // Looks better, but TODO: should not be needed
        let paint_commands = self.drain_paint_lists();
        let num_primitives = paint_commands.len();
        let paint_jobs =
            tessellator::tessellate_paint_commands(paint_options, self.fonts(), paint_commands);

        {
            let mut stats = PaintStats::default();
            stats.num_jobs = paint_jobs.len();
            stats.num_primitives = num_primitives;
            for (_, triangles) in &paint_jobs {
                stats.num_vertices += triangles.vertices.len();
                stats.num_triangles += triangles.indices.len() / 3;
            }
            *self.paint_stats.lock() = stats;
        }

        paint_jobs
    }

    // ---------------------------------------------------------------------

    /// A `Ui` for the entire screen, behind any windows.
    fn fullscreen_ui(self: &Arc<Self>) -> Ui {
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

    /// Generate a id from the given source.
    /// If it is not unique, an error will be printed at the given position.
    pub fn make_unique_id<IdSource>(self: &Arc<Self>, source: IdSource, pos: Pos2) -> Id
    where
        IdSource: std::hash::Hash + std::fmt::Debug + Copy,
    {
        self.register_unique_id(Id::new(source), source, pos)
    }

    pub fn is_unique_id(&self, id: Id) -> bool {
        !self.used_ids.lock().contains_key(&id)
    }

    /// If the given Id is not unique, an error will be printed at the given position.
    pub fn register_unique_id(
        self: &Arc<Self>,
        id: Id,
        source_name: impl std::fmt::Debug,
        pos: Pos2,
    ) -> Id {
        if let Some(clash_pos) = self.used_ids.lock().insert(id, pos) {
            let painter = self.debug_painter();
            if clash_pos.distance(pos) < 4.0 {
                painter.error(
                    pos,
                    &format!("use of non-unique ID {:?} (name clash?)", source_name),
                );
            } else {
                painter.error(
                    clash_pos,
                    &format!("first use of non-unique ID {:?} (name clash?)", source_name),
                );
                painter.error(
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

    // ---------------------------------------------------------------------

    /// Is the mouse over any Egui area?
    pub fn is_mouse_over_area(&self) -> bool {
        if let Some(mouse_pos) = self.input.mouse.pos {
            if let Some(layer) = self.layer_at(mouse_pos) {
                layer.order != Order::Background
            } else {
                false
            }
        } else {
            false
        }
    }

    /// True if Egui is currently interested in the mouse.
    /// Could be the mouse is hovering over a Egui window,
    /// or the user is dragging an Egui widget.
    /// If false, the mouse is outside of any Egui area and so
    /// you may be interested in what it is doing (e.g. controlling your game).
    pub fn wants_mouse_input(&self) -> bool {
        self.is_mouse_over_area() || self.is_using_mouse()
    }

    pub fn is_using_mouse(&self) -> bool {
        self.memory().interaction.is_using_mouse()
    }

    /// If true, Egui is currently listening on text input (e.g. typing text in a `TextEdit`).
    pub fn wants_keyboard_input(&self) -> bool {
        self.memory().interaction.kb_focus_id.is_some()
    }

    // ---------------------------------------------------------------------

    pub fn layer_at(&self, pos: Pos2) -> Option<Layer> {
        let resize_interact_radius_side = self.style().resize_interact_radius_side;
        self.memory().layer_at(pos, resize_interact_radius_side)
    }

    pub fn contains_mouse(&self, layer: Layer, clip_rect: Rect, rect: Rect) -> bool {
        let rect = rect.intersect(clip_rect);
        if let Some(mouse_pos) = self.input.mouse.pos {
            rect.contains(mouse_pos) && self.layer_at(mouse_pos) == Some(layer)
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
        sense: Sense,
    ) -> InteractInfo {
        let interact_rect = rect.expand2(0.5 * self.style().item_spacing); // make it easier to click. TODO: nice way to do this
        let hovered = self.contains_mouse(layer, clip_rect, interact_rect);
        let has_kb_focus = interaction_id
            .map(|id| self.memory().has_kb_focus(id))
            .unwrap_or(false);

        if interaction_id.is_none() || sense == Sense::nothing() {
            // Not interested in input:
            return InteractInfo {
                sense,
                rect,
                hovered,
                clicked: false,
                double_clicked: false,
                active: false,
                has_kb_focus,
            };
        }
        let interaction_id = interaction_id.unwrap();

        let mut memory = self.memory();

        memory.interaction.click_interest |= hovered && sense.click;
        memory.interaction.drag_interest |= hovered && sense.drag;

        let active = memory.interaction.click_id == Some(interaction_id)
            || memory.interaction.drag_id == Some(interaction_id);

        if self.input.mouse.pressed {
            if hovered {
                let mut info = InteractInfo {
                    sense,
                    rect,
                    hovered: true,
                    clicked: false,
                    double_clicked: false,
                    active: false,
                    has_kb_focus,
                };

                if sense.click && memory.interaction.click_id.is_none() {
                    // start of a click
                    memory.interaction.click_id = Some(interaction_id);
                    info.active = true;
                }

                if sense.drag
                    && (memory.interaction.drag_id.is_none() || memory.interaction.drag_is_window)
                {
                    // start of a drag
                    memory.interaction.drag_id = Some(interaction_id);
                    memory.interaction.drag_is_window = false;
                    memory.window_interaction = None; // HACK: stop moving windows (if any)
                    info.active = true;
                }

                info
            } else {
                // miss
                InteractInfo {
                    sense,
                    rect,
                    hovered,
                    clicked: false,
                    double_clicked: false,
                    active: false,
                    has_kb_focus,
                }
            }
        } else if self.input.mouse.released {
            let clicked = hovered && active;
            InteractInfo {
                sense,
                rect,
                hovered,
                clicked,
                double_clicked: clicked && self.input.mouse.double_click,
                active,
                has_kb_focus,
            }
        } else if self.input.mouse.down {
            InteractInfo {
                sense,
                rect,
                hovered: hovered && active,
                clicked: false,
                double_clicked: false,
                active,
                has_kb_focus,
            }
        } else {
            InteractInfo {
                sense,
                rect,
                hovered,
                clicked: false,
                double_clicked: false,
                active,
                has_kb_focus,
            }
        }
    }
}

/// ## Painting
impl Context {
    pub fn debug_painter(self: &Arc<Self>) -> Painter {
        Painter::new(self.clone(), Layer::debug(), self.rect())
    }
}

impl Context {
    pub fn settings_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        CollapsingHeader::new("Style")
            .default_open(false)
            .show(ui, |ui| {
                self.paint_options.lock().ui(ui);
                self.style_ui(ui);
            });

        CollapsingHeader::new("Fonts")
            .default_open(false)
            .show(ui, |ui| {
                let mut font_definitions = self.fonts().definitions().clone();
                font_definitions.ui(ui);
                self.fonts().texture().ui(ui);
                self.set_fonts(font_definitions);
            });
    }

    pub fn inspection_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        CollapsingHeader::new("Input")
            .default_open(true)
            .show(ui, |ui| ui.input().clone().ui(ui));

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
            .tooltip_text("Reset all Egui state")
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

impl Context {
    pub fn style_ui(&self, ui: &mut Ui) {
        let mut style = self.style();
        style.ui(ui);
        self.set_style(style);
    }
}

impl paint::PaintOptions {
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
        ui.add(label!("Jobs: {}", self.num_jobs))
            .tooltip_text("Number of separate clip rectanlges");
        ui.add(label!("Primitives: {}", self.num_primitives))
            .tooltip_text("Boxes, circles, text areas etc");
        ui.add(label!("Vertices: {}", self.num_vertices));
        ui.add(label!("Triangles: {}", self.num_triangles));
    }
}
