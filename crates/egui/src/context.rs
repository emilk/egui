#![warn(missing_docs)] // Let's keep `Context` well-documented.

use std::{borrow::Cow, cell::RefCell, panic::Location, sync::Arc, time::Duration};

use epaint::{
    emath::TSTransform, mutex::*, stats::*, text::Fonts, util::OrderedFloat, TessellationOptions, *,
};

use crate::{
    animation_manager::AnimationManager,
    data::output::PlatformOutput,
    frame_state::FrameState,
    input_state::*,
    layers::GraphicLayers,
    load::{Bytes, Loaders, SizedTexture},
    memory::Options,
    os::OperatingSystem,
    output::FullOutput,
    util::IdTypeMap,
    viewport::ViewportClass,
    TextureHandle, ViewportCommand, *,
};

use self::{hit_test::WidgetHits, interaction::InteractionSnapshot};

/// Information given to the backend about when it is time to repaint the ui.
///
/// This is given in the callback set by [`Context::set_request_repaint_callback`].
#[derive(Clone, Copy, Debug)]
pub struct RequestRepaintInfo {
    /// This is used to specify what viewport that should repaint.
    pub viewport_id: ViewportId,

    /// Repaint after this duration. If zero, repaint as soon as possible.
    pub delay: Duration,

    /// The current frame number.
    ///
    /// This can be compared to [`Context::frame_nr`] to see if we've already
    /// triggered the painting of the next frame.
    pub current_frame_nr: u64,
}

// ----------------------------------------------------------------------------

thread_local! {
    static IMMEDIATE_VIEWPORT_RENDERER: RefCell<Option<Box<ImmediateViewportRendererCallback>>> = Default::default();
}

// ----------------------------------------------------------------------------

struct WrappedTextureManager(Arc<RwLock<epaint::TextureManager>>);

impl Default for WrappedTextureManager {
    fn default() -> Self {
        let mut tex_mngr = epaint::textures::TextureManager::default();

        // Will be filled in later
        let font_id = tex_mngr.alloc(
            "egui_font_texture".into(),
            epaint::FontImage::new([0, 0]).into(),
            Default::default(),
        );
        assert_eq!(font_id, TextureId::default());

        Self(Arc::new(RwLock::new(tex_mngr)))
    }
}

// ----------------------------------------------------------------------------

/// Generic event callback.
pub type ContextCallback = Arc<dyn Fn(&Context) + Send + Sync>;

#[derive(Clone)]
struct NamedContextCallback {
    debug_name: &'static str,
    callback: ContextCallback,
}

/// Callbacks that users can register
#[derive(Clone, Default)]
struct Plugins {
    pub on_begin_frame: Vec<NamedContextCallback>,
    pub on_end_frame: Vec<NamedContextCallback>,
}

impl Plugins {
    fn call(ctx: &Context, _cb_name: &str, callbacks: &[NamedContextCallback]) {
        crate::profile_scope!("plugins", _cb_name);
        for NamedContextCallback {
            debug_name: _name,
            callback,
        } in callbacks
        {
            crate::profile_scope!("plugin", _name);
            (callback)(ctx);
        }
    }

    fn on_begin_frame(&self, ctx: &Context) {
        Self::call(ctx, "on_begin_frame", &self.on_begin_frame);
    }

    fn on_end_frame(&self, ctx: &Context) {
        Self::call(ctx, "on_end_frame", &self.on_end_frame);
    }
}

// ----------------------------------------------------------------------------

/// Repaint-logic
impl ContextImpl {
    /// This is where we update the repaint logic.
    fn begin_frame_repaint_logic(&mut self, viewport_id: ViewportId) {
        let viewport = self.viewports.entry(viewport_id).or_default();

        std::mem::swap(
            &mut viewport.repaint.prev_causes,
            &mut viewport.repaint.causes,
        );
        viewport.repaint.causes.clear();

        viewport.repaint.prev_frame_paint_delay = viewport.repaint.repaint_delay;

        if viewport.repaint.outstanding == 0 {
            // We are repainting now, so we can wait a while for the next repaint.
            viewport.repaint.repaint_delay = Duration::MAX;
        } else {
            viewport.repaint.repaint_delay = Duration::ZERO;
            viewport.repaint.outstanding -= 1;
            if let Some(callback) = &self.request_repaint_callback {
                (callback)(RequestRepaintInfo {
                    viewport_id,
                    delay: Duration::ZERO,
                    current_frame_nr: viewport.repaint.frame_nr,
                });
            }
        }
    }

    fn request_repaint(&mut self, viewport_id: ViewportId, cause: RepaintCause) {
        self.request_repaint_after(Duration::ZERO, viewport_id, cause);
    }

    fn request_repaint_after(
        &mut self,
        delay: Duration,
        viewport_id: ViewportId,
        cause: RepaintCause,
    ) {
        let viewport = self.viewports.entry(viewport_id).or_default();

        if delay == Duration::ZERO {
            // Each request results in two repaints, just to give some things time to settle.
            // This solves some corner-cases of missing repaints on frame-delayed responses.
            viewport.repaint.outstanding = 1;
        } else {
            // For non-zero delays, we only repaint once, because
            // otherwise we would just schedule an immediate repaint _now_,
            // which would then clear the delay and repaint again.
            // Hovering a tooltip is a good example of a case where we want to repaint after a delay.
        }

        viewport.repaint.causes.push(cause);

        // We save some CPU time by only calling the callback if we need to.
        // If the new delay is greater or equal to the previous lowest,
        // it means we have already called the callback, and don't need to do it again.
        if delay < viewport.repaint.repaint_delay {
            viewport.repaint.repaint_delay = delay;

            if let Some(callback) = &self.request_repaint_callback {
                (callback)(RequestRepaintInfo {
                    viewport_id,
                    delay,
                    current_frame_nr: viewport.repaint.frame_nr,
                });
            }
        }
    }

    #[must_use]
    fn requested_immediate_repaint_prev_frame(&self, viewport_id: &ViewportId) -> bool {
        self.viewports.get(viewport_id).map_or(false, |v| {
            v.repaint.requested_immediate_repaint_prev_frame()
        })
    }

    #[must_use]
    fn has_requested_repaint(&self, viewport_id: &ViewportId) -> bool {
        self.viewports.get(viewport_id).map_or(false, |v| {
            0 < v.repaint.outstanding || v.repaint.repaint_delay < Duration::MAX
        })
    }
}

// ----------------------------------------------------------------------------

/// State stored per viewport
#[derive(Default)]
struct ViewportState {
    /// The type of viewport.
    ///
    /// This will never be [`ViewportClass::Embedded`],
    /// since those don't result in real viewports.
    class: ViewportClass,

    /// The latest delta
    builder: ViewportBuilder,

    /// The user-code that shows the GUI, used for deferred viewports.
    ///
    /// `None` for immediate viewports.
    viewport_ui_cb: Option<Arc<DeferredViewportUiCallback>>,

    input: InputState,

    /// State that is collected during a frame and then cleared
    frame_state: FrameState,

    /// Has this viewport been updated this frame?
    used: bool,

    /// Written to during the frame.
    widgets_this_frame: WidgetRects,

    /// Read
    widgets_prev_frame: WidgetRects,

    /// State related to repaint scheduling.
    repaint: ViewportRepaintInfo,

    // ----------------------
    // Updated at the start of the frame:
    //
    /// Which widgets are under the pointer?
    hits: WidgetHits,

    /// What widgets are being interacted with this frame?
    ///
    /// Based on the widgets from last frame, and input in this frame.
    interact_widgets: InteractionSnapshot,

    // ----------------------
    // The output of a frame:
    //
    graphics: GraphicLayers,
    // Most of the things in `PlatformOutput` are not actually viewport dependent.
    output: PlatformOutput,
    commands: Vec<ViewportCommand>,
}

/// What called [`Context::request_repaint`]?
#[derive(Clone)]
pub struct RepaintCause {
    /// What file had the call that requested the repaint?
    pub file: &'static str,

    /// What line number of the the call that requested the repaint?
    pub line: u32,
}

impl std::fmt::Debug for RepaintCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

impl RepaintCause {
    /// Capture the file and line number of the call site.
    #[allow(clippy::new_without_default)]
    #[track_caller]
    pub fn new() -> Self {
        let caller = Location::caller();
        Self {
            file: caller.file(),
            line: caller.line(),
        }
    }
}

impl std::fmt::Display for RepaintCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file, self.line)
    }
}

/// Per-viewport state related to repaint scheduling.
struct ViewportRepaintInfo {
    /// Monotonically increasing counter.
    frame_nr: u64,

    /// The duration which the backend will poll for new events
    /// before forcing another egui update, even if there's no new events.
    ///
    /// Also used to suppress multiple calls to the repaint callback during the same frame.
    ///
    /// This is also returned in [`crate::ViewportOutput`].
    repaint_delay: Duration,

    /// While positive, keep requesting repaints. Decrement at the start of each frame.
    outstanding: u8,

    /// What caused repaints during this frame?
    causes: Vec<RepaintCause>,

    /// What triggered a repaint the previous frame?
    /// (i.e: why are we updating now?)
    prev_causes: Vec<RepaintCause>,

    /// What was the output of `repaint_delay` on the previous frame?
    ///
    /// If this was zero, we are repainting as quickly as possible
    /// (as far as we know).
    prev_frame_paint_delay: Duration,
}

impl Default for ViewportRepaintInfo {
    fn default() -> Self {
        Self {
            frame_nr: 0,

            // We haven't scheduled a repaint yet.
            repaint_delay: Duration::MAX,

            // Let's run a couple of frames at the start, because why not.
            outstanding: 1,

            causes: Default::default(),
            prev_causes: Default::default(),

            prev_frame_paint_delay: Duration::MAX,
        }
    }
}

