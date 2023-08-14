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
                    ui.output_mut(|o| o.copied_text = cmd.into());
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
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("0.0.0.0:8585") {
        Ok(puffin_server) => {
            eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

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
