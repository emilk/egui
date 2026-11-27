#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![expect(rustdoc::missing_crate_level_docs)] // it's an example

use eframe::egui;

fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let args: Vec<String> = std::env::args().collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 700.0]),
        renderer: if has("--wgpu") {
            eframe::Renderer::Wgpu
        } else {
            eframe::Renderer::Glow
        },
        prefer_10_bit_color: !has("--8-bit"),
        // Dithering would hide the 8-bit steps that this example shows.
        dithering: false,
        ..Default::default()
    };
    eframe::run_native(
        "10-bit color",
        options,
        Box::new(|cc| {
            // A bright background makes the dark steps harder to see.
            cc.egui_ctx.set_theme(egui::Theme::Dark);
            Ok(Box::new(MyApp))
        }),
    )
}

struct MyApp;

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |ui| {
            let bits = if frame.is_10_bit_color() { 10 } else { 8 };
            ui.heading(format!("This window renders {bits}-bit color"));
            ui.label(
                "Each gradient is drawn twice: in the upper row as 8 flat steps, as an 8-bit \
                 picture shows it, in the lower row smoothly.\n\
                 • If the lower row shows the same 8 wide steps, something is 8-bit: this window, \
                 the GPU output (set 10 bpc in the GPU driver) or the monitor.\n\
                 • If every step splits into about 4 narrower ones, your monitor shows 10-bit color.",
            );
            ui.add_space(8.0);

            let height = (ui.available_height() / 2.0 - 8.0).max(40.0);
            let steps = 8;
            // Dark gradients show banding best; green is where the eye sees the smallest steps.
            let gradients = [
                (egui::Color32::from_gray(32), egui::Color32::from_gray(32 + steps)),
                (
                    egui::Color32::from_rgb(0, 64, 0),
                    egui::Color32::from_rgb(0, 64 + steps, 0),
                ),
            ];
            for (start, end) in gradients {
                let rect = ui.available_rect_before_wrap();
                let strip = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), height));
                ui.allocate_rect(strip, egui::Sense::hover());
                ui.add_space(8.0);
                let painter = ui.painter();
                let (top, bottom) = strip.split_top_bottom_at_fraction(0.5);

                for i in 0..steps {
                    let t = i as f32 / steps as f32;
                    let x0 = egui::lerp(top.x_range(), t);
                    let x1 = egui::lerp(top.x_range(), (i + 1) as f32 / steps as f32);
                    let rect = egui::Rect::from_x_y_ranges(x0..=x1, top.y_range());
                    painter.rect_filled(rect, 0.0, start.lerp_to_gamma(end, t));
                }

                // Colors at the ends only: the GPU interpolates them per pixel, finer than 8 bits.
                let mut mesh = egui::Mesh::default();
                mesh.colored_vertex(bottom.left_top(), start);
                mesh.colored_vertex(bottom.right_top(), end);
                mesh.colored_vertex(bottom.right_bottom(), end);
                mesh.colored_vertex(bottom.left_bottom(), start);
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(0, 2, 3);
                painter.add(mesh);

                for (row, text) in [(top, "8-bit"), (bottom, "smooth")] {
                    painter.text(
                        row.left_center() + egui::vec2(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        text,
                        egui::FontId::monospace(14.0),
                        egui::Color32::WHITE,
                    );
                }
            }
        });
    }
}
