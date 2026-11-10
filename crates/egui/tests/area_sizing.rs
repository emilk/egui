/// Explicit widget sizes must survive the invisible measurement pass, so that
/// non-top-left pivots place the contents correctly on the first visible frame.
#[test]
#[cfg(feature = "default_fonts")]
fn explicitly_sized_area_is_positioned_on_first_visible_frame() {
    use egui::{Align2, Area, Button, Context, Id, Vec2, pos2};

    for pivot in [
        Align2::LEFT_TOP,
        Align2::CENTER_CENTER,
        Align2::RIGHT_BOTTOM,
    ] {
        for horizontal in [false, true] {
            for default_size in [Vec2::splat(30.0), Vec2::splat(80.0)] {
                let ctx = Context::default();
                let size = Vec2::splat(30.0);

                for frame in 0..8 {
                    let position = if frame < 4 {
                        pos2(200.0, 200.0)
                    } else {
                        pos2(240.0, 190.0)
                    };
                    let force_sizing_pass = frame == 5;
                    let output = ctx.run_ui(Default::default(), |ui| {
                        let area = Area::new(Id::unique("sized_area"))
                            .pivot(pivot)
                            .default_size(default_size)
                            .fixed_pos(position)
                            .sizing_pass(force_sizing_pass)
                            .fade_in(false)
                            .show(ui, |ui| {
                                let button = if horizontal {
                                    ui.horizontal(|ui| ui.add_sized(size, Button::new("Hi")))
                                        .inner
                                } else {
                                    ui.add_sized(size, Button::new("Hi"))
                                };
                                (ui.is_visible(), button.rect)
                            });

                        let (visible, button_rect) = area.inner;
                        assert_eq!(visible, frame != 0 && !force_sizing_pass);
                        assert_eq!(area.response.rect.size(), size);
                        if visible {
                            assert_eq!(button_rect.size(), size);
                            assert_eq!(pivot.pos_in_rect(&button_rect), position);
                            assert_eq!(button_rect, area.response.rect);
                        }
                    });
                    output.drop_without_applying_deltas();
                }
            }
        }
    }
}

#[test]
fn unbounded_cross_axis_still_measures_widget_size() {
    use egui::{
        Align, Context, FontDefinitions, Layout, Rect, Sense, Ui, UiBuilder, Vec2, pos2, vec2,
    };

    for (layout, requested_size, measured_size) in [
        (
            Layout::top_down(Align::Min),
            vec2(f32::INFINITY, 30.0),
            vec2(10.0, 30.0),
        ),
        (
            Layout::left_to_right(Align::Min),
            vec2(30.0, f32::INFINITY),
            vec2(30.0, 10.0),
        ),
    ] {
        let ctx = Context::default();
        ctx.set_fonts(FontDefinitions::empty());
        let output = ctx.run_ui(Default::default(), |ui| {
            ui.scope_builder(
                UiBuilder::new()
                    .max_rect(Rect::from_min_size(pos2(100.0, 100.0), Vec2::splat(120.0)))
                    .layout(layout)
                    .sizing_pass(),
                |ui| {
                    let response = ui.add_sized(requested_size, |ui: &mut Ui| {
                        ui.allocate_response(Vec2::splat(10.0), Sense::hover())
                    });
                    assert_eq!(response.rect.size(), measured_size);
                    assert_eq!(ui.min_size(), measured_size);
                },
            );
        });
        output.drop_without_applying_deltas();
    }
}