impl ViewportRepaintInfo {
    pub fn requested_immediate_repaint_prev_frame(&self) -> bool {
        self.prev_frame_paint_delay == Duration::ZERO
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct ContextImpl {
    /// Since we could have multiple viewports across multiple monitors with
    /// different `pixels_per_point`, we need a `Fonts` instance for each unique
    /// `pixels_per_point`.
    /// This is because the `Fonts` depend on `pixels_per_point` for the font atlas
    /// as well as kerning, font sizes, etc.
    fonts: std::collections::BTreeMap<OrderedFloat<f32>, Fonts>,
    font_definitions: FontDefinitions,

    memory: Memory,
    animation_manager: AnimationManager,

    plugins: Plugins,

    /// All viewports share the same texture manager and texture namespace.
    ///
    /// In all viewports, [`TextureId::default`] is special, and points to the font atlas.
    /// The font-atlas texture _may_ be different across viewports, as they may have different
    /// `pixels_per_point`, so we do special book-keeping for that.
    /// See <https://github.com/emilk/egui/issues/3664>.
    tex_manager: WrappedTextureManager,

    /// Set during the frame, becomes active at the start of the next frame.
    new_zoom_factor: Option<f32>,

    os: OperatingSystem,

    /// How deeply nested are we?
    viewport_stack: Vec<ViewportIdPair>,

    /// What is the last viewport rendered?
    last_viewport: ViewportId,

    paint_stats: PaintStats,

    request_repaint_callback: Option<Box<dyn Fn(RequestRepaintInfo) + Send + Sync>>,

    viewport_parents: ViewportIdMap<ViewportId>,
    viewports: ViewportIdMap<ViewportState>,

    embed_viewports: bool,

    #[cfg(feature = "accesskit")]
    is_accesskit_enabled: bool,
    #[cfg(feature = "accesskit")]
    accesskit_node_classes: accesskit::NodeClassSet,

    loaders: Arc<Loaders>,
}

impl ContextImpl {
    fn begin_frame_mut(&mut self, mut new_raw_input: RawInput) {
        let viewport_id = new_raw_input.viewport_id;
        let parent_id = new_raw_input
            .viewports
            .get(&viewport_id)
            .and_then(|v| v.parent)
            .unwrap_or_default();
        let ids = ViewportIdPair::from_self_and_parent(viewport_id, parent_id);

        let is_outermost_viewport = self.viewport_stack.is_empty(); // not necessarily root, just outermost immediate viewport
        self.viewport_stack.push(ids);

        self.begin_frame_repaint_logic(viewport_id);

        let viewport = self.viewports.entry(viewport_id).or_default();

        if is_outermost_viewport {
            if let Some(new_zoom_factor) = self.new_zoom_factor.take() {
                let ratio = self.memory.options.zoom_factor / new_zoom_factor;
                self.memory.options.zoom_factor = new_zoom_factor;

                let input = &viewport.input;
                // This is a bit hacky, but is required to avoid jitter:
                let mut rect = input.screen_rect;
                rect.min = (ratio * rect.min.to_vec2()).to_pos2();
                rect.max = (ratio * rect.max.to_vec2()).to_pos2();
                new_raw_input.screen_rect = Some(rect);
                // We should really scale everything else in the input too,
                // but the `screen_rect` is the most important part.
            }
        }
        let native_pixels_per_point = new_raw_input
            .viewport()
            .native_pixels_per_point
            .unwrap_or(1.0);
        let pixels_per_point = self.memory.options.zoom_factor * native_pixels_per_point;

        let all_viewport_ids: ViewportIdSet = self.all_viewport_ids();

        let viewport = self.viewports.entry(self.viewport_id()).or_default();

        self.memory.begin_frame(&new_raw_input, &all_viewport_ids);

        viewport.input = std::mem::take(&mut viewport.input).begin_frame(
            new_raw_input,
            viewport.repaint.requested_immediate_repaint_prev_frame(),
            pixels_per_point,
        );

        let screen_rect = viewport.input.screen_rect;

        viewport.frame_state.begin_frame(screen_rect);

        {
            let area_order = self.memory.areas().order_map();

            let mut layers: Vec<LayerId> = viewport.widgets_prev_frame.layer_ids().collect();

            layers.sort_by(|a, b| {
                if a.order == b.order {
                    // Maybe both are windows, so respect area order:
                    area_order.get(a).cmp(&area_order.get(b))
                } else {
                    // comparing e.g. background to tooltips
                    a.order.cmp(&b.order)
                }
            });

            viewport.hits = if let Some(pos) = viewport.input.pointer.interact_pos() {
                let interact_radius = self.memory.options.style.interaction.interact_radius;

                crate::hit_test::hit_test(
                    &viewport.widgets_prev_frame,
                    &layers,
                    &self.memory.layer_transforms,
                    pos,
                    interact_radius,
                )
            } else {
                WidgetHits::default()
            };

            viewport.interact_widgets = crate::interaction::interact(
                &viewport.interact_widgets,
                &viewport.widgets_prev_frame,
                &viewport.hits,
                &viewport.input,
                self.memory.interaction_mut(),
            );
        }

        // Ensure we register the background area so panels and background ui can catch clicks:
        self.memory.areas_mut().set_state(
            LayerId::background(),
            containers::area::State {
                pivot_pos: screen_rect.left_top(),
                pivot: Align2::LEFT_TOP,
                size: screen_rect.size(),
                interactable: true,
            },
        );

        #[cfg(feature = "accesskit")]
        if self.is_accesskit_enabled {
            crate::profile_scope!("accesskit");
            use crate::frame_state::AccessKitFrameState;
            let id = crate::accesskit_root_id();
            let mut builder = accesskit::NodeBuilder::new(accesskit::Role::Window);
            let pixels_per_point = viewport.input.pixels_per_point();
            builder.set_transform(accesskit::Affine::scale(pixels_per_point.into()));
            let mut node_builders = IdMap::default();
            node_builders.insert(id, builder);
            viewport.frame_state.accesskit_state = Some(AccessKitFrameState {
                node_builders,
                parent_stack: vec![id],
            });
        }

        self.update_fonts_mut();
    }

    /// Load fonts unless already loaded.
    fn update_fonts_mut(&mut self) {
        crate::profile_function!();

        let input = &self.viewport().input;
        let pixels_per_point = input.pixels_per_point();
        let max_texture_side = input.max_texture_side;

        if let Some(font_definitions) = self.memory.new_font_definitions.take() {
            // New font definition loaded, so we need to reload all fonts.
            self.fonts.clear();
            self.font_definitions = font_definitions;
            #[cfg(feature = "log")]
            log::debug!("Loading new font definitions");
        }

        let mut is_new = false;

        let fonts = self
            .fonts
            .entry(pixels_per_point.into())
            .or_insert_with(|| {
                #[cfg(feature = "log")]
                log::trace!("Creating new Fonts for pixels_per_point={pixels_per_point}");

                is_new = true;
                crate::profile_scope!("Fonts::new");
                Fonts::new(
                    pixels_per_point,
                    max_texture_side,
                    self.font_definitions.clone(),
                )
            });

        {
            crate::profile_scope!("Fonts::begin_frame");
            fonts.begin_frame(pixels_per_point, max_texture_side);
        }

        if is_new && self.memory.options.preload_font_glyphs {
            crate::profile_scope!("preload_font_glyphs");
            // Preload the most common characters for the most common fonts.
            // This is not very important to do, but may save a few GPU operations.
            for font_id in self.memory.options.style.text_styles.values() {
                fonts.lock().fonts.font(font_id).preload_common_characters();
            }
        }
    }

    #[cfg(feature = "accesskit")]
    fn accesskit_node_builder(&mut self, id: Id) -> &mut accesskit::NodeBuilder {
        let state = self
            .viewport()
            .frame_state
            .accesskit_state
            .as_mut()
            .unwrap();
        let builders = &mut state.node_builders;
        if let std::collections::hash_map::Entry::Vacant(entry) = builders.entry(id) {
            entry.insert(Default::default());
            let parent_id = state.parent_stack.last().unwrap();
            let parent_builder = builders.get_mut(parent_id).unwrap();
            parent_builder.push_child(id.accesskit_id());
        }
        builders.get_mut(&id).unwrap()
    }

    fn pixels_per_point(&mut self) -> f32 {
        self.viewport().input.pixels_per_point
    }

    /// Return the `ViewportId` of the current viewport.
    ///
    /// For the root viewport this will return [`ViewportId::ROOT`].
    pub(crate) fn viewport_id(&self) -> ViewportId {
        self.viewport_stack.last().copied().unwrap_or_default().this
    }

    /// Return the `ViewportId` of his parent.
    ///
    /// For the root viewport this will return [`ViewportId::ROOT`].
    pub(crate) fn parent_viewport_id(&self) -> ViewportId {
        let viewport_id = self.viewport_id();
        *self
            .viewport_parents
            .get(&viewport_id)
            .unwrap_or(&ViewportId::ROOT)
    }

    fn all_viewport_ids(&self) -> ViewportIdSet {
        self.viewports
            .keys()
            .copied()
            .chain([ViewportId::ROOT])
            .collect()
    }

    /// The current active viewport
    pub(crate) fn viewport(&mut self) -> &mut ViewportState {
        self.viewports.entry(self.viewport_id()).or_default()
    }

    fn viewport_for(&mut self, viewport_id: ViewportId) -> &mut ViewportState {
        self.viewports.entry(viewport_id).or_default()
    }
}

// ----------------------------------------------------------------------------

/// Your handle to egui.
///
/// This is the first thing you need when working with egui.
/// Contains the [`InputState`], [`Memory`], [`PlatformOutput`], and more.
///
/// [`Context`] is cheap to clone, and any clones refers to the same mutable data
/// ([`Context`] uses refcounting internally).
///
/// ## Locking
/// All methods are marked `&self`; [`Context`] has interior mutability protected by an [`RwLock`].
///
/// To access parts of a `Context` you need to use some of the helper functions that take closures:
///
/// ```
/// # let ctx = egui::Context::default();
/// if ctx.input(|i| i.key_pressed(egui::Key::A)) {
///     ctx.output_mut(|o| o.copied_text = "Hello!".to_string());
/// }
/// ```
///
/// Within such a closure you may NOT recursively lock the same [`Context`], as that can lead to a deadlock.
/// Therefore it is important that any lock of [`Context`] is short-lived.
///
/// These are effectively transactional accesses.
///
/// [`Ui`] has many of the same accessor functions, and the same applies there.
///
/// ## Example:
///
/// ``` no_run
/// # fn handle_platform_output(_: egui::PlatformOutput) {}
/// # fn paint(textures_delta: egui::TexturesDelta, _: Vec<egui::ClippedPrimitive>) {}
/// let mut ctx = egui::Context::default();
///
/// // Game loop:
/// loop {
///     let raw_input = egui::RawInput::default();
///     let full_output = ctx.run(raw_input, |ctx| {
///         egui::CentralPanel::default().show(&ctx, |ui| {
///             ui.label("Hello world!");
///             if ui.button("Click me").clicked() {
///                 // take some action here
///             }
///         });
///     });
///     handle_platform_output(full_output.platform_output);
///     let clipped_primitives = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
///     paint(full_output.textures_delta, clipped_primitives);
/// }
/// ```
#[derive(Clone)]
pub struct Context(Arc<RwLock<ContextImpl>>);

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").finish_non_exhaustive()
    }
}

impl std::cmp::PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for Context {
    fn default() -> Self {
        let ctx_impl = ContextImpl {
            embed_viewports: true,
            ..Default::default()
        };
        let ctx = Self(Arc::new(RwLock::new(ctx_impl)));

        // Register built-in plugins:
        crate::debug_text::register(&ctx);
        crate::text_selection::LabelSelectionState::register(&ctx);
        crate::DragAndDrop::register(&ctx);

        ctx
    }
}

impl Context {
    /// Do read-only (shared access) transaction on Context
    fn read<R>(&self, reader: impl FnOnce(&ContextImpl) -> R) -> R {
        reader(&self.0.read())
    }

    /// Do read-write (exclusive access) transaction on Context
    fn write<R>(&self, writer: impl FnOnce(&mut ContextImpl) -> R) -> R {
        writer(&mut self.0.write())
    }

    /// Run the ui code for one frame.
    ///
    /// Put your widgets into a [`SidePanel`], [`TopBottomPanel`], [`CentralPanel`], [`Window`] or [`Area`].
    ///
    /// This will modify the internal reference to point to a new generation of [`Context`].
    /// Any old clones of this [`Context`] will refer to the old [`Context`], which will not get new input.
    ///
    /// You can alternatively run [`Self::begin_frame`] and [`Context::end_frame`].
    ///
    /// ```
    /// // One egui context that you keep reusing:
    /// let mut ctx = egui::Context::default();
    ///
    /// // Each frame:
    /// let input = egui::RawInput::default();
    /// let full_output = ctx.run(input, |ctx| {
    ///     egui::CentralPanel::default().show(&ctx, |ui| {
    ///         ui.label("Hello egui!");
    ///     });
    /// });
    /// // handle full_output
    /// ```
    #[must_use]
    pub fn run(&self, new_input: RawInput, run_ui: impl FnOnce(&Self)) -> FullOutput {
        crate::profile_function!();

        self.begin_frame(new_input);
        run_ui(self);
        self.end_frame()
    }

    /// An alternative to calling [`Self::run`].
    ///
    /// ```
    /// // One egui context that you keep reusing:
    /// let mut ctx = egui::Context::default();
    ///
    /// // Each frame:
    /// let input = egui::RawInput::default();
    /// ctx.begin_frame(input);
    ///
    /// egui::CentralPanel::default().show(&ctx, |ui| {
    ///     ui.label("Hello egui!");
    /// });
    ///
    /// let full_output = ctx.end_frame();
    /// // handle full_output
    /// ```
    pub fn begin_frame(&self, new_input: RawInput) {
        crate::profile_function!();
        self.read(|ctx| ctx.plugins.clone()).on_begin_frame(self);
        self.write(|ctx| ctx.begin_frame_mut(new_input));
    }
}

/// ## Borrows parts of [`Context`]
/// These functions all lock the [`Context`].
/// Please see the documentation of [`Context`] for how locking works!
impl Context {
    /// Read-only access to [`InputState`].
    ///
    /// Note that this locks the [`Context`].
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.input(|i| {
    ///     // ⚠️ Using `ctx` (even from other `Arc` reference) again here will lead to a deadlock!
    /// });
    ///
    /// if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
    ///     // This is fine!
    /// }
    /// ```
    #[inline]
    pub fn input<R>(&self, reader: impl FnOnce(&InputState) -> R) -> R {
        self.write(move |ctx| reader(&ctx.viewport().input))
    }

    /// This will create a `InputState::default()` if there is no input state for that viewport
    #[inline]
    pub fn input_for<R>(&self, id: ViewportId, reader: impl FnOnce(&InputState) -> R) -> R {
        self.write(move |ctx| reader(&ctx.viewport_for(id).input))
    }

    /// Read-write access to [`InputState`].
    #[inline]
    pub fn input_mut<R>(&self, writer: impl FnOnce(&mut InputState) -> R) -> R {
        self.input_mut_for(self.viewport_id(), writer)
    }

    /// This will create a `InputState::default()` if there is no input state for that viewport
    #[inline]
    pub fn input_mut_for<R>(&self, id: ViewportId, writer: impl FnOnce(&mut InputState) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.viewport_for(id).input))
    }

    /// Read-only access to [`Memory`].
    #[inline]
    pub fn memory<R>(&self, reader: impl FnOnce(&Memory) -> R) -> R {
        self.read(move |ctx| reader(&ctx.memory))
    }

