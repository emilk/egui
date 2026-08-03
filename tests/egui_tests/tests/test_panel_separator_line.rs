//! Snapshot tests for where a [`Panel`] puts its separator line, and how much room it reserves.
//!
//! Going outwards from the panel contents, the order is:
//!
//! contents | `Frame::inner_margin` | `Frame::stroke` | separator line | `Frame::outer_margin`
//!
//! i.e. the line is painted _outside_ the frame's outline, in room the panel reserves for it in the
//! frame's outer margin. A panel that opted out of the separator line must not reserve that room,
//! or it ends up with a permanently visible gap along that edge — even though it is `resizable` and
//! therefore still shows a line while hovered or dragged.
//!
//! The snapshots span `show_separator_line` on/off × resize handle hovered/not. The panel uses a
//! garish frame outline and separator colors so both are unmistakable, and its only content is a
//! [`egui::SelectableLabel`] vertically centered in the panel: if the panel reserves room it
//! shouldn't, the label drifts off center.

use egui::{Color32, CornerRadius, Frame, Margin, Panel, Pos2, Stroke, Vec2};
use egui_kittest::{Harness, SnapshotResults};

/// [`Frame::fill`] of the test panel.
const FILL: Color32 = Color32::from_rgb(20, 20, 40);

/// [`Frame::stroke`] color of the test panel.
const OUTLINE: Color32 = Color32::from_rgb(255, 0, 255);

/// The dim, always-visible separator line (`noninteractive.bg_stroke`).
const SEPARATOR: Color32 = Color32::from_rgb(0, 255, 0);

/// The bright separator line shown while the resize handle is hovered (`hovered.fg_stroke`).
const HOVERED_SEPARATOR: Color32 = Color32::from_rgb(255, 255, 0);

const PANEL_ID: &str = "test_panel";

fn build_harness(show_separator_line: bool) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 120.0))
        // So the thin lines are legible to a human reviewing the snapshots:
        .with_pixels_per_point(2.0)
        .build_ui(move |ui| {
            // Loud, distinguishable colors, so we can tell the separator line, the frame outline
            // and the frame fill apart.
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.noninteractive.bg_stroke = Stroke::new(1.0, SEPARATOR);
            widgets.hovered.fg_stroke = Stroke::new(1.0, HOVERED_SEPARATOR);

            let frame = Frame::new()
                .fill(FILL)
                .stroke(Stroke::new(2.0, OUTLINE))
                .corner_radius(CornerRadius::ZERO)
                .inner_margin(Margin::same(4))
                .outer_margin(Margin::same(2));

            Panel::top(PANEL_ID)
                .frame(frame)
                .resizable(true)
                .default_size(60.0)
                .show_separator_line(show_separator_line)
                .show(ui, |ui| {
                    // Vertically centered in whatever room the panel gave us.
                    ui.horizontal_centered(|ui| {
                        let _ = ui.selectable_label(true, "Centered");
                    });
                });

            egui::CentralPanel::default()
                .frame(Frame::default().fill(Color32::GRAY))
                .show(ui, |ui| {
                    ui.label("CentralPanel");
                });
        });
    harness.run();
    harness
}

fn hover_resize_handle(harness: &mut Harness<'_>) {
    let outer = egui::PanelState::load(&harness.ctx, egui::Id::new(PANEL_ID))
        .expect("PanelState should be persisted after the first frame")
        .outer_rect;

    // Hover just _inside_ the panel's inner edge, but still well within the resize grab radius:
    // the `CentralPanel` and its label start exactly at that edge, and would otherwise take the
    // hover from the resize handle.
    harness.hover_at(Pos2::new(outer.center().x, outer.bottom() - 1.0));
    harness.run();
}

#[test]
fn separator_line_matrix() {
    let mut results = SnapshotResults::new();

    for show_separator_line in [false, true] {
        let suffix = if show_separator_line { "on" } else { "off" };

        // Not hovered: the line is dim (`show_separator_line`) or absent.
        let mut harness = build_harness(show_separator_line);
        results.add(harness.try_snapshot(format!("panel_separator_line/separator_{suffix}_idle")));

        // Hovered: a `resizable` panel shows a bright line regardless of `show_separator_line`,
        // and must not shift its contents to make room for it.
        let mut harness = build_harness(show_separator_line);
        hover_resize_handle(&mut harness);
        results
            .add(harness.try_snapshot(format!("panel_separator_line/separator_{suffix}_hovered")));
    }
}
