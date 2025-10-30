#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;
use std::sync::atomic::AtomicUsize;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Multiple viewports",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

#[derive(Default)]
struct MyApp {
    /// Immediate viewports are show immediately, so passing state to/from them is easy.
    /// The downside is that their painting is linked with the parent viewport:
    /// if either needs repainting, they are both repainted.
    show_immediate_viewport: bool,
    immediate_viewport_redraw_counter: usize,

    /// Deferred viewports run independent of the parent viewport, which can save
    /// CPU if only some of the viewports require repainting.
    /// However, this requires passing state with `Arc` and locks.
    show_deferred_viewport: Arc<AtomicBool>,
    deferred_viewport_redraw_counter: Arc<AtomicUsize>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut deferred_viewport_refresh_requested = false;
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

            deferred_viewport_refresh_requested =
                ui.button("request refresh of deferred viewport").clicked();

            ui.group(|ui| {
                ui.label(format!(
                    "Immediate viewport counter: {}",
                    self.immediate_viewport_redraw_counter
                ));
                ui.label(format!(
                    "Deferred viewport counter: {}",
                    self.deferred_viewport_redraw_counter
                        .load(Ordering::Relaxed)
                ));
            })
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

                    self.immediate_viewport_redraw_counter += 1;

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from immediate viewport");
                        ui.label(format!(
                            "Counter: {}",
                            self.immediate_viewport_redraw_counter
                        ));
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
            let counter = self.deferred_viewport_redraw_counter.clone();

            let deferred_viewport_id = egui::ViewportId::from_hash_of("deferred_viewport");

            if deferred_viewport_refresh_requested {
                ctx.request_repaint_of(deferred_viewport_id);
            }

            ctx.show_viewport_deferred(
                deferred_viewport_id,
                egui::ViewportBuilder::default()
                    .with_title("Deferred Viewport")
                    .with_inner_size([200.0, 100.0]),
                move |ctx, class| {
                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    let value = counter.fetch_add(1, Ordering::Relaxed);

                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.label("Hello from deferred viewport");
                        ui.label(format!("Counter: {value}"));
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