    /// Read-write access to [`Memory`].
    #[inline]
    pub fn memory_mut<R>(&self, writer: impl FnOnce(&mut Memory) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.memory))
    }

    /// Read-only access to [`IdTypeMap`], which stores superficial widget state.
    #[inline]
    pub fn data<R>(&self, reader: impl FnOnce(&IdTypeMap) -> R) -> R {
        self.read(move |ctx| reader(&ctx.memory.data))
    }

    /// Read-write access to [`IdTypeMap`], which stores superficial widget state.
    #[inline]
    pub fn data_mut<R>(&self, writer: impl FnOnce(&mut IdTypeMap) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.memory.data))
    }

    /// Read-write access to [`GraphicLayers`], where painted [`crate::Shape`]s are written to.
    #[inline]
    pub fn graphics_mut<R>(&self, writer: impl FnOnce(&mut GraphicLayers) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.viewport().graphics))
    }

    /// Read-only access to [`GraphicLayers`], where painted [`crate::Shape`]s are written to.
    #[inline]
    pub fn graphics<R>(&self, reader: impl FnOnce(&GraphicLayers) -> R) -> R {
        self.write(move |ctx| reader(&ctx.viewport().graphics))
    }

    /// Read-only access to [`PlatformOutput`].
    ///
    /// This is what egui outputs each frame.
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::Progress);
    /// ```
    #[inline]
    pub fn output<R>(&self, reader: impl FnOnce(&PlatformOutput) -> R) -> R {
        self.write(move |ctx| reader(&ctx.viewport().output))
    }

    /// Read-write access to [`PlatformOutput`].
    #[inline]
    pub fn output_mut<R>(&self, writer: impl FnOnce(&mut PlatformOutput) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.viewport().output))
    }

    /// Read-only access to [`FrameState`].
    #[inline]
    pub(crate) fn frame_state<R>(&self, reader: impl FnOnce(&FrameState) -> R) -> R {
        self.write(move |ctx| reader(&ctx.viewport().frame_state))
    }

    /// Read-write access to [`FrameState`].
    #[inline]
    pub(crate) fn frame_state_mut<R>(&self, writer: impl FnOnce(&mut FrameState) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.viewport().frame_state))
    }

    /// Read-only access to [`Fonts`].
    ///
    /// Not valid until first call to [`Context::run()`].
    /// That's because since we don't know the proper `pixels_per_point` until then.
    #[inline]
    pub fn fonts<R>(&self, reader: impl FnOnce(&Fonts) -> R) -> R {
        self.write(move |ctx| {
            let pixels_per_point = ctx.pixels_per_point();
            reader(
                ctx.fonts
                    .get(&pixels_per_point.into())
                    .expect("No fonts available until first call to Context::run()"),
            )
        })
    }

    /// Read-only access to [`Options`].
    #[inline]
    pub fn options<R>(&self, reader: impl FnOnce(&Options) -> R) -> R {
        self.read(move |ctx| reader(&ctx.memory.options))
    }

    /// Read-write access to [`Options`].
    #[inline]
    pub fn options_mut<R>(&self, writer: impl FnOnce(&mut Options) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.memory.options))
    }

    /// Read-only access to [`TessellationOptions`].
    #[inline]
    pub fn tessellation_options<R>(&self, reader: impl FnOnce(&TessellationOptions) -> R) -> R {
        self.read(move |ctx| reader(&ctx.memory.options.tessellation_options))
    }

    /// Read-write access to [`TessellationOptions`].
    #[inline]
    pub fn tessellation_options_mut<R>(
        &self,
        writer: impl FnOnce(&mut TessellationOptions) -> R,
    ) -> R {
        self.write(move |ctx| writer(&mut ctx.memory.options.tessellation_options))
    }

    /// If the given [`Id`] has been used previously the same frame at at different position,
    /// then an error will be printed on screen.
    ///
    /// This function is already called for all widgets that do any interaction,
    /// but you can call this from widgets that store state but that does not interact.
    ///
    /// The given [`Rect`] should be approximately where the widget will be.
    /// The most important thing is that [`Rect::min`] is approximately correct,
    /// because that's where the warning will be painted. If you don't know what size to pick, just pick [`Vec2::ZERO`].
    pub fn check_for_id_clash(&self, id: Id, new_rect: Rect, what: &str) {
        let prev_rect = self.frame_state_mut(move |state| state.used_ids.insert(id, new_rect));

        if !self.options(|opt| opt.warn_on_id_clash) {
            return;
        }

        let Some(prev_rect) = prev_rect else { return };

        // it is ok to reuse the same ID for e.g. a frame around a widget,
        // or to check for interaction with the same widget twice:
        let is_same_rect = prev_rect.expand(0.1).contains_rect(new_rect)
            || new_rect.expand(0.1).contains_rect(prev_rect);
        if is_same_rect {
            return;
        }

        let show_error = |widget_rect: Rect, text: String| {
            let screen_rect = self.screen_rect();

            let text = format!("🔥 {text}");
            let color = self.style().visuals.error_fg_color;
            let painter = self.debug_painter();
            painter.rect_stroke(widget_rect, 0.0, (1.0, color));

            let below = widget_rect.bottom() + 32.0 < screen_rect.bottom();

            let text_rect = if below {
                painter.debug_text(
                    widget_rect.left_bottom() + vec2(0.0, 2.0),
                    Align2::LEFT_TOP,
                    color,
                    text,
                )
            } else {
                painter.debug_text(
                    widget_rect.left_top() - vec2(0.0, 2.0),
                    Align2::LEFT_BOTTOM,
                    color,
                    text,
                )
            };

            if let Some(pointer_pos) = self.pointer_hover_pos() {
                if text_rect.contains(pointer_pos) {
                    let tooltip_pos = if below {
                        text_rect.left_bottom() + vec2(2.0, 4.0)
                    } else {
                        text_rect.left_top() + vec2(2.0, -4.0)
                    };

                    painter.error(
                        tooltip_pos,
                        format!("Widget is {} this text.\n\n\
                             ID clashes happens when things like Windows or CollapsingHeaders share names,\n\
                             or when things like Plot and Grid:s aren't given unique id_source:s.\n\n\
                             Sometimes the solution is to use ui.push_id.",
                         if below { "above" } else { "below" })
                    );
                }
            }
        };

        let id_str = id.short_debug_format();

        if prev_rect.min.distance(new_rect.min) < 4.0 {
            show_error(new_rect, format!("Double use of {what} ID {id_str}"));
        } else {
            show_error(prev_rect, format!("First use of {what} ID {id_str}"));
            show_error(new_rect, format!("Second use of {what} ID {id_str}"));
        }
    }

    // ---------------------------------------------------------------------

    /// Create a widget and check for interaction.
    ///
    /// If this is not called, the widget doesn't exist.
    ///
    /// You should use [`Ui::interact`] instead.
    ///
    /// If the widget already exists, its state (sense, Rect, etc) will be updated.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create_widget(&self, w: WidgetRect) -> Response {
        // Remember this widget
        self.write(|ctx| {
            let viewport = ctx.viewport();

            // We add all widgets here, even non-interactive ones,
            // because we need this list not only for checking for blocking widgets,
            // but also to know when we have reached the widget we are checking for cover.
            viewport.widgets_this_frame.insert(w.layer_id, w);

            if w.sense.focusable {
                ctx.memory.interested_in_focus(w.id);
            }
        });

        if !w.enabled || !w.sense.focusable || !w.layer_id.allow_interaction() {
            // Not interested or allowed input:
            self.memory_mut(|mem| mem.surrender_focus(w.id));
        }

        if w.sense.interactive() || w.sense.focusable {
            self.check_for_id_clash(w.id, w.rect, "widget");
        }

        #[allow(clippy::let_and_return)]
        let res = self.get_response(w);

        #[cfg(feature = "accesskit")]
        if w.sense.focusable {
            // Make sure anything that can receive focus has an AccessKit node.
            // TODO(mwcampbell): For nodes that are filled from widget info,
            // some information is written to the node twice.
            self.accesskit_node_builder(w.id, |builder| res.fill_accesskit_node_common(builder));
        }

        res
    }

    /// Read the response of some widget, which may be called _before_ creating the widget (!).
    ///
    /// This is because widget interaction happens at the start of the frame, using the previous frame's widgets.
    ///
    /// If the widget was not visible the previous frame (or this frame), this will return `None`.
    pub fn read_response(&self, id: Id) -> Option<Response> {
        self.write(|ctx| {
            let viewport = ctx.viewport();
            viewport
                .widgets_this_frame
                .get(id)
                .or_else(|| viewport.widgets_prev_frame.get(id))
                .copied()
        })
        .map(|widget_rect| self.get_response(widget_rect))
    }

    /// Returns `true` if the widget with the given `Id` contains the pointer.
    #[deprecated = "Use Response.contains_pointer or Context::read_response instead"]
    pub fn widget_contains_pointer(&self, id: Id) -> bool {
        self.read_response(id)
            .map_or(false, |response| response.contains_pointer)
    }

    /// Do all interaction for an existing widget, without (re-)registering it.
    fn get_response(&self, widget_rect: WidgetRect) -> Response {
        let WidgetRect {
            id,
            layer_id,
            rect,
            interact_rect,
            sense,
            enabled,
        } = widget_rect;

        let highlighted = self.frame_state(|fs| fs.highlight_this_frame.contains(&id));

        let mut res = Response {
            ctx: self.clone(),
            layer_id,
            id,
            rect,
            interact_rect,
            sense,
            enabled,
            contains_pointer: false,
            hovered: false,
            highlighted,
            clicked: false,
            fake_primary_click: false,
            long_touched: false,
            drag_started: false,
            dragged: false,
            drag_stopped: false,
            is_pointer_button_down_on: false,
            interact_pointer_pos: None,
            changed: false,
        };

        self.write(|ctx| {
            let viewport = ctx.viewports.entry(ctx.viewport_id()).or_default();

            res.contains_pointer = viewport.interact_widgets.contains_pointer.contains(&id);

            let input = &viewport.input;
            let memory = &mut ctx.memory;

            if enabled
                && sense.click
                && memory.has_focus(id)
                && (input.key_pressed(Key::Space) || input.key_pressed(Key::Enter))
            {
                // Space/enter works like a primary click for e.g. selected buttons
                res.fake_primary_click = true;
            }

            #[cfg(feature = "accesskit")]
            if enabled
                && sense.click
                && input.has_accesskit_action_request(id, accesskit::Action::Default)
            {
                res.fake_primary_click = true;
            }

            if enabled && sense.click && Some(id) == viewport.interact_widgets.long_touched {
                res.long_touched = true;
            }

            let interaction = memory.interaction();

            res.is_pointer_button_down_on = interaction.potential_click_id == Some(id)
                || interaction.potential_drag_id == Some(id);

            if res.enabled {
                res.hovered = viewport.interact_widgets.hovered.contains(&id);
                res.dragged = Some(id) == viewport.interact_widgets.dragged;
                res.drag_started = Some(id) == viewport.interact_widgets.drag_started;
                res.drag_stopped = Some(id) == viewport.interact_widgets.drag_stopped;
            }

            let clicked = Some(id) == viewport.interact_widgets.clicked;
            let mut any_press = false;

            for pointer_event in &input.pointer.pointer_events {
                match pointer_event {
                    PointerEvent::Moved(_) => {}
                    PointerEvent::Pressed { .. } => {
                        any_press = true;
                    }
                    PointerEvent::Released { click, .. } => {
                        if enabled && sense.click && clicked && click.is_some() {
                            res.clicked = true;
                        }

                        res.is_pointer_button_down_on = false;
                        res.dragged = false;
                    }
                }
            }

            // is_pointer_button_down_on is false when released, but we want interact_pointer_pos
            // to still work.
            let is_interacted_with =
                res.is_pointer_button_down_on || res.long_touched || clicked || res.drag_stopped;
            if is_interacted_with {
                res.interact_pointer_pos = input.pointer.interact_pos();
                if let (Some(transform), Some(pos)) = (
                    memory.layer_transforms.get(&res.layer_id),
                    &mut res.interact_pointer_pos,
                ) {
                    *pos = transform.inverse() * *pos;
                }
            }

            if input.pointer.any_down() && !is_interacted_with {
                // We don't hover widgets while interacting with *other* widgets:
                res.hovered = false;
            }

            let pointer_pressed_elsewhere = any_press && !res.hovered;
            if pointer_pressed_elsewhere && memory.has_focus(id) {
                memory.surrender_focus(id);
            }
        });

        res
    }

    /// Get a full-screen painter for a new or existing layer
    pub fn layer_painter(&self, layer_id: LayerId) -> Painter {
        let screen_rect = self.screen_rect();
        Painter::new(self.clone(), layer_id, screen_rect)
    }

    /// Paint on top of everything else
    pub fn debug_painter(&self) -> Painter {
        Self::layer_painter(self, LayerId::debug())
    }

    /// Print this text next to the cursor at the end of the frame.
    ///
    /// If you call this multiple times, the text will be appended.
    ///
    /// This only works if compiled with `debug_assertions`.
    ///
    /// ```
    /// # let ctx = egui::Context::default();
    /// # let state = true;
    /// ctx.debug_text(format!("State: {state:?}"));
    /// ```
    ///
    /// This is just a convenience for calling [`crate::debug_text::print`].
    #[track_caller]
    pub fn debug_text(&self, text: impl Into<WidgetText>) {
        crate::debug_text::print(self, text);
    }

    /// What operating system are we running on?
    ///
    /// When compiling natively, this is
    /// figured out from the `target_os`.
    ///
    /// For web, this can be figured out from the user-agent,
    /// and is done so by [`eframe`](https://github.com/emilk/egui/tree/master/crates/eframe).
    pub fn os(&self) -> OperatingSystem {
        self.read(|ctx| ctx.os)
    }

    /// Set the operating system we are running on.
    ///
    /// If you are writing wasm-based integration for egui you
    /// may want to set this based on e.g. the user-agent.
    pub fn set_os(&self, os: OperatingSystem) {
        self.write(|ctx| ctx.os = os);
    }

    /// Set the cursor icon.
    ///
    /// Equivalent to:
    /// ```
    /// # let ctx = egui::Context::default();
    /// ctx.output_mut(|o| o.cursor_icon = egui::CursorIcon::PointingHand);
    /// ```
    pub fn set_cursor_icon(&self, cursor_icon: CursorIcon) {
        self.output_mut(|o| o.cursor_icon = cursor_icon);
    }

    /// Open an URL in a browser.
    ///
    /// Equivalent to:
    /// ```
    /// # let ctx = egui::Context::default();
    /// # let open_url = egui::OpenUrl::same_tab("http://www.example.com");
    /// ctx.output_mut(|o| o.open_url = Some(open_url));
    /// ```
    pub fn open_url(&self, open_url: crate::OpenUrl) {
        self.output_mut(|o| o.open_url = Some(open_url));
    }

    /// Copy the given text to the system clipboard.
    ///
    /// Empty strings are ignored.
    ///
    /// Equivalent to:
    /// ```
    /// # let ctx = egui::Context::default();
    /// ctx.output_mut(|o| o.copied_text = "Copy this".to_owned());
    /// ```
    pub fn copy_text(&self, text: String) {
        self.output_mut(|o| o.copied_text = text);
    }

    /// Format the given shortcut in a human-readable way (e.g. `Ctrl+Shift+X`).
    ///
    /// Can be used to get the text for [`Button::shortcut_text`].
    pub fn format_shortcut(&self, shortcut: &KeyboardShortcut) -> String {
        let os = self.os();

        let is_mac = matches!(os, OperatingSystem::Mac | OperatingSystem::IOS);

        let can_show_symbols = || {
            let ModifierNames {
                alt,
                ctrl,
                shift,
                mac_cmd,
                ..
            } = ModifierNames::SYMBOLS;

            let font_id = TextStyle::Body.resolve(&self.style());
            self.fonts(|f| {
                let mut lock = f.lock();
                let font = lock.fonts.font(&font_id);
                font.has_glyphs(alt)
                    && font.has_glyphs(ctrl)
                    && font.has_glyphs(shift)
                    && font.has_glyphs(mac_cmd)
            })
        };

        if is_mac && can_show_symbols() {
            shortcut.format(&ModifierNames::SYMBOLS, is_mac)
        } else {
            shortcut.format(&ModifierNames::NAMES, is_mac)
        }
    }

    /// The current frame number for the current viewport.
    ///
    /// Starts at zero, and is incremented at the end of [`Self::run`] or by [`Self::end_frame`].
    ///
    /// Between calls to [`Self::run`], this is the frame number of the coming frame.
    pub fn frame_nr(&self) -> u64 {
        self.frame_nr_for(self.viewport_id())
    }

    /// The current frame number.
    ///
    /// Starts at zero, and is incremented at the end of [`Self::run`] or by [`Self::end_frame`].
    ///
    /// Between calls to [`Self::run`], this is the frame number of the coming frame.
    pub fn frame_nr_for(&self, id: ViewportId) -> u64 {
        self.read(|ctx| ctx.viewports.get(&id).map_or(0, |v| v.repaint.frame_nr))
    }

    /// Call this if there is need to repaint the UI, i.e. if you are showing an animation.
    ///
    /// If this is called at least once in a frame, then there will be another frame right after this.
    /// Call as many times as you wish, only one repaint will be issued.
    ///
    /// To request repaint with a delay, use [`Self::request_repaint_after`].
    ///
    /// If called from outside the UI thread, the UI thread will wake up and run,
    /// provided the egui integration has set that up via [`Self::set_request_repaint_callback`]
    /// (this will work on `eframe`).
    ///
    /// This will repaint the current viewport.
    #[track_caller]
    pub fn request_repaint(&self) {
        self.request_repaint_of(self.viewport_id());
    }

    /// Call this if there is need to repaint the UI, i.e. if you are showing an animation.
    ///
    /// If this is called at least once in a frame, then there will be another frame right after this.
    /// Call as many times as you wish, only one repaint will be issued.
    ///
    /// To request repaint with a delay, use [`Self::request_repaint_after_for`].
    ///
    /// If called from outside the UI thread, the UI thread will wake up and run,
    /// provided the egui integration has set that up via [`Self::set_request_repaint_callback`]
    /// (this will work on `eframe`).
    ///
    /// This will repaint the specified viewport.
    #[track_caller]
    pub fn request_repaint_of(&self, id: ViewportId) {
        let cause = RepaintCause::new();
        self.write(|ctx| ctx.request_repaint(id, cause));
    }

    /// Request repaint after at most the specified duration elapses.
    ///
    /// The backend can chose to repaint sooner, for instance if some other code called
    /// this method with a lower duration, or if new events arrived.
    ///
    /// The function can be multiple times, but only the *smallest* duration will be considered.
    /// So, if the function is called two times with `1 second` and `2 seconds`, egui will repaint
    /// after `1 second`
    ///
    /// This is primarily useful for applications who would like to save battery by avoiding wasted
    /// redraws when the app is not in focus. But sometimes the GUI of the app might become stale
    /// and outdated if it is not updated for too long.
    ///
    /// Let's say, something like a stopwatch widget that displays the time in seconds. You would waste
    /// resources repainting multiple times within the same second (when you have no input),
    /// just calculate the difference of duration between current time and next second change,
    /// and call this function, to make sure that you are displaying the latest updated time, but
    /// not wasting resources on needless repaints within the same second.
    ///
    /// ### Quirk:
    /// Duration begins at the next frame. Let's say for example that it's a very inefficient app
    /// and takes 500 milliseconds per frame at 2 fps. The widget / user might want a repaint in
    /// next 500 milliseconds. Now, app takes 1000 ms per frame (1 fps) because the backend event
    /// timeout takes 500 milliseconds AFTER the vsync swap buffer.
    /// So, it's not that we are requesting repaint within X duration. We are rather timing out
    /// during app idle time where we are not receiving any new input events.
    ///
    /// This repaints the current viewport
    #[track_caller]
    pub fn request_repaint_after(&self, duration: Duration) {
        self.request_repaint_after_for(duration, self.viewport_id());
    }

    /// Request repaint after at most the specified duration elapses.
    ///
    /// The backend can chose to repaint sooner, for instance if some other code called
    /// this method with a lower duration, or if new events arrived.
    ///
    /// The function can be multiple times, but only the *smallest* duration will be considered.
    /// So, if the function is called two times with `1 second` and `2 seconds`, egui will repaint
    /// after `1 second`
    ///
    /// This is primarily useful for applications who would like to save battery by avoiding wasted
    /// redraws when the app is not in focus. But sometimes the GUI of the app might become stale
    /// and outdated if it is not updated for too long.
    ///
    /// Let's say, something like a stopwatch widget that displays the time in seconds. You would waste
    /// resources repainting multiple times within the same second (when you have no input),
    /// just calculate the difference of duration between current time and next second change,
    /// and call this function, to make sure that you are displaying the latest updated time, but
    /// not wasting resources on needless repaints within the same second.
    ///
    /// ### Quirk:
    /// Duration begins at the next frame. Let's say for example that it's a very inefficient app
    /// and takes 500 milliseconds per frame at 2 fps. The widget / user might want a repaint in
    /// next 500 milliseconds. Now, app takes 1000 ms per frame (1 fps) because the backend event
    /// timeout takes 500 milliseconds AFTER the vsync swap buffer.
    /// So, it's not that we are requesting repaint within X duration. We are rather timing out
    /// during app idle time where we are not receiving any new input events.
    ///
    /// This repaints the specified viewport
    #[track_caller]
    pub fn request_repaint_after_for(&self, duration: Duration, id: ViewportId) {
        let cause = RepaintCause::new();
        self.write(|ctx| ctx.request_repaint_after(duration, id, cause));
    }

    /// Was a repaint requested last frame for the current viewport?
    #[must_use]
    pub fn requested_repaint_last_frame(&self) -> bool {
        self.requested_repaint_last_frame_for(&self.viewport_id())
    }

    /// Was a repaint requested last frame for the given viewport?
    #[must_use]
    pub fn requested_repaint_last_frame_for(&self, viewport_id: &ViewportId) -> bool {
        self.read(|ctx| ctx.requested_immediate_repaint_prev_frame(viewport_id))
    }

    /// Has a repaint been requested for the current viewport?
    #[must_use]
    pub fn has_requested_repaint(&self) -> bool {
        self.has_requested_repaint_for(&self.viewport_id())
    }

    /// Has a repaint been requested for the given viewport?
    #[must_use]
    pub fn has_requested_repaint_for(&self, viewport_id: &ViewportId) -> bool {
        self.read(|ctx| ctx.has_requested_repaint(viewport_id))
    }

    /// Why are we repainting?
    ///
    /// This can be helpful in debugging why egui is constantly repainting.
    pub fn repaint_causes(&self) -> Vec<RepaintCause> {
        self.read(|ctx| {
            ctx.viewports
                .get(&ctx.viewport_id())
                .map(|v| v.repaint.prev_causes.clone())
        })
        .unwrap_or_default()
    }

    /// For integrations: this callback will be called when an egui user calls [`Self::request_repaint`] or [`Self::request_repaint_after`].
    ///
    /// This lets you wake up a sleeping UI thread.
    ///
    /// Note that only one callback can be set. Any new call overrides the previous callback.
    pub fn set_request_repaint_callback(
        &self,
        callback: impl Fn(RequestRepaintInfo) + Send + Sync + 'static,
    ) {
        let callback = Box::new(callback);
        self.write(|ctx| ctx.request_repaint_callback = Some(callback));
    }
}

