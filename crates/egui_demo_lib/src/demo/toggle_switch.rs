//! Source code example of how to create your own widget.
//! This is meant to be read as a tutorial, hence the plethora of comments.

/// iOS-style toggle switch:
///
/// ``` text
///      _____________
///     /       /.....\
///    |       |.......|
///     \_______\_____/
/// ```
///
/// ## Example:
/// ``` ignore
/// toggle_ui(ui, &mut my_bool);
/// ```
pub fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    // Widget code can be broken up in four steps:
    //  1. Decide a size for the widget
    //  2. Allocate space for it
    //  3. Handle interactions with the widget (if any)
    //  4. Paint the widget

    // 1. Deciding widget size:
    // You can query the `ui` how much space is available,
    // but in this example we have a fixed size widget based on the height of a standard button:
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);

    // 2. Allocating space:
    // This is where we get a region of the screen assigned.
    // We also tell the Ui to sense clicks in the allocated region.
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    // 3. Interact: Time to check for clicks!
    if response.clicked() {
        *on = !*on;
        response.mark_changed(); // report back that the value changed
    }

    // Attach some meta-data to the response which can be used by screen readers:
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), *on, ""));

    // 4. Paint!
    // Make sure we need to paint:
    if ui.is_rect_visible(rect) {
        // Let's ask for a simple animation from egui.
        // egui keeps track of changes in the boolean associated with the id and
        // returns an animated value in the 0-1 range for how much "on" we are.
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        // We will follow the current style by asking
        // "how should something that is being interacted with be painted?".
        // This will, for instance, give us different colors when the widget is hovered or clicked.
        let visuals = ui.style().interact_selectable(&response, *on);
        // All coordinates are in absolute screen coordinates so we use `rect` to place the elements.
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        // Paint the circle, animating it from the reading start to the reading end with
        // `how_on`:
        let circle_x = egui::lerp(knob_x_endpoints(rect, radius, ui.is_rtl()), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    // All done! Return the interaction response so the user can check what happened
    // (hovered, clicked, ...) and maybe show a tooltip:
    response
}

/// Here is the same code again, but a bit more compact:
#[expect(dead_code)]
fn toggle_ui_compact(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    response
        .widget_info(|| egui::WidgetInfo::selected(egui::Role::CheckBox, ui.is_enabled(), *on, ""));

    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(
            rect,
            radius,
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let circle_x = egui::lerp(knob_x_endpoints(rect, radius, ui.is_rtl()), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter()
            .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
    }

    response
}

// A wrapper that allows the more idiomatic usage pattern: `ui.add(toggle(&mut my_bool))`
/// iOS-style toggle switch.
///
/// ## Example:
/// ``` ignore
/// ui.add(toggle(&mut my_bool));
/// ```
pub fn toggle(on: &mut bool) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| toggle_ui(ui, on)
}

pub fn url_to_file_source_code() -> String {
    format!("https://github.com/emilk/egui/blob/main/{}", file!())
}

/// The knob center's x endpoints: off at the reading start, on at the reading end.
///
/// Mirrored under a right-to-left reading direction ([`egui::Ui::is_rtl`]), so the
/// switch always animates toward the reading end as it turns on.
fn knob_x_endpoints(rect: egui::Rect, radius: f32, is_rtl: bool) -> core::ops::RangeInclusive<f32> {
    let (start, end) = if is_rtl {
        (rect.right() - radius, rect.left() + radius)
    } else {
        (rect.left() + radius, rect.right() - radius)
    };
    start..=end
}

#[cfg(test)]
mod tests {
    use super::toggle;
    use egui::epaint::{CircleShape, RectShape, Shape};
    use egui::{Align, Context, FullOutput, Layout, Pos2, RawInput, Rect, Ui, Vec2, vec2};

    /// Which mirroring mechanism a test case runs under.
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum Dir {
        /// No flag, default layout: the baseline that must stay unchanged.
        Ltr,

        /// The interface-direction flag, set before the first frame.
        FlagRtl,

        /// An explicit right-to-left layout with the flag off.
        LayoutRtl,
    }

    fn make_ctx(dir: Dir) -> Context {
        let ctx = Context::default();
        if dir == Dir::FlagRtl {
            // Set before the first frame, so the flag reaches the first pass.
            ctx.set_right_to_left(true);
        }
        ctx
    }

    fn input(size: Vec2) -> RawInput {
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            ..Default::default()
        }
    }

    /// Run one ui pass and discard the texture deltas (these tests never render).
    fn run(ctx: &Context, input: RawInput, run_ui: impl FnMut(&mut Ui)) -> FullOutput {
        let mut output = ctx.run_ui(input, run_ui);
        output.textures_delta.clear();
        output
    }

    /// The painted track: the one rect that sits inside the widget's allocation.
    /// (Its fill changes with the on state, so it is not matched by color.)
    fn track_of(output: &FullOutput, widget: Rect) -> Rect {
        let tracks: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Rect(RectShape { rect, .. }) if widget.contains_rect(*rect) => Some(*rect),
                _ => None,
            })
            .collect();
        assert_eq!(
            tracks.len(),
            1,
            "expected exactly one track rect, got {tracks:?}"
        );
        tracks[0]
    }

    /// The painted knob: the one circle inside the track.
    fn knob_of(output: &FullOutput, track: Rect) -> CircleShape {
        let knobs: Vec<&CircleShape> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                Shape::Circle(circle @ CircleShape { center, .. })
                    if track.contains(*center) && circle.radius < 0.5 * track.height() =>
                {
                    Some(circle)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            knobs.len(),
            1,
            "expected exactly one knob circle, got {knobs:?}"
        );
        *knobs[0]
    }

    /// Paint the toggle switch off or on in `dir`, and return the painted track and knob.
    fn painted(dir: Dir, on: bool) -> (Rect, CircleShape) {
        let ctx = make_ctx(dir);
        let screen = vec2(400.0, 100.0);
        let mut state = on;
        let mut widget = Rect::NOTHING;
        let output = run(&ctx, input(screen), |ui| {
            if dir == Dir::LayoutRtl {
                widget = ui
                    .with_layout(Layout::right_to_left(Align::TOP), |ui| {
                        ui.add(toggle(&mut state)).rect
                    })
                    .inner;
            } else {
                widget = ui.add(toggle(&mut state)).rect;
            }
        });
        let track = track_of(&output, widget);
        (track, knob_of(&output, track))
    }

    /// The knob must hug the reading-start edge when off and the reading-end edge
    /// when on, with the knob's center exactly one radius in from that edge.
    #[test]
    fn knob_sits_at_the_reading_start_when_off_and_the_reading_end_when_on() {
        fn assert_endpoint(dir: Dir, on: bool) {
            let (track, knob) = painted(dir, on);
            let radius = 0.5 * track.height();
            let expected_x = match (dir, on) {
                (Dir::Ltr | Dir::LayoutRtl, false) | (Dir::FlagRtl, true) => track.left() + radius,
                (Dir::Ltr | Dir::LayoutRtl, true) | (Dir::FlagRtl, false) => track.right() - radius,
            };
            assert!(
                (knob.center.x - expected_x).abs() < 1e-3,
                "{dir:?}, on = {on}: the knob center should sit at x {expected_x}, \
                 got {knob:?} (track {track:?})"
            );
            assert!(
                (knob.center.y - track.center().y).abs() < 1e-3,
                "{dir:?}, on = {on}: the knob should stay vertically centered, \
                 got {knob:?} (track {track:?})"
            );
        }

        // LTR: off on the physical left, on on the physical right — unchanged.
        assert_endpoint(Dir::Ltr, false);
        assert_endpoint(Dir::Ltr, true);

        // RTL via the interface-direction flag: mirrored.
        assert_endpoint(Dir::FlagRtl, false);
        assert_endpoint(Dir::FlagRtl, true);

        // An explicit right-to-left layout with the flag off is widget chrome that
        // follows the flag, so it keeps its left-to-right paint (as with the
        // progress bar).
        assert_endpoint(Dir::LayoutRtl, false);
        assert_endpoint(Dir::LayoutRtl, true);
    }

    /// The knob's travel is mirrored, not shifted: for each state, the knob sits the
    /// same distance from its reading edge in both directions, so the on knob under
    /// RTL lands exactly where the off knob sits under LTR, and vice versa.
    #[test]
    fn rtl_knob_positions_mirror_the_ltr_ones() {
        let (ltr_track, ltr_off) = painted(Dir::Ltr, false);
        let (rtl_track, rtl_off) = painted(Dir::FlagRtl, false);
        assert_eq!(
            ltr_track.height(),
            rtl_track.height(),
            "the track should be the same size in both directions"
        );
        let inset = ltr_track.height() * 0.5;
        assert!(
            ((ltr_off.center.x - (ltr_track.left() + inset))
                - ((rtl_track.right() - inset) - rtl_off.center.x))
                .abs()
                < 1e-3,
            "off should hug the reading start in both directions: \
             ltr off {ltr_off:?} (track {ltr_track:?}), rtl off {rtl_off:?} (track {rtl_track:?})"
        );

        let (_, ltr_on) = painted(Dir::Ltr, true);
        let (_, rtl_on) = painted(Dir::FlagRtl, true);
        assert!(
            (((ltr_track.right() - inset) - ltr_on.center.x)
                - (rtl_on.center.x - (rtl_track.left() + inset)))
                .abs()
                < 1e-3,
            "on should hug the reading end in both directions: \
             ltr on {ltr_on:?} (track {ltr_track:?}), rtl on {rtl_on:?} (track {rtl_track:?})"
        );
    }
}
