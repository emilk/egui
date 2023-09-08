#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use eframe::egui::panel::Side;
use eframe::egui::panel::TopBottomSide;
use eframe::egui::ImageFit;
use eframe::egui::Slider;
use eframe::emath::Rot2;
use eframe::epaint::Vec2;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        drag_and_drop_support: true,
        ..Default::default()
    };
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // The following call is needed to load images when using `ui.image`:
            egui_extras::loaders::install(&cc.egui_ctx);
            Box::<MyApp>::default()
        }),
    )
}

struct MyApp {
    current_uri: String,
    uri_edit_text: String,
    image_options: egui::ImageOptions,
    chosen_fit: ChosenFit,
    fit: ImageFit,
    maintain_aspect_ratio: bool,
    max_size: Option<Vec2>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ChosenFit {
    ExactSize,
    Fraction,
    OriginalSize,
}

impl ChosenFit {
    fn as_str(&self) -> &'static str {
        match self {
            ChosenFit::ExactSize => "exact size",
            ChosenFit::Fraction => "fraction",
            ChosenFit::OriginalSize => "original size",
        }
    }
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            current_uri: "https://picsum.photos/seed/1.759706314/1024".to_owned(),
            uri_edit_text: "https://picsum.photos/seed/1.759706314/1024".to_owned(),
            image_options: egui::ImageOptions::default(),
            chosen_fit: ChosenFit::Fraction,
            fit: ImageFit::Fraction(Vec2::splat(1.0)),
            maintain_aspect_ratio: true,
            max_size: None,
        }
    }
}

