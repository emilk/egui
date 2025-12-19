#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use eframe::egui;

fn main() -> eframe::Result {
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_owned());

    // SAFETY: we call this from the main thread without any other threads running.
    #[expect(unsafe_code)]
    unsafe {
        std::env::set_var("RUST_LOG", rust_log);
    };

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    start_puffin_server(); // NOTE: you may only want to call this if the users specifies some flag or clicks a button!

    eframe::run_native(
        "My egui App",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default(),

            #[cfg(feature = "wgpu")]
            renderer: eframe::Renderer::Wgpu,

            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    )
}

struct MyApp {
    keep_repainting: bool,

    // It is useful to be able to inspect how eframe acts with multiple viewport
    // so we have two viewports here that we can toggle on/off.
    show_immediate_viewport: bool,
    show_deferred_viewport: Arc<AtomicBool>,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            keep_repainting: true,
            show_immediate_viewport: Default::default(),
            show_deferred_viewport: Default::default(),
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Example of how to use the puffin profiler with egui");
            ui.separator();

            let cmd = "cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585";

            ui.label("To connect, run this:");
            ui.horizontal(|ui| {
                ui.monospace(cmd);
                if ui.small_button("📋").clicked() {
                    ui.copy_text(cmd.into());
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.keep_repainting, "Keep repainting");
                if self.keep_repainting {
                    ui.spinner();
                    ui.request_repaint();
                } else {
                    ui.label("Repainting on events (e.g. mouse movement)");
                }
            });

            if ui
                .button(
                    "Click to sleep a bit. That should be visible as a spike in the profiler view!",
                )
                .clicked()
            {
                puffin::profile_scope!("long_sleep");
                std::thread::sleep(std::time::Duration::from_millis(50));
            }

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
            ui.ctx().show_viewport_immediate(
                egui::ViewportId::from_hash_of("immediate_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Immediate Viewport")
                    .with_inner_size([200.0, 100.0]),
                |ui, class| {
                    puffin::profile_scope!("immediate_viewport");

                    assert!(
                        class == egui::ViewportClass::Immediate,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        ui.label("Hello from immediate viewport");
                    });

                    if ui.input(|i| i.viewport().close_requested()) {
                        // Tell parent viewport that we should not show next frame:
                        self.show_immediate_viewport = false;
                    }
                },
            );
        }

        if self.show_deferred_viewport.load(Ordering::Relaxed) {
            let show_deferred_viewport = Arc::clone(&self.show_deferred_viewport);
            ui.ctx().show_viewport_deferred(
                egui::ViewportId::from_hash_of("deferred_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Deferred Viewport")
                    .with_inner_size([200.0, 100.0]),
                move |ui, class| {
                    puffin::profile_scope!("deferred_viewport");

                    assert!(
                        class == egui::ViewportClass::Deferred,
                        "This egui backend doesn't support multiple viewports"
                    );

                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        ui.label("Hello from deferred viewport");
                    });
                    if ui.input(|i| i.viewport().close_requested()) {
                        // Tell parent to close us.
                        show_deferred_viewport.store(false, Ordering::Relaxed);
                    }
                },
            );
        }
    }
}

fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            log::info!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[expect(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            log::error!("Failed to start puffin server: {err}");
        }
    }
}
