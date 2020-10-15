use std::sync::{
    atomic::{AtomicU32, Ordering::SeqCst},
    Arc,
};

use {
    ahash::AHashMap,
    parking_lot::{Mutex, MutexGuard},
};

use crate::{animation_manager::AnimationManager, paint::*, *};

#[derive(Clone, Copy, Default)]
struct PaintStats {
    num_jobs: usize,
    num_primitives: usize,
    num_vertices: usize,
    num_triangles: usize,
}

#[derive(Clone, Debug, Default)]
struct Options {
    /// The default style for new `Ui`:s.
    style: Arc<Style>,
    /// Controls the tessellator.
    paint_options: paint::PaintOptions,
    /// Font sizes etc.
    font_configuration: FontConfiguration,
}

/// Thi is the first thing you need when working with Egui.
///
/// Contains the input state, memory, options and output.
/// `Ui`:s keep an `Arc` pointer to this.
/// This allows us to create several child `Ui`:s at once,
/// all working against the same shared Context.
// TODO: too many mutexes. Maybe put it all behind one Mutex instead.
#[derive(Default)]
pub struct Context {
    options: Mutex<Options>,
    /// None until first call to `begin_frame`.
    fonts: Option<Arc<Mutex<Fonts>>>,
    memory: Arc<Mutex<Memory>>,
    animation_manager: Arc<Mutex<AnimationManager>>,

    input: InputState,

    // The output of a frame:
    graphics: Mutex<GraphicLayers>,
    output: Mutex<Output>,
    /// Used to debug name clashes of e.g. windows
    used_ids: Mutex<AHashMap<Id, Pos2>>,

    paint_stats: Mutex<PaintStats>,

    /// While positive, keep requesting repaints. Decrement at the end of each frame.
    repaint_requests: AtomicU32,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            options: Mutex::new(lock(&self.options, "options").clone()),
            fonts: self.fonts.clone(),
            memory: self.memory.clone(),
            animation_manager: self.animation_manager.clone(),
            input: self.input.clone(),
            graphics: Mutex::new(self.graphics.lock().clone()),
            output: Mutex::new(self.output.lock().clone()),
            used_ids: Mutex::new(self.used_ids.lock().clone()),
            paint_stats: Mutex::new(*self.paint_stats.lock()),
            repaint_requests: self.repaint_requests.load(SeqCst).into(),
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