// TODO(jprochazk): expand this example to showcase different image/texture options

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::new(TopBottomSide::Top, "url bar").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("URI:");
                ui.text_edit_singleline(&mut self.uri_edit_text);
                if ui.small_button("✔").clicked() {
                    ctx.forget_image(&self.current_uri);
                    self.uri_edit_text = self.uri_edit_text.trim().to_owned();
                    self.current_uri = self.uri_edit_text.clone();
                };
                if ui.button("file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.uri_edit_text = format!("file://{}", path.display());
                        self.current_uri = self.uri_edit_text.clone();
                    }
                }
            });
        });

        egui::SidePanel::new(Side::Left, "controls").show(ctx, |ui| {
            // uv
            ui.label("UV");
            ui.add(Slider::new(&mut self.image_options.uv.min.x, 0.0..=1.0).text("min x"));
            ui.add(Slider::new(&mut self.image_options.uv.min.y, 0.0..=1.0).text("min y"));
            ui.add(Slider::new(&mut self.image_options.uv.max.x, 0.0..=1.0).text("max x"));
            ui.add(Slider::new(&mut self.image_options.uv.max.y, 0.0..=1.0).text("max y"));

            // rotation
            ui.add_space(2.0);
            let had_rotation = self.image_options.rotation.is_some();
            let mut has_rotation = had_rotation;
            ui.checkbox(&mut has_rotation, "Rotation");
            match (had_rotation, has_rotation) {
                (true, false) => self.image_options.rotation = None,
                (false, true) => {
                    self.image_options.rotation =
                        Some((Rot2::from_angle(0.0), Vec2::new(0.5, 0.5)));
                }
                (true, true) | (false, false) => {}
            }

            if let Some((rot, origin)) = self.image_options.rotation.as_mut() {
                let mut angle = rot.angle();

                ui.label("angle");
                ui.drag_angle(&mut angle);
                *rot = Rot2::from_angle(angle);

                ui.add(Slider::new(&mut origin.x, 0.0..=1.0).text("origin x"));
                ui.add(Slider::new(&mut origin.y, 0.0..=1.0).text("origin y"));
            }

            // bg_fill
            ui.add_space(2.0);
            ui.label("Background color");
            ui.color_edit_button_srgba(&mut self.image_options.bg_fill);

            // tint
            ui.add_space(2.0);
            ui.label("Tint");
            ui.color_edit_button_srgba(&mut self.image_options.tint);

            // aspect ratio
            ui.add_space(2.0);
            ui.checkbox(&mut self.maintain_aspect_ratio, "Maintain aspect ratio");

            // fit
            ui.add_space(2.0);
            egui::ComboBox::from_label("Fit")
                .selected_text(self.chosen_fit.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.chosen_fit,
                        ChosenFit::ExactSize,
                        ChosenFit::ExactSize.as_str(),
                    );
                    ui.selectable_value(
                        &mut self.chosen_fit,
                        ChosenFit::Fraction,
                        ChosenFit::Fraction.as_str(),
                    );
                    ui.selectable_value(
                        &mut self.chosen_fit,
                        ChosenFit::OriginalSize,
                        ChosenFit::OriginalSize.as_str(),
                    );
                });

            match self.chosen_fit {
                ChosenFit::ExactSize => {
                    if !matches!(self.fit, ImageFit::Exact(_)) {
                        self.fit = ImageFit::Exact(Vec2::splat(128.0));
                    }
                    let ImageFit::Exact(size) = &mut self.fit else { unreachable!() };
                    ui.add(Slider::new(&mut size.x, 0.0..=2048.0).text("width"));
                    ui.add(Slider::new(&mut size.y, 0.0..=2048.0).text("height"));
                }
                ChosenFit::Fraction => {
                    if !matches!(self.fit, ImageFit::Fraction(_)) {
                        self.fit = ImageFit::Fraction(Vec2::splat(1.0));
                    }
                    let ImageFit::Fraction(fract) = &mut self.fit else { unreachable!() };
                    ui.add(Slider::new(&mut fract.x, 0.0..=1.0).text("width"));
                    ui.add(Slider::new(&mut fract.y, 0.0..=1.0).text("height"));
                }
                ChosenFit::OriginalSize => {
                    if !matches!(self.fit, ImageFit::Original(_)) {
                        self.fit = ImageFit::Original(Some(1.0));
                    }
                    let ImageFit::Original(Some(scale)) = &mut self.fit else { unreachable!() };
                    ui.add(Slider::new(scale, 0.1..=4.0).text("scale"));
                }
            }

            // max size
            ui.add_space(2.0);
            let had_max_size = self.max_size.is_some();
            let mut has_max_size = had_max_size;
            ui.checkbox(&mut has_max_size, "Max size");
            match (had_max_size, has_max_size) {
                (true, false) => self.max_size = None,
                (false, true) => {
                    self.max_size = Some(ui.available_size());
                }
                (true, true) | (false, false) => {}
            }

            if let Some(max_size) = self.max_size.as_mut() {
                ui.add(Slider::new(&mut max_size.x, 0.0..=2048.0).text("width"));
                ui.add(Slider::new(&mut max_size.y, 0.0..=2048.0).text("height"));
            }

            // TODO:
            // texture_options
            // extent
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::new([true, true]).show(ui, |ui| {
                let mut image = egui::Image::from_uri(&self.current_uri);
                image = image.uv(self.image_options.uv);
                image = image.bg_fill(self.image_options.bg_fill);
                image = image.tint(self.image_options.tint);
                let (angle, origin) = self
                    .image_options
                    .rotation
                    .map_or((0.0, Vec2::splat(0.5)), |(rot, origin)| {
                        (rot.angle(), origin)
                    });
                image = image.rotate(angle, origin);
                match self.fit {
                    ImageFit::Original(scale) => image = image.fit_to_original_size(scale),
                    ImageFit::Fraction(fract) => image = image.fit_to_fraction(fract),
                    ImageFit::Exact(size) => image = image.fit_to_exact_size(size),
                }
                image = image.maintain_aspect_ratio(self.maintain_aspect_ratio);
                image = image.max_size(self.max_size);

                ui.add_sized(ui.available_size(), image);
            });
        });
    }
}
