//! Demo-code for showing how Egui is used.
//!
//! The demo-code is also used in benchmarks and tests.
mod app;
mod color_test;
mod dancing_strings;
pub mod demo_window;
mod demo_windows;
mod drag_and_drop;
mod font_book;
pub mod font_contents_emoji;
pub mod font_contents_ubuntu;
mod fractal_clock;
mod painting;
mod sliders;
mod tests;
pub mod toggle_switch;
mod widgets;

pub use {
    app::*, color_test::ColorTest, dancing_strings::DancingStrings, demo_window::DemoWindow,
    demo_windows::*, drag_and_drop::*, font_book::FontBook, fractal_clock::FractalClock,
    painting::Painting, sliders::Sliders, tests::Tests, widgets::Widgets,
};

pub const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.";

pub const LOREM_IPSUM_LONG: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum.

Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam varius, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt sapien risus a quam. Maecenas fermentum consequat mi. Donec fermentum. Pellentesque malesuada nulla a mi. Duis sapien sem, aliquet nec, commodo eget, consequat quis, neque. Aliquam faucibus, elit ut dictum aliquet, felis nisl adipiscing sapien, sed malesuada diam lacus eget erat. Cras mollis scelerisque nunc. Nullam arcu. Aliquam consequat. Curabitur augue lorem, dapibus quis, laoreet et, pretium ac, nisi. Aenean magna nisl, mollis quis, molestie eu, feugiat in, orci. In hac habitasse platea dictumst.";

// ----------------------------------------------------------------------------

/// Something to view in the demo windows
pub trait View {
    fn ui(&mut self, ui: &mut crate::Ui);
}

/// Something to view
pub trait Demo {
    fn name(&self) -> &str;

    /// Show windows, etc
    fn show(&mut self, ctx: &crate::CtxRef, open: &mut bool);
}

// ----------------------------------------------------------------------------

pub fn warn_if_debug_build(ui: &mut crate::Ui) {
    if crate::has_debug_assertions() {
        ui.label(
            crate::Label::new("‼ Debug build ‼")
                .small()
                .text_color(crate::color::RED),
        )
        .on_hover_text("Egui was compiled with debug assertions enabled.");
    }
}

// ----------------------------------------------------------------------------

/// Create a [`Hyperlink`](crate::Hyperlink) to this file (and line) on Github
///
/// Example: `ui.add(github_link_file_line!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));`
#[macro_export]
macro_rules! github_link_file_line {
    ($github_url:expr, $label:expr) => {{
        let url = format!("{}{}#L{}", $github_url, file!(), line!());
        $crate::Hyperlink::new(url).text($label)
    }};
}

/// Create a [`Hyperlink`](crate::Hyperlink) to this file on github.
///
/// Example: `ui.add(github_link_file!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));`
#[macro_export]
macro_rules! github_link_file {
    ($github_url:expr, $label:expr) => {{
        let url = format!("{}{}", $github_url, file!());
        $crate::Hyperlink::new(url).text($label)
    }};
}

/// Create a [`Hyperlink`](crate::Hyperlink) to this egui source code file on github.
#[doc(hidden)]
#[macro_export]
macro_rules! __egui_github_link_file {
    () => {
        __egui_github_link_file!("(source code)")
    };
    ($label:expr) => {
        github_link_file!("https://github.com/emilk/egui/blob/master/", $label).small()
    };
}

/// Create a [`Hyperlink`](crate::Hyperlink) to this egui source code file and line on github.
#[doc(hidden)]
#[macro_export]
macro_rules! __egui_github_link_file_line {
    () => {
        __egui_github_link_file_line!("(source code)")
    };
    ($label:expr) => {
        github_link_file_line!("https://github.com/emilk/egui/blob/master/", $label).small()
    };
}