    pub fn memory(&self) -> MutexGuard<'_, Memory> {
        lock(&self.memory, "memory")
    }

    pub fn graphics(&self) -> MutexGuard<'_, GraphicLayers> {
        lock(&self.graphics, "graphics")
    }

    pub fn output(&self) -> MutexGuard<'_, Output> {
        lock(&self.output, "output")
    }

    /// Call this if there is need to repaint the UI, i.e. if you are showing an animation.
    /// If this is called at least once in a frame, then there will be another frame right after this.
    /// Call as many times as you wish, only one repaint will be issued.
    pub fn request_repaint(&self) {
        // request two frames of repaint, just to cover some corner cases (frame delays):
        let times_to_repaint = 2;
        self.repaint_requests.store(times_to_repaint, SeqCst);
    }

    pub fn input(&self) -> &InputState {
        &self.input
    }

    /// Not valid until first call to `begin_frame()`
    /// That's because since we don't know the proper `pixels_per_point` until then.
    pub fn fonts(&self) -> Arc<Mutex<Fonts>> {
        self.fonts
            .as_ref()
            .expect("No fonts available until first call to Context::begin_frame()`")
            .clone()
    }

    /// The Egui texture, containing font characters etc..
    /// Not valid until first call to `begin_frame()`
    /// That's because since we don't know the proper `pixels_per_point` until then.
    pub fn texture(&self) -> Arc<paint::Texture> {
        self.fonts().lock().texture()
    }

    /// Will become active at the start of the next frame.
    /// `pixels_per_point` will be ignored (overwritten at start of each frame with the contents of input)
    pub fn set_fonts(&self, font_configuration: FontConfiguration) {
        lock(&self.options, "options").font_configuration = font_configuration;
    }

    pub fn style(&self) -> Arc<Style> {
        lock(&self.options, "options").style.clone()
    }

    pub fn set_style(&self, style: impl Into<Arc<Style>>) {
        lock(&self.options, "options").style = style.into();
    }

    pub fn pixels_per_point(&self) -> f32 {
        self.input.pixels_per_point()
    }

    /// Useful for pixel-perfect rendering
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        let pixels_per_point = self.pixels_per_point();
        (point * pixels_per_point).round() / pixels_per_point
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
        let mut font_configuration = lock(&self.options, "options").font_configuration.clone();
        font_configuration.pixels_per_point = self.input.pixels_per_point();
        let same_as_current = match &self.fonts {
            None => false,
            Some(fonts) => *fonts.lock().configuration() == font_configuration,
        };
        if !same_as_current {
            self.fonts = Some(Arc::new(Mutex::new(Fonts::from_definitions(
                font_configuration,
            ))));
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

        let mut output: Output = std::mem::take(&mut self.output());
        if self.repaint_requests.load(SeqCst) > 0 {
            self.repaint_requests.fetch_sub(1, SeqCst);
            output.needs_repaint = true;
        }

        let paint_jobs = self.paint();
        (output, paint_jobs)
    }

    fn drain_paint_lists(&self) -> Vec<(Rect, PaintCmd)> {
        let memory = self.memory();
        self.graphics().drain(memory.areas.order()).collect()
    }

    fn paint(&self) -> PaintJobs {
        let mut paint_options = lock(&self.options, "options").paint_options;
        paint_options.aa_size = 1.0 / self.pixels_per_point();
        let paint_commands = self.drain_paint_lists();
        let num_primitives = paint_commands.len();
        let paint_jobs =
            tessellator::tessellate_paint_commands(paint_commands, paint_options, self.fonts());

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
        let resize_grab_radius_side = self.style().interaction.resize_grab_radius_side;
        self.memory().layer_at(pos, resize_grab_radius_side)
    }

    pub fn contains_mouse(&self, layer: Layer, clip_rect: Rect, rect: Rect) -> bool {
        let rect = rect.intersect(clip_rect);
        if let Some(mouse_pos) = self.input.mouse.pos {
            rect.contains(mouse_pos) && self.layer_at(mouse_pos) == Some(layer)
        } else {
            false
        }
    }

    /// Use `ui.interact` instead
    pub(crate) fn interact(
        self: &Arc<Self>,
        layer: Layer,
        clip_rect: Rect,
        rect: Rect,
        interaction_id: Option<Id>,
        sense: Sense,
    ) -> Response {
        let interact_rect = rect.expand2(0.5 * self.style().spacing.item_spacing); // make it easier to click. TODO: nice way to do this
        let hovered = self.contains_mouse(layer, clip_rect, interact_rect);
        let has_kb_focus = interaction_id
            .map(|id| self.memory().has_kb_focus(id))
            .unwrap_or(false);

        if interaction_id.is_none() || sense == Sense::nothing() {
            // Not interested in input:
            return Response {
                ctx: self.clone(),
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
                let mut response = Response {
                    ctx: self.clone(),
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
                    response.active = true;
                }

                if sense.drag
                    && (memory.interaction.drag_id.is_none() || memory.interaction.drag_is_window)
                {
                    // start of a drag
                    memory.interaction.drag_id = Some(interaction_id);
                    memory.interaction.drag_is_window = false;
                    memory.window_interaction = None; // HACK: stop moving windows (if any)
                    response.active = true;
                }

                response
            } else {
                // miss
                Response {
                    ctx: self.clone(),
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
            let clicked = hovered && active && self.input.mouse.could_be_click;
            Response {
                ctx: self.clone(),
                sense,
                rect,
                hovered,
                clicked,
                double_clicked: clicked && self.input.mouse.double_click,
                active,
                has_kb_focus,
            }
        } else if self.input.mouse.down {
            Response {
                ctx: self.clone(),
                sense,
                rect,
                hovered: hovered && active,
                clicked: false,
                double_clicked: false,
                active,
                has_kb_focus,
            }
        } else {
            Response {
                ctx: self.clone(),
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

/// ## Animation
impl Context {
    /// Returns a value in the range [0, 1], to indicate "how on" this thing is.
    ///
    /// The first time called it will return `if value { 1.0 } else { 0.0 }`
    /// Calling this with `value = true` will always yield a number larger than zero, quickly going towards one.
    /// Calling this with `value = false` will always yield a number less than one, quickly going towards zero.
    ///
    /// The function will call `request_repaint()` when appropriate.
    pub fn animate_bool(&self, id: Id, value: bool) -> f32 {
        let animation_time = self.style().animation_time;
        let animated_value =
            self.animation_manager
                .lock()
                .animate_bool(&self.input, animation_time, id, value);
        let animation_in_progress = 0.0 < animated_value && animated_value < 1.0;
        if animation_in_progress {
            self.request_repaint();
        }
        animated_value
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
            .default_open(true)
            .show(ui, |ui| {
                self.style_ui(ui);
            });

        CollapsingHeader::new("Fonts")
            .default_open(false)
            .show(ui, |ui| {
                let mut font_configuration = self.fonts().lock().configuration().clone();
                font_configuration.ui(ui);
                let texture = self.fonts().lock().texture();
                texture.ui(ui);
                self.set_fonts(font_configuration);
            });

        CollapsingHeader::new("Painting")
            .default_open(true)
            .show(ui, |ui| {
                let mut paint_options = lock(&self.options, "options").paint_options;
                paint_options.ui(ui);
                lock(&self.options, "options").paint_options = paint_options;
            });
    }

    pub fn inspection_ui(&self, ui: &mut Ui) {
        use crate::containers::*;
        ui.style_mut().body_text_style = TextStyle::Monospace;

        CollapsingHeader::new("Input")
            .default_open(true)
            .show(ui, |ui| ui.input().clone().ui(ui));

        ui.collapsing("Stats", |ui| {
            ui.label(format!(
                "Screen size: {} x {} points, pixels_per_point: {:?}",
                ui.input().screen_size.x,
                ui.input().screen_size.y,
                ui.input().pixels_per_point,
            ));

            ui.heading("Painting:");
            self.paint_stats.lock().ui(ui);
        });
    }

    pub fn memory_ui(&self, ui: &mut crate::Ui) {
        if ui
            .button("Reset all")
            .on_hover_text("Reset all Egui state")
            .clicked
        {
            *self.memory() = Default::default();
        }

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} areas (window positions)",
                self.memory().areas.count()
            ));
            if ui.button("Reset").clicked {
                self.memory().areas = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} collapsing headers",
                self.memory().collapsing_headers.len()
            ));
            if ui.button("Reset").clicked {
                self.memory().collapsing_headers = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} menu bars", self.memory().menu_bar.len()));
            if ui.button("Reset").clicked {
                self.memory().menu_bar = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} scroll areas", self.memory().scroll_areas.len()));
            if ui.button("Reset").clicked {
                self.memory().scroll_areas = Default::default();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!("{} resize areas", self.memory().resize.len()));
            if ui.button("Reset").clicked {
                self.memory().resize = Default::default();
            }
        });

        ui.shrink_width_to_current(); // don't let the text below grow this window wider
        ui.label("NOTE: the position of this window cannot be reset from within itself.");
    }
}

