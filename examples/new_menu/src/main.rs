use eframe::{egui::*, NativeOptions};
use new_menu::{menu, MenuHandle};

fn main() -> Result<(), eframe::Error> {
    eframe::run_native(
        "New menu",
        NativeOptions::default(),
        Box::new(|_| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {
    checked: bool,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            let button = ui.button("Menu button");

            menu(ui, &button, MenuHandle::Context, |ui| {
                let button = ui.button("subMenu");
                menu(ui, &button, MenuHandle::Hover, |ui| {
                    let _ = ui.button("Button");
                    ui.label("Text");
                });
            });

            ui.button("Context menu").context_menu(|ui| {
                ui.checkbox(&mut self.checked, "Checkbox");
            });

            ui.menu_button("Normal menu", |ui| {
                ui.menu_button("Sub", |ui| ui.label("text"));
                ui.menu_button("Sub", |ui| ui.label("text"))
                    .response
                    .context_menu(|ui| {
                        ui.label("Context menu! Wow!");
                    });
            });
        });
    }
}
