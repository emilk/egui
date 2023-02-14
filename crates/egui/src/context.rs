// #![warn(missing_docs)]
use std::sync::Arc;

use crate::{
    animation_manager::AnimationManager, data::output::PlatformOutput, frame_state::FrameState,
    input_state::*, layers::GraphicLayers, memory::Options, os::OperatingSystem,
    output::FullOutput, util::IdTypeMap, TextureHandle, *,
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
            epaint::FontImage::new([0, 0]).into(),
            Default::default(),
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

    os: OperatingSystem,

    input: InputState,

    /// State that is collected during a frame and then cleared
    frame_state: FrameState,

    // The output of a frame:
    graphics: GraphicLayers,
    output: PlatformOutput,

    paint_stats: PaintStats,

    /// the duration backend will poll for new events, before forcing another egui update
    /// even if there's no new events.
    repaint_after: std::time::Duration,

    /// While positive, keep requesting repaints. Decrement at the end of each frame.
    repaint_requests: u32,
    request_repaint_callback: Option<Box<dyn Fn() + Send + Sync>>,

    /// used to suppress multiple calls to [`Self::request_repaint_callback`] during the same frame.
    has_requested_repaint_this_frame: bool,

    requested_repaint_last_frame: bool,

    /// Written to during the frame.
    layer_rects_this_frame: ahash::HashMap<LayerId, Vec<(Id, Rect)>>,

    /// Read
    layer_rects_prev_frame: ahash::HashMap<LayerId, Vec<(Id, Rect)>>,

    #[cfg(feature = "accesskit")]
    is_accesskit_enabled: bool,
    #[cfg(feature = "accesskit")]
    accesskit_node_classes: accesskit::NodeClassSet,
}

impl ContextImpl {
    fn begin_frame_mut(&mut self, mut new_raw_input: RawInput) {
        self.has_requested_repaint_this_frame = false; // allow new calls during the frame

        if let Some(new_pixels_per_point) = self.memory.new_pixels_per_point.take() {
            new_raw_input.pixels_per_point = Some(new_pixels_per_point);

            // This is a bit hacky, but is required to avoid jitter:
            let ratio = self.input.pixels_per_point / new_pixels_per_point;
            let mut rect = self.input.screen_rect;
            rect.min = (ratio * rect.min.to_vec2()).to_pos2();
            rect.max = (ratio * rect.max.to_vec2()).to_pos2();
            new_raw_input.screen_rect = Some(rect);
        }

        self.layer_rects_prev_frame = std::mem::take(&mut self.layer_rects_this_frame);

        self.memory.begin_frame(&self.input, &new_raw_input);

        self.input = std::mem::take(&mut self.input)
            .begin_frame(new_raw_input, self.requested_repaint_last_frame);

        self.frame_state.begin_frame(&self.input);

        self.update_fonts_mut();

        // Ensure we register the background area so panels and background ui can catch clicks:
        let screen_rect = self.input.screen_rect();
        self.memory.areas.set_state(
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
            use crate::frame_state::AccessKitFrameState;
            let id = crate::accesskit_root_id();
            let mut builder = accesskit::NodeBuilder::new(accesskit::Role::Window);
            builder.set_transform(accesskit::Affine::scale(
                self.input.pixels_per_point().into(),
            ));
            let mut node_builders = IdMap::default();
            node_builders.insert(id, builder);
            self.frame_state.accesskit_state = Some(AccessKitFrameState {
                node_builders,
                parent_stack: vec![id],
            });
        }
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

    #[cfg(feature = "accesskit")]
    fn accesskit_node_builder(&mut self, id: Id) -> &mut accesskit::NodeBuilder {
        let state = self.frame_state.accesskit_state.as_mut().unwrap();
        let builders = &mut state.node_builders;
        if let std::collections::hash_map::Entry::Vacant(entry) = builders.entry(id) {
            entry.insert(Default::default());
            let parent_id = state.parent_stack.last().unwrap();
            let parent_builder = builders.get_mut(parent_id).unwrap();
            parent_builder.push_child(id.accesskit_id());
        }
        builders.get_mut(&id).unwrap()
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

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context").finish_non_exhaustive()
    }
}

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
    // Do read-only (shared access) transaction on Context
    fn read<R>(&self, reader: impl FnOnce(&ContextImpl) -> R) -> R {
        reader(&self.0.read())
    }

