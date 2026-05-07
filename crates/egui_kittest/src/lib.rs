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
#[cfg(feature = "inspector")]
mod inspector;
#[cfg(feature = "inspector_api")]
pub mod inspector_api;
mod node;
#[cfg(feature = "recording")]
mod recording;
mod renderer;
#[cfg(feature = "wgpu")]
mod texture_to_image;
#[cfg(feature = "wgpu")]
pub mod wgpu;

#[cfg(feature = "recording")]
pub use crate::recording::{RecordKind, RecordingError, RecordingOptions, RecordingTrigger};

#[cfg(feature = "inspector")]
pub use crate::inspector::{INSPECTOR_ENV_VAR, INSPECTOR_PATH_ENV_VAR, InspectorError};

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
pub struct Harness<'a, State = ()> {
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

    #[cfg(feature = "snapshot")]
    default_snapshot_options: SnapshotOptions,
    #[cfg(feature = "snapshot")]
    snapshot_results: SnapshotResults,

    #[cfg(feature = "recording")]
    recording: Option<recording::RecordingState>,

    #[cfg(feature = "inspector")]
    inspector: Option<inspector::Inspector>,
    #[cfg(feature = "inspector")]
    last_accesskit_update: Option<egui::accesskit::TreeUpdate>,
    /// Backtrace captured at the most recent public runner call (e.g. `.run()` / `.step()`).
    /// Used to find the topmost common test-source file across the call and its events.
    #[cfg(feature = "inspector")]
    current_call_site: node::EventSite,
    /// Backtraces of events consumed in the step that produced the current frame.
    #[cfg(feature = "inspector")]
    consumed_event_sites: Vec<node::EventSite>,
}

impl<State> Debug for Harness<'_, State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.kittest.fmt(f)
    }
}

impl<'a, State> Harness<'a, State> {
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

        let mut harness = Self {
            app,
            ctx,
            input,
            kittest: kittest::State::new(
                output
                    .platform_output
                    .accesskit_update
                    .take()
                    .expect("AccessKit was disabled"),
            ),
            output,
            response,
            state,
            renderer,
            max_steps,
            step_dt,
            wait_for_pending_images,
            queued_events: Default::default(),

            #[cfg(feature = "snapshot")]
            default_snapshot_options,

            #[cfg(feature = "snapshot")]
            snapshot_results: SnapshotResults::default(),

            #[cfg(feature = "recording")]
            recording: None,

            #[cfg(feature = "inspector")]
            inspector: None,
            #[cfg(feature = "inspector")]
            last_accesskit_update: None,
            #[cfg(feature = "inspector")]
            current_call_site: node::empty_site(),
            #[cfg(feature = "inspector")]
            consumed_event_sites: Vec::new(),
        };
        // Run the harness until it is stable, ensuring that all Areas are shown and animations are done
        harness.run_ok();

        #[cfg(feature = "inspector")]
        if inspector::env_enabled() {
            match inspector::Inspector::launch(std::thread::current().name().map(String::from)) {
                Ok(insp) => harness.inspector = Some(insp),
                Err(err) => {
                    #[expect(clippy::print_stderr)]
                    {
                        eprintln!("egui_kittest: failed to launch inspector: {err}");
                    }
                }
            }
        }

        #[cfg(all(feature = "recording", feature = "snapshot"))]
        {
            // Env var takes precedence (always saves), then config (only saves on failure).
            let auto_mode = if recording::record_env_enabled() {
                Some(recording::AutoSaveMode::Always)
            } else if config::config().save_gif_on_failure() {
                Some(recording::AutoSaveMode::OnFailure)
            } else {
                None
            };
            if let Some(mode) = auto_mode {
                let options = recording::RecordingOptions::gif(std::path::PathBuf::new(), 10.0);
                harness.recording = Some(recording::RecordingState::new(options).with_auto_save(mode));
            }
        }

