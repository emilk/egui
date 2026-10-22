use egui::{Align, Context, Layout, UiBuilder, vec2};
use egui_extras::{Column, TableBuilder};

#[test]
fn clipped_table_sizing_pass() {
    for clip in [false, true] {
        let ctx = Context::default();
        let mut measured_width = 0.0;
        let output = ctx.run_ui(Default::default(), |ui| {
            let mut ui = ui.new_child(UiBuilder::new().sizing_pass());
            TableBuilder::new(&mut ui)
                .cell_layout(Layout::left_to_right(Align::Center))
                .column(Column::exact(100.0).clip(clip))
                .vscroll(false)
                .body(|mut body| {
                    body.row(20.0, |mut row| {
                        let (used, _) = row.col(|ui| {
                            ui.allocate_space(vec2(400.0, 20.0));
                        });
                        // Auto-sizing must still be able to measure the full content.
                        assert_eq!(used.width(), 400.0);
                    });
                });
            measured_width = ui.min_size().x;
        });
        output.drop_without_applying_deltas();
        assert_eq!(measured_width, if clip { 100.0 } else { 400.0 });
    }
}

#[test]
fn window_with_clipped_table_can_shrink() {
    for clip in [false, true] {
        for (truncate, remainder) in [(false, false), (true, false), (false, true), (true, true)] {
            let ctx = Context::default();
            let mut window_rect = egui::Rect::NOTHING;
            let mut content_width = 0.0;
            let mut frame = |events| {
                let input = egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        vec2(1000.0, 600.0),
                    )),
                    events,
                    ..Default::default()
                };
                let output = ctx.run_ui(input, |ui| {
                    let response = egui::Window::new("Table")
                        .default_pos([20.0, 20.0])
                        .default_size([500.0, 150.0])
                        .min_width(100.0)
                        .show(ui, |ui| {
                            content_width = ui.available_width();
                            TableBuilder::new(ui)
                                .cell_layout(Layout::left_to_right(Align::Center))
                                .column(if remainder {
                                    Column::remainder().clip(clip)
                                } else {
                                    Column::exact(content_width).clip(clip)
                                })
                                .auto_shrink([false, false])
                                .body(|mut body| {
                                    body.row(20.0, |mut row| {
                                        row.col(|ui| {
                                            let label = egui::Label::new(
                                                "A long table label that should not prevent the window from shrinking",
                                            );
                                            ui.add(if truncate { label.truncate() } else { label });
                                        });
                                    });
                                });
                        });
                    if let Some(response) = response {
                        window_rect = response.response.rect;
                    }
                });
                output.drop_without_applying_deltas();
                (window_rect, content_width)
            };

            for _ in 0..5 {
                frame(vec![]);
            }
            let (rect, initial_width) = frame(vec![]);
            assert!(initial_width > 400.0);
            let start = egui::pos2(rect.right(), rect.center().y);
            frame(vec![egui::Event::PointerMoved(start)]);
            frame(vec![egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }]);
            let end = start - vec2(350.0, 0.0);
            for _ in 0..5 {
                frame(vec![egui::Event::PointerMoved(end)]);
            }
            frame(vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }]);
            let (_, final_width) = frame(vec![]);
            if clip || truncate {
                assert!(
                    final_width < 200.0,
                    "clip={clip}, truncate={truncate}, remainder={remainder}: {initial_width} -> {final_width}"
                );
            } else {
                assert!(
                    final_width > 300.0,
                    "Unclipped content lost its minimum: {final_width}"
                );
            }
        }
    }
}

#[test]
fn automatic_columns_measure_clipped_content() {
    for clip in [false, true] {
        let ctx = Context::default();
        let mut width = 0.0;
        for _ in 0..5 {
            ctx.run_ui(Default::default(), |ui| {
                TableBuilder::new(ui)
                    .column(Column::auto().clip(clip))
                    .vscroll(false)
                    .body(|mut body| {
                        body.row(20.0, |mut row| {
                            row.col(|ui| {
                                width = ui.available_width();
                                ui.allocate_space(vec2(400.0, 20.0));
                            });
                        });
                    });
            })
            .drop_without_applying_deltas();
        }
        assert_eq!(width, 400.0);
    }
}

#[test]
fn strip_sizing_respects_clipping_in_both_directions() {
    for clip in [false, true] {
        for horizontal in [false, true] {
            let ctx = Context::default();
            let mut measured = egui::Vec2::ZERO;
            ctx.run_ui(Default::default(), |ui| {
                let mut ui = ui.new_child(UiBuilder::new().sizing_pass().max_rect(
                    egui::Rect::from_min_size(ui.cursor().min, vec2(100.0, 80.0)),
                ));
                let builder = egui_extras::StripBuilder::new(&mut ui)
                    .clip(clip)
                    .size(egui_extras::Size::exact(40.0));
                let contents = |mut strip: egui_extras::Strip<'_, '_>| {
                    strip.cell(|ui| {
                        ui.allocate_space(vec2(400.0, 300.0));
                    });
                };
                if horizontal {
                    builder.horizontal(contents);
                } else {
                    builder.vertical(contents);
                }
                measured = ui.min_size();
            })
            .drop_without_applying_deltas();
            let expected = if !clip {
                vec2(400.0, 300.0)
            } else if horizontal {
                vec2(40.0, 80.0)
            } else {
                vec2(100.0, 40.0)
            };
            assert_eq!(measured, expected);
        }
    }
}