    // Do read-write (exclusive access) transaction on Context
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
    ///     // ⚠️ Using `ctx` (even from other `Arc` reference) again here will lead to a dead-lock!
    /// });
    ///
    /// if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
    ///     // This is fine!
    /// }
    /// ```
    #[inline]
    pub fn input<R>(&self, reader: impl FnOnce(&InputState) -> R) -> R {
        self.read(move |ctx| reader(&ctx.input))
    }

    /// Read-write access to [`InputState`].
    #[inline]
    pub fn input_mut<R>(&self, writer: impl FnOnce(&mut InputState) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.input))
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
    pub(crate) fn graphics_mut<R>(&self, writer: impl FnOnce(&mut GraphicLayers) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.graphics))
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
        self.read(move |ctx| reader(&ctx.output))
    }

    /// Read-write access to [`PlatformOutput`].
    #[inline]
    pub fn output_mut<R>(&self, writer: impl FnOnce(&mut PlatformOutput) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.output))
    }

    /// Read-only access to [`FrameState`].
    #[inline]
    pub(crate) fn frame_state<R>(&self, reader: impl FnOnce(&FrameState) -> R) -> R {
        self.read(move |ctx| reader(&ctx.frame_state))
    }

    /// Read-write access to [`FrameState`].
    #[inline]
    pub(crate) fn frame_state_mut<R>(&self, writer: impl FnOnce(&mut FrameState) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.frame_state))
    }

    /// Read-only access to [`Fonts`].
    ///
    /// Not valid until first call to [`Context::run()`].
    /// That's because since we don't know the proper `pixels_per_point` until then.
    #[inline]
    pub fn fonts<R>(&self, reader: impl FnOnce(&Fonts) -> R) -> R {
        self.read(move |ctx| {
            reader(
                ctx.fonts
                    .as_ref()
                    .expect("No fonts available until first call to Context::run()"),
            )
        })
    }

    /// Read-write access to [`Fonts`].
    #[inline]
    pub fn fonts_mut<R>(&self, writer: impl FnOnce(&mut Option<Fonts>) -> R) -> R {
        self.write(move |ctx| writer(&mut ctx.fonts))
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
}

