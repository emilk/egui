use egui::emath::Rot2;
use egui::panel::Side;
use egui::panel::TopBottomSide;
use egui::ImageFit;
use egui::Slider;
use egui::Vec2;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ImageViewer {
    current_uri: String,
    uri_edit_text: String,
    image_options: egui::ImageOptions,
    chosen_fit: ChosenFit,
    fit: ImageFit,
    maintain_aspect_ratio: bool,
    max_size: Vec2,
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
enum ChosenFit {
    ExactSize,
    Fraction,
    OriginalSize,
}

impl ChosenFit {
    fn as_str(&self) -> &'static str {
        match self {
            Self::ExactSize => "exact size",
            Self::Fraction => "fraction",
            Self::OriginalSize => "original size",
        }
    }
}

impl Default for ImageViewer {
    fn default() -> Self {
        Self {
            current_uri: "https://picsum.photos/seed/1.759706314/1024".to_owned(),
            uri_edit_text: "https://picsum.photos/seed/1.759706314/1024".to_owned(),
            image_options: egui::ImageOptions::default(),
            chosen_fit: ChosenFit::Fraction,
            fit: ImageFit::Fraction(Vec2::splat(1.0)),
            maintain_aspect_ratio: true,
            max_size: Vec2::splat(2048.0),
        }
    }
}

impl eframe::App for ImageViewer {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::TopBottomPanel::new(TopBottomSide::Top, "url bar").show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.label("URI:");
                ui.text_edit_singleline(&mut self.uri_edit_text);
                if ui.small_button("✔").clicked() {
                    ctx.forget_image(&self.current_uri);
                    self.uri_edit_text = self.uri_edit_text.trim().to_owned();
                    self.current_uri = self.uri_edit_text.clone();
                };

                #[cfg(not(target_arch = "wasm32"))]
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
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba(&mut self.image_options.bg_fill);
                ui.label("Background color");
            });

            // tint
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.color_edit_button_srgba(&mut self.image_options.tint);
                ui.label("Tint");
            });

            // fit
            ui.add_space(10.0);
            ui.label(
                "The chosen fit will determine how the image tries to fill the available space",
            );
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
                    let ImageFit::Exact(size) = &mut self.fit else {
                        unreachable!()
                    };
                    ui.add(Slider::new(&mut size.x, 0.0..=2048.0).text("width"));
                    ui.add(Slider::new(&mut size.y, 0.0..=2048.0).text("height"));
                }
                ChosenFit::Fraction => {
                    if !matches!(self.fit, ImageFit::Fraction(_)) {
                        self.fit = ImageFit::Fraction(Vec2::splat(1.0));
                    }
                    let ImageFit::Fraction(fract) = &mut self.fit else {
                        unreachable!()
                    };
                    ui.add(Slider::new(&mut fract.x, 0.0..=1.0).text("width"));
                    ui.add(Slider::new(&mut fract.y, 0.0..=1.0).text("height"));
                }
                ChosenFit::OriginalSize => {
                    if !matches!(self.fit, ImageFit::Original { .. }) {
                        self.fit = ImageFit::Original { scale: 1.0 };
                    }
                    let ImageFit::Original { scale } = &mut self.fit else {
                        unreachable!()
                    };
                    ui.add(Slider::new(scale, 0.1..=4.0).text("scale"));
                }
            }

            // max size
            ui.add_space(5.0);
            ui.label("The calculated size will not exceed the maximum size");
            ui.add(Slider::new(&mut self.max_size.x, 0.0..=2048.0).text("width"));
            ui.add(Slider::new(&mut self.max_size.y, 0.0..=2048.0).text("height"));

            // aspect ratio
            ui.add_space(5.0);
            ui.label("Aspect ratio is maintained by scaling both sides as necessary");
            ui.checkbox(&mut self.maintain_aspect_ratio, "Maintain aspect ratio");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
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
                    ImageFit::Original { scale } => image = image.fit_to_original_size(scale),
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
