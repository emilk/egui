//! Tests for where a [`Panel`] puts its separator line, and how much room it reserves for it.
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
//! Both tests span the matrix of `show_separator_line` on/off × resize handle hovered/not:
//! * [`separator_line_matrix`] takes snapshots of a top panel.
//! * [`separator_line_sits_between_the_outline_and_the_outer_margin`] probes the rendered pixels
//!   along the inner edge of a panel on each of the four sides.
//!
//! The panel uses a garish frame outline and separator colors so both are unmistakable, and its
//! only content is a [`egui::SelectableLabel`] centered along the panel's axis: if the panel
//! reserves room it shouldn't, the label drifts off center.

use egui::{Color32, CornerRadius, Frame, Margin, Panel, Pos2, Rect, Stroke, Vec2};
use egui_kittest::{Harness, SnapshotResults};

/// [`Frame::fill`] of the test panel.
const FILL: Color32 = Color32::from_rgb(20, 20, 40);

/// [`Frame::stroke`] color of the test panel.
const OUTLINE: Color32 = Color32::from_rgb(255, 0, 255);

/// The dim, always-visible separator line (`noninteractive.bg_stroke`).
const SEPARATOR: Color32 = Color32::from_rgb(0, 255, 0);

/// The bright separator line shown while the resize handle is hovered (`hovered.fg_stroke`).
const HOVERED_SEPARATOR: Color32 = Color32::from_rgb(255, 255, 0);

const OUTLINE_WIDTH: i8 = 2;
const INNER_MARGIN: i8 = 8;
const OUTER_MARGIN: i8 = 6;

/// Width of both separator strokes, and therefore of the room the panel reserves for the line.
const SEPARATOR_WIDTH: i8 = 1;

const PANEL_ID: &str = "test_panel";

#[derive(Clone, Copy, Debug)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

const ALL_SIDES: [Side; 4] = [Side::Left, Side::Right, Side::Top, Side::Bottom];

impl Side {
    fn panel(self) -> Panel {
        match self {
            Self::Left => Panel::left(PANEL_ID),
            Self::Right => Panel::right(PANEL_ID),
            Self::Top => Panel::top(PANEL_ID),
            Self::Bottom => Panel::bottom(PANEL_ID),
        }
    }

    /// Index into [`Pos2`]/[`Vec2`] of the axis the panel grows along.
    fn axis(self) -> usize {
        match self {
            Self::Left | Self::Right => 0,
            Self::Top | Self::Bottom => 1,
        }
    }

    /// Direction, along [`Self::axis`], from inside the panel towards the rest of the ui.
    fn outward(self) -> f32 {
        match self {
            Self::Left | Self::Top => 1.0,
            Self::Right | Self::Bottom => -1.0,
        }
    }

    /// Coordinate, along [`Self::axis`], of the panel's inner (resizable) edge.
    fn resize_pos(self, rect: Rect) -> f32 {
        match self {
            Self::Left => rect.right(),
            Self::Right => rect.left(),
            Self::Top => rect.bottom(),
            Self::Bottom => rect.top(),
        }
    }
}

/// `pixels_per_point`: `2.0` for the snapshots (so the thin lines are legible to a human),
/// `1.0` for the pixel-probing test (so one point is one pixel).
fn build_harness(side: Side, show_separator_line: bool, pixels_per_point: f32) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 120.0))
        .with_pixels_per_point(pixels_per_point)
        .build_ui(move |ui| {
            // Loud, distinguishable colors, so we can tell the separator line, the frame outline
            // and the frame fill apart pixel by pixel.
            let widgets = &mut ui.visuals_mut().widgets;
            widgets.noninteractive.bg_stroke = Stroke::new(f32::from(SEPARATOR_WIDTH), SEPARATOR);
            widgets.hovered.fg_stroke = Stroke::new(f32::from(SEPARATOR_WIDTH), HOVERED_SEPARATOR);

            let frame = Frame::new()
                .fill(FILL)
                .stroke(Stroke::new(f32::from(OUTLINE_WIDTH), OUTLINE))
                .corner_radius(CornerRadius::ZERO)
                .inner_margin(Margin::same(INNER_MARGIN))
                .outer_margin(Margin::same(OUTER_MARGIN));

            side.panel()
                .frame(frame)
                .resizable(true)
                .default_size(60.0)
                .show_separator_line(show_separator_line)
                .show(ui, |ui| {
                    // Centered along the panel's axis, in whatever room the panel gave us.
                    if side.axis() == 0 {
                        ui.vertical_centered(|ui| {
                            let _ = ui.selectable_label(true, "Centered");
                        });
                    } else {
                        ui.horizontal_centered(|ui| {
                            let _ = ui.selectable_label(true, "Centered");
                        });
                    }
                });

            egui::CentralPanel::default().show(ui, |ui| {
                ui.label("Central");
            });
        });
    harness.run();
    harness
}

/// The panel's outer rect, i.e. everything it allocated, including both frame margins.
fn outer_rect(harness: &Harness<'_>) -> Rect {
    let panel_state = egui::PanelState::load(&harness.ctx, egui::Id::new(PANEL_ID))
        .expect("PanelState should be persisted after the first frame");
    panel_state.outer_rect
}

/// A point on the panel's resize handle, i.e. its inner edge.
///
/// Deliberately a quarter of the way along the edge: the harness paints the mouse cursor where we
/// hover, and we don't want it covering the pixels we probe at the center of the edge.
fn resize_pos(side: Side, outer: Rect) -> Pos2 {
    let mut pos = outer.min + 0.25 * outer.size();
    pos[side.axis()] = side.resize_pos(outer);
    pos
}

