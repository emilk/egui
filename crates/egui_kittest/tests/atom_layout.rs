#![cfg(feature = "snapshot")]
#![cfg(feature = "wgpu")]

use egui::widget_style::{Classes, WidgetState};
use egui::{Atom, AtomExt, AtomLayout, Direction, Frame};

/// A root [`AtomLayout`] (in a [`Frame::canvas`]) that stacks several nested [`AtomLayout`]s
/// (each in an inactive button frame), one per [`Direction`], to exercise nesting and the
/// direction setting together.
#[test]
fn atom_layout_nesting_and_direction() {
    let mut harness = egui_kittest::Harness::new_ui(|ui| {
        let style = ui.style().clone();

        // The frame of an inactive button.
        let button_frame = style
            .button_style(&Classes::default(), WidgetState::Inactive)
            .frame;

        // A nested layout laid out along `direction`, labelled to read in that direction.
        let row = |direction: Direction, atoms: [&'static str; 3]| {
            Atom::layout(
                AtomLayout::new((atoms[0], atoms[1], atoms[2]))
                    .direction(direction)
                    .frame(button_frame.clone()),
            )
        };

        // Each axis pair gets the same label order, so the reversed direction visibly flips it
        // (e.g. `RightToLeft` reads "right to left").
        AtomLayout::new((
            // The two horizontal rows stacked into their own `TopDown` layout.
            Atom::layout(
                AtomLayout::new((
                    row(Direction::LeftToRight, ["left", "to", "right"]).atom_grow(true),
                    row(Direction::RightToLeft, ["left", "to", "right"]).atom_grow(true),
                ))
                .direction(Direction::TopDown),
            ).atom_grow(true),
            row(Direction::TopDown, ["top", "to", "bottom"]),
            row(Direction::BottomUp, ["top", "to", "bottom"]),
        ))
        .direction(Direction::LeftToRight)
        .frame(Frame::canvas(&style))
        .show(ui);
    });

    harness.fit_contents();

    #[cfg(all(feature = "snapshot", feature = "wgpu"))]
    harness.snapshot("atom_layout_nesting");
}
