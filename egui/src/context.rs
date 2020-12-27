use std::sync::{
    atomic::{AtomicU32, Ordering::SeqCst},
    Arc,
};

use crate::{
    animation_manager::AnimationManager,
    mutex::{Mutex, MutexGuard},
    paint::{stats::*, *},
    *,
};

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct Options {
    /// The default style for new `Ui`:s.
    style: Arc<Style>,
    /// Controls the tessellator.
    tesselation_options: paint::TesselationOptions,
    /// Font sizes etc.
    font_definitions: FontDefinitions,
}

// ----------------------------------------------------------------------------

/// State that is collected during a frame and then cleared
#[derive(Clone)]
pub(crate) struct FrameState {
    /// Starts off as the screen_rect, shrinks as panels are added.
    /// The `CentralPanel` does not change this.
    /// This is the area available to Window's.
    available_rect: Rect,

    /// Starts off as the screen_rect, shrinks as panels are added.
    /// The `CentralPanel` retracts from this.
    unused_rect: Rect,

    /// How much space is used by panels.
    used_by_panels: Rect,
    // TODO: move some things from `Memory` to here
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            available_rect: Rect::invalid(),
            unused_rect: Rect::invalid(),
            used_by_panels: Rect::invalid(),
        }
    }
}

impl FrameState {
    pub fn begin_frame(&mut self, input: &InputState) {
        self.available_rect = input.screen_rect();
        self.unused_rect = input.screen_rect();
        self.used_by_panels = Rect::nothing();
    }

