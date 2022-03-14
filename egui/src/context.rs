// #![warn(missing_docs)]

use crate::{
    animation_manager::AnimationManager, data::output::PlatformOutput, frame_state::FrameState,
    input_state::*, layers::GraphicLayers, memory::Options, output::FullOutput, TextureHandle, *,
};
use epaint::{mutex::*, stats::*, text::Fonts, TessellationOptions, *};

// ----------------------------------------------------------------------------

struct WrappedTextureManager(Arc<RwLock<epaint::TextureManager>>);

impl Default for WrappedTextureManager {
    fn default() -> Self {
        let mut tex_mngr = epaint::textures::TextureManager::default();

        // Will be filled in later
        let font_id = tex_mngr.alloc(
            "egui_font_texture".into(),
            epaint::AlphaImage::new([0, 0]).into(),
        );
        assert_eq!(font_id, TextureId::default());

        Self(Arc::new(RwLock::new(tex_mngr)))
    }
}

// ----------------------------------------------------------------------------

#[derive(Default)]
struct ContextImpl {
    /// `None` until the start of the first frame.
    fonts: Option<Fonts>,
    memory: Memory,
    animation_manager: AnimationManager,
    tex_manager: WrappedTextureManager,

    input: InputState,

    /// State that is collected during a frame and then cleared
    frame_state: FrameState,

    // The output of a frame:
    graphics: GraphicLayers,
    output: PlatformOutput,

    paint_stats: PaintStats,

    /// While positive, keep requesting repaints. Decrement at the end of each frame.
    repaint_requests: u32,
}

impl ContextImpl {
    fn begin_frame_mut(&mut self, new_raw_input: RawInput) {
        self.memory.begin_frame(&self.input, &new_raw_input);

        self.input = std::mem::take(&mut self.input).begin_frame(new_raw_input);

        if let Some(new_pixels_per_point) = self.memory.new_pixels_per_point.take() {
            self.input.pixels_per_point = new_pixels_per_point;
        }

        self.frame_state.begin_frame(&self.input);

        self.update_fonts_mut();

        // Ensure we register the background area so panels and background ui can catch clicks:
        let screen_rect = self.input.screen_rect();
        self.memory.areas.set_state(
            LayerId::background(),
            containers::area::State {
                pos: screen_rect.min,
                size: screen_rect.size(),
                interactable: true,
            },
        );
    }

    /// Load fonts unless already loaded.
    fn update_fonts_mut(&mut self) {
        let pixels_per_point = self.input.pixels_per_point();
        let max_texture_side = self.input.max_texture_side;

        if let Some(font_definitions) = self.memory.new_font_definitions.take() {
            let fonts = Fonts::new(pixels_per_point, max_texture_side, font_definitions);
            self.fonts = Some(fonts);
        }

        let fonts = self.fonts.get_or_insert_with(|| {
            let font_definitions = FontDefinitions::default();
            Fonts::new(pixels_per_point, max_texture_side, font_definitions)
        });

        fonts.begin_frame(pixels_per_point, max_texture_side);

        if self.memory.options.preload_font_glyphs {
            // Preload the most common characters for the most common fonts.
            // This is not very important to do, but may a few GPU operations.
            for font_id in self.memory.options.style.text_styles.values() {
                fonts.lock().fonts.font(font_id).preload_common_characters();
            }
        }
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
/// All methods are marked `&self`; `Context` has interior mutability (protected by a mutex).
///
///
/// You can store
///
/// # Example:
///
/// ``` no_run
/// # fn handle_platform_output(_: egui::PlatformOutput) {}
/// # fn paint(textures_detla: egui::TexturesDelta, _: Vec<egui::ClippedPrimitive>) {}
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
///     let clipped_primitives = ctx.tessellate(full_output.shapes); // create triangles to paint
///     paint(full_output.textures_delta, clipped_primitives);
/// }
/// ```
#[derive(Clone)]
pub struct Context(Arc<RwLock<ContextImpl>>);

impl std::cmp::PartialEq for Context {
    fn eq(&self, other: &Context) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Default for Context {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(ContextImpl {
            // Start with painting an extra frame to compensate for some widgets
            // that take two frames before they "settle":
            repaint_requests: 1,
            ..ContextImpl::default()
        })))
    }
}

