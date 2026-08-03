//! Tests for how much room a [`Panel`] reserves for its separator line.
//!
//! The always-visible separator line is painted on top of the panel's inner edge, so the panel
//! insets its contents by the line's thickness. A panel that opted out of the separator line must
//! not reserve that room, or it ends up with a permanently visible gap along that edge — even
//! though it is `resizable` and therefore still shows a line while hovered or dragged.

use std::cell::Cell;

use egui::{Frame, Panel, Rect, Vec2};
use egui_kittest::Harness;

#[derive(Clone, Copy, Debug)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

const ALL_SIDES: [Side; 4] = [Side::Left, Side::Right, Side::Top, Side::Bottom];

impl Side {
    fn panel(self, id: &'static str) -> Panel {
        match self {
            Self::Left => Panel::left(id),
            Self::Right => Panel::right(id),
            Self::Top => Panel::top(id),
            Self::Bottom => Panel::bottom(id),
        }
    }

    /// How far the content rect is inset from the outer rect along the panel's inner edge,
    /// i.e. the edge facing the rest of the ui, where the separator line is painted.
    fn inner_edge_inset(self, outer: Rect, content: Rect) -> f32 {
        match self {
            Self::Left => outer.right() - content.right(),
            Self::Right => content.left() - outer.left(),
            Self::Top => outer.bottom() - content.bottom(),
            Self::Bottom => content.top() - outer.top(),
        }
    }
}

struct PanelGeometry {
    outer: Rect,
    content: Rect,
    separator_width: f32,
}

/// Show a resizable, zero-margin panel on `side` and report its geometry.
///
/// [`Frame::NONE`] keeps the frame from contributing any margin or stroke of its own, so the only
/// difference between the content rect and the outer rect is what the panel reserves itself.
fn panel_geometry(side: Side, show_separator_line: bool) -> PanelGeometry {
    let outer = Cell::new(Rect::NOTHING);
    let content = Cell::new(Rect::NOTHING);
    let separator_width = Cell::new(0.0);

    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui(|ui| {
            separator_width.set(ui.visuals().widgets.noninteractive.bg_stroke.width.round());

            let response = side
                .panel("test_panel")
                .frame(Frame::NONE)
                .resizable(true)
                .show_separator_line(show_separator_line)
                .exact_size(60.0)
                .show(ui, |ui| {
                    content.set(ui.max_rect());
                })
                .response;
            outer.set(response.rect);

            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("Central");
            });
        });
    harness.run();

    PanelGeometry {
        outer: outer.get(),
        content: content.get(),
        separator_width: separator_width.get(),
    }
}

#[test]
fn visible_separator_line_reserves_its_own_thickness() {
    for side in ALL_SIDES {
        let PanelGeometry {
            outer,
            content,
            separator_width,
        } = panel_geometry(side, true);

        assert!(
            0.0 < separator_width,
            "test is meaningless with a zero-width separator line"
        );
        assert_eq!(
            side.inner_edge_inset(outer, content),
            separator_width,
            "{side:?} panel should inset its contents by the separator line's thickness"
        );
    }
}

/// Regression test: `show_separator_line(false)` must not leave a gap, not even on a
/// `resizable` panel (which still shows a line while hovered or dragged).
#[test]
fn hidden_separator_line_reserves_nothing() {
    for side in ALL_SIDES {
        let PanelGeometry {
            outer,
            content,
            separator_width: _,
        } = panel_geometry(side, false);

        assert_eq!(
            side.inner_edge_inset(outer, content),
            0.0,
            "{side:?} panel opted out of the separator line, so it should not reserve room for it"
        );
    }
}
