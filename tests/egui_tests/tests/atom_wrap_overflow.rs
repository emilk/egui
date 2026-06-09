//! Repro for a wrapping bug: a `grow`, wrapping nested [`AtomLayout`] placed next to a fixed-width
//! sibling (think: the atom-layout demo's tag-filter sidebar + card gallery) wraps "too late" and
//! overflows the container.
//!
//! The layout under test is a non-wrapping row of `[ fixed sidebar · grow wrapping gallery ]`,
//! forced to the container width via `min_size`. We render it at a range of shrinking container
//! widths and assert the laid-out width never exceeds the container.

use egui::{Atom, AtomExt as _, AtomLayout, Atoms, CornerRadius, Frame, Margin, Stroke, Vec2};
use egui_kittest::Harness;
use std::cell::Cell;

const SIDEBAR_WIDTH: f32 = 120.0;
const GAP: f32 = 12.0;

fn chip_frame() -> Frame {
    Frame::new()
        .inner_margin(Margin::symmetric(6, 2))
        .corner_radius(CornerRadius::same(4))
        .stroke(Stroke::new(1.0, egui::Color32::GRAY))
}

/// A single grow chip, like a tag button in the demo.
fn chip(word: &str) -> Atom<'static> {
    Atom::layout(AtomLayout::new(word.to_owned()).frame(chip_frame())).atom_grow(true)
}

/// `[ fixed sidebar · grow wrapping gallery of chips ]`, forced to `width` via `min_size`.
fn root(width: f32) -> AtomLayout<'static> {
    let words = [
        "aurora",
        "night",
        "long-exposure",
        "iceland",
        "winter",
        "jungle",
        "macro",
        "wildlife",
        "humid",
        "sahara",
        "golden-hour",
        "minimal",
    ];
    let mut chips = Atoms::default();
    for word in words {
        chips.push_right(chip(word));
    }
    let gallery = Atom::layout(AtomLayout::new(chips).wrap(true).gap(6.0)).atom_grow(true);

    let sidebar =
        Atom::layout(AtomLayout::new("sidebar").frame(chip_frame())).atom_max_width(SIDEBAR_WIDTH);

    AtomLayout::new((sidebar, gallery))
        .gap(GAP)
        .min_size(Vec2::new(width, 0.0))
}

/// Render `root` at the given container width and return the laid-out (allocated) width.
fn laid_out_width(width: f32) -> f32 {
    let measured = Cell::new(0.0_f32);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, 600.0))
        .build_ui(|ui| {
            let avail = ui.available_width();
            let response = root(avail).show(ui);
            measured.set(response.response.rect.width());
        });
    harness.run();
    measured.get()
}

#[test]
fn atom_wrap_no_overflow_when_shrinking() {
    let mut failures = Vec::new();

    // Shrink the container step by step.
    for width in (260..=820).rev().step_by(40) {
        let w = width as f32;
        let laid_out = laid_out_width(w);
        let overflow = laid_out - w;
        println!("container {w:6.1} -> laid out {laid_out:7.1} (overflow {overflow:+.1})");
        // Allow a pixel of rounding slack.
        if overflow > 1.0 {
            failures.push(format!(
                "container {w:.1}: laid out {laid_out:.1} (overflow {overflow:.1}px)"
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "the wrapping layout overflowed its container:\n{}",
        failures.join("\n")
    );
}
