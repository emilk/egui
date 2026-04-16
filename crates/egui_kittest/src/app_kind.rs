use egui::Frame;

type AppKindUiState<'a, State> = Box<dyn FnMut(&mut egui::Ui, &mut State) + 'a>;
type AppKindUi<'a> = Box<dyn FnMut(&mut egui::Ui) + 'a>;

/// In order to access the [`eframe::App`] trait from the generic `State`, we store a function pointer
/// here that will return the dyn trait from the struct. In the builder we have the correct where
/// clause to be able to create this.
/// Later we can use it anywhere to get the [`eframe::App`] from the `State`.
#[cfg(feature = "eframe")]
type AppKindEframe<'a, State> = (fn(&mut State) -> &mut dyn eframe::App, eframe::Frame);

pub(crate) enum AppKind<'a, State> {
    Ui(AppKindUi<'a>),
    UiState(AppKindUiState<'a, State>),
    #[cfg(feature = "eframe")]
    Eframe(AppKindEframe<'a, State>),
}

impl<State> AppKind<'_, State> {
    pub fn run(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut State,
        sizing_pass: bool,
    ) -> Option<egui::Response> {
        match self {
            #[cfg(feature = "eframe")]
            AppKind::Eframe((get_app, frame)) => {
                let app = get_app(state);

                app.logic(ui, frame);

                app.ui(ui, frame);

                None
            }
            kind_ui => Some(kind_ui.run_ui(ui, state, sizing_pass)),
        }
    }

    fn run_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut State,
        sizing_pass: bool,
    ) -> egui::Response {
        let mut builder = egui::UiBuilder::new();
        if sizing_pass {
            builder.sizing_pass = true;
        }
        ui.scope_builder(builder, |ui| {
            Frame::central_panel(ui.style())
                // Only set outer margin, so we show no frame for tests with only free-floating windows/popups:
                .outer_margin(8.0)
                .inner_margin(0.0)
                .show(ui, |ui| match self {
                    AppKind::Ui(f) => f(ui),
                    AppKind::UiState(f) => f(ui, state),
                    #[cfg(feature = "eframe")]
                    AppKind::Eframe(_) => unreachable!(
                        "run_ui should only be called with AppKind::Ui or AppKind::UiState"
                    ),
                });
        })
        .response
    }
}
