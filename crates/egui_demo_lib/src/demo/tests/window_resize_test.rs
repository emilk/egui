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

        Window::new("↔ auto-sized")
            .open(open)
            .auto_sized()
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label("This window will auto-size based on its contents.");
                ui.heading("Resize this area:");
                Resize::default().show(ui, |ui| {
                    lorem_ipsum(ui, crate::LOREM_IPSUM);
                });
                ui.heading("Resize the above area!");
            });

        Window::new("↔ resizable + scroll")
            .open(open)
            .vscroll(true)
            .resizable(true)
            .default_height(300.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label(
                    "This window is resizable and has a scroll area. You can shrink it to any size.",
                );
                ui.separator();
                lorem_ipsum(ui, crate::LOREM_IPSUM_LONG);
            });

        Window::new("↔ resizable + embedded scroll")
            .open(open)
            .vscroll(false)
            .resizable(true)
            .default_height(300.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label("This window is resizable but has no built-in scroll area.");
                ui.label("However, we have a sub-region with a scroll bar:");
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| {
                    let lorem_ipsum_extra_long =
                        format!("{}\n\n{}", crate::LOREM_IPSUM_LONG, crate::LOREM_IPSUM_LONG);
                    lorem_ipsum(ui, &lorem_ipsum_extra_long);
                });
                // ui.heading("Some additional text here, that should also be visible"); // this works, but messes with the resizing a bit
            });

        Window::new("↔ resizable without scroll")
            .open(open)
            .vscroll(false)
            .resizable(true)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label("This window is resizable but has no scroll area. This means it can only be resized to a size where all the contents is visible.");
                ui.label("egui will not clip the contents of a window, nor add whitespace to it.");
                ui.separator();
                lorem_ipsum(ui, crate::LOREM_IPSUM);
            });

        Window::new("↔ resizable with TextEdit")
            .open(open)
            .vscroll(false)
            .resizable(true)
            .default_height(300.0)
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label("Shows how you can fill an area with a widget.");
                ui.add_sized(ui.available_size(), TextEdit::multiline(&mut self.text));
            });

        Window::new("↔ freely resized")
            .open(open)
            .vscroll(false)
            .resizable(true)
            .default_size([250.0, 150.0])
            .constrain_to(ui.available_rect_before_wrap())
            .show(ui, |ui| {
                ui.label("This window has empty space that fills up the available space, preventing auto-shrink.");
                ui.vertical_centered(|ui| {
                    ui.add(crate::egui_github_link_file!());
                });
                ui.allocate_space(ui.available_size());
            });
    }
}

fn lorem_ipsum(ui: &mut egui::Ui, text: &str) {
    ui.with_layout(
        egui::Layout::top_down(egui::Align::LEFT).with_cross_justify(true),
        |ui| {
            ui.label(egui::RichText::new(text).weak());
        },
    );
}
