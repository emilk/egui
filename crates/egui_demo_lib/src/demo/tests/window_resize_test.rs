pub struct WindowResizeTest {
    text: String,
}

impl Default for WindowResizeTest {
    fn default() -> Self {
        Self {
            text: crate::LOREM_IPSUM_LONG.to_owned(),
        }
    }
}

impl crate::Demo for WindowResizeTest {
    fn name(&self) -> &'static str {
        "Window Resize Test"
    }

    fn show(&mut self, ui: &mut egui::Ui, open: &mut bool) {
        use egui::{Resize, ScrollArea, TextEdit, Window};

        Window::new(crate::i18n::t("↔️ auto-sized"))
            .open(open)
            .auto_sized()
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(crate::i18n::t(
                    "This window will auto-size based on its contents.",
                ));
                ui.heading(crate::i18n::t("Resize this area:"));
                Resize::default().show(ui, |ui| {
                    lorem_ipsum(ui, crate::LOREM_IPSUM);
                });
                ui.heading(crate::i18n::t("Resize the above area!"));
            });

        Window::new(crate::i18n::t("↔️ resizable + scroll"))
            .open(open)
            .vscroll(true)
            .resizable(true)
            .default_height(300.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(crate::i18n::t(
                    "This window is resizable and has a scroll area. You can shrink it to any size.",
                ));
                ui.separator();
                lorem_ipsum(ui, crate::LOREM_IPSUM_LONG);
            });

        Window::new(crate::i18n::t("↔️ resizable + embedded scroll"))
            .open(open)
            .vscroll(false)
            .resizable(true)
            .default_height(300.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(crate::i18n::t(
                    "This window is resizable but has no built-in scroll area.",
                ));
                ui.label(crate::i18n::t(
                    "However, we have a sub-region with a scroll bar:",
                ));
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| {
                    let lorem_ipsum_extra_long =
                        format!("{}\n\n{}", crate::LOREM_IPSUM_LONG, crate::LOREM_IPSUM_LONG);
                    lorem_ipsum(ui, &lorem_ipsum_extra_long);
                });
                // ui.heading("Some additional text here, that should also be visible"); // this works, but messes with the resizing a bit
            });

        Window::new(crate::i18n::t("↔️ resizable without scroll"))
            .open(open)
            .vscroll(false)
            .resizable(true)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(crate::i18n::t(
                    "This window is resizable but has no scroll area. This means it can only be resized to a size where all the contents is visible.",
                ));
                ui.label(crate::i18n::t(
                    "egui will not clip the contents of a window, nor add whitespace to it.",
                ));
                ui.separator();
                lorem_ipsum(ui, crate::LOREM_IPSUM);
            });

        Window::new(crate::i18n::t("↔️ resizable with TextEdit"))
            .open(open)
            .vscroll(false)
            .resizable(true)
            .default_height(300.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(crate::i18n::t(
                    "Shows how you can fill an area with a widget.",
                ));
                ui.add_sized(
                    ui.available_size(),
                    TextEdit::multiline(&mut self.text).hint_text(crate::i18n::t("Type here")),
                );
            });

        Window::new(crate::i18n::t("↔️ freely resized"))
            .open(open)
            .vscroll(false)
            .resizable(true)
            .default_size([250.0, 150.0])
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(crate::i18n::t(
                    "This window has empty space that fills up the available space, preventing auto-shrink.",
                ));
                ui.vertical_centered(|ui| {
                    ui.add(crate::egui_github_link_file!());
                });
                ui.allocate_space(ui.available_size());
            });
    }
}

// The lorem-ipsum bodies are Latin placeholder filler on purpose; only the chrome translates.
fn lorem_ipsum(ui: &mut egui::Ui, text: &str) {
    ui.with_layout(
        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
        |ui| {
            ui.label(egui::RichText::new(text).weak());
        },
    );
}
