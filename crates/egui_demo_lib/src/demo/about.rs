#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct About {}

impl crate::Demo for About {
    fn name(&self) -> &'static str {
        "About egui"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(self.name())
            .default_width(320.0)
            .default_height(480.0)
            .open(open)
            .resizable([true, false])
            .scroll(false)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                use crate::View as _;
                self.ui(ui);
            });
    }
}

impl crate::View for About {
    fn ui(&mut self, ui: &mut egui::Ui) {
        use egui::special_emojis::{OS_APPLE, OS_LINUX, OS_WINDOWS};

        ui.vertical_centered(|ui| {
            ui.add_space(4.0);
            let egui_icon = egui::include_image!("../../data/egui-logo.svg");
            ui.add(
                egui::Image::new(egui_icon.clone())
                    .max_height(30.0)
                    .tint(ui.visuals().strong_text_color()),
            );
            ui.add_space(4.0);
        });

        ui.label(format!(
            "egui is an immediate mode GUI library written in Rust. egui runs natively on {}{}{}, and \
            on the web it is compiled to WebAssembly and rendered with WebGL or WebGPU.{}",
            OS_APPLE, OS_LINUX, OS_WINDOWS,
            if cfg!(target_arch = "wasm32") {
                " Everything you see is rendered as textured triangles. There is no DOM, HTML, JS or CSS. Just Rust."
            } else {""}
        ));

        ui.add_space(12.0);
        ui.label("egui is easy to use, portable, and fast.");

        ui.add_space(12.0);
        about_immediate_mode(ui);

        ui.add_space(12.0);

        ui.heading("Links");
        links(ui);

        ui.add_space(12.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.weak("egui development is sponsored by ");
            ui.hyperlink_to("Rerun.io", "https://www.rerun.io/");
            ui.weak(", a startup building a data platform for robotics. ");
            ui.weak("For an example of a professional egui app, run ");
            ui.hyperlink_to("rerun.io/viewer", "https://www.rerun.io/viewer");
            ui.weak(" (in your browser!).");
        });

        ui.add_space(12.0);

        ui.vertical_centered(|ui| {
            ui.add(crate::egui_github_link_file!());
        });
    }
}

fn about_immediate_mode(ui: &mut egui::Ui) {
    ui.style_mut().spacing.interact_size.y = 0.0; // hack to make `horizontal_wrapped` work better with text.

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("This is how you create a button in egui:");
    });

    ui.add_space(8.0);
    crate::rust_view_ui(
        ui,
        r#"
  if ui.button("Save").clicked() {
      my_state.save();
  }"#
        .trim_start_matches('\n'),
    );
    ui.add_space(8.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("There are no callbacks or messages, and no button state to store. ");
        ui.label("Read more about immediate mode ");
        ui.hyperlink_to("here", "https://github.com/emilk/egui#why-immediate-mode");
        ui.label(".");
    });
}

fn links(ui: &mut egui::Ui) {
    use egui::special_emojis::GITHUB;
    ui.hyperlink_to(
        format!("{GITHUB} github.com/emilk/egui"),
        "https://github.com/emilk/egui",
    );
    ui.hyperlink_to(
        "@ernerfeldt.bsky.social",
        "https://bsky.app/profile/ernerfeldt.bsky.social",
    );
    ui.hyperlink_to("📓 egui documentation", "https://docs.rs/egui/");
}