    /// How much space is still available after panels has been added.
    /// This is the "background" area, what Egui doesn't cover with panels (but may cover with windows).
    /// This is also the area to which windows are constrained.
    pub fn available_rect(&self) -> Rect {
        debug_assert!(
            self.available_rect.is_finite(),
            "Called `available_rect()` before `CtxRef::begin_frame()`"
        );
        self.available_rect
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_left_panel(&mut self, panel_rect: Rect) {
        debug_assert!(
            panel_rect.min == self.available_rect.min,
            "Mismatching panels. You must not create a panel from within another panel."
        );
        self.available_rect.min.x = panel_rect.max.x;
        self.unused_rect.min.x = panel_rect.max.x;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_top_panel(&mut self, panel_rect: Rect) {
        debug_assert!(
            panel_rect.min == self.available_rect.min,
            "Mismatching panels. You must not create a panel from within another panel."
        );
        self.available_rect.min.y = panel_rect.max.y;
        self.unused_rect.min.y = panel_rect.max.y;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    pub(crate) fn allocate_central_panel(&mut self, panel_rect: Rect) {
        // Note: we do not shrink `available_rect`, because
        // we allow windows to cover the CentralPanel.
        self.unused_rect = Rect::nothing(); // Nothing left unused after this
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }
}

// ----------------------------------------------------------------------------

/// A wrapper around [`Arc`](std::sync::Arc)`<`[`Context`]`>`.
/// This is how you will normally create and access a [`Context`].
#[derive(Clone)]
pub struct CtxRef(std::sync::Arc<Context>);

impl std::ops::Deref for CtxRef {
    type Target = Context;

    fn deref(&self) -> &Context {
        self.0.deref()
    }
}

impl AsRef<Context> for CtxRef {
    fn as_ref(&self) -> &Context {
        self.0.as_ref()
    }
}

impl std::borrow::Borrow<Context> for CtxRef {
    fn borrow(&self) -> &Context {
        self.0.borrow()
    }
}

impl std::cmp::PartialEq for CtxRef {
    fn eq(&self, other: &CtxRef) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for CtxRef {
    fn default() -> Self {
        Self(Arc::new(Context {
            // Start with painting an extra frame to compensate for some widgets
            // that take two frames before they "settle":
            repaint_requests: AtomicU32::new(1),
            ..Context::default()
        }))
    }
}

impl CtxRef {
    /// Call at the start of every frame.
    /// Put your widgets into a [`SidePanel`], [`TopPanel`], [`CentralPanel`], [`Window`] or [`Area`].
    pub fn begin_frame(&mut self, new_input: RawInput) {
        let mut self_: Context = (*self.0).clone();
        self_.begin_frame_mut(new_input);
        *self = Self(Arc::new(self_));
    }

    // ---------------------------------------------------------------------

    /// If the given [`Id`] is not unique, an error will be printed at the given position.
    /// Call this for [`Id`]:s that need interaction or persistence.
    pub(crate) fn register_interaction_id(&self, id: Id, new_pos: Pos2) {
        let prev_pos = self.memory().used_ids.insert(id, new_pos);
        if let Some(prev_pos) = prev_pos {
            if prev_pos.distance(new_pos) < 0.1 {
                // Likely same Widget being interacted with twice, which is fine.
                return;
            }

            let show_error = |pos: Pos2, text: String| {
                let painter = self.debug_painter();
                let rect = painter.error(pos, text);
                if let Some(mouse_pos) = self.input.mouse.pos {
                    if rect.contains(mouse_pos) {
                        painter.error(
                            rect.left_bottom() + vec2(2.0, 4.0),
                            "ID clashes happens when things like Windows or CollpasingHeaders share names,\n\
                             or when things like ScrollAreas and Resize areas aren't given unique id_source:s.",
                        );
                    }
                }
            };

            let id_str = id.short_debug_format();

            if prev_pos.distance(new_pos) < 4.0 {
                show_error(new_pos, format!("Double use of ID {}", id_str));
            } else {
                show_error(prev_pos, format!("First use of ID {}", id_str));
                show_error(new_pos, format!("Second use of ID {}", id_str));
            }

            // TODO: a tooltip explaining this.
        }
    }

    // ---------------------------------------------------------------------

    /// Use `ui.interact` instead
    pub(crate) fn interact(
        &self,
        clip_rect: Rect,
        item_spacing: Vec2,
        layer_id: LayerId,
        id: Id,
        rect: Rect,
        sense: Sense,
    ) -> Response {
        let interact_rect = rect.expand2((0.5 * item_spacing).min(Vec2::splat(5.0))); // make it easier to click
        let hovered = self.rect_contains_mouse(layer_id, clip_rect.intersect(interact_rect));
        self.interact_with_hovered(layer_id, id, rect, sense, hovered)
    }

    /// You specify if a thing is hovered, and the function gives a `Response`.
    pub(crate) fn interact_with_hovered(
        &self,
        layer_id: LayerId,
        id: Id,
        rect: Rect,
        sense: Sense,
        hovered: bool,
    ) -> Response {
        let has_kb_focus = self.memory().has_kb_focus(id);

        // If the the focus is lost after the call to interact,
        // this will be `false`, so `TextEdit` also sets this manually.
        let lost_kb_focus = self.memory().lost_kb_focus(id);

        if sense == Sense::hover() || !layer_id.allow_interaction() {
            // Not interested or allowed input:
            return Response {
                ctx: self.clone(),
                layer_id,
                id,
                rect,
                sense,
                hovered,
                clicked: false,
                double_clicked: false,
                active: false,
                has_kb_focus,
                lost_kb_focus,
            };
        }

        self.register_interaction_id(id, rect.min);

        let mut memory = self.memory();

        memory.interaction.click_interest |= hovered && sense.click;
        memory.interaction.drag_interest |= hovered && sense.drag;

        let active =
            memory.interaction.click_id == Some(id) || memory.interaction.drag_id == Some(id);

        if self.input.mouse.pressed {
            if hovered {
                let mut response = Response {
                    ctx: self.clone(),
                    layer_id,
                    id,
                    rect,
                    sense,
                    hovered: true,
                    clicked: false,
                    double_clicked: false,
                    active: false,
                    has_kb_focus,
                    lost_kb_focus,
                };

                if sense.click && memory.interaction.click_id.is_none() {
                    // start of a click
                    memory.interaction.click_id = Some(id);
                    response.active = true;
                }

                if sense.drag
                    && (memory.interaction.drag_id.is_none() || memory.interaction.drag_is_window)
                {
                    // start of a drag
                    memory.interaction.drag_id = Some(id);
                    memory.interaction.drag_is_window = false;
                    memory.window_interaction = None; // HACK: stop moving windows (if any)
                    response.active = true;
                }

                response
            } else {
                // miss
                Response {
                    ctx: self.clone(),
                    layer_id,
                    id,
                    rect,
                    sense,
                    hovered,
                    clicked: false,
                    double_clicked: false,
                    active: false,
                    has_kb_focus,
                    lost_kb_focus,
                }
            }
        } else if self.input.mouse.released {
            let clicked = hovered && active && self.input.mouse.could_be_click;
            Response {
                ctx: self.clone(),
                layer_id,
                id,
                rect,
                sense,
                hovered,
                clicked,
                double_clicked: clicked && self.input.mouse.double_click,
                active,
                has_kb_focus,
                lost_kb_focus,
            }
        } else if self.input.mouse.down {
            Response {
                ctx: self.clone(),
                layer_id,
                id,
                rect,
                sense,
                hovered: hovered && active,
                clicked: false,
                double_clicked: false,
                active,
                has_kb_focus,
                lost_kb_focus,
            }
        } else {
            Response {
                ctx: self.clone(),
                layer_id,
                id,
                rect,
                sense,
                hovered,
                clicked: false,
                double_clicked: false,
                active,
                has_kb_focus,
                lost_kb_focus,
            }
        }
    }

    pub fn debug_painter(&self) -> Painter {
        Painter::new(self.clone(), LayerId::debug(), self.input.screen_rect())
    }
}

// ----------------------------------------------------------------------------

/// This is the first thing you need when working with Egui. Create using [`CtxRef`].
///
/// Contains the [`InputState`], [`Memory`], [`Output`], options and more.
// TODO: too many mutexes. Maybe put it all behind one Mutex instead.
#[derive(Default)]
pub struct Context {
    options: Mutex<Options>,
    /// None until first call to `begin_frame`.
    fonts: Option<Arc<Fonts>>,
    memory: Arc<Mutex<Memory>>,
    animation_manager: Arc<Mutex<AnimationManager>>,

    input: InputState,

    /// State that is collected during a frame and then cleared
    frame_state: Mutex<FrameState>,

    // The output of a frame:
    graphics: Mutex<GraphicLayers>,
    output: Mutex<Output>,

    paint_stats: Mutex<PaintStats>,

    /// While positive, keep requesting repaints. Decrement at the end of each frame.
    repaint_requests: AtomicU32,
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Context {
            options: self.options.clone(),
            fonts: self.fonts.clone(),
            memory: self.memory.clone(),
            animation_manager: self.animation_manager.clone(),
            input: self.input.clone(),
            frame_state: self.frame_state.clone(),
            graphics: self.graphics.clone(),
            output: self.output.clone(),
            paint_stats: self.paint_stats.clone(),
            repaint_requests: self.repaint_requests.load(SeqCst).into(),
        }
    }
}

impl Context {
    #[allow(clippy::new_ret_no_self)]
    #[deprecated = "Use CtxRef::default() instead"]
    pub fn new() -> CtxRef {
        CtxRef::default()
    }

    /// How much space is still available after panels has been added.
    /// This is the "background" area, what Egui doesn't cover with panels (but may cover with windows).
    /// This is also the area to which windows are constrained.
    pub fn available_rect(&self) -> Rect {
        self.frame_state.lock().available_rect()
    }

    pub fn memory(&self) -> MutexGuard<'_, Memory> {
        self.memory.lock()
    }

    pub(crate) fn graphics(&self) -> MutexGuard<'_, GraphicLayers> {
        self.graphics.lock()
    }

    pub fn output(&self) -> MutexGuard<'_, Output> {
        self.output.lock()
    }