/// Callbacks
impl Context {
    /// Call the given callback at the start of each frame
    /// of each viewport.
    ///
    /// This can be used for egui _plugins_.
    /// See [`crate::debug_text`] for an example.
    pub fn on_begin_frame(&self, debug_name: &'static str, cb: ContextCallback) {
        let named_cb = NamedContextCallback {
            debug_name,
            callback: cb,
        };
        self.write(|ctx| ctx.plugins.on_begin_frame.push(named_cb));
    }

    /// Call the given callback at the end of each frame
    /// of each viewport.
    ///
    /// This can be used for egui _plugins_.
    /// See [`crate::debug_text`] for an example.
    pub fn on_end_frame(&self, debug_name: &'static str, cb: ContextCallback) {
        let named_cb = NamedContextCallback {
            debug_name,
            callback: cb,
        };
        self.write(|ctx| ctx.plugins.on_end_frame.push(named_cb));
    }
}

impl Context {
    /// Tell `egui` which fonts to use.
    ///
    /// The default `egui` fonts only support latin and cyrillic alphabets,
    /// but you can call this to install additional fonts that support e.g. korean characters.
    ///
    /// The new fonts will become active at the start of the next frame.
    pub fn set_fonts(&self, font_definitions: FontDefinitions) {
        crate::profile_function!();

        let pixels_per_point = self.pixels_per_point();

        let mut update_fonts = true;

        self.read(|ctx| {
            if let Some(current_fonts) = ctx.fonts.get(&pixels_per_point.into()) {
                // NOTE: this comparison is expensive since it checks TTF data for equality
                if current_fonts.lock().fonts.definitions() == &font_definitions {
                    update_fonts = false; // no need to update
                }
            }
        });

        if update_fonts {
            self.memory_mut(|mem| mem.new_font_definitions = Some(font_definitions));
        }
    }

    /// The [`Style`] used by all subsequent windows, panels etc.
    pub fn style(&self) -> Arc<Style> {
        self.options(|opt| opt.style.clone())
    }

    /// Mutate the [`Style`] used by all subsequent windows, panels etc.
    ///
    /// Example:
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.style_mut(|style| {
    ///     style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    /// });
    /// ```
    pub fn style_mut(&self, mutate_style: impl FnOnce(&mut Style)) {
        self.options_mut(|opt| mutate_style(std::sync::Arc::make_mut(&mut opt.style)));
    }

    /// The [`Style`] used by all new windows, panels etc.
    ///
    /// You can also change this using [`Self::style_mut]`
    ///
    /// You can use [`Ui::style_mut`] to change the style of a single [`Ui`].
    pub fn set_style(&self, style: impl Into<Arc<Style>>) {
        self.options_mut(|opt| opt.style = style.into());
    }

    /// The [`Visuals`] used by all subsequent windows, panels etc.
    ///
    /// You can also use [`Ui::visuals_mut`] to change the visuals of a single [`Ui`].
    ///
    /// Example:
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.set_visuals(egui::Visuals::light()); // Switch to light mode
    /// ```
    pub fn set_visuals(&self, visuals: crate::Visuals) {
        self.options_mut(|opt| std::sync::Arc::make_mut(&mut opt.style).visuals = visuals);
    }

    /// The number of physical pixels for each logical point.
    ///
    /// This is calculated as [`Self::zoom_factor`] * [`Self::native_pixels_per_point`]
    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.input(|i| i.pixels_per_point)
    }

    /// Set the number of physical pixels for each logical point.
    /// Will become active at the start of the next frame.
    ///
    /// This will actually translate to a call to [`Self::set_zoom_factor`].
    pub fn set_pixels_per_point(&self, pixels_per_point: f32) {
        if pixels_per_point != self.pixels_per_point() {
            self.set_zoom_factor(pixels_per_point / self.native_pixels_per_point().unwrap_or(1.0));
        }
    }

    /// The number of physical pixels for each logical point on this monitor.
    ///
    /// This is given as input to egui via [`ViewportInfo::native_pixels_per_point`]
    /// and cannot be changed.
    #[inline(always)]
    pub fn native_pixels_per_point(&self) -> Option<f32> {
        self.input(|i| i.viewport().native_pixels_per_point)
    }

    /// Global zoom factor of the UI.
    ///
    /// This is used to calculate the `pixels_per_point`
    /// for the UI as `pixels_per_point = zoom_fator * native_pixels_per_point`.
    ///
    /// The default is 1.0.
    /// Make larger to make everything larger.
    #[inline(always)]
    pub fn zoom_factor(&self) -> f32 {
        self.options(|o| o.zoom_factor)
    }

    /// Sets zoom factor of the UI.
    /// Will become active at the start of the next frame.
    ///
    /// Note that calling this will not update [`Self::zoom_factor`] until the end of the frame.
    ///
    /// This is used to calculate the `pixels_per_point`
    /// for the UI as `pixels_per_point = zoom_fator * native_pixels_per_point`.
    ///
    /// The default is 1.0.
    /// Make larger to make everything larger.
    ///
    /// It is better to call this than modifying
    /// [`Options::zoom_factor`].
    #[inline(always)]
    pub fn set_zoom_factor(&self, zoom_factor: f32) {
        let cause = RepaintCause::new();
        self.write(|ctx| {
            if ctx.memory.options.zoom_factor != zoom_factor {
                ctx.new_zoom_factor = Some(zoom_factor);
                for viewport_id in ctx.all_viewport_ids() {
                    ctx.request_repaint(viewport_id, cause.clone());
                }
            }
        });
    }

    /// Useful for pixel-perfect rendering
    #[inline]
    pub(crate) fn round_to_pixel(&self, point: f32) -> f32 {
        let pixels_per_point = self.pixels_per_point();
        (point * pixels_per_point).round() / pixels_per_point
    }

    /// Useful for pixel-perfect rendering
    #[inline]
    pub(crate) fn round_pos_to_pixels(&self, pos: Pos2) -> Pos2 {
        pos2(self.round_to_pixel(pos.x), self.round_to_pixel(pos.y))
    }

    /// Useful for pixel-perfect rendering
    #[inline]
    pub(crate) fn round_vec_to_pixels(&self, vec: Vec2) -> Vec2 {
        vec2(self.round_to_pixel(vec.x), self.round_to_pixel(vec.y))
    }

    /// Useful for pixel-perfect rendering
    #[inline]
    pub(crate) fn round_rect_to_pixels(&self, rect: Rect) -> Rect {
        Rect {
            min: self.round_pos_to_pixels(rect.min),
            max: self.round_pos_to_pixels(rect.max),
        }
    }

    /// Allocate a texture.
    ///
    /// This is for advanced users.
    /// Most users should use [`crate::Ui::image`] or [`Self::try_load_texture`]
    /// instead.
    ///
    /// In order to display an image you must convert it to a texture using this function.
    /// The function will hand over the image data to the egui backend, which will
    /// upload it to the GPU.
    ///
    /// ⚠️ Make sure to only call this ONCE for each image, i.e. NOT in your main GUI code.
    /// The call is NOT immediate safe.
    ///
    /// The given name can be useful for later debugging, and will be visible if you call [`Self::texture_ui`].
    ///
    /// For how to load an image, see [`ImageData`] and [`ColorImage::from_rgba_unmultiplied`].
    ///
    /// ```
    /// struct MyImage {
    ///     texture: Option<egui::TextureHandle>,
    /// }
    ///
    /// impl MyImage {
    ///     fn ui(&mut self, ui: &mut egui::Ui) {
    ///         let texture: &egui::TextureHandle = self.texture.get_or_insert_with(|| {
    ///             // Load the texture only once.
    ///             ui.ctx().load_texture(
    ///                 "my-image",
    ///                 egui::ColorImage::example(),
    ///                 Default::default()
    ///             )
    ///         });
    ///
    ///         // Show the image:
    ///         ui.image((texture.id(), texture.size_vec2()));
    ///     }
    /// }
    /// ```
    ///
    /// See also [`crate::ImageData`], [`crate::Ui::image`] and [`crate::Image`].
    pub fn load_texture(
        &self,
        name: impl Into<String>,
        image: impl Into<ImageData>,
        options: TextureOptions,
    ) -> TextureHandle {
        let name = name.into();
        let image = image.into();
        let max_texture_side = self.input(|i| i.max_texture_side);
        crate::egui_assert!(
            image.width() <= max_texture_side && image.height() <= max_texture_side,
            "Texture {:?} has size {}x{}, but the maximum texture side is {}",
            name,
            image.width(),
            image.height(),
            max_texture_side
        );
        let tex_mngr = self.tex_manager();
        let tex_id = tex_mngr.write().alloc(name, image, options);
        TextureHandle::new(tex_mngr, tex_id)
    }

    /// Low-level texture manager.
    ///
    /// In general it is easier to use [`Self::load_texture`] and [`TextureHandle`].
    ///
    /// You can show stats about the allocated textures using [`Self::texture_ui`].
    pub fn tex_manager(&self) -> Arc<RwLock<epaint::textures::TextureManager>> {
        self.read(|ctx| ctx.tex_manager.0.clone())
    }

    // ---------------------------------------------------------------------

    /// Constrain the position of a window/area so it fits within the provided boundary.
    ///
    /// If area is `None`, will constrain to [`Self::available_rect`].
    pub(crate) fn constrain_window_rect_to_area(&self, window: Rect, area: Option<Rect>) -> Rect {
        let mut area = area.unwrap_or_else(|| self.available_rect());

        if window.width() > area.width() {
            // Allow overlapping side bars.
            // This is important for small screens, e.g. mobiles running the web demo.
            let screen_rect = self.screen_rect();
            (area.min.x, area.max.x) = (screen_rect.min.x, screen_rect.max.x);
        }
        if window.height() > area.height() {
            // Allow overlapping top/bottom bars:
            let screen_rect = self.screen_rect();
            (area.min.y, area.max.y) = (screen_rect.min.y, screen_rect.max.y);
        }

        let mut pos = window.min;

        // Constrain to screen, unless window is too large to fit:
        let margin_x = (window.width() - area.width()).at_least(0.0);
        let margin_y = (window.height() - area.height()).at_least(0.0);

        pos.x = pos.x.at_most(area.right() + margin_x - window.width()); // move left if needed
        pos.x = pos.x.at_least(area.left() - margin_x); // move right if needed
        pos.y = pos.y.at_most(area.bottom() + margin_y - window.height()); // move right if needed
        pos.y = pos.y.at_least(area.top() - margin_y); // move down if needed

        pos = self.round_pos_to_pixels(pos);

        Rect::from_min_size(pos, window.size())
    }
}

impl Context {
    /// Call at the end of each frame.
    #[must_use]
    pub fn end_frame(&self) -> FullOutput {
        crate::profile_function!();

        if self.options(|o| o.zoom_with_keyboard) {
            crate::gui_zoom::zoom_with_keyboard(self);
        }

        self.read(|ctx| ctx.plugins.clone()).on_end_frame(self);

        #[cfg(debug_assertions)]
        self.debug_painting();

        self.write(|ctx| ctx.end_frame())
    }