        harness
    }

    /// Create a [`Harness`] via a [`HarnessBuilder`].
    pub fn builder() -> HarnessBuilder<State> {
        HarnessBuilder::default()
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
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
        let events = std::mem::take(&mut *self.queued_events.lock());
        if events.is_empty() {
            #[cfg(feature = "inspector")]
            self.consumed_event_sites.clear();
            self._step(false);
        }
        for event in events {
            #[cfg(feature = "inspector")]
            self.consumed_event_sites.clear();
            match event {
                EventType::Event(event, _site) => {
                    #[cfg(feature = "inspector")]
                    self.consumed_event_sites.push(_site);
                    self.input.events.push(event);
                }
                EventType::Modifiers(modifiers, _site) => {
                    #[cfg(feature = "inspector")]
                    self.consumed_event_sites.push(_site);
                    self.input.modifiers = modifiers;
                }
            }
            self._step(false);
        }
    }

    /// Run a single step. This will not process any events.
    fn _step(&mut self, sizing_pass: bool) {
        self._step_inner(sizing_pass);

        #[cfg(feature = "recording")]
        self.capture_frame_if_recording(false);

        // Inspector Control mode: each event the user triggers in the inspector drives one
        // extra `_step_inner` so the UI re-renders, but the outer test caller stays parked
        // inside this method until the inspector replies with an empty event list
        // (i.e. user clicked Next/Play, or has Control off and we're just forwarding).
        #[cfg(feature = "inspector")]
        self.drive_inspector();
    }

    /// The core of `_step`: run egui once and update internal state. Does not touch the
    /// inspector or the recording hook, so it can be called in a loop from those.
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
        #[cfg(feature = "inspector")]
        {
            self.last_accesskit_update = Some(accesskit_update.clone());
        }
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
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
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
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
        match self.try_run() {
            Ok(steps) => steps,
            Err(err) => {
                panic!("{err}");
            }
        }
    }

    fn _try_run(&mut self, sleep: bool) -> Result<u64, ExceededMaxStepsError> {
        let mut steps = 0;
        loop {
            steps += 1;
            self.step();

            let wait_for_images = self.wait_for_pending_images && self.ctx.has_pending_images();

            // We only care about immediate repaints
            if self.root_viewport_output().repaint_delay != Duration::ZERO && !wait_for_images {
                break;
            } else if sleep || wait_for_images {
                std::thread::sleep(Duration::from_secs_f32(self.step_dt));
            }
            if steps > self.max_steps {
                return Err(ExceededMaxStepsError {
                    max_steps: self.max_steps,
                    repaint_causes: self.ctx.repaint_causes(),
                });
            }
        }

        #[cfg(feature = "recording")]
        self.capture_frame_if_recording(true);

        Ok(steps)
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
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
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
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
        self.try_run().ok()
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
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
        self._try_run(true)
    }

    /// Run a number of steps.
    /// Equivalent to calling [`Harness::step`] x times.
    #[track_caller]
    pub fn run_steps(&mut self, steps: usize) {
        #[cfg(feature = "inspector")]
        {
            self.current_call_site = node::capture_site();
        }
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

    /// Consume the harness and return the state.
    pub fn into_state(self) -> State {
        self.state
    }

    /// Queue an event to be processed in the next frame.
    pub fn event(&self, event: egui::Event) {
        self.queued_events
            .lock()
            .push(EventType::Event(event, node::capture_site()));
    }

    /// Queue an event with modifiers.
    ///
    /// Queues the modifiers to be pressed, then the event, then the modifiers to be released.
    pub fn event_modifiers(&self, event: egui::Event, modifiers: Modifiers) {
        let mut queue = self.queued_events.lock();
        queue.push(EventType::Modifiers(modifiers, node::capture_site()));
        queue.push(EventType::Event(event, node::capture_site()));
        queue.push(EventType::Modifiers(Modifiers::default(), node::capture_site()));
    }

    fn modifiers(&self, modifiers: Modifiers) {
        self.queued_events
            .lock()
            .push(EventType::Modifiers(modifiers, node::capture_site()));
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
    #[cfg(any(feature = "wgpu", feature = "snapshot", feature = "recording", feature = "inspector"))]
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

        self.renderer.render(&self.ctx, &output)
    }

    /// Start recording the test session.
    ///
    /// Captures one frame per [`Self::step`] (or per [`Self::run`], depending on the
    /// configured [`RecordingTrigger`]). Replaces any previously active recording.
    /// Call [`Self::finish_recording`] to write the output.
    ///
    /// Requires a renderer (e.g. enable the `wgpu` feature, or set one via
    /// [`HarnessBuilder::renderer`]).
    #[cfg(feature = "recording")]
    pub fn start_recording(&mut self, options: RecordingOptions) {
        self.recording = Some(recording::RecordingState::new(options));
    }

    /// Stop the active recording and write its output (GIF or PNG sequence).
    ///
    /// # Errors
    /// Returns [`RecordingError::NotRecording`] if no recording is active, or an I/O / encode
    /// error if writing fails.
    #[cfg(feature = "recording")]
    pub fn finish_recording(&mut self) -> Result<(), RecordingError> {
        let state = self.recording.take().ok_or(RecordingError::NotRecording)?;
        state.save()
    }

    /// Whether a recording is currently active.
    #[cfg(feature = "recording")]
    pub fn is_recording(&self) -> bool {
        self.recording.is_some()
    }

    /// Render the current frame and append it to the active recording according to its trigger.
    /// Called from [`Self::_step`] (with `after_run = false`) and at the end of [`Self::_try_run`]
    /// (with `after_run = true`).
    #[cfg(feature = "recording")]
    fn capture_frame_if_recording(&mut self, after_run: bool) {
        let Some(state) = self.recording.as_mut() else {
            return;
        };
        if !state.should_capture(after_run) {
            return;
        }
        match self.render() {
            Ok(image) => {
                if let Some(state) = self.recording.as_mut() {
                    state.push_frame(image);
                }
            }
            Err(err) => {
                #[expect(clippy::print_stderr)]
                {
                    eprintln!("egui_kittest recording: render failed, skipping frame: {err}");
                }
            }
        }
    }

    /// Launch a `kittest_inspector` process and attach this harness to it.
    ///
    /// After this call, every [`Self::step`] sends the rendered frame + accesskit tree to the
    /// inspector and blocks until the inspector replies. When paused, the harness blocks until
    /// the user clicks Play or Next in the inspector.
    ///
    /// # Errors
    /// If the inspector binary cannot be launched or the connection fails.
    #[cfg(feature = "inspector")]
    pub fn launch_inspector(&mut self) -> Result<(), InspectorError> {
        let label = std::thread::current().name().map(String::from);
        self.inspector = Some(inspector::Inspector::launch(label)?);
        Ok(())
    }

    /// Detach the inspector if attached. The inspector window will close on next message.
    #[cfg(feature = "inspector")]
    pub fn detach_inspector(&mut self) {
        self.inspector = None;
    }

    /// Block at the inspector until it tells us to resume, re-rendering after each batch of
    /// events it sends. Events drive an internal `_step_inner` (and recording capture), but
    /// do NOT return control to the outer test — the test advances only when the inspector
    /// replies with no events (i.e. user hit Next/Play/Step, or Control mode is off).
    ///
    /// We only loop while the inspector is *feeding events back* (Control mode) — animation
    /// frames driven by `request_repaint` are handled by the outer `try_run` loop calling
    /// `step()` again, so we don't need to drive them here. Doing so would send extra "no
    /// event highlighted" frames between each event and confuse the Step UX.
    #[cfg(feature = "inspector")]
    fn drive_inspector(&mut self) {
        if self.inspector.is_none() {
            return;
        }
        loop {
            let image = match self.render() {
                Ok(img) => img,
                Err(err) => {
                    #[expect(clippy::print_stderr)]
                    {
                        eprintln!("egui_kittest inspector: render failed: {err}");
                    }
                    return;
                }
            };
            let tree = self.last_accesskit_update.clone();
            let ppp = self.ctx.pixels_per_point();
            let call_site = self.current_call_site.clone();
            let event_sites: Vec<_> = self.consumed_event_sites.clone();
            let events = if let Some(inspector) = self.inspector.as_mut() {
                inspector.send_step(&image, ppp, tree, &call_site, &event_sites)
            } else {
                return;
            };
            if events.is_empty() {
                return;
            }
            for event in events {
                self.input.events.push(event);
            }
            // Events driven by the inspector itself don't have a test-source location.
            self.consumed_event_sites.clear();
            self._step_inner(false);
            #[cfg(feature = "recording")]
            self.capture_frame_if_recording(false);

            // Run one more step so effects of the just-delivered events are visible in the
            // next frame we send (e.g. a clicked button's state change). Without this we'd
            // show the frame *during* the click but not *after*.
            self._step_inner(false);
            #[cfg(feature = "recording")]
            self.capture_frame_if_recording(false);
        }
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

        struct UiApp {
            f: Box<dyn FnMut(&mut egui::Ui)>,
        }

        impl eframe::App for UiApp {
            fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
                (self.f)(ui);
            }
        }

        struct UiStateApp<State> {
            f: Box<dyn FnMut(&mut egui::Ui, &mut State)>,
            state: State,
        }

        impl<State: 'static> eframe::App for UiStateApp<State> {
            fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
                let Self { f, state } = self;
                f(ui, state);
            }
        }

        use crate::app_kind::AppKindEframe;

        let Self {
            ctx, state, app, ..
        } = self;

        let eframe_app: Box<dyn eframe::App> = match app {
            AppKind::Ui(f) => Box::new(UiApp { f }),
            AppKind::UiState(f) => Box::new(UiStateApp { f, state }),
            AppKind::Eframe(AppKindEframe { take_app, .. }) => take_app(state),
        };

        eframe::run_native_ext(
            "egui_kittest",
            eframe::NativeOptions::default(),
            Some(ctx),
            Box::new(|_cc| Ok(eframe_app)),
        )
        .unwrap();
    }
}