    pub(crate) fn frame_state(&self) -> MutexGuard<'_, FrameState> {
        self.frame_state.lock()
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

    /// Not valid until first call to [`CtxRef::begin_frame()`].
    /// That's because since we don't know the proper `pixels_per_point` until then.
    pub fn fonts(&self) -> &Fonts {
        &*self
            .fonts
            .as_ref()
            .expect("No fonts available until first call to CtxRef::begin_frame()")
    }

    /// The Egui texture, containing font characters etc.
    /// Not valid until first call to [`CtxRef::begin_frame()`].
    /// That's because since we don't know the proper `pixels_per_point` until then.
    pub fn texture(&self) -> Arc<paint::Texture> {
        self.fonts().texture()
    }

    /// Will become active at the start of the next frame.
    /// `pixels_per_point` will be ignored (overwritten at start of each frame with the contents of input)
    pub fn set_fonts(&self, font_definitions: FontDefinitions) {
        self.options.lock().font_definitions = font_definitions;
    }

    /// The [`Style`] used by all new windows, panels etc.
    pub fn style(&self) -> Arc<Style> {
        self.options.lock().style.clone()
    }

    /// The [`Style`] used by all new windows, panels etc.
    pub fn set_style(&self, style: impl Into<Arc<Style>>) {
        self.options.lock().style = style.into();
    }

