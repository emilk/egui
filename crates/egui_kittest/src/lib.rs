#![doc = include_str!("../README.md")]
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]

mod builder;
mod event;
#[cfg(feature = "snapshot")]
mod snapshot;

#[cfg(feature = "snapshot")]
pub use snapshot::*;
use std::fmt::{Debug, Display, Formatter};
use std::time::Duration;

mod app_kind;
mod renderer;
#[cfg(feature = "wgpu")]
mod texture_to_image;
#[cfg(feature = "wgpu")]
pub mod wgpu;

pub use kittest;

use crate::app_kind::AppKind;
use crate::event::EventState;

pub use builder::*;
pub use renderer::*;

use egui::{Modifiers, Pos2, Rect, RepaintCause, Vec2, ViewportId};
use kittest::{Node, Queryable};

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
/// Create a new Harness using [`Harness::new`] or [`Harness::builder`].
///
/// The [Harness] has a optional generic state that can be used to pass data to the app / ui closure.
/// In _most cases_ it should be fine to just store the state in the closure itself.
/// The state functions are useful if you need to access the state after the harness has been created.
pub struct Harness<'a, State = ()> {
    pub ctx: egui::Context,
    input: egui::RawInput,
    kittest: kittest::State,
    output: egui::FullOutput,
    app: AppKind<'a, State>,
    event_state: EventState,
    response: Option<egui::Response>,
    state: State,
    renderer: Box<dyn TestRenderer>,
    max_steps: u64,
    step_dt: f32,
}

impl<State> Debug for Harness<'_, State> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.kittest.fmt(f)
    }
}

