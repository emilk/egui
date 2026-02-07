use crate::app_kind::AppKind;
use crate::{Harness, LazyRenderer, TestRenderer};
use egui::{Pos2, Rect, Vec2};
use std::marker::PhantomData;

/// Builder for [`Harness`].
#[must_use]
pub struct HarnessBuilder<State = ()> {
    pub(crate) screen_rect: Rect,
    pub(crate) pixels_per_point: f32,
    pub(crate) theme: egui::Theme,
    pub(crate) os: egui::os::OperatingSystem,
    pub(crate) max_steps: u64,
    pub(crate) step_dt: f32,
    pub(crate) state: PhantomData<State>,
    pub(crate) renderer: Box<dyn TestRenderer>,
    pub(crate) wait_for_pending_images: bool,

    #[cfg(feature = "snapshot")]
    pub(crate) default_snapshot_options: crate::SnapshotOptions,
}

impl<State> Default for HarnessBuilder<State> {
    fn default() -> Self {
        Self {
            screen_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            pixels_per_point: 1.0,
            theme: egui::Theme::Dark,
            state: PhantomData,
            renderer: Box::new(LazyRenderer::default()),
            max_steps: 4,
            step_dt: 1.0 / 4.0,
            wait_for_pending_images: true,
            os: egui::os::OperatingSystem::Nix,

            #[cfg(feature = "snapshot")]
            default_snapshot_options: crate::SnapshotOptions::default(),
        }
    }
}

impl<State> HarnessBuilder<State> {
    /// Set the size of the window.
    #[inline]
    pub fn with_size(mut self, size: impl Into<Vec2>) -> Self {
        let size = size.into();
        self.screen_rect.set_width(size.x);
        self.screen_rect.set_height(size.y);
        self
    }

    /// Set the `pixels_per_point` of the window.
    #[inline]
    pub fn with_pixels_per_point(mut self, pixels_per_point: f32) -> Self {
        self.pixels_per_point = pixels_per_point;
        self
    }

    /// Set the desired theme (dark or light).
    #[inline]
    pub fn with_theme(mut self, theme: egui::Theme) -> Self {
        self.theme = theme;
        self
    }

    /// Set the default options used for snapshot tests on this harness.
    #[cfg(feature = "snapshot")]
    #[inline]
    pub fn with_options(mut self, options: crate::SnapshotOptions) -> Self {
        self.default_snapshot_options = options;
        self
    }

    /// Override the [`egui::os::OperatingSystem`] reported to egui.
    ///
    /// This affects e.g. the way shortcuts are displayed. So for snapshot tests,
    /// it makes sense to set this to a specific OS, so snapshots don't change when running
    /// the same tests on different OSes.
    ///
    /// Default is [`egui::os::OperatingSystem::Nix`].
    /// Use [`egui::os::OperatingSystem::from_target_os()`] to use the current OS (this restores
    /// eguis default behavior).
    #[inline]
    pub fn with_os(mut self, os: egui::os::OperatingSystem) -> Self {
        self.os = os;
        self
    }

    /// Set the maximum number of steps to run when calling [`Harness::run`].
    ///
    /// Default is 4.
    /// With the default `step_dt`, this means 1 second of simulation.
    #[inline]
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Set the time delta for a single [`Harness::step`].
    ///
    /// Default is 1.0 / 4.0 (4fps).
    /// The default is low so we don't waste cpu waiting for animations.
    #[inline]
    pub fn with_step_dt(mut self, step_dt: f32) -> Self {
        self.step_dt = step_dt;
        self
    }

    /// Should we wait for pending images?
    ///
    /// If `true`, [`Harness::run`] and related methods will check if there are pending images
    /// (via [`egui::Context::has_pending_images`]) and sleep for [`Self::with_step_dt`] up to
    /// [`Self::with_max_steps`] times.
    ///
    /// Default: `true`
    #[inline]
    pub fn with_wait_for_pending_images(mut self, wait_for_pending_images: bool) -> Self {
        self.wait_for_pending_images = wait_for_pending_images;
        self
    }

    /// Set the [`TestRenderer`] to use for rendering.
    ///
    /// By default, a [`LazyRenderer`] is used.
    #[inline]
    pub fn renderer(mut self, renderer: impl TestRenderer + 'static) -> Self {
        self.renderer = Box::new(renderer);
        self
    }

    /// Enable wgpu rendering with a default setup suitable for testing.
    ///
    /// This sets up a [`crate::wgpu::WgpuTestRenderer`] with the default setup.
    #[cfg(feature = "wgpu")]
    pub fn wgpu(self) -> Self {
        self.renderer(crate::wgpu::WgpuTestRenderer::default())
    }

