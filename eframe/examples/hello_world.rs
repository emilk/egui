use eframe::{egui, epi};
use egui::{Sense, Shape};

struct MyApp {
    name: String,
    age: u32,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            name: "Arthur".to_owned(),
            age: 42,
        }
    }
}

impl epi::App for MyApp {
    fn name(&self) -> &str {
        "My egui App"
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        let Self { name, age } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.horizontal(|ui| {
                ui.label("Your name: ");
                ui.text_edit_singleline(name);
            });
            ui.add(egui::Slider::new(age, 0..=120).text("age"));
            if ui.button("Click each year").clicked() {
                *age += 1;
            }
            ui.label(format!("Hello '{}', age {}", name, age));

            ui.separator();

            test_galley2(ui);
        });

        // Resize the native window to be just the size we need it to be:
        frame.set_window_size(ctx.used_size());
    }
}

fn main() {
    let options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(MyApp::default()), options);
}

fn test_galley2(ui: &mut egui::Ui) {
    use egui::epaint::text::text_layout::{layout, LayoutJob, Section};
    use egui::Color32;
    use egui::TextStyle;

    let mut job = LayoutJob::default();
    job.sections.push(Section::HorizontalSpacing(20.0));

    job.append("Hello ".into(), TextStyle::Body, Color32::WHITE, false);
    job.append("there ".into(), TextStyle::Heading, Color32::RED, false);
    job.append("brave ".into(), TextStyle::Small, Color32::WHITE, false);
    job.append("new ".into(), TextStyle::Body, Color32::LIGHT_BLUE, true);
    job.append("world!".into(), TextStyle::Monospace, Color32::WHITE, false);

    job.first_row_min_height = 50.0;
    job.wrap_width = 100.0;

    let galley = layout(ui.fonts(), job.into());

    let (response, painter) = ui.allocate_painter(galley.size, Sense::hover());
    painter.add(Shape::Text2 {
        pos: response.rect.min,
        galley: galley.into(),
    });
}