/// Save the in-progress recording (auto-started by `save_gif_on_failure` or `KITTEST_RECORD`)
/// when the harness is dropped.
///
/// Recordings started by an explicit `start_recording` call are *not* saved here — the user
/// is expected to call `finish_recording`.
#[cfg(all(feature = "recording", feature = "snapshot"))]
#[expect(clippy::print_stderr)] // Drop path: stderr is the only signal we have.
impl<State> Drop for Harness<'_, State> {
    fn drop(&mut self) {
        let Some(mut state) = self.recording.take() else {
            return;
        };
        let Some(mode) = state.auto_save_mode else {
            // Explicit recording — discard if not finished.
            return;
        };

        let should_save = match mode {
            recording::AutoSaveMode::Always => true,
            recording::AutoSaveMode::OnFailure => {
                std::thread::panicking() || self.snapshot_results.has_errors()
            }
        };
        if !should_save {
            return;
        }

        let subdir = match mode {
            recording::AutoSaveMode::Always => "recordings",
            recording::AutoSaveMode::OnFailure => "failures",
        };
        let name = std::thread::current()
            .name()
            .map(sanitize_thread_name)
            .unwrap_or_else(default_recording_name);
        let resolved_path = config::config()
            .output_path()
            .join(subdir)
            .join(format!("{name}.gif"));

        // Replace the placeholder path with the resolved one.
        if let recording::RecordKind::Gif { path, .. } = &mut state.options.kind {
            *path = resolved_path.clone();
        }

        match state.save() {
            Ok(()) => eprintln!("egui_kittest: saved GIF to {}", resolved_path.display()),
            Err(err) => eprintln!(
                "egui_kittest: failed to save GIF to {}: {err}",
                resolved_path.display()
            ),
        }
    }
}

#[cfg(all(feature = "recording", feature = "snapshot"))]
fn sanitize_thread_name(name: &str) -> String {
    // Test thread names look like `module::tests::name` — make that filesystem-safe.
    name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_")
}

#[cfg(all(feature = "recording", feature = "snapshot"))]
fn default_recording_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("recording-{ts}")
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

impl<'tree, 'node, State> Queryable<'tree, 'node, Node<'tree>> for Harness<'_, State>
where
    'node: 'tree,
{
    fn queryable_node(&'node self) -> Node<'tree> {
        self.root()
    }
}
