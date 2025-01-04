use egui::Frame;

type AppKindContextState<'a, State> = Box<dyn FnMut(&egui::Context, &mut State) + 'a>;
type AppKindUiState<'a, State> = Box<dyn FnMut(&mut egui::Ui, &mut State) + 'a>;
type AppKindContext<'a> = Box<dyn FnMut(&egui::Context) + 'a>;
type AppKindUi<'a> = Box<dyn FnMut(&mut egui::Ui) + 'a>;

/// In order to access the [`eframe::App`] trait from the generic `State`, we store a function pointer
/// here that will return the dyn trait from the struct. In the builder we have the correct where
/// clause to be able to create this.
/// Later we can use it anywhere to get the [`eframe::App`] from the `State`.
#[cfg(feature = "eframe")]
type AppKindEframe<'a, State> = (fn(&mut State) -> &mut dyn eframe::App, eframe::Frame);

pub(crate) enum AppKind<'a, State> {
    Context(AppKindContext<'a>),
    Ui(AppKindUi<'a>),
    ContextState(AppKindContextState<'a, State>),
    UiState(AppKindUiState<'a, State>),
    #[cfg(feature = "eframe")]
    Eframe(AppKindEframe<'a, State>),
}

impl<'a, State> AppKind<'a, State> {
    pub fn run(
        &mut self,
        ctx: &egui::Context,
        state: &mut State,
        sizing_pass: bool,
    ) -> Option<egui::Response> {
        match self {
            AppKind::Context(f) => {
                debug_assert!(!sizing_pass, "Context closures cannot do a sizing pass");
                f(ctx);
                None
            }
            AppKind::ContextState(f) => {
                debug_assert!(!sizing_pass, "Context closures cannot do a sizing pass");
                f(ctx, state);
                None
            }
            #[cfg(feature = "eframe")]
            AppKind::Eframe((get_app, frame)) => {
                let app = get_app(state);
                app.update(ctx, frame);
                None
            }
            kind_ui => Some(kind_ui.run_ui(ctx, state, sizing_pass)),
        }
    }

    fn run_ui(
        &mut self,
        ctx: &egui::Context,
        state: &mut State,
        sizing_pass: bool,
    ) -> egui::Response {
        egui::CentralPanel::default()
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let mut builder = egui::UiBuilder::new();
                if sizing_pass {
                    builder.sizing_pass = true;
                }
                ui.scope_builder(builder, |ui| {
                    Frame::central_panel(ui.style())
                        .outer_margin(8.0)
                        .inner_margin(0.0)
                        .show(ui, |ui| match self {
                            AppKind::Ui(f) => f(ui),
                            AppKind::UiState(f) => f(ui, state),
                            _ => unreachable!(
                                "run_ui should only be called with AppKind::Ui or AppKind UiState"
                            ),
                        });
                })
                .response
            })
            .inner
    }
}