impl Context {
    fn read(&self) -> RwLockReadGuard<'_, ContextImpl> {
        self.0.read()
    }

    fn write(&self) -> RwLockWriteGuard<'_, ContextImpl> {
        self.0.write()
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
    pub fn run(&self, new_input: RawInput, run_ui: impl FnOnce(&Context)) -> FullOutput {
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
        self.write().begin_frame_mut(new_input);
    }

    // ---------------------------------------------------------------------

    /// If the given [`Id`] is not unique, an error will be printed at the given position.
    /// Call this for [`Id`]:s that need interaction or persistence.
    pub(crate) fn register_interaction_id(&self, id: Id, new_rect: Rect) {
        let prev_rect = self.frame_state().used_ids.insert(id, new_rect);
        if let Some(prev_rect) = prev_rect {
            // it is ok to reuse the same ID for e.g. a frame around a widget,
            // or to check for interaction with the same widget twice:
            if prev_rect.expand(0.1).contains_rect(new_rect)
                || new_rect.expand(0.1).contains_rect(prev_rect)
            {
                return;
            }

            let show_error = |pos: Pos2, text: String| {
                let painter = self.debug_painter();
                let rect = painter.error(pos, text);
                if let Some(pointer_pos) = self.pointer_hover_pos() {
                    if rect.contains(pointer_pos) {
                        painter.error(
                            rect.left_bottom() + vec2(2.0, 4.0),
                            "ID clashes happens when things like Windows or CollapsingHeaders share names,\n\
                             or when things like ScrollAreas and Resize areas aren't given unique id_source:s.",
                        );
                    }
                }
            };

            let id_str = id.short_debug_format();

            if prev_rect.min.distance(new_rect.min) < 4.0 {
                show_error(new_rect.min, format!("Double use of ID {}", id_str));
            } else {
                show_error(prev_rect.min, format!("First use of ID {}", id_str));
                show_error(new_rect.min, format!("Second use of ID {}", id_str));
            }
        }
    }

    // ---------------------------------------------------------------------

    /// Use `ui.interact` instead
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn interact(
        &self,
        clip_rect: Rect,
        item_spacing: Vec2,
        layer_id: LayerId,
        id: Id,
        rect: Rect,
        sense: Sense,
        enabled: bool,
    ) -> Response {
        let gap = 0.5; // Just to make sure we don't accidentally hover two things at once (a small eps should be sufficient).

        // Make it easier to click things:
        let interact_rect = rect.expand2(
            (0.5 * item_spacing - Vec2::splat(gap))
                .at_least(Vec2::splat(0.0))
                .at_most(Vec2::splat(5.0)),
        ); // make it easier to click
        let hovered = self.rect_contains_pointer(layer_id, clip_rect.intersect(interact_rect));
        self.interact_with_hovered(layer_id, id, rect, sense, enabled, hovered)
    }

    /// You specify if a thing is hovered, and the function gives a `Response`.
    pub(crate) fn interact_with_hovered(
        &self,
        layer_id: LayerId,
        id: Id,
        rect: Rect,
        sense: Sense,
        enabled: bool,
        hovered: bool,
    ) -> Response {
        let hovered = hovered && enabled; // can't even hover disabled widgets

        let mut response = Response {
            ctx: self.clone(),
            layer_id,
            id,
            rect,
            sense,
            enabled,
            hovered,
            clicked: Default::default(),
            double_clicked: Default::default(),
            dragged: false,
            drag_released: false,
            is_pointer_button_down_on: false,
            interact_pointer_pos: None,
            changed: false, // must be set by the widget itself
        };

        if !enabled || !sense.focusable || !layer_id.allow_interaction() {
            // Not interested or allowed input:
            self.memory().surrender_focus(id);
            return response;
        }

        self.register_interaction_id(id, rect);

        let clicked_elsewhere = response.clicked_elsewhere();
        let ctx_impl = &mut *self.write();
        let memory = &mut ctx_impl.memory;
        let input = &mut ctx_impl.input;

        // We only want to focus labels if the screen reader is on.
        let interested_in_focus =
            sense.interactive() || sense.focusable && memory.options.screen_reader;

        if interested_in_focus {
            memory.interested_in_focus(id);
        }

        if sense.click
            && memory.has_focus(response.id)
            && (input.key_pressed(Key::Space) || input.key_pressed(Key::Enter))
        {
            // Space/enter works like a primary click for e.g. selected buttons
            response.clicked[PointerButton::Primary as usize] = true;
        }

        if sense.click || sense.drag {
            memory.interaction.click_interest |= hovered && sense.click;
            memory.interaction.drag_interest |= hovered && sense.drag;

            response.dragged = memory.interaction.drag_id == Some(id);
            response.is_pointer_button_down_on =
                memory.interaction.click_id == Some(id) || response.dragged;

            for pointer_event in &input.pointer.pointer_events {
                match pointer_event {
                    PointerEvent::Moved(_) => {}
                    PointerEvent::Pressed(_) => {
                        if hovered {
                            if sense.click && memory.interaction.click_id.is_none() {
                                // potential start of a click
                                memory.interaction.click_id = Some(id);
                                response.is_pointer_button_down_on = true;
                            }

                            // HACK: windows have low priority on dragging.
                            // This is so that if you drag a slider in a window,
                            // the slider will steal the drag away from the window.
                            // This is needed because we do window interaction first (to prevent frame delay),
                            // and then do content layout.
                            if sense.drag
                                && (memory.interaction.drag_id.is_none()
                                    || memory.interaction.drag_is_window)
                            {
                                // potential start of a drag
                                memory.interaction.drag_id = Some(id);
                                memory.interaction.drag_is_window = false;
                                memory.window_interaction = None; // HACK: stop moving windows (if any)
                                response.is_pointer_button_down_on = true;
                                response.dragged = true;
                            }
                        }
                    }
                    PointerEvent::Released(click) => {
                        response.drag_released = response.dragged;
                        response.dragged = false;

                        if hovered && response.is_pointer_button_down_on {
                            if let Some(click) = click {
                                let clicked = hovered && response.is_pointer_button_down_on;
                                response.clicked[click.button as usize] = clicked;
                                response.double_clicked[click.button as usize] =
                                    clicked && click.is_double();
                            }
                        }
                    }
                }
            }
        }

        if response.is_pointer_button_down_on {
            response.interact_pointer_pos = input.pointer.interact_pos();
        }

        if input.pointer.any_down() {
            response.hovered &= response.is_pointer_button_down_on; // we don't hover widgets while interacting with *other* widgets
        }

        if memory.has_focus(response.id) && clicked_elsewhere {
            memory.surrender_focus(id);
        }

        if response.dragged() && !memory.has_focus(response.id) {
            // e.g.: remove focus from a widget when you drag something else
            memory.stop_text_input();
        }

        response
    }

    /// Get a full-screen painter for a new or existing layer
    pub fn layer_painter(&self, layer_id: LayerId) -> Painter {
        let screen_rect = self.input().screen_rect();
        Painter::new(self.clone(), layer_id, screen_rect)
    }

    /// Paint on top of everything else
    pub fn debug_painter(&self) -> Painter {
        Self::layer_painter(self, LayerId::debug())
    }

    /// How much space is still available after panels has been added.
    /// This is the "background" area, what egui doesn't cover with panels (but may cover with windows).
    /// This is also the area to which windows are constrained.
    pub fn available_rect(&self) -> Rect {
        self.frame_state().available_rect()
    }
}