    #[cfg(debug_assertions)]
    fn debug_painting(&self) {
        let paint_widget = |widget: &WidgetRect, text: &str, color: Color32| {
            let rect = widget.interact_rect;
            if rect.is_positive() {
                let painter = Painter::new(self.clone(), widget.layer_id, Rect::EVERYTHING);
                painter.debug_rect(rect, color, text);
            }
        };

        let paint_widget_id = |id: Id, text: &str, color: Color32| {
            if let Some(widget) =
                self.write(|ctx| ctx.viewport().widgets_this_frame.get(id).cloned())
            {
                paint_widget(&widget, text, color);
            }
        };

        if self.style().debug.show_interactive_widgets {
            // Show all interactive widgets:
            let rects = self.write(|ctx| ctx.viewport().widgets_this_frame.clone());
            for (layer_id, rects) in rects.layers() {
                let painter = Painter::new(self.clone(), *layer_id, Rect::EVERYTHING);
                for rect in rects {
                    if rect.sense.interactive() {
                        let (color, text) = if rect.sense.click && rect.sense.drag {
                            (Color32::from_rgb(0x88, 0, 0x88), "click+drag")
                        } else if rect.sense.click {
                            (Color32::from_rgb(0x88, 0, 0), "click")
                        } else if rect.sense.drag {
                            (Color32::from_rgb(0, 0, 0x88), "drag")
                        } else {
                            continue;
                            // (Color32::from_rgb(0, 0, 0x88), "hover")
                        };
                        painter.debug_rect(rect.interact_rect, color, text);
                    }
                }
            }

            // Show the ones actually interacted with:
            {
                let interact_widgets = self.write(|ctx| ctx.viewport().interact_widgets.clone());
                let InteractionSnapshot {
                    clicked,
                    long_touched: _,
                    drag_started: _,
                    dragged,
                    drag_stopped: _,
                    contains_pointer,
                    hovered,
                } = interact_widgets;

                if false {
                    for widget in contains_pointer {
                        paint_widget_id(widget, "contains_pointer", Color32::BLUE);
                    }
                }
                if true {
                    for widget in hovered {
                        paint_widget_id(widget, "hovered", Color32::WHITE);
                    }
                }
                for &widget in &clicked {
                    paint_widget_id(widget, "clicked", Color32::RED);
                }
                for &widget in &dragged {
                    paint_widget_id(widget, "dragged", Color32::GREEN);
                }
            }
        }

        if self.style().debug.show_widget_hits {
            let hits = self.write(|ctx| ctx.viewport().hits.clone());
            let WidgetHits {
                contains_pointer,
                click,
                drag,
            } = hits;

            if false {
                for widget in &contains_pointer {
                    paint_widget(widget, "contains_pointer", Color32::BLUE);
                }
            }
            for widget in &click {
                paint_widget(widget, "click", Color32::RED);
            }
            for widget in &drag {
                paint_widget(widget, "drag", Color32::GREEN);
            }
        }
    }
}

impl ContextImpl {
    fn end_frame(&mut self) -> FullOutput {
        let ended_viewport_id = self.viewport_id();
        let viewport = self.viewports.entry(ended_viewport_id).or_default();
        let pixels_per_point = viewport.input.pixels_per_point;

        viewport.repaint.frame_nr += 1;

        self.memory.end_frame(&viewport.frame_state.used_ids);

        if let Some(fonts) = self.fonts.get(&pixels_per_point.into()) {
            let tex_mngr = &mut self.tex_manager.0.write();
            if let Some(font_image_delta) = fonts.font_image_delta() {
                // A partial font atlas update, e.g. a new glyph has been entered.
                tex_mngr.set(TextureId::default(), font_image_delta);
            }

            if 1 < self.fonts.len() {
                // We have multiple different `pixels_per_point`,
                // e.g. because we have many viewports spread across
                // monitors with different DPI scaling.
                // All viewports share the same texture namespace and renderer,
                // so the all use `TextureId::default()` for the font texture.
                // This is a problem.
                // We solve this with a hack: we always upload the full font atlas
                // every frame, for all viewports.
                // This ensures it is up-to-date, solving
                // https://github.com/emilk/egui/issues/3664
                // at the cost of a lot of performance.
                // (This will override any smaller delta that was uploaded above.)
                crate::profile_scope!("full_font_atlas_update");
                let full_delta = ImageDelta::full(fonts.image(), TextureAtlas::texture_options());
                tex_mngr.set(TextureId::default(), full_delta);
            }
        }

        // Inform the backend of all textures that have been updated (including font atlas).
        let textures_delta = self.tex_manager.0.write().take_delta();

        #[cfg_attr(not(feature = "accesskit"), allow(unused_mut))]
        let mut platform_output: PlatformOutput = std::mem::take(&mut viewport.output);

        #[cfg(feature = "accesskit")]
        {
            crate::profile_scope!("accesskit");
            let state = viewport.frame_state.accesskit_state.take();
            if let Some(state) = state {
                let root_id = crate::accesskit_root_id().accesskit_id();
                let nodes = {
                    state
                        .node_builders
                        .into_iter()
                        .map(|(id, builder)| {
                            (
                                id.accesskit_id(),
                                builder.build(&mut self.accesskit_node_classes),
                            )
                        })
                        .collect()
                };
                let focus_id = self
                    .memory
                    .focused()
                    .map_or(root_id, |id| id.accesskit_id());
                platform_output.accesskit_update = Some(accesskit::TreeUpdate {
                    nodes,
                    tree: Some(accesskit::Tree::new(root_id)),
                    focus: focus_id,
                });
            }
        }

        let shapes = viewport
            .graphics
            .drain(self.memory.areas().order(), &self.memory.layer_transforms);

        let mut repaint_needed = false;

        {
            if self.memory.options.repaint_on_widget_change {
                crate::profile_function!("compare-widget-rects");
                if viewport.widgets_prev_frame != viewport.widgets_this_frame {
                    repaint_needed = true; // Some widget has moved
                }
            }

            std::mem::swap(
                &mut viewport.widgets_prev_frame,
                &mut viewport.widgets_this_frame,
            );
            viewport.widgets_this_frame.clear();
        }

        if repaint_needed || viewport.input.wants_repaint() {
            self.request_repaint(ended_viewport_id, RepaintCause::new());
        }

        //  -------------------

        let all_viewport_ids = self.all_viewport_ids();

        self.last_viewport = ended_viewport_id;

        self.viewports.retain(|&id, viewport| {
            let parent = *self.viewport_parents.entry(id).or_default();

            if !all_viewport_ids.contains(&parent) {
                #[cfg(feature = "log")]
                log::debug!(
                    "Removing viewport {:?} ({:?}): the parent is gone",
                    id,
                    viewport.builder.title
                );

                return false;
            }

            let is_our_child = parent == ended_viewport_id && id != ViewportId::ROOT;
            if is_our_child {
                if !viewport.used {
                    #[cfg(feature = "log")]
                    log::debug!(
                        "Removing viewport {:?} ({:?}): it was never used this frame",
                        id,
                        viewport.builder.title
                    );

                    return false; // Only keep children that have been updated this frame
                }

                viewport.used = false; // reset so we can check again next frame
            }

            true
        });

        // If we are an immediate viewport, this will resume the previous viewport.
        self.viewport_stack.pop();

        // The last viewport is not necessarily the root viewport,
        // just the top _immediate_ viewport.
        let is_last = self.viewport_stack.is_empty();

        let viewport_output = self
            .viewports
            .iter_mut()
            .map(|(&id, viewport)| {
                let parent = *self.viewport_parents.entry(id).or_default();
                let commands = if is_last {
                    // Let the primary immediate viewport handle the commands of its children too.
                    // This can make things easier for the backend, as otherwise we may get commands
                    // that affect a viewport while its egui logic is running.
                    std::mem::take(&mut viewport.commands)
                } else {
                    vec![]
                };

                (
                    id,
                    ViewportOutput {
                        parent,
                        class: viewport.class,
                        builder: viewport.builder.clone(),
                        viewport_ui_cb: viewport.viewport_ui_cb.clone(),
                        commands,
                        repaint_delay: viewport.repaint.repaint_delay,
                    },
                )
            })
            .collect();

        if is_last {
            // Remove dead viewports:
            self.viewports.retain(|id, _| all_viewport_ids.contains(id));
            self.viewport_parents
                .retain(|id, _| all_viewport_ids.contains(id));
        } else {
            let viewport_id = self.viewport_id();
            self.memory.set_viewport_id(viewport_id);
        }

        let active_pixels_per_point: std::collections::BTreeSet<OrderedFloat<f32>> = self
            .viewports
            .values()
            .map(|v| v.input.pixels_per_point.into())
            .collect();
        self.fonts.retain(|pixels_per_point, _| {
            if active_pixels_per_point.contains(pixels_per_point) {
                true
            } else {
                #[cfg(feature = "log")]
                log::trace!(
                    "Freeing Fonts with pixels_per_point={} because it is no longer needed",
                    pixels_per_point.into_inner()
                );
                false
            }
        });

        FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            viewport_output,
        }
    }
}