    /// The number of physical pixels for each logical point.
    pub fn pixels_per_point(&self) -> f32 {
        self.input.pixels_per_point()
    }

    /// Useful for pixel-perfect rendering
    pub(crate) fn round_to_pixel(&self, point: f32) -> f32 {
        let pixels_per_point = self.pixels_per_point();
        (point * pixels_per_point).round() / pixels_per_point
    }

    /// Useful for pixel-perfect rendering
    pub(crate) fn round_pos_to_pixels(&self, pos: Pos2) -> Pos2 {
        pos2(self.round_to_pixel(pos.x), self.round_to_pixel(pos.y))
    }

    /// Useful for pixel-perfect rendering
    pub(crate) fn round_vec_to_pixels(&self, vec: Vec2) -> Vec2 {
        vec2(self.round_to_pixel(vec.x), self.round_to_pixel(vec.y))
    }

    /// Useful for pixel-perfect rendering
    pub(crate) fn round_rect_to_pixels(&self, rect: Rect) -> Rect {
        Rect {
            min: self.round_pos_to_pixels(rect.min),
            max: self.round_pos_to_pixels(rect.max),
        }
    }

    // ---------------------------------------------------------------------

    /// Constraint the position of a window/area
    /// so it fits within the screen.
    pub(crate) fn constrain_window_rect(&self, window: Rect) -> Rect {
        let mut screen = self.available_rect();

        if window.width() > screen.width() {
            // Allow overlapping side bars.
            // This is important for small screens, e.g. mobiles running the web demo.
            screen.max.x = self.input().screen_rect().max.x;
            screen.min.x = self.input().screen_rect().min.x;
        }
        if window.height() > screen.height() {
            // Allow overlapping top/bottom bars:
            screen.max.y = self.input().screen_rect().max.y;
            screen.min.y = self.input().screen_rect().min.y;
        }

        let mut pos = window.min;

        // Constrain to screen, unless window is too large to fit:
        let margin_x = (window.width() - screen.width()).at_least(0.0);
        let margin_y = (window.height() - screen.height()).at_least(0.0);

        pos.x = pos.x.at_most(screen.right() + margin_x - window.width()); // move left if needed
        pos.x = pos.x.at_least(screen.left() - margin_x); // move right if needed
        pos.y = pos.y.at_most(screen.bottom() + margin_y - window.height()); // move right if needed
        pos.y = pos.y.at_least(screen.top() - margin_y); // move down if needed

        pos = self.round_pos_to_pixels(pos);

        Rect::from_min_size(pos, window.size())
    }

