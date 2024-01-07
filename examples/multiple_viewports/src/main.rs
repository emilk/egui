#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Multiple viewports",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyApp {
    /// Immediate viewports are show immediately, so passing state to/from them is easy.
    /// The downside is that their painting is linked with the parent viewport:
    /// if either needs repainting, they are both repainted.
    show_immediate_viewport: bool,

    /// Deferred viewports run independent of the parent viewport, which can save
    /// CPU if only some of the viewports require repainting.
    /// However, this requires passing state with `Arc` and locks.
    show_deferred_viewport: Arc<AtomicBool>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Hello from the root viewport");

            ui.checkbox(
                &mut self.show_immediate_viewport,
                "Show immediate child viewport",
            );

            let mut show_deferred_viewport = self.show_deferred_viewport.load(Ordering::Relaxed);
            ui.checkbox(&mut show_deferred_viewport, "Show deferred child viewport");
            self.show_deferred_viewport
                .store(show_deferred_viewport, Ordering::Relaxed);
        });

        if self.show_immediate_viewport {
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("immediate_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Immediate Viewport")
                    .with_inner_size([200.0, 100.0]),
                |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from immediate viewport");
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.show_immediate_viewport = false;
                    }
                },
            );
        }

        if self.show_deferred_viewport.load(Ordering::Relaxed) {
            let show_deferred_viewport = self.show_deferred_viewport.clone();
            ctx.show_viewport_deferred(
                egui::ViewportId::from_hash_of("deferred_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Deferred Viewport")
                    .with_inner_size([200.0, 100.0]),
                move |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from deferred viewport");
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        // Tell parent to close us.
                        show_deferred_viewport.store(false, Ordering::Relaxed);
                    }
                },
            );
        }
    }
}
