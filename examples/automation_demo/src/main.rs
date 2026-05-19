//! Minimal end-to-end demo of [`eframe::AutomationHandle`].
//!
//! Runs a tiny eframe app on the main thread and a controller on a spawned
//! thread. The controller injects synthetic `Space` key presses into the
//! running app and prints how many AccessKit nodes the app exposes each
//! frame. After a few seconds it requests the app to close.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![expect(rustdoc::missing_crate_level_docs)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::{AutomationHandle, egui};

fn main() -> eframe::Result {
    env_logger::init();

    let automation = Arc::new(AutomationHandle::new());
    let controller_handle = Arc::clone(&automation);

    // The controller drives the app from outside the winit event loop.
    let controller = std::thread::spawn(move || run_controller(controller_handle));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 200.0]),
        automation: Some(automation),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Automation demo",
        options,
        Box::new(|_cc| Ok(Box::<DemoApp>::default())),
    );

    // Make sure we wait for the controller to finish even on error so its log
    // lines don't get cut off.
    controller.join().ok();
    result
}

#[derive(Default)]
struct DemoApp {
    counter: u32,
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Increment whenever Space is pressed — including events the
        // controller pushed via AutomationHandle.
        if ui.input(|i| i.key_pressed(egui::Key::Space)) {
            self.counter += 1;
        }
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("Automation demo");
            ui.label(format!("Space presses received: {}", self.counter));
            ui.label("Press Space (or wait for the controller).");
        });
    }
}

fn run_controller(handle: Arc<AutomationHandle>) {
    let Some(ctx) = handle.wait_for_ctx(Duration::from_secs(5)) else {
        eprintln!("controller: timed out waiting for app to start");
        return;
    };
    eprintln!("controller: app started; ctx attached");

    // Drain the initial tree update(s).
    if let Some(updates) = handle.wait_for_tree_update(Duration::from_secs(2)) {
        let total_nodes: usize = updates.iter().map(|u| u.nodes.len()).sum();
        eprintln!(
            "controller: first {} tree update(s) carried {} node(s)",
            updates.len(),
            total_nodes
        );
    }

    let started = Instant::now();
    let mut presses = 0;
    while started.elapsed() < Duration::from_secs(20) {
        std::thread::sleep(Duration::from_millis(700));

        // Push a synthetic Space press + release.
        handle.push_events([
            egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
            egui::Event::Key {
                key: egui::Key::Space,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        presses += 1;

        // Wait until the app paints the frame that consumed our events.
        if let Some(updates) = handle.wait_for_tree_update(Duration::from_secs(1)) {
            let nodes: usize = updates.iter().map(|u| u.nodes.len()).sum();
            eprintln!(
                "controller: press #{presses} delivered, observed {} node update(s)",
                nodes
            );
        }
    }

    eprintln!("controller: done, asking app to close");
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}
