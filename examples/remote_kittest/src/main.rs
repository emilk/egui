//! End-to-end demo of [`egui_kittest::AutomationHarness`] driving a real eframe app.
//!
//! The eframe app runs on the main thread (required on macOS). A controller
//! thread attaches a [`AutomationHarness`] to the running app, queries the
//! AccessKit tree by label, clicks a button, and asserts the counter went up
//! — all against the live, visible app rather than a test-only harness.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![expect(rustdoc::missing_crate_level_docs)]

use std::sync::Arc;
use std::time::Duration;

use eframe::{AutomationHandle, egui};
use egui_kittest::{AutomationHarness, kittest::Queryable as _};

fn main() -> eframe::Result {
    env_logger::init();

    let automation = Arc::new(AutomationHandle::new());
    let controller_handle = Arc::clone(&automation);

    let controller = std::thread::spawn(move || run_controller(controller_handle));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 200.0]),
        automation: Some(automation),
        ..Default::default()
    };
    let result = eframe::run_native(
        "AutomationHarness demo",
        options,
        Box::new(|_cc| Ok(Box::<DemoApp>::default())),
    );

    controller.join().ok();
    result
}

#[derive(Default)]
struct DemoApp {
    counter: u32,
}

impl eframe::App for DemoApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.heading("AutomationHarness demo");
            ui.label(format!("Counter: {}", self.counter));
            if ui.button("Increment").clicked() {
                self.counter += 1;
            }
        });
    }
}

fn run_controller(handle: Arc<AutomationHandle>) {
    let Ok(mut harness) = AutomationHarness::attach(handle) else {
        eprintln!("controller: attach failed: timed out");
        return;
    };
    eprintln!("controller: attached to remote app");

    // The Increment button uses AccessKit click, which works without
    // synthesizing pointer coordinates (and is robust to layout changes).
    for i in 1..=10 {
        let Some(button) = harness.query_by_label("Increment") else {
            eprintln!("controller: increment button not found");
            return;
        };
        button.click_accesskit();
        harness.run();
        std::thread::sleep(Duration::from_millis(500));
        eprintln!("controller: click #{i} delivered");
    }

    // Verify via the AccessKit tree that the label updated to "Counter: 3".
    // TODO: this will fail.
    if harness.query_by_label("Counter: 3").is_some() {
        eprintln!("controller: ✓ AccessKit reports Counter: 3");
    } else {
        eprintln!("controller: ✗ expected label not found; dumping tree:\n{harness:#?}");
    }

    std::thread::sleep(Duration::from_millis(500));
    eprintln!("controller: done, asking app to close");
    harness
        .ctx()
        .send_viewport_cmd(egui::ViewportCommand::Close);
}
