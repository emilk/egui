use egui::Frame;

type AppKindContext<'a> = Box<dyn FnMut(&egui::Context) + 'a>;
type AppKindUi<'a> = Box<dyn FnMut(&mut egui::Ui) + 'a>;

pub(crate) enum AppKind<'a> {
    Context(AppKindContext<'a>),
    Ui(AppKindUi<'a>),
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

impl<'a> AppKind<'a> {
    pub fn run(&mut self, ctx: &egui::Context) -> Option<egui::Response> {
        match self {
            AppKind::Context(f) => {
                f(ctx);
                None
            }
            AppKind::Ui(f) => Some(Self::run_ui(f, ctx, false)),
        }
    }

    pub(crate) fn run_sizing_pass(&mut self, ctx: &egui::Context) -> Option<egui::Response> {
        match self {
            AppKind::Context(f) => {
                f(ctx);
                None
            }
            AppKind::Ui(f) => Some(Self::run_ui(f, ctx, true)),
        }
    }

    fn run_ui(f: &mut AppKindUi<'a>, ctx: &egui::Context, sizing_pass: bool) -> egui::Response {
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
                        .show(ui, |ui| f(ui));
                })
                .response
            })
            .inner
    }
}