impl Context {
    /// Tessellate the given shapes into triangle meshes.
    ///
    /// `pixels_per_point` is used for feathering (anti-aliasing).
    /// For this you can use [`FullOutput::pixels_per_point`], [`Self::pixels_per_point`],
    /// or whatever is appropriate for your viewport.
    pub fn tessellate(
        &self,
        shapes: Vec<ClippedShape>,
        pixels_per_point: f32,
    ) -> Vec<ClippedPrimitive> {
        crate::profile_function!();

        // A tempting optimization is to reuse the tessellation from last frame if the
        // shapes are the same, but just comparing the shapes takes about 50% of the time
        // it takes to tessellate them, so it is not a worth optimization.

        self.write(|ctx| {
            let tessellation_options = ctx.memory.options.tessellation_options;
            let texture_atlas = ctx
                .fonts
                .get(&pixels_per_point.into())
                .expect("tessellate called with a different pixels_per_point than the font atlas was created with. \
                         You should use egui::FullOutput::pixels_per_point when tessellating.")
                .texture_atlas();
            let (font_tex_size, prepared_discs) = {
                let atlas = texture_atlas.lock();
                (atlas.size(), atlas.prepared_discs())
            };

            let paint_stats = PaintStats::from_shapes(&shapes);
            let clipped_primitives = {
                crate::profile_scope!("tessellator::tessellate_shapes");
                tessellator::Tessellator::new(
                    pixels_per_point,
                    tessellation_options,
                    font_tex_size,
                    prepared_discs,
                )
                .tessellate_shapes(shapes)
            };
            ctx.paint_stats = paint_stats.with_clipped_primitives(&clipped_primitives);
            clipped_primitives
        })
    }

    // ---------------------------------------------------------------------

    /// Position and size of the egui area.
    pub fn screen_rect(&self) -> Rect {
        self.input(|i| i.screen_rect())
    }

    /// How much space is still available after panels has been added.
    ///
    /// This is the "background" area, what egui doesn't cover with panels (but may cover with windows).
    /// This is also the area to which windows are constrained.
    pub fn available_rect(&self) -> Rect {
        self.frame_state(|s| s.available_rect())
    }

    /// How much space is used by panels and windows.
    pub fn used_rect(&self) -> Rect {
        self.write(|ctx| {
            let mut used = ctx.viewport().frame_state.used_by_panels;
            for window in ctx.memory.areas().visible_windows() {
                used = used.union(window.rect());
            }
            used
        })
    }

    /// How much space is used by panels and windows.
    ///
    /// You can shrink your egui area to this size and still fit all egui components.
    pub fn used_size(&self) -> Vec2 {
        self.used_rect().max - Pos2::ZERO
    }

    // ---------------------------------------------------------------------

    /// Is the pointer (mouse/touch) over any egui area?
    pub fn is_pointer_over_area(&self) -> bool {
        let pointer_pos = self.input(|i| i.pointer.interact_pos());
        if let Some(pointer_pos) = pointer_pos {
            if let Some(layer) = self.layer_id_at(pointer_pos) {
                if layer.order == Order::Background {
                    !self.frame_state(|state| state.unused_rect.contains(pointer_pos))
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

    /// True if egui is currently interested in the pointer (mouse or touch).
    ///
    /// Could be the pointer is hovering over a [`Window`] or the user is dragging a widget.
    /// If `false`, the pointer is outside of any egui area and so
    /// you may be interested in what it is doing (e.g. controlling your game).
    /// Returns `false` if a drag started outside of egui and then moved over an egui area.
    pub fn wants_pointer_input(&self) -> bool {
        self.is_using_pointer()
            || (self.is_pointer_over_area() && !self.input(|i| i.pointer.any_down()))
    }

    /// Is egui currently using the pointer position (e.g. dragging a slider)?
    ///
    /// NOTE: this will return `false` if the pointer is just hovering over an egui area.
    pub fn is_using_pointer(&self) -> bool {
        self.memory(|m| m.interaction().is_using_pointer())
    }

    /// If `true`, egui is currently listening on text input (e.g. typing text in a [`TextEdit`]).
    pub fn wants_keyboard_input(&self) -> bool {
        self.memory(|m| m.focused().is_some())
    }

    /// Highlight this widget, to make it look like it is hovered, even if it isn't.
    ///
    /// The highlight takes on frame to take effect if you call this after the widget has been fully rendered.
    ///
    /// See also [`Response::highlight`].
    pub fn highlight_widget(&self, id: Id) {
        self.frame_state_mut(|fs| fs.highlight_next_frame.insert(id));
    }

    /// Is an egui context menu open?
    pub fn is_context_menu_open(&self) -> bool {
        self.data(|d| {
            d.get_temp::<crate::menu::BarState>(menu::CONTEXT_MENU_ID_STR.into())
                .map_or(false, |state| state.has_root())
        })
    }
}

// Ergonomic methods to forward some calls often used in 'if let' without holding the borrow
impl Context {
    /// Latest reported pointer position.
    ///
    /// When tapping a touch screen, this will be `None`.
    #[inline(always)]
    pub fn pointer_latest_pos(&self) -> Option<Pos2> {
        self.input(|i| i.pointer.latest_pos())
    }

    /// If it is a good idea to show a tooltip, where is pointer?
    #[inline(always)]
    pub fn pointer_hover_pos(&self) -> Option<Pos2> {
        self.input(|i| i.pointer.hover_pos())
    }

    /// If you detect a click or drag and wants to know where it happened, use this.
    ///
    /// Latest position of the mouse, but ignoring any [`Event::PointerGone`]
    /// if there were interactions this frame.
    /// When tapping a touch screen, this will be the location of the touch.
    #[inline(always)]
    pub fn pointer_interact_pos(&self) -> Option<Pos2> {
        self.input(|i| i.pointer.interact_pos())
    }

    /// Calls [`InputState::multi_touch`].
    pub fn multi_touch(&self) -> Option<MultiTouchInfo> {
        self.input(|i| i.multi_touch())
    }
}

impl Context {
    /// Transform the graphics of the given layer.
    ///
    /// This will also affect input.
    ///
    /// This is a sticky setting, remembered from one frame to the next.
    ///
    /// Can be used to implement pan and zoom (see relevant demo).
    ///
    /// For a temporary transform, use [`Self::transform_layer_shapes`] instead.
    pub fn set_transform_layer(&self, layer_id: LayerId, transform: TSTransform) {
        self.memory_mut(|m| {
            if transform == TSTransform::IDENTITY {
                m.layer_transforms.remove(&layer_id)
            } else {
                m.layer_transforms.insert(layer_id, transform)
            }
        });
    }

    /// Move all the graphics at the given layer.
    ///
    /// Is used to implement drag-and-drop preview.
    ///
    /// This only applied to the existing graphics at the layer, not to new graphics added later.
    ///
    /// For a persistent transform, use [`Self::set_transform_layer`] instead.
    #[deprecated = "Use `transform_layer_shapes` instead"]
    pub fn translate_layer(&self, layer_id: LayerId, delta: Vec2) {
        if delta != Vec2::ZERO {
            let transform = emath::TSTransform::from_translation(delta);
            self.transform_layer_shapes(layer_id, transform);
        }
    }

    /// Transform all the graphics at the given layer.
    ///
    /// Is used to implement drag-and-drop preview.
    ///
    /// This only applied to the existing graphics at the layer, not to new graphics added later.
    ///
    /// For a persistent transform, use [`Self::set_transform_layer`] instead.
    pub fn transform_layer_shapes(&self, layer_id: LayerId, transform: TSTransform) {
        if transform != TSTransform::IDENTITY {
            self.graphics_mut(|g| g.entry(layer_id).transform(transform));
        }
    }

    /// Top-most layer at the given position.
    pub fn layer_id_at(&self, pos: Pos2) -> Option<LayerId> {
        self.memory(|mem| mem.layer_id_at(pos))
    }

    /// Moves the given area to the top in its [`Order`].
    ///
    /// [`Area`]:s and [`Window`]:s also do this automatically when being clicked on or interacted with.
    pub fn move_to_top(&self, layer_id: LayerId) {
        self.memory_mut(|mem| mem.areas_mut().move_to_top(layer_id));
    }

    /// Retrieve the [`LayerId`] of the top level windows.
    pub fn top_layer_id(&self) -> Option<LayerId> {
        self.memory(|mem| mem.areas().top_layer_id(Order::Middle))
    }

    /// Does the given rectangle contain the mouse pointer?
    ///
    /// Will return false if some other area is covering the given layer.
    ///
    /// The given rectangle is assumed to have been clipped by its parent clip rect.
    ///
    /// See also [`Response::contains_pointer`].
    pub fn rect_contains_pointer(&self, layer_id: LayerId, rect: Rect) -> bool {
        let rect =
            if let Some(transform) = self.memory(|m| m.layer_transforms.get(&layer_id).cloned()) {
                transform * rect
            } else {
                rect
            };
        if !rect.is_positive() {
            return false;
        }

        let pointer_pos = self.input(|i| i.pointer.interact_pos());
        let Some(pointer_pos) = pointer_pos else {
            return false;
        };

        if !rect.contains(pointer_pos) {
            return false;
        }

        if self.layer_id_at(pointer_pos) != Some(layer_id) {
            return false;
        }

        true
    }

    // ---------------------------------------------------------------------

    /// Whether or not to debug widget layout on hover.
    #[cfg(debug_assertions)]
    pub fn debug_on_hover(&self) -> bool {
        self.options(|opt| opt.style.debug.debug_on_hover)
    }

    /// Turn on/off whether or not to debug widget layout on hover.
    #[cfg(debug_assertions)]
    pub fn set_debug_on_hover(&self, debug_on_hover: bool) {
        self.style_mut(|style| style.debug.debug_on_hover = debug_on_hover);
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
    ///
    /// The animation time is taken from [`Style::animation_time`].
    #[track_caller] // To track repaint cause
    pub fn animate_bool(&self, id: Id, value: bool) -> f32 {
        let animation_time = self.style().animation_time;
        self.animate_bool_with_time(id, value, animation_time)
    }

    /// Like [`Self::animate_bool`] but allows you to control the animation time.
    #[track_caller] // To track repaint cause
    pub fn animate_bool_with_time(&self, id: Id, target_value: bool, animation_time: f32) -> f32 {
        let animated_value = self.write(|ctx| {
            ctx.animation_manager.animate_bool(
                &ctx.viewports.entry(ctx.viewport_id()).or_default().input,
                animation_time,
                id,
                target_value,
            )
        });
        let animation_in_progress = 0.0 < animated_value && animated_value < 1.0;
        if animation_in_progress {
            self.request_repaint();
        }
        animated_value
    }

    /// Smoothly animate an `f32` value.
    ///
    /// At the first call the value is written to memory.
    /// When it is called with a new value, it linearly interpolates to it in the given time.
    #[track_caller] // To track repaint cause
    pub fn animate_value_with_time(&self, id: Id, target_value: f32, animation_time: f32) -> f32 {
        let animated_value = self.write(|ctx| {
            ctx.animation_manager.animate_value(
                &ctx.viewports.entry(ctx.viewport_id()).or_default().input,
                animation_time,
                id,
                target_value,
            )
        });
        let animation_in_progress = animated_value != target_value;
        if animation_in_progress {
            self.request_repaint();
        }

        animated_value
    }

    /// Clear memory of any animations.
    pub fn clear_animations(&self) {
        self.write(|ctx| ctx.animation_manager = Default::default());
    }
}

impl Context {
    /// Show a ui for settings (style and tessellation options).
    pub fn settings_ui(&self, ui: &mut Ui) {
        let prev_options = self.options(|o| o.clone());
        let mut options = prev_options.clone();

        options.ui(ui);

        if options != prev_options {
            self.options_mut(move |o| *o = options);
        }
    }

    /// Show the state of egui, including its input and output.
    pub fn inspection_ui(&self, ui: &mut Ui) {
        use crate::containers::*;

        ui.label(format!("Is using pointer: {}", self.is_using_pointer()))
            .on_hover_text(
                "Is egui currently using the pointer actively (e.g. dragging a slider)?",
            );
        ui.label(format!("Wants pointer input: {}", self.wants_pointer_input()))
            .on_hover_text("Is egui currently interested in the location of the pointer (either because it is in use, or because it is hovering over a window).");
        ui.label(format!(
            "Wants keyboard input: {}",
            self.wants_keyboard_input()
        ))
        .on_hover_text("Is egui currently listening for text input?");
        ui.label(format!(
            "Keyboard focus widget: {}",
            self.memory(|m| m.focused())
                .as_ref()
                .map(Id::short_debug_format)
                .unwrap_or_default()
        ))
        .on_hover_text("Is egui currently listening for text input?");

        let pointer_pos = self
            .pointer_hover_pos()
            .map_or_else(String::new, |pos| format!("{pos:?}"));
        ui.label(format!("Pointer pos: {pointer_pos}"));

        let top_layer = self
            .pointer_hover_pos()
            .and_then(|pos| self.layer_id_at(pos))
            .map_or_else(String::new, |layer| layer.short_debug_format());
        ui.label(format!("Top layer under mouse: {top_layer}"));

        ui.add_space(16.0);

        ui.label(format!(
            "There are {} text galleys in the layout cache",
            self.fonts(|f| f.num_galleys_in_cache())
        ))
        .on_hover_text("This is approximately the number of text strings on screen");
        ui.add_space(16.0);

        CollapsingHeader::new("🔃 Repaint Causes")
            .default_open(false)
            .show(ui, |ui| {
                ui.set_min_height(120.0);
                ui.label("What caused egui to repaint:");
                ui.add_space(8.0);
                let causes = ui.ctx().repaint_causes();
                for cause in causes {
                    ui.label(cause.to_string());
                }
            });

        CollapsingHeader::new("📥 Input")
            .default_open(false)
            .show(ui, |ui| {
                let input = ui.input(|i| i.clone());
                input.ui(ui);
            });

        CollapsingHeader::new("📊 Paint stats")
            .default_open(false)
            .show(ui, |ui| {
                let paint_stats = self.read(|ctx| ctx.paint_stats);
                paint_stats.ui(ui);
            });

        CollapsingHeader::new("🖼 Textures")
            .default_open(false)
            .show(ui, |ui| {
                self.texture_ui(ui);
            });

        CollapsingHeader::new("🔠 Font texture")
            .default_open(false)
            .show(ui, |ui| {
                let font_image_size = self.fonts(|f| f.font_image_size());
                crate::introspection::font_texture_ui(ui, font_image_size);
            });

        CollapsingHeader::new("Label text selection state")
            .default_open(false)
            .show(ui, |ui| {
                ui.label(format!(
                    "{:#?}",
                    crate::text_selection::LabelSelectionState::load(ui.ctx())
                ));
            });

        CollapsingHeader::new("Interaction")
            .default_open(false)
            .show(ui, |ui| {
                let interact_widgets = self.write(|ctx| ctx.viewport().interact_widgets.clone());
                interact_widgets.ui(ui);
            });
    }

    /// Show stats about the allocated textures.
    pub fn texture_ui(&self, ui: &mut crate::Ui) {
        let tex_mngr = self.tex_manager();
        let tex_mngr = tex_mngr.read();

        let mut textures: Vec<_> = tex_mngr.allocated().collect();
        textures.sort_by_key(|(id, _)| *id);

        let mut bytes = 0;
        for (_, tex) in &textures {
            bytes += tex.bytes_used();
        }

        ui.label(format!(
            "{} allocated texture(s), using {:.1} MB",
            textures.len(),
            bytes as f64 * 1e-6
        ));
        let max_preview_size = vec2(48.0, 32.0);

        ui.group(|ui| {
            ScrollArea::vertical()
                .max_height(300.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(TextStyle::Monospace);
                    Grid::new("textures")
                        .striped(true)
                        .num_columns(4)
                        .spacing(vec2(16.0, 2.0))
                        .min_row_height(max_preview_size.y)
                        .show(ui, |ui| {
                            for (&texture_id, meta) in textures {
                                let [w, h] = meta.size;

                                let mut size = vec2(w as f32, h as f32);
                                size *= (max_preview_size.x / size.x).min(1.0);
                                size *= (max_preview_size.y / size.y).min(1.0);
                                ui.image(SizedTexture::new(texture_id, size))
                                    .on_hover_ui(|ui| {
                                        // show larger on hover
                                        let max_size = 0.5 * ui.ctx().screen_rect().size();
                                        let mut size = vec2(w as f32, h as f32);
                                        size *= max_size.x / size.x.max(max_size.x);
                                        size *= max_size.y / size.y.max(max_size.y);
                                        ui.image(SizedTexture::new(texture_id, size));
                                    });

                                ui.label(format!("{w} x {h}"));
                                ui.label(format!("{:.3} MB", meta.bytes_used() as f64 * 1e-6));
                                ui.label(format!("{:?}", meta.name));
                                ui.end_row();
                            }
                        });
                });
        });
    }

    /// Shows the contents of [`Self::memory`].
    pub fn memory_ui(&self, ui: &mut crate::Ui) {
        if ui
            .button("Reset all")
            .on_hover_text("Reset all egui state")
            .clicked()
        {
            self.memory_mut(|mem| *mem = Default::default());
        }

        let (num_state, num_serialized) = self.data(|d| (d.len(), d.count_serialized()));
        ui.label(format!(
            "{num_state} widget states stored (of which {num_serialized} are serialized)."
        ));

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} areas (panels, windows, popups, …)",
                self.memory(|mem| mem.areas().count())
            ));
            if ui.button("Reset").clicked() {
                self.memory_mut(|mem| *mem.areas_mut() = Default::default());
            }
        });
        ui.indent("areas", |ui| {
            ui.label("Visible areas, ordered back to front.");
            ui.label("Hover to highlight");
            let layers_ids: Vec<LayerId> = self.memory(|mem| mem.areas().order().to_vec());
            for layer_id in layers_ids {
                let area = self.memory(|mem| mem.areas().get(layer_id.id).copied());
                if let Some(area) = area {
                    let is_visible = self.memory(|mem| mem.areas().is_visible(&layer_id));
                    if !is_visible {
                        continue;
                    }
                    let text = format!("{} - {:?}", layer_id.short_debug_format(), area.rect(),);
                    // TODO(emilk): `Sense::hover_highlight()`
                    if ui
                        .add(Label::new(RichText::new(text).monospace()).sense(Sense::click()))
                        .hovered
                        && is_visible
                    {
                        ui.ctx()
                            .debug_painter()
                            .debug_rect(area.rect(), Color32::RED, "");
                    }
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} collapsing headers",
                self.data(|d| d.count::<containers::collapsing_header::InnerState>())
            ));
            if ui.button("Reset").clicked() {
                self.data_mut(|d| d.remove_by_type::<containers::collapsing_header::InnerState>());
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} menu bars",
                self.data(|d| d.count::<menu::BarState>())
            ));
            if ui.button("Reset").clicked() {
                self.data_mut(|d| d.remove_by_type::<menu::BarState>());
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} scroll areas",
                self.data(|d| d.count::<scroll_area::State>())
            ));
            if ui.button("Reset").clicked() {
                self.data_mut(|d| d.remove_by_type::<scroll_area::State>());
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} resize areas",
                self.data(|d| d.count::<resize::State>())
            ));
            if ui.button("Reset").clicked() {
                self.data_mut(|d| d.remove_by_type::<resize::State>());
            }
        });

        ui.shrink_width_to_current(); // don't let the text below grow this window wider
        ui.label("NOTE: the position of this window cannot be reset from within itself.");

        ui.collapsing("Interaction", |ui| {
            let interaction = self.memory(|mem| mem.interaction().clone());
            interaction.ui(ui);
        });
    }
}

