#![cfg_attr(doc, doc = include_str!("../README.md"))]
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
#![expect(clippy::unwrap_used)] // TODO(emilk): avoid unwraps

mod builder;
#[cfg(feature = "snapshot")]
mod snapshot;

#[cfg(feature = "snapshot")]
pub use crate::snapshot::*;

mod app_kind;
mod config;
mod node;
mod plugin;
mod renderer;
#[cfg(feature = "wgpu")]
mod texture_to_image;
#[cfg(feature = "wgpu")]
pub mod wgpu;

pub use crate::plugin::{PanicLocation, Plugin, TestResult, install_panic_hook};

// re-exports:
pub use {
    self::{builder::*, node::*, renderer::*},
    kittest,
};

use std::{
    fmt::{Debug, Display, Formatter},
    time::Duration,
};

use egui::{
    Color32, Key, Modifiers, PointerButton, Pos2, Rect, RepaintCause, Shape, Vec2, ViewportId,
    epaint::{ClippedShape, RectShape},
    style::ScrollAnimation,
};
use kittest::Queryable;

use crate::app_kind::AppKind;

#[derive(Debug, Clone)]
pub struct ExceededMaxStepsError {
    pub max_steps: u64,
    pub repaint_causes: Vec<RepaintCause>,
}

impl Display for ExceededMaxStepsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Harness::run exceeded max_steps ({}). If your expect your ui to keep repainting \
            (e.g. when showing a spinner) call Harness::step or Harness::run_steps instead.\
            \nRepaint causes: {:#?}",
            self.max_steps, self.repaint_causes,
        )
    }
}

/// The test Harness. This contains everything needed to run the test.
///
/// Create a new Harness using [`Harness::new_ui`] or [`Harness::builder`].
///
/// The [Harness] has a optional generic state that can be used to pass data to the app / ui closure.
/// In _most cases_ it should be fine to just store the state in the closure itself.
/// The state functions are useful if you need to access the state after the harness has been created.
///
/// Some egui style options are changed from the defaults:
/// - The cursor blinking is disabled
/// - The scroll animation is disabled
pub struct Harness<'a, State: 'static = ()> {
    pub ctx: egui::Context,
    input: egui::RawInput,
    kittest: kittest::State,
    output: egui::FullOutput,
    app: AppKind<'a, State>,
    response: Option<egui::Response>,
    state: State,
    renderer: Box<dyn TestRenderer>,
    max_steps: u64,
    step_dt: f32,
    wait_for_pending_images: bool,
    queued_events: EventQueue,

    plugins: Vec<Box<dyn Plugin<State>>>,
    entry_location: Option<&'static std::panic::Location<'static>>,
    consumed_event_locations: Vec<&'static std::panic::Location<'static>>,
    last_accesskit_update: Option<egui::accesskit::TreeUpdate>,

    #[cfg(feature = "snapshot")]
    default_snapshot_options: SnapshotOptions,
    #[cfg(feature = "snapshot")]
    snapshot_results: Option<SnapshotResults>,
}

impl<State> Debug for Harness<'_, State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.kittest.fmt(f)
    }
}

