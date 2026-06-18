//! Demo-code for showing how egui is used.
//!
//! This library can be used to test 3rd party egui integrations (see for instance <https://github.com/not-fl3/egui-miniquad/blob/master/examples/demo.rs>).
//!
//! The demo is also used in benchmarks and tests.
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![expect(clippy::unwrap_used)] // TODO(emilk): avoid unwraps

mod demo;
pub mod easy_mark;
mod rendering_test;

pub use demo::{Demo, DemoWindows, View, WidgetGallery};
pub use rendering_test::ColorTest;

/// View some Rust code with syntax highlighting and selection.
pub(crate) fn rust_view_ui(ui: &mut egui::Ui, code: &str) {
    let language = "rs";
    let theme = egui_extras::syntax_highlighting::CodeTheme::from_memory(ui.ctx(), ui.style());
    egui_extras::syntax_highlighting::code_view_ui(ui, &theme, code, language);
}

// ----------------------------------------------------------------------------

/// Create a [`Hyperlink`](egui::Hyperlink) to this egui source code file on github.
#[macro_export]
macro_rules! egui_github_link_file {
    () => {
        $crate::egui_github_link_file!("(source code)")
    };
    ($label: expr) => {
        egui::github_link_file!(
            "https://github.com/emilk/egui/blob/main/",
            egui::RichText::new($label).small()
        )
    };
}

/// Create a [`Hyperlink`](egui::Hyperlink) to this egui source code file and line on github.
#[macro_export]
macro_rules! egui_github_link_file_line {
    () => {
        $crate::egui_github_link_file_line!("(source code)")
    };
    ($label: expr) => {
        egui::github_link_file_line!(
            "https://github.com/emilk/egui/blob/main/",
            egui::RichText::new($label).small()
        )
    };
}

// ----------------------------------------------------------------------------

pub const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

pub const LOREM_IPSUM_LONG: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam various, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt sapien risus a quam. Maecenas fermentum consequat mi. Donec fermentum. Pellentesque malesuada nulla a mi. Duis sapien sem, aliquet nec, commodo eget, consequat quis, neque. Aliquam faucibus, elit ut dictum aliquet, felis nisl adipiscing sapien, sed malesuada diam lacus eget erat. Cras mollis scelerisque nunc. Nullam arcu. Aliquam consequat. Curabitur augue lorem, dapibus quis, laoreet et, pretium ac, nisi. Aenean magna nisl, mollis quis, molestie eu, feugiat in, orci. In hac habitasse platea dictumst.";

// ----------------------------------------------------------------------------

#[test]
fn test_egui_e2e() {
    let mut demo_windows = crate::DemoWindows::default();
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput::default();

    const NUM_FRAMES: usize = 5;
    for _ in 0..NUM_FRAMES {
        let full_output = ctx.run_ui(raw_input.clone(), |ui| {
            demo_windows.ui(ui);
        });
        let clipped_primitives = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        assert!(!clipped_primitives.is_empty());
    }
}

#[test]
fn test_egui_zero_window_size() {
    let mut demo_windows = crate::DemoWindows::default();
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::ZERO)),
        ..Default::default()
    };

    const NUM_FRAMES: usize = 5;
    for _ in 0..NUM_FRAMES {
        let full_output = ctx.run_ui(raw_input.clone(), |ui| {
            demo_windows.ui(ui);
        });
        let clipped_primitives = ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        assert!(
            clipped_primitives.is_empty(),
            "There should be nothing to show, has at least one primitive with clip_rect: {:?}",
            clipped_primitives[0].clip_rect
        );
    }
}

// ----------------------------------------------------------------------------

/// Detect narrow screens. This is used to show a simpler UI on mobile devices,
/// especially for the web demo at <https://egui.rs>.
pub fn is_mobile(ctx: &egui::Context) -> bool {
    let screen_size = ctx.content_rect().size();
    screen_size.x < 550.0
}
