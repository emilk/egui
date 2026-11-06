use crate::i18n;

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct About {}

impl crate::Demo for About {
    fn name(&self) -> &'static str {
        "About egui"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        egui::Window::new(i18n::t(self.name()))
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
            ui.add(
                egui::Image::new(egui::include_image!("../../data/egui-logo.svg"))
                    .max_height(30.0)
                    .tint(ui.visuals().strong_text_color()),
            );
            ui.add_space(4.0);
        });

        let os = format!("{OS_APPLE}{OS_LINUX}{OS_WINDOWS}");
        // The `{os}` placeholder is filled by `i18n::fmt`, not by `format!`.
        #[expect(clippy::literal_string_with_formatting_args)]
        let mut description = i18n::fmt(
            i18n::t(
                "egui is an immediate mode GUI library written in Rust. egui runs natively on \
                {os}, and on the web it is compiled to WebAssembly and rendered with WebGL or \
                WebGPU.",
            ),
            &[("os", os.as_str())],
        );
        if cfg!(target_arch = "wasm32") {
            description.push_str(i18n::t(
                " Everything you see is rendered as textured triangles. There is no DOM, HTML, \
                JS or CSS. Just Rust.",
            ));
        }
        ui.label(description);

        ui.add_space(12.0);
        ui.label(i18n::t("egui is easy to use, portable, and fast."));

        ui.add_space(12.0);
        about_immediate_mode(ui);

        ui.add_space(12.0);

        ui.heading(i18n::t("Links"));
        links(ui);

        ui.add_space(12.0);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let sponsored = i18n::t(
                "egui development is sponsored by {rerun}, a startup building a data platform \
                for robotics. ",
            );
            let (head, tail) = i18n::split_template(sponsored, "rerun");
            ui.weak(head);
            ui.hyperlink_to("Rerun.io", "https://www.rerun.io/");
            ui.weak(tail);
            let example = i18n::t(
                "For an example of a professional egui app, run {viewer} (in your browser!).",
            );
            let (head, tail) = i18n::split_template(example, "viewer");
            ui.weak(head);
            ui.hyperlink_to("rerun.io/viewer", "https://www.rerun.io/viewer");
            ui.weak(tail);
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
        ui.label(i18n::t("This is how you create a button in egui:"));
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
        ui.label(i18n::t(
            "There are no callbacks or messages, and no button state to store. ",
        ));
        let read_more = i18n::t("Read more about immediate mode {here}.");
        let (head, tail) = i18n::split_template(read_more, "here");
        ui.label(head);
        ui.hyperlink_to(
            i18n::t("here"),
            "https://github.com/emilk/egui#why-immediate-mode",
        );
        ui.label(tail);
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
    ui.hyperlink_to(i18n::t("egui documentation"), "https://docs.rs/egui/");
}