/// ## Borrows parts of [`Context`]
impl Context {
    /// Stores all the egui state.
    ///
    /// If you want to store/restore egui, serialize this.
    #[inline]
    pub fn memory(&self) -> RwLockWriteGuard<'_, Memory> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.memory)
    }

    /// Stores superficial widget state.
    #[inline]
    pub fn data(&self) -> RwLockWriteGuard<'_, crate::util::IdTypeMap> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.memory.data)
    }

    #[inline]
    pub(crate) fn graphics(&self) -> RwLockWriteGuard<'_, GraphicLayers> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.graphics)
    }

    /// What egui outputs each frame.
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// ctx.output().cursor_icon = egui::CursorIcon::Progress;
    /// ```
    #[inline]
    pub fn output(&self) -> RwLockWriteGuard<'_, PlatformOutput> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.output)
    }

    #[inline]
    pub(crate) fn frame_state(&self) -> RwLockWriteGuard<'_, FrameState> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.frame_state)
    }

    /// Access the [`InputState`].
    ///
    /// Note that this locks the [`Context`], so be careful with if-let bindings:
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// if let Some(pos) = ctx.input().pointer.hover_pos() {
    ///     // ⚠️ Using `ctx` again here will lead to a dead-lock!
    /// }
    ///
    /// if let Some(pos) = { ctx.input().pointer.hover_pos() } {
    ///     // This is fine!
    /// }
    ///
    /// let pos = ctx.input().pointer.hover_pos();
    /// if let Some(pos) = pos {
    ///     // This is fine!
    /// }
    /// ```
    #[inline]
    pub fn input(&self) -> RwLockReadGuard<'_, InputState> {
        RwLockReadGuard::map(self.read(), |c| &c.input)
    }

    #[inline]
    pub fn input_mut(&self) -> RwLockWriteGuard<'_, InputState> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.input)
    }

    /// Not valid until first call to [`Context::run()`].
    /// That's because since we don't know the proper `pixels_per_point` until then.
    #[inline]
    pub fn fonts(&self) -> RwLockReadGuard<'_, Fonts> {
        RwLockReadGuard::map(self.read(), |c| {
            c.fonts
                .as_ref()
                .expect("No fonts available until first call to Context::run()")
        })
    }

    #[inline]
    fn fonts_mut(&self) -> RwLockWriteGuard<'_, Option<Fonts>> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.fonts)
    }

    #[inline]
    pub fn options(&self) -> RwLockWriteGuard<'_, Options> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.memory.options)
    }

    /// Change the options used by the tessellator.
    #[inline]
    pub fn tessellation_options(&self) -> RwLockWriteGuard<'_, TessellationOptions> {
        RwLockWriteGuard::map(self.write(), |c| &mut c.memory.options.tessellation_options)
    }
}