impl Context {
    pub fn style_ui(&self, ui: &mut Ui) {
        let mut style: Style = (*self.style()).clone();
        style.ui(ui);
        self.set_style(style);
    }
}

impl paint::PaintOptions {
    pub fn ui(&mut self, ui: &mut Ui) {
        let Self {
            aa_size: _,
            anti_alias,
            coarse_tessellation_culling,
            debug_paint_clip_rects,
            debug_ignore_clip_rects,
        } = self;
        ui.checkbox(anti_alias, "Antialias");
        ui.checkbox(
            coarse_tessellation_culling,
            "Do coarse culling in the tessellator",
        );
        ui.checkbox(debug_paint_clip_rects, "Paint clip rectangles (debug)");
        ui.checkbox(debug_ignore_clip_rects, "Ignore clip rectangles (debug)");
    }
}

impl PaintStats {
    pub fn ui(&self, ui: &mut Ui) {
        ui.label(format!("Jobs: {}", self.num_jobs))
            .on_hover_text("Number of separate clip rectangles");
        ui.label(format!("Primitives: {}", self.num_primitives))
            .on_hover_text("Boxes, circles, text areas etc");
        ui.label(format!("Vertices: {}", self.num_vertices));
        ui.label(format!("Triangles: {}", self.num_triangles));
    }
}

#[cfg(debug_assertions)]
fn lock<'m, T>(mutex: &'m Mutex<T>, what: &'static str) -> MutexGuard<'m, T> {
    // TODO: detect if we are trying to lock the same mutex *from the same thread*.
    // at the moment we just panic on any double-locking of a mutex (so no multithreaded support in debug builds)
    mutex
        .try_lock()
        .unwrap_or_else(|| panic!("The Mutex for {} is already locked. Probably a bug", what))
}

#[cfg(not(debug_assertions))]
fn lock<'m, T>(mutex: &'m Mutex<T>, _what: &'static str) -> MutexGuard<'m, T> {
    mutex.lock()
}