    /// Enable wgpu rendering with the given setup.
    #[cfg(feature = "wgpu")]
    pub fn wgpu_setup(self, setup: egui_wgpu::WgpuSetup) -> Self {
        self.renderer(crate::wgpu::WgpuTestRenderer::from_setup(setup))
    }

    /// Create a new Harness with the given app closure and a state.
    ///
    /// The app closure will immediately be called once to create the initial ui.
    ///
    /// If you don't need to create Windows / Panels, you can use [`HarnessBuilder::build_ui`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use egui::CentralPanel;
    /// # use egui_kittest::{Harness, kittest::Queryable};
    /// let checked = false;
    /// let mut harness = Harness::builder()
    ///     .with_size(egui::Vec2::new(300.0, 200.0))
    ///     .build_state(|ctx, checked| {
    ///         CentralPanel::default().show(ctx, |ui| {
    ///             ui.checkbox(checked, "Check me!");
    ///         });
    ///     }, checked);
    ///
    /// harness.get_by_label("Check me!").click();
    /// harness.run();
    ///
    /// assert_eq!(*harness.state(), true);
    /// ```
    #[track_caller]
    #[deprecated = "use `build_ui_state` instead"]
    pub fn build_state<'a>(
        self,
        app: impl FnMut(&egui::Context, &mut State) + 'a,
        state: State,
    ) -> Harness<'a, State> {
        Harness::from_builder(self, AppKind::ContextState(Box::new(app)), state, None)
    }

    /// Create a new Harness with the given ui closure and a state.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// If you need to create Windows / Panels, you can use [`HarnessBuilder::build`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use egui_kittest::{Harness, kittest::Queryable};
    /// let mut checked = false;
    /// let mut harness = Harness::builder()
    ///     .with_size(egui::Vec2::new(300.0, 200.0))
    ///     .build_ui_state(|ui, checked| {
    ///        ui.checkbox(checked, "Check me!");
    ///     }, checked);
    ///
    /// harness.get_by_label("Check me!").click();
    /// harness.run();
    ///
    /// assert_eq!(*harness.state(), true);
    /// ```
    #[track_caller]
    pub fn build_ui_state<'a>(
        self,
        app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
        state: State,
    ) -> Harness<'a, State> {
        Harness::from_builder(self, AppKind::UiState(Box::new(app)), state, None)
    }

    /// Create a new [Harness] from the given eframe creation closure.
    /// The app can be accessed via the [`Harness::state`] / [`Harness::state_mut`] methods.
    #[cfg(feature = "eframe")]
    #[track_caller]
    pub fn build_eframe<'a>(
        self,
        build: impl FnOnce(&mut eframe::CreationContext<'a>) -> State,
    ) -> Harness<'a, State>
    where
        State: eframe::App,
    {
        let ctx = egui::Context::default();

        let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut frame = eframe::Frame::_new_kittest();

        self.renderer.setup_eframe(&mut cc, &mut frame);

        let app = build(&mut cc);

        let kind = AppKind::Eframe((|state| state, frame));
        Harness::from_builder(self, kind, app, Some(ctx))
    }
}

impl HarnessBuilder {
    /// Create a new Harness with the given app closure.
    ///
    /// The app closure will immediately be called once to create the initial ui.
    ///
    /// If you don't need to create Windows / Panels, you can use [`HarnessBuilder::build_ui`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use egui::CentralPanel;
    /// # use egui_kittest::{Harness, kittest::Queryable};
    /// let mut harness = Harness::builder()
    ///     .with_size(egui::Vec2::new(300.0, 200.0))
    ///     .build(|ctx| {
    ///         CentralPanel::default().show(ctx, |ui| {
    ///             ui.label("Hello, world!");
    ///         });
    ///     });
    /// ```
    #[must_use]
    #[track_caller]
    #[deprecated = "use `build_ui` instead"]
    pub fn build<'a>(self, app: impl FnMut(&egui::Context) + 'a) -> Harness<'a> {
        Harness::from_builder(self, AppKind::Context(Box::new(app)), (), None)
    }

    /// Create a new Harness with the given ui closure.
    ///
    /// The ui closure will immediately be called once to create the initial ui.
    ///
    /// If you need to create Windows / Panels, you can use [`HarnessBuilder::build`] instead.
    ///
    /// # Example
    /// ```rust
    /// # use egui_kittest::{Harness, kittest::Queryable};
    /// let mut harness = Harness::builder()
    ///     .with_size(egui::Vec2::new(300.0, 200.0))
    ///     .build_ui(|ui| {
    ///         ui.label("Hello, world!");
    ///     });
    /// ```
    #[must_use]
    #[track_caller]
    pub fn build_ui<'a>(self, app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
        Harness::from_builder(self, AppKind::Ui(Box::new(app)), (), None)
    }
}
