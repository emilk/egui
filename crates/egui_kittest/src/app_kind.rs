use egui::Frame;

type AppKindContextState<'a, State> = Box<dyn FnMut(&egui::Context, &mut State) + 'a>;
type AppKindUiState<'a, State> = Box<dyn FnMut(&mut egui::Ui, &mut State) + 'a>;
type AppKindContext<'a> = Box<dyn FnMut(&egui::Context) + 'a>;
type AppKindUi<'a> = Box<dyn FnMut(&mut egui::Ui) + 'a>;

pub(crate) enum AppKind<'a, State> {
    Context(AppKindContext<'a>),
    Ui(AppKindUi<'a>),
    ContextState(AppKindContextState<'a, State>),
    UiState(AppKindUiState<'a, State>),
}

// TODO(lucasmerlin): These aren't working unfortunately :(
// I think they should work though: https://geo-ant.github.io/blog/2021/rust-traits-and-variadic-functions/
// pub trait IntoAppKind<'a, UiKind> {
//     fn into_harness_kind(self) -> AppKind<'a>;
// }
//
// impl<'a, F> IntoAppKind<'a, &egui::Context> for F
// where
//     F: FnMut(&egui::Context) + 'a,
// {
//     fn into_harness_kind(self) -> AppKind<'a> {
//         AppKind::Context(Box::new(self))
//     }
// }
//
// impl<'a, F> IntoAppKind<'a, &mut egui::Ui> for F
// where
//     F: FnMut(&mut egui::Ui) + 'a,
// {
//     fn into_harness_kind(self) -> AppKind<'a> {
//         AppKind::Ui(Box::new(self))
//     }
// }

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
            .frame(Frame::none())
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
                            _ => unreachable!(),
                        });
                })
                .response
            })
            .inner
    }
}