impl<'a, State> Harness<'a, State> {
    pub(crate) fn from_builder(
        builder: HarnessBuilder<State>,
        mut app: AppKind<'a, State>,
        mut state: State,
        ctx: Option<egui::Context>,
    ) -> Self {
        let HarnessBuilder {
            screen_rect,
            pixels_per_point,
            max_steps,
            step_dt,
            state: _,
            mut renderer,
        } = builder;
        let ctx = ctx.unwrap_or_default();
        ctx.enable_accesskit();
        // Disable cursor blinking so it doesn't interfere with snapshots
        ctx.all_styles_mut(|style| style.visuals.text_cursor.blink = false);
        let mut input = egui::RawInput {
            screen_rect: Some(screen_rect),
            ..Default::default()
        };
        let viewport = input.viewports.get_mut(&ViewportId::ROOT).unwrap();
        viewport.native_pixels_per_point = Some(pixels_per_point);

        let mut response = None;

        // We need to run egui for a single frame so that the AccessKit state can be initialized
        // and users can immediately start querying for widgets.
        let mut output = ctx.run(input.clone(), |ctx| {
            response = app.run(ctx, &mut state, false);
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
            event_state: EventState::default(),
            state,
            renderer,
            max_steps,
            step_dt,
        };
        // Run the harness until it is stable, ensuring that all Areas are shown and animations are done
        harness.run_ok();
        harness
    }

    /// Create a [`Harness`] via a [`HarnessBuilder`].
    pub fn builder() -> HarnessBuilder<State> {
        HarnessBuilder::default()
    }

    /// Create a new Harness with the given app closure and a state.
    ///
    /// The app closure will immediately be called once to create the initial ui.
    ///
    /// If you don't need to create Windows / Panels, you can use [`Harness::new_ui`] instead.
    ///
    /// If you e.g. want to customize the size of the window, you can use [`Harness::builder`].
    ///
    /// # Example
    /// ```rust
    /// # use egui::CentralPanel;
    /// # use egui_kittest::{Harness, kittest::Queryable};
    /// let mut checked = false;
    /// let mut harness = Harness::new_state(|ctx, checked| {
    ///     CentralPanel::default().show(ctx, |ui| {
    ///         ui.checkbox(checked, "Check me!");
    ///     });
    /// }, checked);
    ///
    /// harness.get_by_label("Check me!").click();
    /// harness.run();
    ///
    /// assert_eq!(*harness.state(), true);
    /// ```
    pub fn new_state(app: impl FnMut(&egui::Context, &mut State) + 'a, state: State) -> Self {
        Self::builder().build_state(app, state)
    }

    /// Create a new Harness with the given ui closure and a state.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// If you need to create Windows / Panels, you can use [`Harness::new`] instead.
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
    pub fn new_ui_state(app: impl FnMut(&mut egui::Ui, &mut State) + 'a, state: State) -> Self {
        Self::builder().build_ui_state(app, state)
    }

    /// Create a new [Harness] from the given eframe creation closure.
    #[cfg(feature = "eframe")]
    pub fn new_eframe(builder: impl FnOnce(&mut eframe::CreationContext<'a>) -> State) -> Self
    where
        State: eframe::App,
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

    /// Run a frame.
    /// This will call the app closure with the queued events and current context and
    /// update the Harness.
    pub fn step(&mut self) {
        self._step(false);
    }

    fn _step(&mut self, sizing_pass: bool) {
        for event in self.kittest.take_events() {
            if let Some(event) = self.event_state.kittest_event_to_egui(event) {
                self.input.events.push(event);
            }
        }

        self.input.predicted_dt = self.step_dt;

        let mut output = self.ctx.run(self.input.take(), |ctx| {
            self.response = self.app.run(ctx, &mut self.state, sizing_pass);
        });
        self.kittest.update(
            output
                .platform_output
                .accesskit_update
                .take()
                .expect("AccessKit was disabled"),
        );
        self.renderer.handle_delta(&output.textures_delta);
        self.output = output;
    }

    /// Resize the test harness to fit the contents. This only works when creating the Harness via
    /// [`Harness::new_ui`] / [`Harness::new_ui_state`] or
    /// [`HarnessBuilder::build_ui`] / [`HarnessBuilder::build_ui_state`].
    pub fn fit_contents(&mut self) {
        self._step(true);
        if let Some(response) = &self.response {
            self.set_size(response.rect.size());
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
    /// - [`Harness::run_ok`].
    /// - [`Harness::step`].
    /// - [`Harness::run_steps`].
    #[track_caller]
    pub fn run(&mut self) -> u64 {
        match self.try_run() {
            Ok(steps) => steps,
            Err(err) => {
                panic!("{err}");
            }
        }
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
    pub fn try_run(&mut self) -> Result<u64, ExceededMaxStepsError> {
        let mut steps = 0;
        loop {
            steps += 1;
            self.step();
            // We only care about immediate repaints
            if self.root_viewport_output().repaint_delay != Duration::ZERO {
                break;
            }
            if steps > self.max_steps {
                return Err(ExceededMaxStepsError {
                    max_steps: self.max_steps,
                    repaint_causes: self.ctx.repaint_causes(),
                });
            }
        }
        Ok(steps)
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
    pub fn run_ok(&mut self) -> Option<u64> {
        self.try_run().ok()
    }

    /// Run a number of steps.
    /// Equivalent to calling [`Harness::step`] x times.
    pub fn run_steps(&mut self, steps: usize) {
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

    /// Press a key.
    /// This will create a key down event and a key up event.
    pub fn press_key(&mut self, key: egui::Key) {
        self.press_key_modifiers(Modifiers::default(), key);
    }

    /// Press a key with modifiers.
    /// This will create a key down event and a key up event.
    pub fn press_key_modifiers(&mut self, modifiers: Modifiers, key: egui::Key) {
        self.input.events.push(egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            repeat: false,
            physical_key: None,
        });
        self.input.events.push(egui::Event::Key {
            key,
            pressed: false,
            modifiers,
            repeat: false,
            physical_key: None,
        });
    }

    /// Render the last output to an image.
    ///
    /// # Errors
    /// Returns an error if the rendering fails.
    pub fn render(&mut self) -> Result<image::RgbaImage, String> {
        self.renderer.render(&self.ctx, &self.output)
    }

    /// Get the root viewport output
    fn root_viewport_output(&self) -> &egui::ViewportOutput {
        self.output
            .viewport_output
            .get(&ViewportId::ROOT)
            .expect("Missing root viewport")
    }
}

/// Utilities for stateless harnesses.
impl<'a> Harness<'a> {
    /// Create a new Harness with the given app closure.
    /// Use the [`Harness::run`], [`Harness::step`], etc... methods to run the app.
    ///
    /// The app closure will immediately be called once to create the initial ui.
    ///
    /// If you don't need to create Windows / Panels, you can use [`Harness::new_ui`] instead.
    ///
    /// If you e.g. want to customize the size of the window, you can use [`Harness::builder`].
    ///
    /// # Example
    /// ```rust
    /// # use egui::CentralPanel;
    /// # use egui_kittest::Harness;
    /// let mut harness = Harness::new(|ctx| {
    ///     CentralPanel::default().show(ctx, |ui| {
    ///         ui.label("Hello, world!");
    ///     });
    /// });
    /// ```
    pub fn new(app: impl FnMut(&egui::Context) + 'a) -> Self {
        Self::builder().build(app)
    }

    /// Create a new Harness with the given ui closure.
    /// Use the [`Harness::run`], [`Harness::step`], etc... methods to run the app.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// If you need to create Windows / Panels, you can use [`Harness::new`] instead.
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
    pub fn new_ui(app: impl FnMut(&mut egui::Ui) + 'a) -> Self {
        Self::builder().build_ui(app)
    }
}

impl<'t, 'n, State> Queryable<'t, 'n> for Harness<'_, State>
where
    'n: 't,
{
    fn node(&'n self) -> Node<'t> {
        self.kittest_state().node()
    }
}
