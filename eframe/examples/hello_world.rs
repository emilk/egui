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
    use egui::epaint::text::{layout, LayoutJob2, TextFormat};
    use egui::{Color32, Stroke, TextStyle};

    let mut job = LayoutJob2::default();

    job.append(
        "Hello ".into(),
        20.0,
        TextFormat {
            style: TextStyle::Body,
            color: Color32::WHITE,
            background: Color32::GRAY,
            strikethrough: Stroke::new(1.0, Color32::GREEN),
            ..Default::default()
        },
    );
    job.append(
        "there ".into(),
        0.0,
        TextFormat {
            style: TextStyle::Heading,
            color: Color32::RED,
            strikethrough: Stroke::new(1.0, Color32::GREEN),
            ..Default::default()
        },
    );
    job.append(
        "brave ".into(),
        0.0,
        TextFormat {
            style: TextStyle::Small,
            color: Color32::WHITE,
            underline: Stroke::new(1.0, Color32::WHITE),
            ..Default::default()
        },
    );
    job.append(
        "new ".into(),
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: Color32::LIGHT_BLUE,
            italics: true,
            underline: Stroke::new(1.0, Color32::WHITE),
            ..Default::default()
        },
    );
    job.append(
        "world!\n".into(),
        0.0,
        TextFormat {
            style: TextStyle::Monospace,
            color: Color32::WHITE,
            underline: Stroke::new(1.0, Color32::RED),
            ..Default::default()
        },
    );
    job.append(
        "Text can be ".into(),
        0.0,
        TextFormat {
            style: TextStyle::Body,
            color: Color32::WHITE,
            ..Default::default()
        },
    );
    job.append(
        "small ".into(),
        0.0,
        TextFormat {
            style: TextStyle::Small,
            color: Color32::WHITE,
            ..Default::default()
        },
    );
    job.append(
        "and raised!".into(),
        0.0,
        TextFormat {
            style: TextStyle::Small,
            color: Color32::WHITE,
            raised: true,
            ..Default::default()
        },
    );

    job.first_row_min_height = 50.0;
    job.wrap_width = 150.0;

    let galley = layout(ui.fonts(), job.into());

    let (response, painter) = ui.allocate_painter(galley.size, Sense::hover());
    painter.add(Shape::shape2(response.rect.min, galley.into()));
}