impl Context {
    /// Call this if there is need to repaint the UI, i.e. if you are showing an animation.
    /// If this is called at least once in a frame, then there will be another frame right after this.
    /// Call as many times as you wish, only one repaint will be issued.
    pub fn request_repaint(&self) {
        // request two frames of repaint, just to cover some corner cases (frame delays):
        self.write().repaint_requests = 2;
    }

    /// Tell `egui` which fonts to use.
    ///
    /// The default `egui` fonts only support latin and cyrillic alphabets,
    /// but you can call this to install additional fonts that support e.g. korean characters.
    ///
    /// The new fonts will become active at the start of the next frame.
    pub fn set_fonts(&self, font_definitions: FontDefinitions) {
        if let Some(current_fonts) = &*self.fonts_mut() {
            // NOTE: this comparison is expensive since it checks TTF data for equality
            if current_fonts.lock().fonts.definitions() == &font_definitions {
                return; // no change - save us from reloading font textures
            }
        }

        self.memory().new_font_definitions = Some(font_definitions);
    }

    /// The [`Style`] used by all subsequent windows, panels etc.
    pub fn style(&self) -> Arc<Style> {
        self.options().style.clone()
    }

    /// The [`Style`] used by all new windows, panels etc.
    ///
    /// You can also use [`Ui::style_mut`] to change the style of a single [`Ui`].
    ///
    /// Example:
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// let mut style: egui::Style = (*ctx.style()).clone();
    /// style.spacing.item_spacing = egui::vec2(10.0, 20.0);
    /// ctx.set_style(style);
    /// ```
    pub fn set_style(&self, style: impl Into<Arc<Style>>) {
        self.options().style = style.into();
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
        std::sync::Arc::make_mut(&mut self.options().style).visuals = visuals;
    }

