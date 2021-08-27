use super::drag_and_drop::DragAndDropDemo;

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct ContextMenus {
    title: String,
    plot: Plot,
    drag_and_drop: DragAndDropDemo,
}
const DEFAULT_TITLE: &str = "☰ Context Menus";
impl Default for ContextMenus {
    fn default() -> Self {
        Self {
            title: DEFAULT_TITLE.to_owned(),
            plot: Plot::Sin,
            drag_and_drop: DragAndDropDemo::default().editable(true),
        }
    }
}

impl super::Demo for ContextMenus {
    fn name(&self) -> &'static str {
        DEFAULT_TITLE
    }

    fn show(&mut self, ctx: &egui::CtxRef, open: &mut bool) {
        let Self { title, .. } = self.clone();

        use super::View;
        let window = egui::Window::new(title)
            .id(egui::Id::new("demo_context_menus")) // required since we change the title
            .scroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for ContextMenus {
    fn ui(&mut self, ui: &mut egui::Ui) {
        let Self {
            title,
            plot,
            drag_and_drop,
        } = self;
        ui.horizontal(|ui| {
            ui.label("Right click to clear:");
            ui.text_edit_singleline(title)
                .context_menu(|ui, menu_state| {
                    if menu_state.item("Clear..").show(ui).clicked() {
                        *title = String::new();
                        menu_state.close();
                    }
                    if menu_state.item("Reset..").show(ui).clicked() {
                        *title = DEFAULT_TITLE.to_owned();
                        menu_state.close();
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.label("Right click to set plot:");
            ui.add(example_plot(plot)).context_menu(|ui, menu_state| {
                if menu_state.item("Sin").show(ui).clicked() {
                    *plot = Plot::Sin;
                    menu_state.close();
                } else if menu_state.item("Bell").show(ui).clicked() {
                    *plot = Plot::Bell;
                    menu_state.close();
                } else if menu_state.item("Sigmoid").show(ui).clicked() {
                    *plot = Plot::Sigmoid;
                    menu_state.close();
                }
            });
        });
        ui.label("Right click to edit items");
        drag_and_drop.ui(ui);
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
enum Plot {
    Sin,
    Bell,
    Sigmoid,
}

fn gaussian(x: f64) -> f64 {
    let var: f64 = 2.0;
    f64::exp(-0.5 * (x / var).powi(2)) / (var * f64::sqrt(std::f64::consts::TAU))
}
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + f64::exp(-x))
}
fn example_plot(plot: &Plot) -> egui::plot::Plot {
    use egui::plot::{Line, Value, Values};
    let n = 128;
    let line = Line::new(Values::from_values_iter((0..=n).map(|i| {
        use std::f64::consts::TAU;
        let x = egui::remap(i as f64, 0.0..=n as f64, -TAU..=TAU);
        match plot {
            Plot::Sin => Value::new(x, x.sin()),
            Plot::Bell => Value::new(x, 5.0 * gaussian(x)),
            Plot::Sigmoid => Value::new(x, sigmoid(x)),
        }
    })));
    egui::plot::Plot::new("example_plot")
        .line(line)
        .height(32.0)
        .data_aspect(1.0)
}
