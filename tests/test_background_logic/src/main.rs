#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![expect(rustdoc::missing_crate_level_docs)]
#![allow(clippy::print_stderr)]

use std::time::Duration;

use eframe::egui::{self, ViewportInfo};

fn main() {
    env_logger::init();

    let _ = eframe::run_native(
        "Background Logic Test",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 200.0]),
            ..Default::default()
        },
        Box::new(|_cc| Ok(Box::new(App))),
    );
}

struct App;

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        eprintln!("App::logic called {}", viewport_info(ctx));
        ctx.request_repaint_after(Duration::from_secs(1));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        eprintln!("App::ui called {}", viewport_info(ui.ctx()));
        ui.centered_and_justified(|ui| {
            ui.heading("Minimize this window");
        });
    }
}

fn viewport_info(ctx: &egui::Context) -> String {
    ctx.input(|i| {
        let ViewportInfo {
            minimized,
            focused,
            occluded,
            ..
        } = i.viewport();

        let visible = i.viewport().visible();

        let mut s = String::new();

        let flags = [
            ("focused", focused),
            ("occluded", occluded),
            ("minimized", minimized),
            ("visible", &visible),
        ];
        for (name, value) in flags {
            if let Some(value) = value {
                use std::fmt::Write as _;
                write!(s, " {name}={value}").ok();
            }
        }
        s
    })
}
