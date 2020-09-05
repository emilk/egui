//! Source code example of how to create your own widget.
use crate::{paint::PaintCmd, *};

// iOS style toggle switch
pub fn toggle(ui: &mut Ui, on: &mut bool) -> Response {
    // First we must reserve some space to use:
    let desired_size = vec2(2.0, 1.0) * ui.style().spacing.clickable_diameter;
    let rect = ui.allocate_space(desired_size);

    // Now that we have an area, we want to check for clicks.
    // To do that we need an `Id` for the button.
    // Id's are best created from unique identifiers (like fixed labels)
    // but since we have no label for the switch we here just generate an `Id` automatically
    // (based on a rolling counter in the `Ui`).
    let id = ui.make_position_id();
    let response = ui.interact(rect, id, Sense::click());
    if response.clicked {
        *on = !*on;
    }

    // For painting, let's ask for a simple animation from Egui.
    // Egui keeps track of changes in the boolean associated with the id and
    // returns an animated value between `[0, 1]` for how far along "toggled" we are.
    let how_on = ui.ctx().animate_bool(id, *on);

    // After interaction (to avoid frame delay), we paint the widget.
    // We can follow the standard style theme by asking
    // "how should something that is being interacted with be painted?".
    // This gives us visual style change when the user hovers and clicks on the widget.
    let visuals = ui.style().interact(&response);
    let off_color = Rgba::new(0.0, 0.0, 0.0, 0.0);
    let on_color = Rgba::new(0.0, 0.5, 0.0, 0.5);
    // All coordinates are in screen coordinates (not relative)
    // so we use `rect` to place the elements.
    let radius = 0.5 * rect.height();
    ui.painter().add(PaintCmd::Rect {
        rect,
        corner_radius: radius,
        fill: lerp(off_color..=on_color, how_on).into(),
        stroke: visuals.bg_stroke,
    });
    // Animate the circle from left to right:
    let circle_x = lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
    ui.painter().add(PaintCmd::Circle {
        center: pos2(circle_x, rect.center().y),
        radius: 0.75 * radius,
        fill: visuals.fg_fill,
        stroke: visuals.fg_stroke,
    });

    // All done! Return the response so the user can check what happened
    // (hoovered, clicked, ...) and show a tooltip.

    response
}

pub fn demo(ui: &mut Ui, on: &mut bool) {
    ui.label("Example of how to create your own widget from scratch.");
    let url = format!("https://github.com/emilk/egui/blob/master/{}", file!());
    ui.horizontal_centered(|ui| {
        ui.label("My beautiful toggle switch:");
        toggle(ui, on).tooltip_text("Click to toggle");
        ui.add(Hyperlink::new(url).text("(source code)"));
    });
}
