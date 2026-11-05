//! Regression tests for shortcut text following the interface's reading
//! direction: [`egui::Context::format_shortcut`] mirrors the display names of
//! the horizontal arrow keys ([`egui::Key::name_for`] /
//! [`egui::Key::symbol_or_name_for`]) so a "toward the reading start" action is
//! labeled `Right` in a right-to-left interface, while the bidi isolates from
//! the shortcut formatting wrap the mirrored result.

use egui::{Context, Key, KeyboardShortcut, Modifiers};

#[test]
fn arrow_shortcut_names_follow_the_interface_direction() {
    let ctx = Context::default();
    let left = KeyboardShortcut::new(Modifiers::NONE, Key::ArrowLeft);

    let ltr = ctx.format_shortcut(&left);
    assert!(ltr.contains("Left"), "LTR should say Left, got {ltr:?}");

    ctx.set_right_to_left(true);
    let rtl = ctx.format_shortcut(&left);
    assert!(rtl.contains("Right"), "RTL should say Right, got {rtl:?}");
    assert!(!rtl.contains("Left"), "RTL must not say Left, got {rtl:?}");

    // The mirrored twin: under RTL, ArrowRight reads "toward the reading end".
    let right = KeyboardShortcut::new(Modifiers::NONE, Key::ArrowRight);
    let mirrored = ctx.format_shortcut(&right);
    assert!(
        mirrored.contains("Left"),
        "RTL ArrowRight should be mirrored to Left, got {mirrored:?}"
    );
}