impl Context {
    // ---------------------------------------------------------------------

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
        if let Some(prev_rect) = prev_rect {
            // it is ok to reuse the same ID for e.g. a frame around a widget,
            // or to check for interaction with the same widget twice:
            if prev_rect.expand(0.1).contains_rect(new_rect)
                || new_rect.expand(0.1).contains_rect(prev_rect)
            {
                return;
            }

            let show_error = |widget_rect: Rect, text: String| {
                let text = format!("🔥 {}", text);
                let color = self.style().visuals.error_fg_color;
                let painter = self.debug_painter();
                painter.rect_stroke(widget_rect, 0.0, (1.0, color));

                let below = widget_rect.bottom() + 32.0 < self.input(|i| i.screen_rect.bottom());

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
                show_error(new_rect, format!("Double use of {} ID {}", what, id_str));
            } else {
                show_error(prev_rect, format!("First use of {} ID {}", what, id_str));
                show_error(new_rect, format!("Second use of {} ID {}", what, id_str));
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
        let gap = 0.1; // Just to make sure we don't accidentally hover two things at once (a small eps should be sufficient).

        // Make it easier to click things:
        let interact_rect = rect.expand2(
            (0.5 * item_spacing - Vec2::splat(gap))
                .at_least(Vec2::splat(0.0))
                .at_most(Vec2::splat(5.0)),
        );

        // Respect clip rectangle when interacting
        let interact_rect = clip_rect.intersect(interact_rect);
        let mut hovered = self.rect_contains_pointer(layer_id, interact_rect);

        // This solves the problem of overlapping widgets.
        // Whichever widget is added LAST (=on top) gets the input:
        if interact_rect.is_positive() && sense.interactive() {
            if self.style().debug.show_interactive_widgets {
                Self::layer_painter(self, LayerId::debug()).rect(
                    interact_rect,
                    0.0,
                    Color32::YELLOW.additive().linear_multiply(0.005),
                    Stroke::new(1.0, Color32::YELLOW.additive().linear_multiply(0.05)),
                );
            }

            self.write(|ctx| {
                ctx.layer_rects_this_frame
                    .entry(layer_id)
                    .or_default()
                    .push((id, interact_rect));

                if hovered {
                    let pointer_pos = ctx.input.pointer.interact_pos();
                    if let Some(pointer_pos) = pointer_pos {
                        if let Some(rects) = ctx.layer_rects_prev_frame.get(&layer_id) {
                            for &(prev_id, prev_rect) in rects.iter().rev() {
                                if prev_id == id {
                                    break; // there is no other interactive widget covering us at the pointer position.
                                }
                                if prev_rect.contains(pointer_pos) {
                                    // Another interactive widget is covering us at the pointer position,
                                    // so we aren't hovered.

                                    if ctx.memory.options.style.debug.show_blocking_widget {
                                        Self::layer_painter(self, LayerId::debug()).debug_rect(
                                            interact_rect,
                                            Color32::GREEN,
                                            "Covered",
                                        );
                                        Self::layer_painter(self, LayerId::debug()).debug_rect(
                                            prev_rect,
                                            Color32::LIGHT_BLUE,
                                            "On top",
                                        );
                                    }

                                    hovered = false;
                                    break;
                                }
                            }
                        }
                    }
                }
            });
        }

        self.interact_with_hovered(layer_id, id, rect, sense, enabled, hovered)
    }

    /// You specify if a thing is hovered, and the function gives a [`Response`].
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

        let highlighted = self.frame_state(|fs| fs.highlight_this_frame.contains(&id));

        let mut response = Response {
            ctx: self.clone(),
            layer_id,
            id,
            rect,
            sense,
            enabled,
            hovered,
            highlighted,
            clicked: Default::default(),
            double_clicked: Default::default(),
            triple_clicked: Default::default(),
            dragged: false,
            drag_released: false,
            is_pointer_button_down_on: false,
            interact_pointer_pos: None,
            changed: false, // must be set by the widget itself
        };

        if !enabled || !sense.focusable || !layer_id.allow_interaction() {
            // Not interested or allowed input:
            self.memory_mut(|mem| mem.surrender_focus(id));
            return response;
        }

        self.check_for_id_clash(id, rect, "widget");

        #[cfg(feature = "accesskit")]
        if sense.focusable {
            // Make sure anything that can receive focus has an AccessKit node.
            // TODO(mwcampbell): For nodes that are filled from widget info,
            // some information is written to the node twice.
            self.accesskit_node_builder(id, |builder| response.fill_accesskit_node_common(builder));
        }

        let clicked_elsewhere = response.clicked_elsewhere();
        self.write(|ctx| {
            let memory = &mut ctx.memory;
            let input = &mut ctx.input;

            if sense.focusable {
                memory.interested_in_focus(id);
            }

            if sense.click
                && memory.has_focus(response.id)
                && (input.key_pressed(Key::Space) || input.key_pressed(Key::Enter))
            {
                // Space/enter works like a primary click for e.g. selected buttons
                response.clicked[PointerButton::Primary as usize] = true;
            }

            #[cfg(feature = "accesskit")]
            {
                if sense.click
                    && input.has_accesskit_action_request(response.id, accesskit::Action::Default)
                {
                    response.clicked[PointerButton::Primary as usize] = true;
                }
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
                        PointerEvent::Pressed { .. } => {
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
                        PointerEvent::Released { click, button } => {
                            response.drag_released = response.dragged;
                            response.dragged = false;

                            if hovered && response.is_pointer_button_down_on {
                                if let Some(click) = click {
                                    let clicked = hovered && response.is_pointer_button_down_on;
                                    response.clicked[*button as usize] = clicked;
                                    response.double_clicked[*button as usize] =
                                        clicked && click.is_double();
                                    response.triple_clicked[*button as usize] =
                                        clicked && click.is_triple();
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
        });

        response
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

    /// Call this if there is need to repaint the UI, i.e. if you are showing an animation.
    ///
    /// If this is called at least once in a frame, then there will be another frame right after this.
    /// Call as many times as you wish, only one repaint will be issued.
    ///
    /// If called from outside the UI thread, the UI thread will wake up and run,
    /// provided the egui integration has set that up via [`Self::set_request_repaint_callback`]
    /// (this will work on `eframe`).
    pub fn request_repaint(&self) {
        // request two frames of repaint, just to cover some corner cases (frame delays):
        self.write(|ctx| {
            ctx.repaint_requests = 2;
            if let Some(callback) = &ctx.request_repaint_callback {
                if !ctx.has_requested_repaint_this_frame {
                    (callback)();
                    ctx.has_requested_repaint_this_frame = true;
                }
            }
        });
    }

    /// Request repaint after the specified duration elapses in the case of no new input
    /// events being received.
    ///
    /// The function can be multiple times, but only the *smallest* duration will be considered.
    /// So, if the function is called two times with `1 second` and `2 seconds`, egui will repaint
    /// after `1 second`
    ///
    /// This is primarily useful for applications who would like to save battery by avoiding wasted
    /// redraws when the app is not in focus. But sometimes the GUI of the app might become stale
    /// and outdated if it is not updated for too long.
    ///
    /// Lets say, something like a stop watch widget that displays the time in seconds. You would waste
    /// resources repainting multiple times within the same second (when you have no input),
    /// just calculate the difference of duration between current time and next second change,
    /// and call this function, to make sure that you are displaying the latest updated time, but
    /// not wasting resources on needless repaints within the same second.
    ///
    /// NOTE: only works if called before `Context::end_frame()`. to force egui to update,
    /// use `Context::request_repaint()` instead.
    ///
    /// ### Quirk:
    /// Duration begins at the next frame. lets say for example that its a very inefficient app
    /// and takes 500 milliseconds per frame at 2 fps. The widget / user might want a repaint in
    /// next 500 milliseconds. Now, app takes 1000 ms per frame (1 fps) because the backend event
    /// timeout takes 500 milli seconds AFTER the vsync swap buffer.
    /// So, its not that we are requesting repaint within X duration. We are rather timing out
    /// during app idle time where we are not receiving any new input events.
    pub fn request_repaint_after(&self, duration: std::time::Duration) {
        // Maybe we can check if duration is ZERO, and call self.request_repaint()?
        self.write(|ctx| ctx.repaint_after = ctx.repaint_after.min(duration));
    }

    /// For integrations: this callback will be called when an egui user calls [`Self::request_repaint`].
    ///
    /// This lets you wake up a sleeping UI thread.
    ///
    /// Note that only one callback can be set. Any new call overrides the previous callback.
    pub fn set_request_repaint_callback(&self, callback: impl Fn() + Send + Sync + 'static) {
        let callback = Box::new(callback);
        self.write(|ctx| ctx.request_repaint_callback = Some(callback));
    }

    /// Tell `egui` which fonts to use.
    ///
    /// The default `egui` fonts only support latin and cyrillic alphabets,
    /// but you can call this to install additional fonts that support e.g. korean characters.
    ///
    /// The new fonts will become active at the start of the next frame.
    pub fn set_fonts(&self, font_definitions: FontDefinitions) {
        let update_fonts = self.fonts_mut(|fonts| {
            if let Some(current_fonts) = fonts {
                // NOTE: this comparison is expensive since it checks TTF data for equality
                current_fonts.lock().fonts.definitions() != &font_definitions
            } else {
                true
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
    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.input(|i| i.pixels_per_point())
    }

    /// Set the number of physical pixels for each logical point.
    /// Will become active at the start of the next frame.
    ///
    /// Note that this may be overwritten by input from the integration via [`RawInput::pixels_per_point`].
    /// For instance, when using `eframe` on web, the browsers native zoom level will always be used.
    pub fn set_pixels_per_point(&self, pixels_per_point: f32) {
        if pixels_per_point != self.pixels_per_point() {
            self.request_repaint();
            self.memory_mut(|mem| mem.new_pixels_per_point = Some(pixels_per_point));
        }
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
    ///             ui.ctx().load_texture(
    ///                 "my-image",
    ///                 egui::ColorImage::example(),
    ///                 Default::default()
    ///             )
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
        if self.input(|i| i.wants_repaint()) {
            self.request_repaint();
        }

        let textures_delta = self.write(|ctx| {
            ctx.memory.end_frame(&ctx.input, &ctx.frame_state.used_ids);

            let font_image_delta = ctx.fonts.as_ref().unwrap().font_image_delta();
            if let Some(font_image_delta) = font_image_delta {
                ctx.tex_manager
                    .0
                    .write()
                    .set(TextureId::default(), font_image_delta);
            }

            ctx.tex_manager.0.write().take_delta()
        });

        #[cfg_attr(not(feature = "accesskit"), allow(unused_mut))]
        let mut platform_output: PlatformOutput = self.output_mut(|o| std::mem::take(o));

        #[cfg(feature = "accesskit")]
        {
            let state = self.frame_state_mut(|fs| fs.accesskit_state.take());
            if let Some(state) = state {
                let has_focus = self.input(|i| i.raw.has_focus);
                let root_id = crate::accesskit_root_id().accesskit_id();
                let nodes = self.write(|ctx| {
                    state
                        .node_builders
                        .into_iter()
                        .map(|(id, builder)| {
                            (
                                id.accesskit_id(),
                                builder.build(&mut ctx.accesskit_node_classes),
                            )
                        })
                        .collect()
                });
                platform_output.accesskit_update = Some(accesskit::TreeUpdate {
                    nodes,
                    tree: Some(accesskit::Tree::new(root_id)),
                    focus: has_focus.then(|| {
                        let focus_id = self.memory(|mem| mem.interaction.focus.id);
                        focus_id.map_or(root_id, |id| id.accesskit_id())
                    }),
                });
            }
        }

        // if repaint_requests is greater than zero. just set the duration to zero for immediate
        // repaint. if there's no repaint requests, then we can use the actual repaint_after instead.
        let repaint_after = self.write(|ctx| {
            if ctx.repaint_requests > 0 {
                ctx.repaint_requests -= 1;
                std::time::Duration::ZERO
            } else {
                ctx.repaint_after
            }
        });

        self.write(|ctx| {
            ctx.requested_repaint_last_frame = repaint_after.is_zero();

            ctx.has_requested_repaint_this_frame = false; // allow new calls between frames

            // make sure we reset the repaint_after duration.
            // otherwise, if repaint_after is low, then any widget setting repaint_after next frame,
            // will fail to overwrite the previous lower value. and thus, repaints will never
            // go back to higher values.
            ctx.repaint_after = std::time::Duration::MAX;
        });
        let shapes = self.drain_paint_lists();

        FullOutput {
            platform_output,
            repaint_after,
            textures_delta,
            shapes,
        }
    }

    fn drain_paint_lists(&self) -> Vec<ClippedShape> {
        self.write(|ctx| ctx.graphics.drain(ctx.memory.areas.order()).collect())
    }

    /// Tessellate the given shapes into triangle meshes.
    pub fn tessellate(&self, shapes: Vec<ClippedShape>) -> Vec<ClippedPrimitive> {
        // A tempting optimization is to reuse the tessellation from last frame if the
        // shapes are the same, but just comparing the shapes takes about 50% of the time
        // it takes to tessellate them, so it is not a worth optimization.

        // here we expect that we are the only user of context, since frame is ended
        self.write(|ctx| {
            let pixels_per_point = ctx.input.pixels_per_point();
            let tessellation_options = ctx.memory.options.tessellation_options;
            let texture_atlas = ctx
                .fonts
                .as_ref()
                .expect("tessellate called before first call to Context::run()")
                .texture_atlas();
            let (font_tex_size, prepared_discs) = {
                let atlas = texture_atlas.lock();
                (atlas.size(), atlas.prepared_discs())
            };

            let paint_stats = PaintStats::from_shapes(&shapes);
            let clipped_primitives = tessellator::tessellate_shapes(
                pixels_per_point,
                tessellation_options,
                font_tex_size,
                prepared_discs,
                shapes,
            );
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
        self.read(|ctx| {
            let mut used = ctx.frame_state.used_by_panels;
            for window in ctx.memory.areas.visible_windows() {
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
        self.memory(|m| m.interaction.is_using_pointer())
    }

    /// If `true`, egui is currently listening on text input (e.g. typing text in a [`TextEdit`]).
    pub fn wants_keyboard_input(&self) -> bool {
        self.memory(|m| m.interaction.focus.focused().is_some())
    }

    /// Highlight this widget, to make it look like it is hovered, even if it isn't.
    ///
    /// The highlight takes on frame to take effect if you call this after the widget has been fully rendered.
    ///
    /// See also [`Response::highlight`].
    pub fn highlight_widget(&self, id: Id) {
        self.frame_state_mut(|fs| fs.highlight_next_frame.insert(id));
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
    /// Move all the graphics at the given layer.
    ///
    /// Can be used to implement drag-and-drop (see relevant demo).
    pub fn translate_layer(&self, layer_id: LayerId, delta: Vec2) {
        if delta != Vec2::ZERO {
            self.graphics_mut(|g| g.list(layer_id).translate(delta));
        }
    }

    /// Top-most layer at the given position.
    pub fn layer_id_at(&self, pos: Pos2) -> Option<LayerId> {
        self.memory(|mem| {
            mem.layer_id_at(pos, mem.options.style.interaction.resize_grab_radius_side)
        })
    }

    /// Moves the given area to the top in its [`Order`].
    ///
    /// [`Area`]:s and [`Window`]:s also do this automatically when being clicked on or interacted with.
    pub fn move_to_top(&self, layer_id: LayerId) {
        self.memory_mut(|mem| mem.areas.move_to_top(layer_id));
    }

    pub(crate) fn rect_contains_pointer(&self, layer_id: LayerId, rect: Rect) -> bool {
        rect.is_positive() && {
            let pointer_pos = self.input(|i| i.pointer.interact_pos());
            if let Some(pointer_pos) = pointer_pos {
                rect.contains(pointer_pos) && self.layer_id_at(pointer_pos) == Some(layer_id)
            } else {
                false
            }
        }
    }

    // ---------------------------------------------------------------------

    /// Whether or not to debug widget layout on hover.
    pub fn debug_on_hover(&self) -> bool {
        self.options(|opt| opt.style.debug.debug_on_hover)
    }

    /// Turn on/off whether or not to debug widget layout on hover.
    pub fn set_debug_on_hover(&self, debug_on_hover: bool) {
        let mut style = self.options(|opt| (*opt.style).clone());
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
    pub fn animate_bool_with_time(&self, id: Id, target_value: bool, animation_time: f32) -> f32 {
        let animated_value = self.write(|ctx| {
            ctx.animation_manager
                .animate_bool(&ctx.input, animation_time, id, target_value)
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
    pub fn animate_value_with_time(&self, id: Id, target_value: f32, animation_time: f32) -> f32 {
        let animated_value = self.write(|ctx| {
            ctx.animation_manager
                .animate_value(&ctx.input, animation_time, id, target_value)
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
                let prev_tessellation_options = self.tessellation_options(|o| *o);
                let mut tessellation_options = prev_tessellation_options;
                tessellation_options.ui(ui);
                ui.vertical_centered(|ui| reset_button(ui, &mut tessellation_options));
                if tessellation_options != prev_tessellation_options {
                    self.tessellation_options_mut(move |o| *o = tessellation_options);
                }
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
            self.memory(|m| m.interaction.focus.focused())
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
            self.fonts(|f| f.num_galleys_in_cache())
        ))
        .on_hover_text("This is approximately the number of text strings on screen");
        ui.add_space(16.0);

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
                                ui.image(texture_id, size).on_hover_ui(|ui| {
                                    // show larger on hover
                                    let max_size = 0.5 * ui.ctx().screen_rect().size();
                                    let mut size = vec2(w as f32, h as f32);
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
            self.memory_mut(|mem| *mem = Default::default());
        }

        let (num_state, num_serialized) = self.data(|d| (d.len(), d.count_serialized()));
        ui.label(format!(
            "{} widget states stored (of which {} are serialized).",
            num_state, num_serialized
        ));

        ui.horizontal(|ui| {
            ui.label(format!(
                "{} areas (panels, windows, popups, …)",
                self.memory(|mem| mem.areas.count())
            ));
            if ui.button("Reset").clicked() {
                self.memory_mut(|mem| mem.areas = Default::default());
            }
        });
        ui.indent("areas", |ui| {
            ui.label("Visible areas, ordered back to front.");
            ui.label("Hover to highlight");
            let layers_ids: Vec<LayerId> = self.memory(|mem| mem.areas.order().to_vec());
            for layer_id in layers_ids {
                let area = self.memory(|mem| mem.areas.get(layer_id.id).copied());
                if let Some(area) = area {
                    let is_visible = self.memory(|mem| mem.areas.is_visible(&layer_id));
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
            let interaction = self.memory(|mem| mem.interaction.clone());
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

/// ## Accessibility
impl Context {
    /// Call the provided function with the given ID pushed on the stack of
    /// parent IDs for accessibility purposes. If the `accesskit` feature
    /// is disabled or if AccessKit support is not active for this frame,
    /// the function is still called, but with no other effect.
    ///
    /// No locks are held while the given closure is called.
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
    // TODO: consider making both RO and RW versions
    #[cfg(feature = "accesskit")]
    pub fn accesskit_node_builder<R>(
        &self,
        id: Id,
        writer: impl FnOnce(&mut accesskit::NodeBuilder) -> R,
    ) -> Option<R> {
        self.write(|ctx| {
            ctx.frame_state
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
        use accesskit::{NodeBuilder, Role, Tree, TreeUpdate};

        let root_id = crate::accesskit_root_id().accesskit_id();
        self.write(|ctx| TreeUpdate {
            nodes: vec![(
                root_id,
                NodeBuilder::new(Role::Window).build(&mut ctx.accesskit_node_classes),
            )],
            tree: Some(Tree::new(root_id)),
            focus: None,
        })
    }
}

#[test]
fn context_impl_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Context>();
}