impl<'a, State: 'static> Harness<'a, State> {
    #[track_caller]
    pub(crate) fn from_builder(
        builder: HarnessBuilder<State>,
        mut app: AppKind<'a, State>,
        mut state: State,
        ctx: Option<egui::Context>,
    ) -> Self {
        let HarnessBuilder {
            screen_rect,
            pixels_per_point,
            theme,
            os,
            max_steps,
            step_dt,
            state: _,
            mut renderer,
            wait_for_pending_images,
            plugins,

            #[cfg(feature = "snapshot")]
            default_snapshot_options,

            // rustfmt adds this weird indentation below.
            // See: https://github.com/rust-lang/rustfmt/issues/5920
            #[cfg(feature = "wgpu")]
                render_options: _,
        } = builder;
        let ctx = ctx.unwrap_or_default();
        ctx.set_theme(theme);
        ctx.set_os(os);
        ctx.enable_accesskit();
        ctx.all_styles_mut(|style| {
            // Disable cursor blinking so it doesn't interfere with snapshots
            style.visuals.text_cursor.blink = false;
            style.scroll_animation = ScrollAnimation::none();
            style.animation_time = 0.0;
        });
        let mut input = egui::RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        };
        let viewport = input.viewports.get_mut(&ViewportId::ROOT).unwrap();
        viewport.native_pixels_per_point = Some(pixels_per_point);

        let mut response = None;

        // We need to run egui for a single frame so that the AccessKit state can be initialized
        // and users can immediately start querying for widgets.
        let mut output = ctx.run_ui(input.clone(), |ui| {
            response = app.run(ui, &mut state, false);
        });

        renderer.handle_delta(&output.textures_delta);

        let initial_accesskit = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit was disabled");

        let mut harness = Self {
            app,
            ctx,
            input,
            kittest: kittest::State::new(initial_accesskit.clone()),
            output,
            response,
            state,
            renderer,
            max_steps,
            step_dt,
            wait_for_pending_images,
            queued_events: Default::default(),

            plugins,
            entry_location: None,
            consumed_event_locations: Vec::new(),
            last_accesskit_update: Some(initial_accesskit),

            #[cfg(feature = "snapshot")]
            default_snapshot_options,

            #[cfg(feature = "snapshot")]
            snapshot_results: Some(SnapshotResults::default()),
        };
        // Run the harness until it is stable, ensuring that all Areas are shown and animations are done
        harness.run_ok();
        harness
    }

    /// Create a [`Harness`] via a [`HarnessBuilder`].
    pub fn builder() -> HarnessBuilder<State> {
        HarnessBuilder::default()
    }

    /// Register a [`Plugin`] after construction.
    ///
    /// See [`HarnessBuilder::with_plugin`] to register before the first frame runs.
    ///
    /// Calling this from inside a plugin hook is allowed — the new plugin is appended to
    /// the list but does not receive the currently-dispatching hook; it starts firing on
    /// the next dispatch.
    pub fn add_plugin(&mut self, plugin: impl Plugin<State>) {
        self.plugins.push(Box::new(plugin));
    }

    /// Borrow a registered plugin by type. Returns the first plugin of the matching type
    /// in registration order, or `None` if no plugin of that type is registered.
    pub fn plugin<P: Plugin<State>>(&self) -> Option<&P> {
        self.plugins
            .iter()
            .find_map(|p| (&**p as &dyn std::any::Any).downcast_ref::<P>())
    }

    /// Mutably borrow a registered plugin by type.
    pub fn plugin_mut<P: Plugin<State>>(&mut self) -> Option<&mut P> {
        self.plugins
            .iter_mut()
            .find_map(|p| (&mut **p as &mut dyn std::any::Any).downcast_mut::<P>())
    }

    /// Remove and return the first plugin of the given type.
    pub fn take_plugin<P: Plugin<State>>(&mut self) -> Option<Box<P>> {
        let idx = self
            .plugins
            .iter()
            .position(|p| (&**p as &dyn std::any::Any).is::<P>())?;
        let boxed = self.plugins.remove(idx);
        let raw: *mut dyn Plugin<State> = Box::into_raw(boxed);
        // SAFETY: `is::<P>()` confirmed the concrete type is `P`. Fat-to-thin pointer
        // cast preserves the data pointer, which is the address of the underlying `P`.
        #[expect(unsafe_code)]
        Some(unsafe { Box::from_raw(raw.cast::<P>()) })
    }

    /// Advance the harness by one frame without firing plugin hooks.
    ///
    /// This is useful for running steps within a plugin, without ending in an infinite loop where
    /// the plugin is called again.
    pub fn step_no_side_effects(&mut self) {
        self._step_inner(false);
    }

    /// The most recent AccessKit tree update, if any. Useful for plugins that mirror
    /// the accessibility tree to an external debugger.
    pub fn accesskit_tree_update(&self) -> Option<&egui::accesskit::TreeUpdate> {
        self.last_accesskit_update.as_ref()
    }

    /// [`std::panic::Location`] of the most recent public `#[track_caller]` entry point
    /// (e.g. the caller of `step()` / `run()`), or `None` if no such call has been made yet.
    pub fn entry_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.entry_location
    }

    /// Locations of the events consumed during the most recent step, in order.
    pub fn consumed_event_locations(&self) -> &[&'static std::panic::Location<'static>] {
        &self.consumed_event_locations
    }

    fn dispatch(&mut self, mut f: impl FnMut(&mut dyn Plugin<State>, &mut Self)) {
        if self.plugins.is_empty() {
            return;
        }
        let mut plugins = std::mem::take(&mut self.plugins);
        for p in &mut plugins {
            f(p.as_mut(), self);
        }
        // Handle the case where a plugin is registered within some other plugin
        let added = std::mem::take(&mut self.plugins);
        self.plugins = plugins;
        self.plugins.extend(added);
    }

    /// Create a new Harness with the given ui closure and a state.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// If you e.g. want to customize the size of the ui, you can use [`Harness::builder`].
    ///
    /// # Example
    /// ```rust
    /// # use egui_kittest::{Harness, kittest::Queryable};
    /// let mut checked = false;
    /// let mut harness = Harness::new_ui_state(|ui, checked| {
    ///     ui.checkbox(checked, "Check me!");
    /// }, checked);
    ///
    /// harness.get_by_label("Check me!").click();
    /// harness.run();
    ///
    /// assert_eq!(*harness.state(), true);
    /// ```
    #[track_caller]
    pub fn new_ui_state(app: impl FnMut(&mut egui::Ui, &mut State) + 'a, state: State) -> Self {
        Self::builder().build_ui_state(app, state)
    }

    /// Create a new [Harness] from the given eframe creation closure.
    #[cfg(feature = "eframe")]
    #[track_caller]
    pub fn new_eframe(builder: impl FnOnce(&mut eframe::CreationContext<'a>) -> State) -> Self
    where
        State: eframe::App + 'static,
    {
        Self::builder().build_eframe(builder)
    }

    /// Set the size of the window.
    /// Note: If you only want to set the size once at the beginning,
    /// prefer using [`HarnessBuilder::with_size`].
    #[inline]
    pub fn set_size(&mut self, size: Vec2) -> &mut Self {
        self.input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, size));
        self
    }

    /// Set the `pixels_per_point` of the window.
    /// Note: If you only want to set the `pixels_per_point` once at the beginning,
    /// prefer using [`HarnessBuilder::with_pixels_per_point`].
    #[inline]
    pub fn set_pixels_per_point(&mut self, pixels_per_point: f32) -> &mut Self {
        self.ctx.set_pixels_per_point(pixels_per_point);
        self
    }

    /// Run a frame for each queued event (or a single frame if there are no events).
    /// This will call the app closure with each queued event and
    /// update the Harness.
    #[track_caller]
    pub fn step(&mut self) {
        self.entry_location = Some(std::panic::Location::caller());
        let events = std::mem::take(&mut *self.queued_events.lock());
        if events.is_empty() {
            self.consumed_event_locations.clear();
            self._step(false);
        }
        for event in events {
            self.consumed_event_locations.clear();
            match event {
                EventType::Event(event, loc) => {
                    self.consumed_event_locations.push(loc);
                    self.input.events.push(event.clone());
                    self.dispatch(|p, h| p.on_event(h, &event));
                }
                EventType::Modifiers(modifiers, loc) => {
                    self.consumed_event_locations.push(loc);
                    self.input.modifiers = modifiers;
                }
            }
            self._step(false);
        }
    }

    /// Run a single step, firing `before_step` / `after_step` plugin hooks.
    fn _step(&mut self, sizing_pass: bool) {
        self.dispatch(|p, h| p.before_step(h));
        self._step_inner(sizing_pass);
        self.dispatch(|p, h| p.after_step(h));
    }

    /// Core frame advance. Does NOT fire plugin hooks — callable from within
    /// hooks via [`Self::step_no_side_effects`] without recursing.
    fn _step_inner(&mut self, sizing_pass: bool) {
        self.input.predicted_dt = self.step_dt;

        let mut output = self.ctx.run_ui(self.input.take(), |ui| {
            self.response = self.app.run(ui, &mut self.state, sizing_pass);
        });
        let accesskit_update = output
            .platform_output
            .accesskit_update
            .take()
            .expect("AccessKit was disabled");
        self.last_accesskit_update = Some(accesskit_update.clone());
        self.kittest.update(accesskit_update);
        self.renderer.handle_delta(&output.textures_delta);
        self.output = output;
    }

    /// Calculate the rect that includes all popups and tooltips.
    fn compute_total_rect_with_popups(&self) -> Option<Rect> {
        // Start with the standard response rect
        let mut used = if let Some(response) = self.response.as_ref() {
            response.rect
        } else {
            return None;
        };

        // Add all visible areas from other orders (popups, tooltips, etc.)
        self.ctx.memory(|mem| {
            mem.areas()
                .visible_layer_ids()
                .into_iter()
                .filter(|layer_id| layer_id.order != egui::Order::Background)
                .filter_map(|layer_id| mem.area_rect(layer_id.id))
                .for_each(|area_rect| used |= area_rect);
        });

        Some(used)
    }

    /// Resize the test harness to fit the contents. This only works when creating the Harness via
    /// [`Harness::new_ui`] / [`Harness::new_ui_state`] or
    /// [`HarnessBuilder::build_ui`] / [`HarnessBuilder::build_ui_state`].
    #[track_caller]
    pub fn fit_contents(&mut self) {
        self.entry_location = Some(std::panic::Location::caller());
        self._step(true);

        // Calculate size including all content (main UI + popups + tooltips)
        if let Some(rect) = self.compute_total_rect_with_popups() {
            self.set_size(rect.size());
        }

        self.run_ok();
    }

    /// Run until
    /// - all animations are done
    /// - no more repaints are requested
    ///
    /// Returns the number of frames that were run.
    ///
    /// # Panics
    /// Panics if the number of steps exceeds the maximum number of steps set
    /// in [`HarnessBuilder::with_max_steps`].
    ///
    /// See also:
    /// - [`Harness::try_run`].
    /// - [`Harness::try_run_realtime`].
    /// - [`Harness::run_ok`].
    /// - [`Harness::step`].
    /// - [`Harness::run_steps`].
    #[track_caller]
    pub fn run(&mut self) -> u64 {
        self.entry_location = Some(std::panic::Location::caller());
        match self._try_run(false) {
            Ok(steps) => steps,
            Err(err) => {
                panic!("{err}");
            }
        }
    }

    fn _try_run(&mut self, sleep: bool) -> Result<u64, ExceededMaxStepsError> {
        self.dispatch(|p, h| p.before_run(h));

        let mut steps = 0;
        let result = loop {
            steps += 1;
            self.step();

            let wait_for_images = self.wait_for_pending_images && self.ctx.has_pending_images();

            // We only care about immediate repaints
            if self.root_viewport_output().repaint_delay != Duration::ZERO && !wait_for_images {
                break Ok(steps);
            } else if sleep || wait_for_images {
                std::thread::sleep(Duration::from_secs_f32(self.step_dt));
            }
            if steps > self.max_steps {
                break Err(ExceededMaxStepsError {
                    max_steps: self.max_steps,
                    repaint_causes: self.ctx.repaint_causes(),
                });
            }
        };
        self.dispatch(|p, h| p.after_run(h, result.as_ref().map(|s| *s)));
        result
    }

    /// Run until
    /// - all animations are done
    /// - no more repaints are requested
    /// - the maximum number of steps is reached (See [`HarnessBuilder::with_max_steps`])
    ///
    /// Returns the number of steps that were run.
    ///
    /// # Errors
    /// Returns an error if the maximum number of steps is exceeded.
    ///
    /// See also:
    /// - [`Harness::run`].
    /// - [`Harness::run_ok`].
    /// - [`Harness::step`].
    /// - [`Harness::run_steps`].
    /// - [`Harness::try_run_realtime`].
    #[track_caller]
    pub fn try_run(&mut self) -> Result<u64, ExceededMaxStepsError> {
        self.entry_location = Some(std::panic::Location::caller());
        self._try_run(false)
    }

    /// Run until
    /// - all animations are done
    /// - no more repaints are requested
    /// - the maximum number of steps is reached (See [`HarnessBuilder::with_max_steps`])
    ///
    /// Returns the number of steps that were run, or None if the maximum number of steps was exceeded.
    ///
    /// See also:
    /// - [`Harness::run`].
    /// - [`Harness::try_run`].
    /// - [`Harness::step`].
    /// - [`Harness::run_steps`].
    /// - [`Harness::try_run_realtime`].
    #[track_caller]
    pub fn run_ok(&mut self) -> Option<u64> {
        self.entry_location = Some(std::panic::Location::caller());
        self._try_run(false).ok()
    }

    /// Run multiple frames, sleeping for [`HarnessBuilder::with_step_dt`] between frames.
    ///
    /// This is useful to e.g. wait for an async operation to complete (e.g. loading of images).
    /// Runs until
    /// - all animations are done
    /// - no more repaints are requested
    /// - the maximum number of steps is reached (See [`HarnessBuilder::with_max_steps`])
    ///
    /// Returns the number of steps that were run.
    ///
    /// # Errors
    /// Returns an error if the maximum number of steps is exceeded.
    ///
    /// See also:
    /// - [`Harness::run`].
    /// - [`Harness::run_ok`].
    /// - [`Harness::step`].
    /// - [`Harness::run_steps`].
    /// - [`Harness::try_run`].
    #[track_caller]
    pub fn try_run_realtime(&mut self) -> Result<u64, ExceededMaxStepsError> {
        self.entry_location = Some(std::panic::Location::caller());
        self._try_run(true)
    }

    /// Run a number of steps.
    /// Equivalent to calling [`Harness::step`] x times.
    #[track_caller]
    pub fn run_steps(&mut self, steps: usize) {
        self.entry_location = Some(std::panic::Location::caller());
        for _ in 0..steps {
            self.step();
        }
    }

    /// Access the [`egui::RawInput`] for the next frame.
    pub fn input(&self) -> &egui::RawInput {
        &self.input
    }

    /// Access the [`egui::RawInput`] for the next frame mutably.
    pub fn input_mut(&mut self) -> &mut egui::RawInput {
        &mut self.input
    }

    /// Access the [`egui::FullOutput`] for the last frame.
    pub fn output(&self) -> &egui::FullOutput {
        &self.output
    }

    /// Access the [`kittest::State`].
    pub fn kittest_state(&self) -> &kittest::State {
        &self.kittest
    }

    /// Access the state.
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Access the state mutably.
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Queue an event to be processed in the next frame.
    #[track_caller]
    pub fn event(&self, event: egui::Event) {
        self.queued_events
            .lock()
            .push(EventType::Event(event, std::panic::Location::caller()));
    }

    /// Queue an event with modifiers.
    ///
    /// Queues the modifiers to be pressed, then the event, then the modifiers to be released.
    #[track_caller]
    pub fn event_modifiers(&self, event: egui::Event, modifiers: Modifiers) {
        let caller = std::panic::Location::caller();
        let mut queue = self.queued_events.lock();
        queue.push(EventType::Modifiers(modifiers, caller));
        queue.push(EventType::Event(event, caller));
        queue.push(EventType::Modifiers(Modifiers::default(), caller));
    }

    #[track_caller]
    fn modifiers(&self, modifiers: Modifiers) {
        self.queued_events.lock().push(EventType::Modifiers(
            modifiers,
            std::panic::Location::caller(),
        ));
    }

    #[track_caller]
    pub fn key_down(&self, key: egui::Key) {
        self.event(egui::Event::Key {
            key,
            pressed: true,
            modifiers: Modifiers::default(),
            repeat: false,
            physical_key: None,
        });
    }

    #[track_caller]
    pub fn key_down_modifiers(&self, modifiers: Modifiers, key: egui::Key) {
        self.event_modifiers(
            egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                repeat: false,
                physical_key: None,
            },
            modifiers,
        );
    }

    #[track_caller]
    pub fn key_up(&self, key: egui::Key) {
        self.event(egui::Event::Key {
            key,
            pressed: false,
            modifiers: Modifiers::default(),
            repeat: false,
            physical_key: None,
        });
    }

    #[track_caller]
    pub fn key_up_modifiers(&self, modifiers: Modifiers, key: egui::Key) {
        self.event_modifiers(
            egui::Event::Key {
                key,
                pressed: false,
                modifiers,
                repeat: false,
                physical_key: None,
            },
            modifiers,
        );
    }

    /// Press the given keys in combination.
    ///
    /// For e.g. [`Key::A`] + [`Key::B`] this would generate:
    /// - Press [`Key::A`]
    /// - Press [`Key::B`]
    /// - Release [`Key::B`]
    /// - Release [`Key::A`]
    #[track_caller]
    pub fn key_combination(&self, keys: &[Key]) {
        for key in keys {
            self.key_down(*key);
        }
        for key in keys.iter().rev() {
            self.key_up(*key);
        }
    }

    /// Press the given keys in combination, with modifiers.
    ///
    /// For e.g. [`Modifiers::COMMAND`] + [`Key::A`] + [`Key::B`] this would generate:
    /// - Press [`Modifiers::COMMAND`]
    /// - Press [`Key::A`]
    /// - Press [`Key::B`]
    /// - Release [`Key::B`]
    /// - Release [`Key::A`]
    /// - Release [`Modifiers::COMMAND`]
    #[track_caller]
    pub fn key_combination_modifiers(&self, modifiers: Modifiers, keys: &[Key]) {
        self.modifiers(modifiers);

        for pressed in [true, false] {
            for key in keys {
                self.event(egui::Event::Key {
                    key: *key,
                    pressed,
                    modifiers,
                    repeat: false,
                    physical_key: None,
                });
            }
        }

        self.modifiers(Modifiers::default());
    }

    /// Press a key.
    ///
    /// This will create a key down event and a key up event.
    #[track_caller]
    pub fn key_press(&self, key: egui::Key) {
        self.key_combination(&[key]);
    }

    /// Press a key with modifiers.
    ///
    /// This will
    /// - set the modifiers
    /// - create a key down event
    /// - create a key up event
    /// - reset the modifiers
    #[track_caller]
    pub fn key_press_modifiers(&self, modifiers: Modifiers, key: egui::Key) {
        self.key_combination_modifiers(modifiers, &[key]);
    }

    /// Move mouse cursor to this position.
    #[track_caller]
    pub fn hover_at(&self, pos: egui::Pos2) {
        self.event(egui::Event::PointerMoved(pos));
    }

    /// Start dragging from a position.
    #[track_caller]
    pub fn drag_at(&self, pos: egui::Pos2) {
        self.event(egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
    }

    /// Stop dragging and remove cursor.
    #[track_caller]
    pub fn drop_at(&self, pos: egui::Pos2) {
        self.event(egui::Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        });
        self.remove_cursor();
    }

    /// Remove the cursor from the screen.
    ///
    /// Will fire a [`egui::Event::PointerGone`] event.
    ///
    /// If you click a button and then take a snapshot, the button will be shown as hovered.
    /// If you don't want that, you can call this method after clicking.
    #[track_caller]
    pub fn remove_cursor(&self) {
        self.event(egui::Event::PointerGone);
    }

    /// Mask something. Useful for snapshot tests.
    ///
    /// Call this _after_ [`Self::run`] and before [`Self::snapshot`].
    /// This will add a [`RectShape`] to the output shapes, for the current frame.
    /// Will be overwritten on the next call to [`Self::run`].
    pub fn mask(&mut self, rect: Rect) {
        self.output.shapes.push(ClippedShape {
            clip_rect: Rect::EVERYTHING,
            shape: Shape::Rect(RectShape::filled(rect, 0.0, Color32::MAGENTA)),
        });
    }

    /// Render the last output to an image.
    ///
    /// # Errors
    /// Returns an error if the rendering fails.
    #[cfg(any(feature = "wgpu", feature = "snapshot"))]
    pub fn render(&mut self) -> Result<image::RgbaImage, String> {
        let mut output = self.output.clone();

        if let Some(mouse_pos) = self.ctx.input(|i| i.pointer.hover_pos()) {
            // Paint a mouse cursor:
            let triangle = vec![
                mouse_pos,
                mouse_pos + egui::vec2(16.0, 8.0),
                mouse_pos + egui::vec2(8.0, 16.0),
            ];

            output.shapes.push(ClippedShape {
                clip_rect: self.ctx.content_rect(),
                shape: egui::epaint::PathShape::convex_polygon(
                    triangle,
                    Color32::WHITE,
                    egui::Stroke::new(1.0, Color32::BLACK),
                )
                .into(),
            });
        }

        let image = self.renderer.render(&self.ctx, &output)?;
        self.dispatch(|p, h| p.on_render(h, &image));
        Ok(image)
    }

    /// Get the root viewport output
    fn root_viewport_output(&self) -> &egui::ViewportOutput {
        self.output
            .viewport_output
            .get(&ViewportId::ROOT)
            .expect("Missing root viewport")
    }

    /// The root node of the test harness.
    pub fn root(&self) -> Node<'_> {
        Node {
            accesskit_node: self.kittest.root(),
            queue: &self.queued_events,
        }
    }

    /// Spawn a real native eframe window running this harness's app, reusing its [`egui::Context`].
    ///
    /// Blocks until the window is closed.
    ///
    /// Useful for interactively debugging a failing test: add a call to this before the failing
    /// assertion to poke at the UI yourself.
    ///
    /// # macOS: must be called on the main thread
    /// `AppKit` requires UI work to happen on the main thread, but by default cargo's test harness
    /// runs each test on a spawned worker thread, so this function will panic on macOS unless
    /// you opt out of the default harness.
    ///
    /// To fix this, disable the default libtest harness for your test target and run tests on
    /// the main thread yourself. In `Cargo.toml`:
    ///
    /// ```toml
    /// [[test]]
    /// name = "your_test"
    /// harness = false
    /// ```
    ///
    /// Then write a `fn main()` in the test file that invokes your test directly.
    ///
    /// See also: <https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-harness-field>
    #[cfg(feature = "eframe")]
    #[deprecated = "Only for debugging, don't commit this."]
    pub fn spawn_eframe_app(self)
    where
        'a: 'static,
        State: 'static,
    {
        #[cfg(target_os = "macos")]
        {
            // AppKit requires UI work to happen on the main thread, but by default cargo's
            // test harness runs each test on a spawned worker thread.
            #[expect(unsafe_code)]
            // SAFETY: `pthread_main_np` is a thread-safe libc query with no arguments.
            let is_main_thread = unsafe {
                unsafe extern "C" {
                    fn pthread_main_np() -> std::ffi::c_int;
                }
                pthread_main_np() != 0
            };
            assert!(
                is_main_thread,
                "spawn_eframe_app must be called on the main thread on macOS, \
                 but the default `cargo test` harness runs each test on a worker thread.\n\
                 \n\
                 To fix this, disable the default libtest harness for your test target and run \
                 tests on the main thread yourself. In Cargo.toml:\n\
                 \n\
                     [[test]]\n\
                     name = \"your_test\"\n\
                     harness = false\n\
                 \n\
                 Then write a `fn main()` in the test file that invokes your test directly.\n\
                 \n\
                 See: https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-harness-field"
            );
        }

        // Wrap the whole `Harness` in an `eframe::App` adapter so we don't need to
        // destructure `self` (which we can't, since `Harness` implements `Drop`).
        // The adapter delegates `ui`/`logic` through the stored `AppKind`.
        struct HarnessAsApp<State: 'static> {
            harness: Harness<'static, State>,
        }

        impl<State: 'static> eframe::App for HarnessAsApp<State> {
            fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
                if let AppKind::Eframe(crate::app_kind::AppKindEframe { get_app, .. }) =
                    &mut self.harness.app
                {
                    get_app(&mut self.harness.state).logic(ctx, frame);
                }
            }

            fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
                let harness = &mut self.harness;
                match &mut harness.app {
                    AppKind::Ui(f) => f(ui),
                    AppKind::UiState(f) => f(ui, &mut harness.state),
                    AppKind::Eframe(crate::app_kind::AppKindEframe { get_app, .. }) => {
                        get_app(&mut harness.state).ui(ui, frame);
                    }
                }
            }
        }

        let ctx = self.ctx.clone();
        let eframe_app: Box<dyn eframe::App> = Box::new(HarnessAsApp { harness: self });

        eframe::run_native_ext(
            "egui_kittest",
            eframe::NativeOptions::default(),
            Some(ctx),
            Box::new(|_cc| Ok(eframe_app)),
        )
        .unwrap();
    }
}