    // ---------------------------------------------------------------------

    fn begin_frame_mut(&mut self, new_raw_input: RawInput) {
        self.memory().begin_frame(&self.input, &new_raw_input);

        self.input = std::mem::take(&mut self.input).begin_frame(new_raw_input);
        self.frame_state.lock().begin_frame(&self.input);

        let font_definitions = self.options.lock().font_definitions.clone();
        let pixels_per_point = self.input.pixels_per_point();
        let same_as_current = match &self.fonts {
            None => false,
            Some(fonts) => {
                *fonts.definitions() == font_definitions
                    && (fonts.pixels_per_point() - pixels_per_point).abs() < 1e-3
            }
        };
        if !same_as_current {
            self.fonts = Some(Arc::new(Fonts::from_definitions(
                pixels_per_point,
                font_definitions,
            )));
        }

        // Ensure we register the background area so panels and background ui can catch clicks:
        let screen_rect = self.input.screen_rect();
        self.memory().areas.set_state(
            LayerId::background(),
            containers::area::State {
                pos: screen_rect.min,
                size: screen_rect.size(),
                interactable: true,
            },
        );
    }

    /// Call at the end of each frame.
    /// Returns what has happened this frame (`Output`) as well as what you need to paint.
    /// You can transform the returned paint commands into triangles with a call to
    /// `Context::tesselate`.
    #[must_use]
    pub fn end_frame(&self) -> (Output, Vec<(Rect, PaintCmd)>) {
        if self.input.wants_repaint() {
            self.request_repaint();
        }

        self.memory().end_frame();

        let mut output: Output = std::mem::take(&mut self.output());
        if self.repaint_requests.load(SeqCst) > 0 {
            self.repaint_requests.fetch_sub(1, SeqCst);
            output.needs_repaint = true;
        }

        let paint_commands = self.drain_paint_lists();
        (output, paint_commands)
    }

    fn drain_paint_lists(&self) -> Vec<(Rect, PaintCmd)> {
        let memory = self.memory();
        self.graphics().drain(memory.areas.order()).collect()
    }

    /// Tesselate the given paint commands into triangle meshes.
    pub fn tesselate(&self, paint_commands: Vec<(Rect, PaintCmd)>) -> PaintJobs {
        let mut tesselation_options = self.options.lock().tesselation_options;
        tesselation_options.aa_size = 1.0 / self.pixels_per_point();
        let paint_stats = PaintStats::from_paint_commands(&paint_commands); // TODO: internal allocations
        let paint_jobs = tessellator::tessellate_paint_commands(
            paint_commands,
            tesselation_options,
            self.fonts(),
        );
        *self.paint_stats.lock() = paint_stats.with_paint_jobs(&paint_jobs);
        paint_jobs
    }

    // ---------------------------------------------------------------------

    /// How much space is used by panels and windows.
    pub fn used_rect(&self) -> Rect {
        let mut used = self.frame_state().used_by_panels;
        for window in self.memory().areas.visible_windows() {
            used = used.union(window.rect());
        }
        used
    }

    /// How much space is used by panels and windows.
    /// You can shrink your Egui area to this size and still fit all Egui components.
    pub fn used_size(&self) -> Vec2 {
        self.used_rect().max - Pos2::new(0.0, 0.0)
    }

    // ---------------------------------------------------------------------

    /// Is the mouse over any Egui area?
    pub fn is_mouse_over_area(&self) -> bool {
        if let Some(mouse_pos) = self.input.mouse.pos {
            if let Some(layer) = self.layer_id_at(mouse_pos) {
                if layer.order == Order::Background {
                    !self.frame_state().unused_rect.contains(mouse_pos)
                } else {
                    true
                }
            } else {
                false
            }
        } else {
            false
        }
    }

