use crate::app_kind::AppKind;
use crate::Harness;
use egui::{Pos2, Rect, Vec2};
use std::marker::PhantomData;
use std::sync::Arc;

/// Builder for [`Harness`].
pub struct HarnessBuilder<State = ()> {
    pub(crate) screen_rect: Rect,
    pub(crate) pixels_per_point: f32,
    pub(crate) state: PhantomData<State>,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu: Option<egui_wgpu::WgpuSetup>,
}

impl<State> Default for HarnessBuilder<State> {
    fn default() -> Self {
        Self {
            screen_rect: Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            pixels_per_point: 1.0,
            state: PhantomData,
            #[cfg(feature = "wgpu")]
            wgpu: None,
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

    /// Enable wgpu rendering with a default setup suitable for testing.
    #[cfg(feature = "wgpu")]
    pub fn wgpu(mut self) -> Self {
        self.wgpu = Some(egui_wgpu::WgpuSetup::CreateNew {
            supported_backends: egui_wgpu::wgpu::Backends::all(),
            device_descriptor: Arc::new(|a| egui_wgpu::wgpu::DeviceDescriptor::default()),
            power_preference: egui_wgpu::wgpu::PowerPreference::default(),
        });
        self
    }

    /// Enable wgpu rendering with the given setup.
    #[cfg(feature = "wgpu")]
    pub fn wgpu_setup(mut self, setup: impl Into<Option<egui_wgpu::WgpuSetup>>) -> Self {
        self.wgpu = setup.into();
        self
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
    pub fn build_state<'a>(
        self,
        app: impl FnMut(&egui::Context, &mut State) + 'a,
        state: State,
    ) -> Harness<'a, State> {
        Harness::from_builder(
            &self,
            AppKind::ContextState(Box::new(app)),
            state,
            None,
            None,
        )
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
    pub fn build_ui_state<'a>(
        self,
        app: impl FnMut(&mut egui::Ui, &mut State) + 'a,
        state: State,
    ) -> Harness<'a, State> {
        Harness::from_builder(&self, AppKind::UiState(Box::new(app)), state, None, None)
    }

    /// Create a new [Harness] from the given eframe creation closure.
    /// The app can be accessed via the [Harness::state] / [Harness::state_mut] methods.
    #[cfg(feature = "eframe")]
    pub fn build_eframe<'a>(
        mut self,
        build: impl FnOnce(&mut eframe::CreationContext<'a>) -> State,
    ) -> Harness<'a, State>
    where
        State: eframe::App,
    {
        let ctx = egui::Context::default();

        let mut cc = eframe::CreationContext::_new_kittest(ctx.clone());
        let mut frame = eframe::Frame::_new_kittest();

        #[cfg(feature = "wgpu")]
        let render_state = {
            let render_state = self.wgpu.take().map(crate::wgpu::create_render_state);
            cc.wgpu_render_state = render_state.clone();
            frame.wgpu_render_state = render_state.clone();
            render_state
        };
        #[cfg(not(feature = "wgpu"))]
        let render_state = None;

        let app = build(&mut cc);

        let kind = AppKind::Eframe((|state| state, frame));
        Harness::from_builder(&self, kind, app, Some(ctx), render_state)
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
    pub fn build<'a>(self, app: impl FnMut(&egui::Context) + 'a) -> Harness<'a> {
        Harness::from_builder(&self, AppKind::Context(Box::new(app)), (), None, None)
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
    pub fn build_ui<'a>(self, app: impl FnMut(&mut egui::Ui) + 'a) -> Harness<'a> {
        Harness::from_builder(&self, AppKind::Ui(Box::new(app)), (), None, None)
    }
}
