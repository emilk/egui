//! `egui`:  an easy-to-use GUI in pure Rust!
//!
//! Try the live web demo: <https://www.egui.rs/#demo>. Read more about egui at <https://github.com/emilk/egui>.
//!
//! `egui` is in heavy development, with each new version having breaking changes.
//! You need to have rust 1.62.0 or later to use `egui`.
//!
//! To quickly get started with egui, you can take a look at [`eframe_template`](https://github.com/emilk/eframe_template)
//! which uses [`eframe`](https://docs.rs/eframe).
//!
//! To create a GUI using egui you first need a [`Context`] (by convention referred to by `ctx`).
//! Then you add a [`Window`] or a [`SidePanel`] to get a [`Ui`], which is what you'll be using to add all the buttons and labels that you need.
//!
//!
//! # Using egui
//!
//! To see what is possible to build with egui you can check out the online demo at <https://www.egui.rs/#demo>.
//!
//! If you like the "learning by doing" approach, clone <https://github.com/emilk/eframe_template> and get started using egui right away.
//!
//! ### A simple example
//!
//! Here is a simple counter that can be incremented and decremented using two buttons:
//! ```
//! fn ui_counter(ui: &mut egui::Ui, counter: &mut i32) {
//!     // Put the buttons and label on the same row:
//!     ui.horizontal(|ui| {
//!         if ui.button("-").clicked() {
//!             *counter -= 1;
//!         }
//!         ui.label(counter.to_string());
//!         if ui.button("+").clicked() {
//!             *counter += 1;
//!         }
//!     });
//! }
//! ```
//!
//! In some GUI frameworks this would require defining multiple types and functions with callbacks or message handlers,
//! but thanks to `egui` being immediate mode everything is one self-contained function!
//!
//! ### Getting a [`Ui`]
//!
//! Use one of [`SidePanel`], [`TopBottomPanel`], [`CentralPanel`], [`Window`] or [`Area`] to
//! get access to an [`Ui`] where you can put widgets. For example:
//!
//! ```
//! # egui::__run_test_ctx(|ctx| {
//! egui::CentralPanel::default().show(&ctx, |ui| {
//!     ui.add(egui::Label::new("Hello World!"));
//!     ui.label("A shorter and more convenient way to add a label.");
//!     if ui.button("Click me").clicked() {
//!         // take some action here
//!     }
//! });
//! # });
//! ```
//!
//! ### Quick start
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! # let mut my_string = String::new();
//! # let mut my_boolean = true;
//! # let mut my_f32 = 42.0;
//! ui.label("This is a label");
//! ui.hyperlink("https://github.com/emilk/egui");
//! ui.text_edit_singleline(&mut my_string);
//! if ui.button("Click me").clicked() { }
//! ui.add(egui::Slider::new(&mut my_f32, 0.0..=100.0));
//! ui.add(egui::DragValue::new(&mut my_f32));
//!
//! ui.checkbox(&mut my_boolean, "Checkbox");
//!
//! #[derive(PartialEq)]
//! enum Enum { First, Second, Third }
//! # let mut my_enum = Enum::First;
//! ui.horizontal(|ui| {
//!     ui.radio_value(&mut my_enum, Enum::First, "First");
//!     ui.radio_value(&mut my_enum, Enum::Second, "Second");
//!     ui.radio_value(&mut my_enum, Enum::Third, "Third");
//! });
//!
//! ui.separator();
//!
//! # let my_image = egui::TextureId::default();
//! ui.image(my_image, [640.0, 480.0]);
//!
//! ui.collapsing("Click to see what is hidden!", |ui| {
//!     ui.label("Not much, as it turns out");
//! });
//! # });
//! ```
//!
//! ## Conventions
//!
//! Conventions unless otherwise specified:
//!
//! * angles are in radians
//! * `Vec2::X` is right and `Vec2::Y` is down.
//! * `Pos2::ZERO` is left top.
//! * Positions and sizes are measured in _points_. Each point may consist of many physical pixels.
//!
//! # Integrating with egui
//!
//! Most likely you are using an existing `egui` backend/integration such as [`eframe`](https://docs.rs/eframe), [`bevy_egui`](https://docs.rs/bevy_egui),
//! or [`egui-miniquad`](https://github.com/not-fl3/egui-miniquad),
//! but if you want to integrate `egui` into a new game engine, this is the section for you.
//!
//! To write your own integration for egui you need to do this:
//!
//! ``` no_run
//! # fn handle_platform_output(_: egui::PlatformOutput) {}
//! # fn gather_input() -> egui::RawInput { egui::RawInput::default() }
//! # fn paint(textures_detla: egui::TexturesDelta, _: Vec<egui::ClippedPrimitive>) {}
//! let mut ctx = egui::Context::default();
//!
//! // Game loop:
//! loop {
//!     let raw_input: egui::RawInput = gather_input();
//!
//!     let full_output = ctx.run(raw_input, |ctx| {
//!         egui::CentralPanel::default().show(&ctx, |ui| {
//!             ui.label("Hello world!");
//!             if ui.button("Click me").clicked() {
//!                 // take some action here
//!             }
//!         });
//!     });
//!     handle_platform_output(full_output.platform_output);
//!     let clipped_primitives = ctx.tessellate(full_output.shapes); // create triangles to paint
//!     paint(full_output.textures_delta, clipped_primitives);
//! }
//! ```
//!
//!
//! # Understanding immediate mode
//!
//! `egui` is an immediate mode GUI library. It is useful to fully grok what "immediate mode" implies.
//!
//! Here is an example to illustrate it:
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! if ui.button("click me").clicked() {
//!     take_action()
//! }
//! # });
//! # fn take_action() {}
//! ```
//!
//! This code is being executed each frame at maybe 60 frames per second.
//! Each frame egui does these things:
//!
//! * lays out the letters `click me` in order to figure out the size of the button
//! * decides where on screen to place the button
//! * check if the mouse is hovering or clicking that location
//! * chose button colors based on if it is being hovered or clicked
//! * add a [`Shape::Rect`] and [`Shape::Text`] to the list of shapes to be painted later this frame
//! * return a [`Response`] with the [`clicked`](`Response::clicked`) member so the user can check for interactions
//!
//! There is no button being created and stored somewhere.
//! The only output of this call is some colored shapes, and a [`Response`].
//!
//! Similarly, consider this code:
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! # let mut value: f32 = 0.0;
//! ui.add(egui::Slider::new(&mut value, 0.0..=100.0).text("My value"));
//! # });
//! ```
//!
//! Here egui will read `value` to display the slider, then look if the mouse is dragging the slider and if so change the `value`.
//! Note that `egui` does not store the slider value for you - it only displays the current value, and changes it
//! by how much the slider has been dragged in the previous few milliseconds.
//! This means it is responsibility of the egui user to store the state (`value`) so that it persists between frames.
//!
//! It can be useful to read the code for the toggle switch example widget to get a better understanding
//! of how egui works: <https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/toggle_switch.rs>.
//!
//! Read more about the pros and cons of immediate mode at <https://github.com/emilk/egui#why-immediate-mode>.
//!
//! # Misc
//!
//! ## How widgets works
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! if ui.button("click me").clicked() { take_action() }
//! # });
//! # fn take_action() {}
//! ```
//!
//! is short for
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! let button = egui::Button::new("click me");
//! if ui.add(button).clicked() { take_action() }
//! # });
//! # fn take_action() {}
//! ```
//!
//! which is short for
//!
//! ```
//! # use egui::Widget;
//! # egui::__run_test_ui(|ui| {
//! let button = egui::Button::new("click me");
//! let response = button.ui(ui);
//! if response.clicked() { take_action() }
//! # });
//! # fn take_action() {}
//! ```
//!
//! [`Button`] uses the builder pattern to create the data required to show it. The [`Button`] is then discarded.
//!
//! [`Button`] implements `trait` [`Widget`], which looks like this:
//! ```
//! # use egui::*;
//! pub trait Widget {
//!     /// Allocate space, interact, paint, and return a [`Response`].
//!     fn ui(self, ui: &mut Ui) -> Response;
//! }
//! ```
//!
//! ## Auto-sizing panels and windows
//! In egui, all panels and windows auto-shrink to fit the content.
//! If the window or panel is also resizable, this can lead to a weird behavior
//! where you can drag the edge of the panel/window to make it larger, and
//! when you release the panel/window shrinks again.
//! This is an artifact of immediate mode, and here are some alternatives on how to avoid it:
//!
//! 1. Turn off resizing with [`Window::resizable`], [`SidePanel::resizable`], [`TopBottomPanel::resizable`].
//! 2. Wrap your panel contents in a [`ScrollArea`], or use [`Window::vscroll`] and [`Window::hscroll`].
//! 3. Use a justified layout:
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! ui.with_layout(egui::Layout::top_down_justified(egui::Align::Center), |ui| {
//!     ui.button("I am becoming wider as needed");
//! });
//! # });
//! ```
//!
//! 4. Fill in extra space with emptiness:
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! ui.allocate_space(ui.available_size()); // put this LAST in your panel/window code
//! # });
//! ```
//!
//! ## Sizes
//! You can control the size of widgets using [`Ui::add_sized`].
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! # let mut my_value = 0.0_f32;
//! ui.add_sized([40.0, 20.0], egui::DragValue::new(&mut my_value));
//! # });
//! ```
//!
//! ## Code snippets
//!
//! ```
//! # egui::__run_test_ui(|ui| {
//! # let mut some_bool = true;
//! // Miscellaneous tips and tricks
//!
//! ui.horizontal_wrapped(|ui| {
//!     ui.spacing_mut().item_spacing.x = 0.0; // remove spacing between widgets
//!     // `radio_value` also works for enums, integers, and more.
//!     ui.radio_value(&mut some_bool, false, "Off");
//!     ui.radio_value(&mut some_bool, true, "On");
//! });
//!
//! ui.group(|ui| {
//!     ui.label("Within a frame");
//!     ui.set_min_height(200.0);
//! });
//!
//! // A `scope` creates a temporary [`Ui`] in which you can change settings:
//! ui.scope(|ui| {
//!     ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
//!     ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
//!     ui.style_mut().wrap = Some(false);
//!
//!     ui.label("This text will be red, monospace, and won't wrap to a new line");
//! }); // the temporary settings are reverted here
//! # });
//! ```
//!
//! ## Feature flags
#![cfg_attr(feature = "document-features", doc = document_features::document_features!())]
//!

#![allow(clippy::float_cmp)]
#![allow(clippy::manual_range_contains)]

mod animation_manager;
pub mod containers;
mod context;
mod data;
mod frame_state;
pub(crate) mod grid;
pub mod gui_zoom;
mod id;
mod input_state;
pub mod introspection;
pub mod layers;
mod layout;
mod memory;
pub mod menu;
pub mod os;
mod painter;
pub(crate) mod placer;
mod response;
mod sense;
pub mod style;
mod ui;
pub mod util;
pub mod widget_text;
pub mod widgets;

#[cfg(feature = "accesskit")]
pub use accesskit;

pub use epaint;
pub use epaint::ecolor;
pub use epaint::emath;

#[cfg(feature = "color-hex")]
pub use ecolor::hex_color;
pub use ecolor::{Color32, Rgba};
pub use emath::{lerp, pos2, remap, remap_clamp, vec2, Align, Align2, NumExt, Pos2, Rect, Vec2};
pub use epaint::{
    mutex,
    text::{FontData, FontDefinitions, FontFamily, FontId, FontTweak},
    textures::{TextureFilter, TextureOptions, TexturesDelta},
    ClippedPrimitive, ColorImage, FontImage, ImageData, Mesh, PaintCallback, PaintCallbackInfo,
    Rounding, Shape, Stroke, TextureHandle, TextureId,
};

pub mod text {
    pub use crate::text_edit::CCursorRange;
    pub use epaint::text::{
        cursor::CCursor, FontData, FontDefinitions, FontFamily, Fonts, Galley, LayoutJob,
        LayoutSection, TextFormat, TAB_SIZE,
    };
}

pub use {
    containers::*,
    context::Context,
    data::{
        input::*,
        output::{self, CursorIcon, FullOutput, PlatformOutput, WidgetInfo},
    },
    grid::Grid,
    id::{Id, IdMap},
    input_state::{InputState, MultiTouchInfo, PointerState},
    layers::{LayerId, Order},
    layout::*,
    memory::Memory,
    painter::Painter,
    response::{InnerResponse, Response},
    sense::Sense,
    style::{FontSelection, Style, TextStyle, Visuals},
    text::{Galley, TextFormat},
    ui::Ui,
    widget_text::{RichText, WidgetText},
    widgets::*,
};

// ----------------------------------------------------------------------------

/// Helper function that adds a label when compiling with debug assertions enabled.
pub fn warn_if_debug_build(ui: &mut crate::Ui) {
    if cfg!(debug_assertions) {
        ui.label(
            RichText::new("⚠ Debug build ⚠")
                .small()
                .color(ui.visuals().warn_fg_color),
        )
        .on_hover_text("egui was compiled with debug assertions enabled.");
    }
}

// ----------------------------------------------------------------------------

/// Create a [`Hyperlink`](crate::Hyperlink) to the current [`file!()`] (and line) on Github
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.add(egui::github_link_file_line!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));
/// # });
/// ```
#[macro_export]
macro_rules! github_link_file_line {
    ($github_url: expr, $label: expr) => {{
        let url = format!("{}{}#L{}", $github_url, file!(), line!());
        $crate::Hyperlink::from_label_and_url($label, url)
    }};
}

/// Create a [`Hyperlink`](crate::Hyperlink) to the current [`file!()`] on github.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// ui.add(egui::github_link_file!("https://github.com/YOUR/PROJECT/blob/master/", "(source code)"));
/// # });
/// ```
#[macro_export]
macro_rules! github_link_file {
    ($github_url: expr, $label: expr) => {{
        let url = format!("{}{}", $github_url, file!());
        $crate::Hyperlink::from_label_and_url($label, url)
    }};
}

// ----------------------------------------------------------------------------

/// Show debug info on hover when [`Context::set_debug_on_hover`] has been turned on.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// // Turn on tracing of widgets
/// ui.ctx().set_debug_on_hover(true);
///
/// /// Show [`std::file`], [`std::line`] and argument on hover
/// egui::trace!(ui, "MyWindow");
///
/// /// Show [`std::file`] and [`std::line`] on hover
/// egui::trace!(ui);
/// # });
/// ```
#[macro_export]
macro_rules! trace {
    ($ui: expr) => {{
        $ui.trace_location(format!("{}:{}", file!(), line!()))
    }};
    ($ui: expr, $label: expr) => {{
        $ui.trace_location(format!("{} - {}:{}", $label, file!(), line!()))
    }};
}

// ----------------------------------------------------------------------------

/// An assert that is only active when `egui` is compiled with the `extra_asserts` feature
/// or with the `extra_debug_asserts` feature in debug builds.
#[macro_export]
macro_rules! egui_assert {
    ($($arg: tt)*) => {
        if cfg!(any(
            feature = "extra_asserts",
            all(feature = "extra_debug_asserts", debug_assertions),
        )) {
            assert!($($arg)*);
        }
    }
}

// ----------------------------------------------------------------------------

/// The default egui fonts supports around 1216 emojis in total.
/// Here are some of the most useful:
/// ∞⊗⎗⎘⎙⏏⏴⏵⏶⏷
/// ⏩⏪⏭⏮⏸⏹⏺■▶📾🔀🔁🔃
/// ☀☁★☆☐☑☜☝☞☟⛃⛶✔
/// ↺↻⟲⟳⬅➡⬆⬇⬈⬉⬊⬋⬌⬍⮨⮩⮪⮫
/// ♡
/// 📅📆
/// 📈📉📊
/// 📋📌📎📤📥🔆
/// 🔈🔉🔊🔍🔎🔗🔘
/// 🕓🖧🖩🖮🖱🖴🖵🖼🗀🗁🗋🗐🗑🗙🚫❓
///
/// NOTE: In egui all emojis are monochrome!
///
/// You can explore them all in the Font Book in [the online demo](https://www.egui.rs/#demo).
///
/// In addition, egui supports a few special emojis that are not part of the unicode standard.
/// This module contains some of them:
pub mod special_emojis {
    /// Tux, the Linux penguin.
    pub const OS_LINUX: char = '🐧';

    /// The Windows logo.
    pub const OS_WINDOWS: char = '';

    /// The Android logo.
    pub const OS_ANDROID: char = '';

    /// The Apple logo.
    pub const OS_APPLE: char = '';

    /// The Github logo.
    pub const GITHUB: char = '';

    /// The Twitter bird.
    pub const TWITTER: char = '';

    /// The word `git`.
    pub const GIT: char = '';

    // I really would like to have ferris here.
}

/// The different types of built-in widgets in egui
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum WidgetType {
    Label, // TODO(emilk): emit Label events
    /// e.g. a hyperlink
    Link,
    TextEdit,
    Button,
    Checkbox,
    RadioButton,
    SelectableLabel,
    ComboBox,
    Slider,
    DragValue,
    ColorButton,
    ImageButton,
    CollapsingHeader,

    /// If you cannot fit any of the above slots.
    ///
    /// If this is something you think should be added, file an issue.
    Other,
}

// ----------------------------------------------------------------------------

/// For use in tests; especially doctests.
pub fn __run_test_ctx(mut run_ui: impl FnMut(&Context)) {
    let ctx = Context::default();
    ctx.set_fonts(FontDefinitions::empty()); // prevent fonts from being loaded (save CPU time)
    let _ = ctx.run(Default::default(), |ctx| {
        run_ui(ctx);
    });
}

/// For use in tests; especially doctests.
pub fn __run_test_ui(mut add_contents: impl FnMut(&mut Ui)) {
    let ctx = Context::default();
    ctx.set_fonts(FontDefinitions::empty()); // prevent fonts from being loaded (save CPU time)
    let _ = ctx.run(Default::default(), |ctx| {
        crate::CentralPanel::default().show(ctx, |ui| {
            add_contents(ui);
        });
    });
}

#[cfg(feature = "accesskit")]
pub fn accesskit_root_id() -> Id {
    Id::new("accesskit_root")
}

/// Return a tree update that the egui integration should provide to the
/// AccessKit adapter if it cannot immediately run the egui application
/// to get a full tree update after running [`Context::enable_accesskit`].
#[cfg(feature = "accesskit")]
pub fn accesskit_placeholder_tree_update() -> accesskit::TreeUpdate {
    use accesskit::{Node, Role, Tree, TreeUpdate};
    use std::sync::Arc;

    let root_id = accesskit_root_id().accesskit_id();
    TreeUpdate {
        nodes: vec![(
            root_id,
            Arc::new(Node {
                role: Role::Window,
                ..Default::default()
            }),
        )],
        tree: Some(Tree::new(root_id)),
        focus: None,
    }
}