    /// True if Egui is currently interested in the mouse.
    /// Could be the mouse is hovering over a [`Window`] or the user is dragging a widget.
    /// If `false`, the mouse is outside of any Egui area and so
    /// you may be interested in what it is doing (e.g. controlling your game).
    /// Returns `false` if a drag started outside of Egui and then moved over an Egui area.
    pub fn wants_mouse_input(&self) -> bool {
        self.is_using_mouse() || (self.is_mouse_over_area() && !self.input().mouse.down)
    }

    /// Is Egui currently using the mouse position (e.g. dragging a slider).
    /// NOTE: this will return `false` if the mouse is just hovering over an Egui area.
    pub fn is_using_mouse(&self) -> bool {
        self.memory().interaction.is_using_mouse()
    }

    /// If `true`, Egui is currently listening on text input (e.g. typing text in a [`TextEdit`]).
    pub fn wants_keyboard_input(&self) -> bool {
        self.memory().interaction.kb_focus_id.is_some()
    }

    // ---------------------------------------------------------------------

    pub fn layer_id_at(&self, pos: Pos2) -> Option<LayerId> {
        let resize_grab_radius_side = self.style().interaction.resize_grab_radius_side;
        self.memory().layer_id_at(pos, resize_grab_radius_side)
    }

    pub(crate) fn rect_contains_mouse(&self, layer_id: LayerId, rect: Rect) -> bool {
        if let Some(mouse_pos) = self.input.mouse.pos {
            rect.contains(mouse_pos) && self.layer_id_at(mouse_pos) == Some(layer_id)
        } else {
            false
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
    /// The function will call [`Self::request_repaint()`] when appropriate.
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

impl Context {
    pub fn settings_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        CollapsingHeader::new("🎑 Style")
            .default_open(true)
            .show(ui, |ui| {
                self.style_ui(ui);
            });

        CollapsingHeader::new("🔠 Fonts")
            .default_open(false)
            .show(ui, |ui| {
                let mut font_definitions = self.fonts().definitions().clone();
                font_definitions.ui(ui);
                self.fonts().texture().ui(ui);
                self.set_fonts(font_definitions);
            });

        CollapsingHeader::new("✒ Painting")
            .default_open(true)
            .show(ui, |ui| {
                let mut tesselation_options = self.options.lock().tesselation_options;
                tesselation_options.ui(ui);
                self.options.lock().tesselation_options = tesselation_options;
            });
    }

    pub fn inspection_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        ui.label(format!("Is using mouse: {}", self.is_using_mouse()))
            .on_hover_text("Is Egui currently using the mouse actively (e.g. dragging a slider)?");
        ui.label(format!("Wants mouse input: {}", self.wants_mouse_input()))
            .on_hover_text("Is Egui currently interested in the location of the mouse (either because it is in use, or because it is hovering over a window).");
        ui.label(format!(
            "Wants keyboard input: {}",
            self.wants_keyboard_input()
        ))
        .on_hover_text("Is Egui currently listening for text input");
        ui.advance_cursor(16.0);

        CollapsingHeader::new("📥 Input")
            .default_open(false)
            .show(ui, |ui| ui.input().clone().ui(ui));

        CollapsingHeader::new("📊 Paint stats")
            .default_open(true)
            .show(ui, |ui| {
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
        ui.indent("areas", |ui| {
            let layers_ids: Vec<LayerId> = self.memory().areas.order().to_vec();
            for layer_id in layers_ids {
                let area = self.memory().areas.get(layer_id.id).cloned();
                if let Some(area) = area {
                    let is_visible = self.memory().areas.is_visible(&layer_id);
                    if ui
                        .label(format!(
                            "{:?} {:?} {}",
                            layer_id.order,
                            area.rect(),
                            if is_visible { "" } else { "(INVISIBLE)" }
                        ))
                        .hovered
                        && is_visible
                    {
                        ui.ctx()
                            .debug_painter()
                            .debug_rect(area.rect(), color::RED, "");
                    }
                }
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

impl paint::TesselationOptions {
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
