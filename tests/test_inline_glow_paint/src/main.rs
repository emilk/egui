#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(
    // it's a test:
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_used,
    rustdoc::missing_crate_level_docs
)]

// Test that we can paint to the screen using glow directly.

use eframe::egui;
use eframe::glow;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "My test app",
        options,
        Box::new(|_cc| Ok(Box::<MyTestApp>::default())),
    )?;
    Ok(())
}

#[derive(Default)]
struct MyTestApp {}

impl eframe::App for MyTestApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        use glow::HasContext as _;
        let gl = frame.gl().unwrap();

        #[expect(unsafe_code)]
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.viewport(0, 0, 100, 100);
            gl.clear_color(1.0, 0.0, 1.0, 1.0); // purple
            gl.clear(glow::COLOR_BUFFER_BIT);
        }

        egui::Window::new("Floating Window").show(ui.ctx(), |ui| {
            ui.label("The background should be purple.");
        });
    }
}