/// Utilities for stateless harnesses.
impl<'a> Harness<'a> {
    /// Create a new Harness with the given ui closure.
    /// Use the [`Harness::run`], [`Harness::step`], etc... methods to run the app.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// If you e.g. want to customize the size of the ui, you can use [`Harness::builder`].
    ///
    /// # Example
    /// ```rust
    /// # use egui_kittest::Harness;
    /// let mut harness = Harness::new_ui(|ui| {
    ///     ui.label("Hello, world!");
    /// });
    /// ```
    #[track_caller]
    pub fn new_ui(app: impl FnMut(&mut egui::Ui) + 'a) -> Self {
        Self::builder().build_ui(app)
    }
}

impl<'tree, 'node, State: 'static> Queryable<'tree, 'node, Node<'tree>> for Harness<'_, State>
where
    'node: 'tree,
{
    fn queryable_node(&'node self) -> Node<'tree> {
        self.root()
    }
}

impl<State: 'static> Drop for Harness<'_, State> {
    fn drop(&mut self) {
        // Consume SnapshotResults first so its own panic-check runs under our control,
        // and so `std::thread::panicking()` reflects snapshot failures when plugins observe
        // the final outcome.
        #[cfg(feature = "snapshot")]
        if let Some(results) = self.snapshot_results.take() {
            // Drop may panic; if so, the panic propagates and plugins still see Fail.
            drop(results);
        }

        if self.plugins.is_empty() {
            return;
        }

        if std::thread::panicking() {
            plugin::with_fail_test_result(|result| {
                self.dispatch(|p, h| p.on_test_result(h, fail_ref(&result)));
            });
        } else {
            self.dispatch(|p, h| p.on_test_result(h, TestResult::Pass));
        }
    }
}

// Helper: reborrow a `TestResult::Fail` so it can be passed to multiple plugins from
// inside `dispatch`'s FnMut closure.
fn fail_ref<'a>(result: &'a TestResult<'a>) -> TestResult<'a> {
    match result {
        TestResult::Pass => TestResult::Pass,
        TestResult::Fail { message, location } => TestResult::Fail {
            message: *message,
            location: *location,
        },
    }
}