impl Context {
    /// Edit the active [`Style`].
    pub fn style_ui(&self, ui: &mut Ui) {
        let mut style: Style = (*self.style()).clone();
        style.ui(ui);
        self.set_style(style);
    }
}

/// ## Accessibility
impl Context {
    /// Call the provided function with the given ID pushed on the stack of
    /// parent IDs for accessibility purposes. If the `accesskit` feature
    /// is disabled or if AccessKit support is not active for this frame,
    /// the function is still called, but with no other effect.
    ///
    /// No locks are held while the given closure is called.
    #[allow(clippy::unused_self)]
    #[inline]
    pub fn with_accessibility_parent(&self, _id: Id, f: impl FnOnce()) {
        // TODO(emilk): this isn't thread-safe - another thread can call this function between the push/pop calls
        #[cfg(feature = "accesskit")]
        self.frame_state_mut(|fs| {
            if let Some(state) = fs.accesskit_state.as_mut() {
                state.parent_stack.push(_id);
            }
        });

        f();

        #[cfg(feature = "accesskit")]
        self.frame_state_mut(|fs| {
            if let Some(state) = fs.accesskit_state.as_mut() {
                assert_eq!(state.parent_stack.pop(), Some(_id));
            }
        });
    }

    /// If AccessKit support is active for the current frame, get or create
    /// a node builder with the specified ID and return a mutable reference to it.
    /// For newly created nodes, the parent is the node with the ID at the top
    /// of the stack managed by [`Context::with_accessibility_parent`].
    ///
    /// The `Context` lock is held while the given closure is called!
    ///
    /// Returns `None` if acesskit is off.
    // TODO(emilk): consider making both read-only and read-write versions
    #[cfg(feature = "accesskit")]
    pub fn accesskit_node_builder<R>(
        &self,
        id: Id,
        writer: impl FnOnce(&mut accesskit::NodeBuilder) -> R,
    ) -> Option<R> {
        self.write(|ctx| {
            ctx.viewport()
                .frame_state
                .accesskit_state
                .is_some()
                .then(|| ctx.accesskit_node_builder(id))
                .map(writer)
        })
    }

    /// Enable generation of AccessKit tree updates in all future frames.
    ///
    /// If it's practical for the egui integration to immediately run the egui
    /// application when it is either initializing the AccessKit adapter or
    /// being called by the AccessKit adapter to provide the initial tree update,
    /// then it should do so, to provide a complete AccessKit tree to the adapter
    /// immediately. Otherwise, it should enqueue a repaint and use the
    /// placeholder tree update from [`Context::accesskit_placeholder_tree_update`]
    /// in the meantime.
    #[cfg(feature = "accesskit")]
    pub fn enable_accesskit(&self) {
        self.write(|ctx| ctx.is_accesskit_enabled = true);
    }

    /// Return a tree update that the egui integration should provide to the
    /// AccessKit adapter if it cannot immediately run the egui application
    /// to get a full tree update after running [`Context::enable_accesskit`].
    #[cfg(feature = "accesskit")]
    pub fn accesskit_placeholder_tree_update(&self) -> accesskit::TreeUpdate {
        crate::profile_function!();

        use accesskit::{NodeBuilder, Role, Tree, TreeUpdate};

        let root_id = crate::accesskit_root_id().accesskit_id();
        self.write(|ctx| TreeUpdate {
            nodes: vec![(
                root_id,
                NodeBuilder::new(Role::Window).build(&mut ctx.accesskit_node_classes),
            )],
            tree: Some(Tree::new(root_id)),
            focus: root_id,
        })
    }
}

/// ## Image loading
impl Context {
    /// Associate some static bytes with a `uri`.
    ///
    /// The same `uri` may be passed to [`Ui::image`] later to load the bytes as an image.
    ///
    /// By convention, the `uri` should start with `bytes://`.
    /// Following that convention will lead to better error messages.
    pub fn include_bytes(&self, uri: impl Into<Cow<'static, str>>, bytes: impl Into<Bytes>) {
        self.loaders().include.insert(uri, bytes);
    }

    /// Returns `true` if the chain of bytes, image, or texture loaders
    /// contains a loader with the given `id`.
    pub fn is_loader_installed(&self, id: &str) -> bool {
        let loaders = self.loaders();

        loaders.bytes.lock().iter().any(|l| l.id() == id)
            || loaders.image.lock().iter().any(|l| l.id() == id)
            || loaders.texture.lock().iter().any(|l| l.id() == id)
    }

    /// Add a new bytes loader.
    ///
    /// It will be tried first, before any already installed loaders.
    ///
    /// See [`load`] for more information.
    pub fn add_bytes_loader(&self, loader: Arc<dyn load::BytesLoader + Send + Sync + 'static>) {
        self.loaders().bytes.lock().push(loader);
    }

    /// Add a new image loader.
    ///
    /// It will be tried first, before any already installed loaders.
    ///
    /// See [`load`] for more information.
    pub fn add_image_loader(&self, loader: Arc<dyn load::ImageLoader + Send + Sync + 'static>) {
        self.loaders().image.lock().push(loader);
    }

    /// Add a new texture loader.
    ///
    /// It will be tried first, before any already installed loaders.
    ///
    /// See [`load`] for more information.
    pub fn add_texture_loader(&self, loader: Arc<dyn load::TextureLoader + Send + Sync + 'static>) {
        self.loaders().texture.lock().push(loader);
    }

    /// Release all memory and textures related to the given image URI.
    ///
    /// If you attempt to load the image again, it will be reloaded from scratch.
    pub fn forget_image(&self, uri: &str) {
        use load::BytesLoader as _;

        crate::profile_function!();

        let loaders = self.loaders();

        loaders.include.forget(uri);
        for loader in loaders.bytes.lock().iter() {
            loader.forget(uri);
        }
        for loader in loaders.image.lock().iter() {
            loader.forget(uri);
        }
        for loader in loaders.texture.lock().iter() {
            loader.forget(uri);
        }
    }

    /// Release all memory and textures related to images used in [`Ui::image`] or [`Image`].
    ///
    /// If you attempt to load any images again, they will be reloaded from scratch.
    pub fn forget_all_images(&self) {
        use load::BytesLoader as _;

        crate::profile_function!();

        let loaders = self.loaders();

        loaders.include.forget_all();
        for loader in loaders.bytes.lock().iter() {
            loader.forget_all();
        }
        for loader in loaders.image.lock().iter() {
            loader.forget_all();
        }
        for loader in loaders.texture.lock().iter() {
            loader.forget_all();
        }
    }