    /// The number of physical pixels for each logical point.
    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.input().pixels_per_point()
    }

    /// Set the number of physical pixels for each logical point.
    /// Will become active at the start of the next frame.
    ///
    /// Note that this may be overwritten by input from the integration via [`RawInput::pixels_per_point`].
    /// For instance, when using `egui_web` the browsers native zoom level will always be used.
    pub fn set_pixels_per_point(&self, pixels_per_point: f32) {
        if pixels_per_point != self.pixels_per_point() {
            self.request_repaint();
        }

        self.memory().new_pixels_per_point = Some(pixels_per_point);
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

    /// Allocate a texture.
    ///
    /// In order to display an image you must convert it to a texture using this function.
    ///
    /// Make sure to only call this once for each image, i.e. NOT in your main GUI code.
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
    ///             ui.ctx().load_texture("my-image", egui::ColorImage::example())
    ///         });
    ///
    ///         // Show the image:
    ///         ui.image(texture, texture.size_vec2());
    ///     }
    /// }
    /// ```
    ///
    /// Se also [`crate::ImageData`], [`crate::Ui::image`] and [`crate::ImageButton`].
    pub fn load_texture(
        &self,
        name: impl Into<String>,
        image: impl Into<ImageData>,
    ) -> TextureHandle {
        let tex_mngr = self.tex_manager();
        let tex_id = tex_mngr.write().alloc(name.into(), image.into());
        TextureHandle::new(tex_mngr, tex_id)
    }

    /// Low-level texture manager.
    ///
    /// In general it is easier to use [`Self::load_texture`] and [`TextureHandle`].
    ///
    /// You can show stats about the allocated textures using [`Self::texture_ui`].
    pub fn tex_manager(&self) -> Arc<RwLock<epaint::textures::TextureManager>> {
        self.read().tex_manager.0.clone()
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
            area.max.x = self.input().screen_rect().max.x;
            area.min.x = self.input().screen_rect().min.x;
        }
        if window.height() > area.height() {
            // Allow overlapping top/bottom bars:
            area.max.y = self.input().screen_rect().max.y;
            area.min.y = self.input().screen_rect().min.y;
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
        if self.input().wants_repaint() {
            self.request_repaint();
        }

        let textures_delta;
        {
            let ctx_impl = &mut *self.write();
            ctx_impl
                .memory
                .end_frame(&ctx_impl.input, &ctx_impl.frame_state.used_ids);

            let font_image_delta = ctx_impl.fonts.as_ref().unwrap().font_image_delta();
            if let Some(font_image_delta) = font_image_delta {
                ctx_impl
                    .tex_manager
                    .0
                    .write()
                    .set(TextureId::default(), font_image_delta);
            }

            textures_delta = ctx_impl.tex_manager.0.write().take_delta();
        };

        let platform_output: PlatformOutput = std::mem::take(&mut self.output());

        let needs_repaint = if self.read().repaint_requests > 0 {
            self.write().repaint_requests -= 1;
            true
        } else {
            false
        };

        let shapes = self.drain_paint_lists();

        FullOutput {
            platform_output,
            needs_repaint,
            textures_delta,
            shapes,
        }
    }

    fn drain_paint_lists(&self) -> Vec<ClippedShape> {
        let ctx_impl = &mut *self.write();
        ctx_impl
            .graphics
            .drain(ctx_impl.memory.areas.order())
            .collect()
    }

    /// Tessellate the given shapes into triangle meshes.
    pub fn tessellate(&self, shapes: Vec<ClippedShape>) -> Vec<ClippedPrimitive> {
        // A tempting optimization is to reuse the tessellation from last frame if the
        // shapes are the same, but just comparing the shapes takes about 50% of the time
        // it takes to tessellate them, so it is not a worth optimization.

        let mut tessellation_options = *self.tessellation_options();
        tessellation_options.pixels_per_point = self.pixels_per_point();
        tessellation_options.aa_size = 1.0 / self.pixels_per_point();
        let paint_stats = PaintStats::from_shapes(&shapes);
        let clipped_primitives = tessellator::tessellate_shapes(
            shapes,
            tessellation_options,
            self.fonts().font_image_size(),
        );
        self.write().paint_stats = paint_stats.with_clipped_primitives(&clipped_primitives);
        clipped_primitives
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
    /// You can shrink your egui area to this size and still fit all egui components.
    pub fn used_size(&self) -> Vec2 {
        self.used_rect().max - Pos2::new(0.0, 0.0)
    }

    // ---------------------------------------------------------------------

    /// Is the pointer (mouse/touch) over any egui area?
    pub fn is_pointer_over_area(&self) -> bool {
        let pointer_pos = self.input().pointer.interact_pos();
        if let Some(pointer_pos) = pointer_pos {
            if let Some(layer) = self.layer_id_at(pointer_pos) {
                if layer.order == Order::Background {
                    !self.frame_state().unused_rect.contains(pointer_pos)
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
    /// Could be the pointer is hovering over a [`Window`] or the user is dragging a widget.
    /// If `false`, the pointer is outside of any egui area and so
    /// you may be interested in what it is doing (e.g. controlling your game).
    /// Returns `false` if a drag started outside of egui and then moved over an egui area.
    pub fn wants_pointer_input(&self) -> bool {
        self.is_using_pointer() || (self.is_pointer_over_area() && !self.input().pointer.any_down())
    }

    /// Is egui currently using the pointer position (e.g. dragging a slider).
    /// NOTE: this will return `false` if the pointer is just hovering over an egui area.
    pub fn is_using_pointer(&self) -> bool {
        self.memory().interaction.is_using_pointer()
    }

    /// If `true`, egui is currently listening on text input (e.g. typing text in a [`TextEdit`]).
    pub fn wants_keyboard_input(&self) -> bool {
        self.memory().interaction.focus.focused().is_some()
    }
}

// Ergonomic methods to forward some calls often used in 'if let' without holding the borrow
impl Context {
    /// Latest reported pointer position.
    /// When tapping a touch screen, this will be `None`.
    #[inline(always)]
    pub(crate) fn latest_pointer_pos(&self) -> Option<Pos2> {
        self.input().pointer.latest_pos()
    }

    /// If it is a good idea to show a tooltip, where is pointer?
    #[inline(always)]
    pub fn pointer_hover_pos(&self) -> Option<Pos2> {
        self.input().pointer.hover_pos()
    }

    /// If you detect a click or drag and wants to know where it happened, use this.
    ///
    /// Latest position of the mouse, but ignoring any [`Event::PointerGone`]
    /// if there were interactions this frame.
    /// When tapping a touch screen, this will be the location of the touch.
    #[inline(always)]
    pub fn pointer_interact_pos(&self) -> Option<Pos2> {
        self.input().pointer.interact_pos()
    }

    /// Calls [`InputState::multi_touch`].
    pub fn multi_touch(&self) -> Option<MultiTouchInfo> {
        self.input().multi_touch()
    }
}

impl Context {
    /// Move all the graphics at the given layer.
    /// Can be used to implement drag-and-drop (see relevant demo).
    pub fn translate_layer(&self, layer_id: LayerId, delta: Vec2) {
        if delta != Vec2::ZERO {
            self.graphics().list(layer_id).translate(delta);
        }
    }

    /// Top-most layer at the given position.
    pub fn layer_id_at(&self, pos: Pos2) -> Option<LayerId> {
        let resize_grab_radius_side = self.style().interaction.resize_grab_radius_side;
        self.memory().layer_id_at(pos, resize_grab_radius_side)
    }

    /// The overall top-most layer. When an area is clicked on or interacted
    /// with, it is moved above all other areas.
    pub fn top_most_layer(&self) -> Option<LayerId> {
        self.memory().top_most_layer()
    }

    /// Moves the given area to the top.
    pub fn move_to_top(&self, layer_id: LayerId) {
        self.memory().areas.move_to_top(layer_id);
    }

    pub(crate) fn rect_contains_pointer(&self, layer_id: LayerId, rect: Rect) -> bool {
        let pointer_pos = self.input().pointer.interact_pos();
        if let Some(pointer_pos) = pointer_pos {
            rect.contains(pointer_pos) && self.layer_id_at(pointer_pos) == Some(layer_id)
        } else {
            false
        }
    }

    // ---------------------------------------------------------------------

    /// Whether or not to debug widget layout on hover.
    pub fn debug_on_hover(&self) -> bool {
        self.options().style.debug.debug_on_hover
    }

    /// Turn on/off whether or not to debug widget layout on hover.
    pub fn set_debug_on_hover(&self, debug_on_hover: bool) {
        let mut style = (*self.options().style).clone();
        style.debug.debug_on_hover = debug_on_hover;
        self.set_style(style);
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
    pub fn animate_bool(&self, id: Id, value: bool) -> f32 {
        let animation_time = self.style().animation_time;
        self.animate_bool_with_time(id, value, animation_time)
    }

    /// Like [`Self::animate_bool`] but allows you to control the animation time.
    pub fn animate_bool_with_time(&self, id: Id, value: bool, animation_time: f32) -> f32 {
        let animated_value = {
            let ctx_impl = &mut *self.write();
            ctx_impl
                .animation_manager
                .animate_bool(&ctx_impl.input, animation_time, id, value)
        };
        let animation_in_progress = 0.0 < animated_value && animated_value < 1.0;
        if animation_in_progress {
            self.request_repaint();
        }
        animated_value
    }

    /// Allows you to smoothly change the f32 value.
    /// At the first call the value is written to memory.
    /// When it is called with a new value, it linearly interpolates to it in the given time.
    pub fn animate_value_with_time(&self, id: Id, value: f32, animation_time: f32) -> f32 {
        let animated_value = {
            let ctx_impl = &mut *self.write();
            ctx_impl
                .animation_manager
                .animate_value(&ctx_impl.input, animation_time, id, value)
        };
        let animation_in_progress = animated_value != value;
        if animation_in_progress {
            self.request_repaint();
        }

        animated_value
    }

    /// Clear memory of any animations.
    pub fn clear_animations(&self) {
        self.write().animation_manager = Default::default();
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

        CollapsingHeader::new("✒ Painting")
            .default_open(true)
            .show(ui, |ui| {
                let mut tessellation_options = self.options().tessellation_options;
                tessellation_options.ui(ui);
                ui.vertical_centered(|ui| reset_button(ui, &mut tessellation_options));
                *self.tessellation_options() = tessellation_options;
            });
    }

    pub fn inspection_ui(&self, ui: &mut Ui) {
        use crate::containers::*;
        crate::trace!(ui);

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
            self.memory()
                .interaction
                .focus
                .focused()
                .as_ref()
                .map(Id::short_debug_format)
                .unwrap_or_default()
        ))
        .on_hover_text("Is egui currently listening for text input?");

        let pointer_pos = self
            .pointer_hover_pos()
            .map_or_else(String::new, |pos| format!("{:?}", pos));
        ui.label(format!("Pointer pos: {}", pointer_pos));

        let top_layer = self
            .pointer_hover_pos()
            .and_then(|pos| self.layer_id_at(pos))
            .map_or_else(String::new, |layer| layer.short_debug_format());
        ui.label(format!("Top layer under mouse: {}", top_layer));

        ui.add_space(16.0);

        ui.label(format!(
            "There are {} text galleys in the layout cache",
            self.fonts().num_galleys_in_cache()
        ))
        .on_hover_text("This is approximately the number of text strings on screen");
        ui.add_space(16.0);

        CollapsingHeader::new("📥 Input")
            .default_open(false)
            .show(ui, |ui| {
                let input = ui.input().clone();
                input.ui(ui);
            });

        CollapsingHeader::new("📊 Paint stats")
            .default_open(false)
            .show(ui, |ui| {
                let paint_stats = self.write().paint_stats;
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
                let font_image_size = self.fonts().font_image_size();
                crate::introspection::font_texture_ui(ui, font_image_size);
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
        let max_preview_size = Vec2::new(48.0, 32.0);

        ui.group(|ui| {
            ScrollArea::vertical()
                .max_height(300.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.style_mut().override_text_style = Some(TextStyle::Monospace);
                    Grid::new("textures")
                        .striped(true)
                        .num_columns(4)
                        .spacing(Vec2::new(16.0, 2.0))
                        .min_row_height(max_preview_size.y)
                        .show(ui, |ui| {
                            for (&texture_id, meta) in textures {
                                let [w, h] = meta.size;

                                let mut size = Vec2::new(w as f32, h as f32);
                                size *= (max_preview_size.x / size.x).min(1.0);
                                size *= (max_preview_size.y / size.y).min(1.0);
                                ui.image(texture_id, size).on_hover_ui(|ui| {
                                    // show larger on hover
                                    let max_size = 0.5 * ui.ctx().input().screen_rect().size();
                                    let mut size = Vec2::new(w as f32, h as f32);
                                    size *= max_size.x / size.x.max(max_size.x);
                                    size *= max_size.y / size.y.max(max_size.y);
                                    ui.image(texture_id, size);
                                });

                                ui.label(format!("{} x {}", w, h));
                                ui.label(format!("{:.3} MB", meta.bytes_used() as f64 * 1e-6));
                                ui.label(format!("{:?}", meta.name));
                                ui.end_row();
                            }
                        });
                });
        });
    }

    pub fn memory_ui(&self, ui: &mut crate::Ui) {
        if ui
            .button("Reset all")
            .on_hover_text("Reset all egui state")
            .clicked()
        {
            *self.memory() = Default::default();
        }

        let num_state = self.data().len();
        let num_serialized = self.data().count_serialized();
        ui.label(format!(
            "{} widget states stored (of which {} are serialized).",
            num_state, num_serialized
        ));

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} areas (panels, windows, popups, …)",
                self.memory().areas.count()
            ));
            if ui.button("Reset").clicked() {
                self.memory().areas = Default::default();
            }
        });
        ui.indent("areas", |ui| {
            ui.label("Visible areas, ordered back to front.");
            ui.label("Hover to highlight");
            let layers_ids: Vec<LayerId> = self.memory().areas.order().to_vec();
            for layer_id in layers_ids {
                let area = self.memory().areas.get(layer_id.id).cloned();
                if let Some(area) = area {
                    let is_visible = self.memory().areas.is_visible(&layer_id);
                    if !is_visible {
                        continue;
                    }
                    let text = format!("{} - {:?}", layer_id.short_debug_format(), area.rect(),);
                    // TODO: `Sense::hover_highlight()`
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
                self.data().count::<containers::collapsing_header::State>()
            ));
            if ui.button("Reset").clicked() {
                self.data()
                    .remove_by_type::<containers::collapsing_header::State>();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} menu bars",
                self.data().count::<menu::BarState>()
            ));
            if ui.button("Reset").clicked() {
                self.data().remove_by_type::<menu::BarState>();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} scroll areas",
                self.data().count::<scroll_area::State>()
            ));
            if ui.button("Reset").clicked() {
                self.data().remove_by_type::<scroll_area::State>();
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} resize areas",
                self.data().count::<resize::State>()
            ));
            if ui.button("Reset").clicked() {
                self.data().remove_by_type::<resize::State>();
            }
        });

        ui.shrink_width_to_current(); // don't let the text below grow this window wider
        ui.label("NOTE: the position of this window cannot be reset from within itself.");

        ui.collapsing("Interaction", |ui| {
            let interaction = self.memory().interaction.clone();
            interaction.ui(ui);
        });
    }
}

impl Context {
    pub fn style_ui(&self, ui: &mut Ui) {
        let mut style: Style = (*self.style()).clone();
        style.ui(ui);
        self.set_style(style);
    }
}

#[cfg(test)]
#[test]
fn context_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Context>();
}