fn hover_resize_handle(side: Side, harness: &mut Harness<'_>) {
    harness.hover_at(resize_pos(side, outer_rect(harness)));
    harness.run();
}

#[test]
fn separator_line_matrix() {
    let mut results = SnapshotResults::new();

    for show_separator_line in [false, true] {
        let suffix = if show_separator_line { "on" } else { "off" };

        // Not hovered: the line is dim (`show_separator_line`) or absent.
        let mut harness = build_harness(Side::Top, show_separator_line, 2.0);
        results.add(harness.try_snapshot(format!("panel_separator_line/separator_{suffix}_idle")));

        // Hovered: a `resizable` panel shows a bright line regardless of `show_separator_line`,
        // and must not shift its contents to make room for it.
        let mut harness = build_harness(Side::Top, show_separator_line, 2.0);
        hover_resize_handle(Side::Top, &mut harness);
        results
            .add(harness.try_snapshot(format!("panel_separator_line/separator_{suffix}_hovered")));
    }
}

/// The rendered pixels crossing the panel's inner edge, indexed by their distance from the outer
/// edge of the frame's outline: `0` is the first pixel outside the outline (where the separator
/// line goes), `-1` the last pixel of the outline itself, and so on.
struct Probe {
    pixels: Vec<Color32>,

    /// Index in `pixels` of distance `0`.
    zero: usize,
}

impl Probe {
    /// Probe outwards across the inner edge of `side`'s panel, through the middle of that edge.
    ///
    /// The harness renders at 1 pixel per point here, so a distance is a pixel count.
    fn render(side: Side, harness: &mut Harness<'_>, outline_edge: f32) -> Self {
        let image = match harness.render() {
            Ok(image) => image,
            Err(err) => panic!("Failed to render harness: {err}"),
        };

        let axis = side.axis();
        let outer = outer_rect(harness);
        let mut pos = outer.center();
        let outward = side.outward();

        // The pixel at distance `d` covers [`outline_edge` + d, `outline_edge` + d + 1) when
        // probing towards increasing coordinates, and mirrors around the edge when not.
        let zero = 32;
        let pixels = (0..64)
            .map(|i| {
                let distance = i as f32 - zero as f32;
                pos[axis] = if 0.0 < outward {
                    outline_edge + distance
                } else {
                    outline_edge - 1.0 - distance
                };
                let [r, g, b, a] = image
                    .get_pixel(pos.x.round() as u32, pos.y.round() as u32)
                    .0;
                Color32::from_rgba_premultiplied(r, g, b, a)
            })
            .collect();

        Self { pixels, zero }
    }

    fn at(&self, distance: i32) -> Color32 {
        self.pixels[(self.zero as i32 + distance) as usize]
    }

    /// All distances in `range` whose pixel is `color`.
    fn distances_with_color(&self, range: std::ops::Range<i32>, color: Color32) -> Vec<i32> {
        range.filter(|&d| self.at(d) == color).collect()
    }
}

/// The requested layering: contents | inner margin | outline | separator line | outer margin.
#[test]
fn separator_line_sits_between_the_outline_and_the_outer_margin() {
    for side in ALL_SIDES {
        for show_separator_line in [false, true] {
            for hovered in [false, true] {
                let mut harness = build_harness(side, show_separator_line, 1.0);
                if hovered {
                    hover_resize_handle(side, &mut harness);
                }
                let case = format!(
                    "{side:?}, show_separator_line={show_separator_line}, hovered={hovered}"
                );

                // Only a panel that shows the always-visible line reserves room for it.
                let reserved = if show_separator_line {
                    SEPARATOR_WIDTH
                } else {
                    0
                };
                let outer_margin = i32::from(OUTER_MARGIN + reserved);

                // The outer edge of the frame's outline, i.e. of the frame's widget rect.
                let outer = outer_rect(&harness);
                let outline_edge = side.resize_pos(outer) - side.outward() * outer_margin as f32;
                let probe = Probe::render(side, &mut harness, outline_edge);

                for distance in -i32::from(OUTLINE_WIDTH)..0 {
                    assert_eq!(
                        probe.at(distance),
                        OUTLINE,
                        "{case}: the frame's outline should cover distance {distance}"
                    );
                }
                assert_eq!(
                    probe.at(-i32::from(OUTLINE_WIDTH) - 1),
                    FILL,
                    "{case}: the frame's inner margin should be filled right up to the outline"
                );

                // The line (if any) goes in the first pixel outside the outline, and nowhere else.
                let line_color = if hovered {
                    Some(HOVERED_SEPARATOR)
                } else if show_separator_line {
                    Some(SEPARATOR)
                } else {
                    None
                };
                for color in [SEPARATOR, HOVERED_SEPARATOR] {
                    let expected: &[i32] = if line_color == Some(color) { &[0] } else { &[] };
                    assert_eq!(
                        probe.distances_with_color(0..outer_margin, color),
                        expected,
                        "{case}: unexpected pixels of {color:?} outside the frame's outline"
                    );
                }

                // The rest of the outer margin is untouched by both the frame and the line.
                let line_width = if line_color.is_some() {
                    i32::from(SEPARATOR_WIDTH)
                } else {
                    0
                };
                assert!(
                    line_width < outer_margin,
                    "test setup: no outer margin left to check"
                );
                let background = probe.at(outer_margin - 1);
                assert!(
                    background != FILL && background != OUTLINE,
                    "{case}: the outer margin should not be painted by the frame"
                );
                for distance in line_width..outer_margin {
                    assert_eq!(
                        probe.at(distance),
                        background,
                        "{case}: the outer margin should be untouched at distance {distance}"
                    );
                }
            }
        }
    }
}
