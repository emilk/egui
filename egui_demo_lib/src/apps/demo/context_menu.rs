use super::drag_and_drop::DragAndDropDemo;

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
enum Plot {
    Sin,
    Bell,
    Sigmoid,
}

fn gaussian(x: f64) -> f64 {
    let var: f64 = 2.0;
    f64::exp(-(x / var).powi(2)) / (var * f64::sqrt(std::f64::consts::TAU))
}
fn sigmoid(x: f64) -> f64 {
    -1.0 + 2.0 / (1.0 + f64::exp(-x))
}

#[derive(Clone, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct ContextMenus {
    title: String,
    plot: Plot,
    drag_and_drop: DragAndDropDemo,
    show_axes: [bool; 2],
    allow_drag: bool,
    allow_zoom: bool,
    center_x_axis: bool,
    center_y_axis: bool,
    width: f32,
    height: f32,
}

impl ContextMenus {
    fn example_plot(&self) -> egui::plot::Plot {
        use egui::plot::{Line, Value, Values};
        let n = 128;
        let line = Line::new(Values::from_values_iter((0..=n).map(|i| {
            use std::f64::consts::TAU;
            let x = egui::remap(i as f64, 0.0..=n as f64, -TAU..=TAU);
            match self.plot {
                Plot::Sin => Value::new(x, x.sin()),
                Plot::Bell => Value::new(x, 10.0 * gaussian(x)),
                Plot::Sigmoid => Value::new(x, sigmoid(x)),
            }
        })));
        egui::plot::Plot::new("example_plot")
            .show_axes(self.show_axes)
            .allow_drag(self.allow_drag)
            .allow_zoom(self.allow_zoom)
            .center_x_axis(self.center_x_axis)
            .center_x_axis(self.center_y_axis)
            .line(line)
            .width(self.width)
            .height(self.height)
            .data_aspect(1.0)
    }
}

const DEFAULT_TITLE: &str = "☰ Context Menus";

impl Default for ContextMenus {
    fn default() -> Self {
        Self {
            title: DEFAULT_TITLE.to_owned(),
            plot: Plot::Sin,
            drag_and_drop: DragAndDropDemo::default().editable(true),
            show_axes: [true, true],
            allow_drag: true,
            allow_zoom: true,
            center_x_axis: false,
            center_y_axis: false,
            width: 400.0,
            height: 200.0,
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
            .vscroll(false)
            .open(open);
        window.show(ctx, |ui| self.ui(ui));
    }
}

impl super::View for ContextMenus {
    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.title)
                .on_hover_text("Right click to clear")
                .context_menu(|ui| {
                    if ui.button("Clear").clicked() {
                        self.title = String::new();
                        ui.close();
                    }
                    if ui.button("Reset").clicked() {
                        self.title = DEFAULT_TITLE.to_owned();
                        ui.close();
                    }
                });
        });
        // das hier setzt das hier zurück
        // reset
        //self.center_x_axis = false;
        //self.center_y_axis = false;
        ui.horizontal(|ui| {
            ui.add(self.example_plot())
                .on_hover_text("Right click for options")
                .context_menu(|ui| {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::DragValue::new(&mut self.width)
                                .speed(1.0)
                                .prefix("Width:"),
                        );
                        ui.add(
                            egui::DragValue::new(&mut self.height)
                                .speed(1.0)
                                .prefix("Height:"),
                        );
                    });
                    ui.menu("Plot", |ui| {
                        if ui.radio_value(&mut self.plot, Plot::Sin, "Sin").clicked()
                            || ui
                                .radio_value(&mut self.plot, Plot::Bell, "Gaussian")
                                .clicked()
                            || ui
                                .radio_value(&mut self.plot, Plot::Sigmoid, "Sigmoid")
                                .clicked()
                        {
                            ui.close();
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.show_axes[0], "x-Axis");
                        ui.checkbox(&mut self.show_axes[1], "y-Axis");
                    });
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut self.allow_drag, "Drag").changed()
                            || ui.checkbox(&mut self.allow_zoom, "Zoom").changed()
                        {
                            ui.close();
                        }
                    });
                });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.button("Nested context menu").context_menu(|ui| {
                if ui.button("Open...").clicked() {
                    ui.close();
                }
                ui.menu("SubMenu", |ui| {
                    ui.menu("SubMenu", |ui| {
                        if ui.button("Open...").clicked() {
                            ui.close();
                        }
                        let _ = ui.button("Item");
                    });
                    ui.menu("SubMenu", |ui| {
                        if ui.button("Open...").clicked() {
                            ui.close();
                        }
                        let _ = ui.button("Item");
                    });
                    let _ = ui.button("Item");
                    if ui.button("Open...").clicked() {
                        ui.close();
                    }
                });
                ui.menu("SubMenu", |ui| {
                    let _ = ui.button("Item1");
                    let _ = ui.button("Item2");
                    let _ = ui.button("Item3");
                    let _ = ui.button("Item4");
                    if ui.button("Open...").clicked() {
                        ui.close();
                    }
                });
                let _ = ui.button("Very long text for this item");
            });
        });
        ui.separator();
        ui.label("Right click to edit items");
        self.drag_and_drop.ui(ui);
    }
}