    /// Try loading the bytes from the given uri using any available bytes loaders.
    ///
    /// Loaders are expected to cache results, so that this call is immediate-mode safe.
    ///
    /// This calls the loaders one by one in the order in which they were registered.
    /// If a loader returns [`LoadError::NotSupported`][not_supported],
    /// then the next loader is called. This process repeats until all loaders have
    /// been exhausted, at which point this returns [`LoadError::NotSupported`][not_supported].
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`][not_supported] if none of the registered loaders support loading the given `uri`.
    /// - [`LoadError::Loading`][custom] if one of the loaders _does_ support loading the `uri`, but the loading process failed.
    ///
    /// ⚠ May deadlock if called from within a `BytesLoader`!
    ///
    /// [not_supported]: crate::load::LoadError::NotSupported
    /// [custom]: crate::load::LoadError::Loading
    pub fn try_load_bytes(&self, uri: &str) -> load::BytesLoadResult {
        crate::profile_function!(uri);

        let loaders = self.loaders();
        let bytes_loaders = loaders.bytes.lock();

        // Try most recently added loaders first (hence `.rev()`)
        for loader in bytes_loaders.iter().rev() {
            match loader.load(self, uri) {
                Err(load::LoadError::NotSupported) => continue,
                result => return result,
            }
        }

        Err(load::LoadError::NoMatchingBytesLoader)
    }

    /// Try loading the image from the given uri using any available image loaders.
    ///
    /// Loaders are expected to cache results, so that this call is immediate-mode safe.
    ///
    /// This calls the loaders one by one in the order in which they were registered.
    /// If a loader returns [`LoadError::NotSupported`][not_supported],
    /// then the next loader is called. This process repeats until all loaders have
    /// been exhausted, at which point this returns [`LoadError::NotSupported`][not_supported].
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NoImageLoaders`][no_image_loaders] if tbere are no registered image loaders.
    /// - [`LoadError::NotSupported`][not_supported] if none of the registered loaders support loading the given `uri`.
    /// - [`LoadError::Loading`][custom] if one of the loaders _does_ support loading the `uri`, but the loading process failed.
    ///
    /// ⚠ May deadlock if called from within an `ImageLoader`!
    ///
    /// [no_image_loaders]: crate::load::LoadError::NoImageLoaders
    /// [not_supported]: crate::load::LoadError::NotSupported
    /// [custom]: crate::load::LoadError::Loading
    pub fn try_load_image(&self, uri: &str, size_hint: load::SizeHint) -> load::ImageLoadResult {
        crate::profile_function!(uri);

        let loaders = self.loaders();
        let image_loaders = loaders.image.lock();
        if image_loaders.is_empty() {
            return Err(load::LoadError::NoImageLoaders);
        }

        // Try most recently added loaders first (hence `.rev()`)
        for loader in image_loaders.iter().rev() {
            match loader.load(self, uri, size_hint) {
                Err(load::LoadError::NotSupported) => continue,
                result => return result,
            }
        }

        Err(load::LoadError::NoMatchingImageLoader)
    }

    /// Try loading the texture from the given uri using any available texture loaders.
    ///
    /// Loaders are expected to cache results, so that this call is immediate-mode safe.
    ///
    /// This calls the loaders one by one in the order in which they were registered.
    /// If a loader returns [`LoadError::NotSupported`][not_supported],
    /// then the next loader is called. This process repeats until all loaders have
    /// been exhausted, at which point this returns [`LoadError::NotSupported`][not_supported].
    ///
    /// # Errors
    /// This may fail with:
    /// - [`LoadError::NotSupported`][not_supported] if none of the registered loaders support loading the given `uri`.
    /// - [`LoadError::Loading`][custom] if one of the loaders _does_ support loading the `uri`, but the loading process failed.
    ///
    /// ⚠ May deadlock if called from within a `TextureLoader`!
    ///
    /// [not_supported]: crate::load::LoadError::NotSupported
    /// [custom]: crate::load::LoadError::Loading
    pub fn try_load_texture(
        &self,
        uri: &str,
        texture_options: TextureOptions,
        size_hint: load::SizeHint,
    ) -> load::TextureLoadResult {
        crate::profile_function!(uri);

        let loaders = self.loaders();
        let texture_loaders = loaders.texture.lock();

        // Try most recently added loaders first (hence `.rev()`)
        for loader in texture_loaders.iter().rev() {
            match loader.load(self, uri, texture_options, size_hint) {
                Err(load::LoadError::NotSupported) => continue,
                result => return result,
            }
        }

        Err(load::LoadError::NoMatchingTextureLoader)
    }

    /// The loaders of bytes, images, and textures.
    pub fn loaders(&self) -> Arc<Loaders> {
        crate::profile_function!();
        self.read(|this| this.loaders.clone())
    }
}

/// ## Viewports
impl Context {
    /// Return the `ViewportId` of the current viewport.
    ///
    /// If this is the root viewport, this will return [`ViewportId::ROOT`].
    ///
    /// Don't use this outside of `Self::run`, or after `Self::end_frame`.
    pub fn viewport_id(&self) -> ViewportId {
        self.read(|ctx| ctx.viewport_id())
    }

    /// Return the `ViewportId` of his parent.
    ///
    /// If this is the root viewport, this will return [`ViewportId::ROOT`].
    ///
    /// Don't use this outside of `Self::run`, or after `Self::end_frame`.
    pub fn parent_viewport_id(&self) -> ViewportId {
        self.read(|ctx| ctx.parent_viewport_id())
    }

    /// For integrations: Set this to render a sync viewport.
    ///
    /// This will only set the callback for the current thread,
    /// which most likely should be the main thread.
    ///
    /// When an immediate viewport is created with [`Self::show_viewport_immediate`] it will be rendered by this function.
    ///
    /// When called, the integration needs to:
    /// * Check if there already is a window for this viewport id, and if not open one
    /// * Set the window attributes (position, size, …) based on [`ImmediateViewport::builder`].
    /// * Call [`Context::run`] with [`ImmediateViewport::viewport_ui_cb`].
    /// * Handle the output from [`Context::run`], including rendering
    #[allow(clippy::unused_self)]
    pub fn set_immediate_viewport_renderer(
        callback: impl for<'a> Fn(&Self, ImmediateViewport<'a>) + 'static,
    ) {
        let callback = Box::new(callback);
        IMMEDIATE_VIEWPORT_RENDERER.with(|render_sync| {
            render_sync.replace(Some(callback));
        });
    }

    /// If `true`, [`Self::show_viewport_deferred`] and [`Self::show_viewport_immediate`] will
    /// embed the new viewports inside the existing one, instead of spawning a new native window.
    ///
    /// `eframe` sets this to `false` on supported platforms, but the default value is `true`.
    pub fn embed_viewports(&self) -> bool {
        self.read(|ctx| ctx.embed_viewports)
    }

    /// If `true`, [`Self::show_viewport_deferred`] and [`Self::show_viewport_immediate`] will
    /// embed the new viewports inside the existing one, instead of spawning a new native window.
    ///
    /// `eframe` sets this to `false` on supported platforms, but the default value is `true`.
    pub fn set_embed_viewports(&self, value: bool) {
        self.write(|ctx| ctx.embed_viewports = value);
    }

    /// Send a command to the current viewport.
    ///
    /// This lets you affect the current viewport, e.g. resizing the window.
    pub fn send_viewport_cmd(&self, command: ViewportCommand) {
        self.send_viewport_cmd_to(self.viewport_id(), command);
    }

    /// Send a command to a specific viewport.
    ///
    /// This lets you affect another viewport, e.g. resizing its window.
    pub fn send_viewport_cmd_to(&self, id: ViewportId, command: ViewportCommand) {
        self.request_repaint_of(id);

        if command.requires_parent_repaint() {
            self.request_repaint_of(self.parent_viewport_id());
        }

        self.write(|ctx| ctx.viewport_for(id).commands.push(command));
    }

    /// Show a deferred viewport, creating a new native window, if possible.
    ///
    /// The given id must be unique for each viewport.
    ///
    /// You need to call this each frame when the child viewport should exist.
    ///
    /// You can check if the user wants to close the viewport by checking the
    /// [`crate::ViewportInfo::close_requested`] flags found in [`crate::InputState::viewport`].
    ///
    /// The given callback will be called whenever the child viewport needs repainting,
    /// e.g. on an event or when [`Self::request_repaint`] is called.
    /// This means it may be called multiple times, for instance while the
    /// parent viewport (the caller) is sleeping but the child viewport is animating.
    ///
    /// You will need to wrap your viewport state in an `Arc<RwLock<T>>` or `Arc<Mutex<T>>`.
    /// When this is called again with the same id in `ViewportBuilder` the render function for that viewport will be updated.
    ///
    /// You can also use [`Self::show_viewport_immediate`], which uses a simpler `FnOnce`
    /// with no need for `Send` or `Sync`. The downside is that it will require
    /// the parent viewport (the caller) to repaint anytime the child is repainted,
    /// and vice versa.
    ///
    /// If [`Context::embed_viewports`] is `true` (e.g. if the current egui
    /// backend does not support multiple viewports), the given callback
    /// will be called immediately, embedding the new viewport in the current one.
    /// You can check this with the [`ViewportClass`] given in the callback.
    /// If you find [`ViewportClass::Embedded`], you need to create a new [`crate::Window`] for you content.
    ///
    /// See [`crate::viewport`] for more information about viewports.
    pub fn show_viewport_deferred(
        &self,
        new_viewport_id: ViewportId,
        viewport_builder: ViewportBuilder,
        viewport_ui_cb: impl Fn(&Self, ViewportClass) + Send + Sync + 'static,
    ) {
        crate::profile_function!();

        if self.embed_viewports() {
            viewport_ui_cb(self, ViewportClass::Embedded);
        } else {
            self.write(|ctx| {
                ctx.viewport_parents
                    .insert(new_viewport_id, ctx.viewport_id());

                let viewport = ctx.viewports.entry(new_viewport_id).or_default();
                viewport.class = ViewportClass::Deferred;
                viewport.builder = viewport_builder;
                viewport.used = true;
                viewport.viewport_ui_cb = Some(Arc::new(move |ctx| {
                    (viewport_ui_cb)(ctx, ViewportClass::Deferred);
                }));
            });
        }
    }

    /// Show an immediate viewport, creating a new native window, if possible.
    ///
    /// This is the easier type of viewport to use, but it is less performant
    /// at it requires both parent and child to repaint if any one of them needs repainting,
    /// which efficvely produce double work for two viewports, and triple work for three viewports, etc.
    /// To avoid this, use [`Self::show_viewport_deferred`] instead.
    ///
    /// The given id must be unique for each viewport.
    ///
    /// You need to call this each frame when the child viewport should exist.
    ///
    /// You can check if the user wants to close the viewport by checking the
    /// [`crate::ViewportInfo::close_requested`] flags found in [`crate::InputState::viewport`].
    ///
    /// The given ui function will be called immediately.
    /// This may only be called on the main thread.
    /// This call will pause the current viewport and render the child viewport in its own window.
    /// This means that the child viewport will not be repainted when the parent viewport is repainted, and vice versa.
    ///
    /// If [`Context::embed_viewports`] is `true` (e.g. if the current egui
    /// backend does not support multiple viewports), the given callback
    /// will be called immediately, embedding the new viewport in the current one.
    /// You can check this with the [`ViewportClass`] given in the callback.
    /// If you find [`ViewportClass::Embedded`], you need to create a new [`crate::Window`] for you content.
    ///
    /// See [`crate::viewport`] for more information about viewports.
    pub fn show_viewport_immediate<T>(
        &self,
        new_viewport_id: ViewportId,
        builder: ViewportBuilder,
        viewport_ui_cb: impl FnOnce(&Self, ViewportClass) -> T,
    ) -> T {
        crate::profile_function!();

        if self.embed_viewports() {
            return viewport_ui_cb(self, ViewportClass::Embedded);
        }

        IMMEDIATE_VIEWPORT_RENDERER.with(|immediate_viewport_renderer| {
            let immediate_viewport_renderer = immediate_viewport_renderer.borrow();
            let Some(immediate_viewport_renderer) = immediate_viewport_renderer.as_ref() else {
                // This egui backend does not support multiple viewports.
                return viewport_ui_cb(self, ViewportClass::Embedded);
            };

            let ids = self.write(|ctx| {
                let parent_viewport_id = ctx.viewport_id();

                ctx.viewport_parents
                    .insert(new_viewport_id, parent_viewport_id);

                let viewport = ctx.viewports.entry(new_viewport_id).or_default();
                viewport.builder = builder.clone();
                viewport.used = true;
                viewport.viewport_ui_cb = None; // it is immediate

                ViewportIdPair::from_self_and_parent(new_viewport_id, parent_viewport_id)
            });

            let mut out = None;
            {
                let out = &mut out;

                let viewport = ImmediateViewport {
                    ids,
                    builder,
                    viewport_ui_cb: Box::new(move |context| {
                        *out = Some(viewport_ui_cb(context, ViewportClass::Immediate));
                    }),
                };

                immediate_viewport_renderer(self, viewport);
            }

            out.expect(
                "egui backend is implemented incorrectly - the user callback was never called",
            )
        })
    }
}

/// ## Interaction
impl Context {
    /// Read you what widgets are currently being interacted with.
    pub fn interaction_snapshot<R>(&self, reader: impl FnOnce(&InteractionSnapshot) -> R) -> R {
        self.write(|w| reader(&w.viewport().interact_widgets))
    }

    /// The widget currently being dragged, if any.
    ///
    /// For widgets that sense both clicks and drags, this will
    /// not be set until the mouse cursor has moved a certain distance.
    ///
    /// NOTE: if the widget was released this frame, this will be `None`.
    /// Use [`Self::drag_stopped_id`] instead.
    pub fn dragged_id(&self) -> Option<Id> {
        self.interaction_snapshot(|i| i.dragged)
    }

    /// Is this specific widget being dragged?
    ///
    /// A widget that sense both clicks and drags is only marked as "dragged"
    /// when the mouse has moved a bit
    ///
    /// See also: [`crate::Response::dragged`].
    pub fn is_being_dragged(&self, id: Id) -> bool {
        self.dragged_id() == Some(id)
    }

    /// This widget just started being dragged this frame.
    ///
    /// The same widget should also be found in [`Self::dragged_id`].
    pub fn drag_started_id(&self) -> Option<Id> {
        self.interaction_snapshot(|i| i.drag_started)
    }

    /// This widget was being dragged, but was released this frame
    pub fn drag_stopped_id(&self) -> Option<Id> {
        self.interaction_snapshot(|i| i.drag_stopped)
    }

    /// Set which widget is being dragged.
    pub fn set_dragged_id(&self, id: Id) {
        self.write(|ctx| {
            let vp = ctx.viewport();
            let i = &mut vp.interact_widgets;
            if i.dragged != Some(id) {
                i.drag_stopped = i.dragged.or(i.drag_stopped);
                i.dragged = Some(id);
                i.drag_started = Some(id);
            }

            ctx.memory.interaction_mut().potential_drag_id = Some(id);
        });
    }

    /// Stop dragging any widget.
    pub fn stop_dragging(&self) {
        self.write(|ctx| {
            let vp = ctx.viewport();
            let i = &mut vp.interact_widgets;
            if i.dragged.is_some() {
                i.drag_stopped = i.dragged;
                i.dragged = None;
            }

            ctx.memory.interaction_mut().potential_drag_id = None;
        });
    }

    /// Is something else being dragged?
    ///
    /// Returns true if we are dragging something, but not the given widget.
    #[inline(always)]
    pub fn dragging_something_else(&self, not_this: Id) -> bool {
        let dragged = self.dragged_id();
        dragged.is_some() && dragged != Some(not_this)
    }
}

#[test]
fn context_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Context>();
}
