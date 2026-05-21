//! Headless `egui_demo_lib` target for the kittest MCP server.
//!
//! Build a [`Harness`] around [`DemoWindows`] and step forever. The
//! [`egui_kittest::InspectorPlugin`] auto-attaches whenever `KITTEST_INSPECTOR` is set
//! (the MCP server's `launch` tool sets it), drives the harness via stdio, and blocks
//! inside `after_step` until the agent requests the next frame.

#![expect(rustdoc::missing_crate_level_docs)]

use egui::Vec2;
use egui_demo_lib::DemoWindows;
use egui_kittest::Harness;

fn main() {
    let mut demo = DemoWindows::default();

    let mut harness = Harness::builder()
        .with_size(Vec2::new(1024.0, 768.0))
        .wgpu()
        .build_ui(move |ui| {
            demo.ui(ui);
        });

    loop {
        harness.step();
    }
}
