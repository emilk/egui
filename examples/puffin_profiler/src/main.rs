#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    start_puffin_server(); // NOTE: you may only want to call this if the users specifies some flag or clicks a button!
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

struct MyApp {
    keep_repainting: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            keep_repainting: true,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Example of how to use the puffin profiler with egui");
            ui.separator();

            let cmd = "cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585";

            ui.label("To connect, run this:");
            ui.horizontal(|ui| {
                ui.monospace(cmd);
                if ui.small_button("📋").clicked() {
                    ui.ctx().copy_text(cmd.into());
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.keep_repainting, "Keep repainting");
                if self.keep_repainting {
                    ui.spinner();
                    ui.ctx().request_repaint();
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

            {
                // Sleep a bit to emulate some work:
                puffin::profile_scope!("small_sleep");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
    }
}

fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            eprintln!("Failed to start puffin server: {err}");
        }
    };
}
