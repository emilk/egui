#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;

fn main() {
    puffin::set_scopes_on(true); // tell puffin to collect data
    start_puffin_server(); // send profile data to puffin_viewer

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "My egui App",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}

#[derive(Default)]
struct MyApp {}

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
                    ui.output().copied_text = cmd.into();
                }
            });

            ui.separator();

            ui.label("Note that this app runs in 'reactive' mode, so you must interact with the app for new profile events to be sent. Waving the mouse over this window is enough.");

            if ui
                .button(
                    "Click to sleep a bit. That should be visible as a spike in the profiler view!",
                )
                .clicked()
            {
                puffin::profile_scope!("sleep");
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        });
    }
}

fn start_puffin_server() {
    match puffin_http::Server::new("0.0.0.0:8585") {
        Ok(puffin_server) => {
            eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");
            std::mem::forget(puffin_server); // Let it run until the end of time
        }
        Err(err) => {
            eprintln!("Failed to start puffin server: {}", err);
        }
    };
}
