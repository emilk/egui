/// A grid in a cell of another grid starts inside the cell margin, so it is stable.
#[test]
fn nested_grid_does_not_grow() {
    let ctx = egui::Context::default();
    let mut nested_rect = egui::Rect::NOTHING;
    let mut cell_rect = egui::Rect::NOTHING;
    let mut nested_rects = vec![];
    for _ in 0..4 {
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::Grid::new("parent").show(ui, |ui| {
                cell_rect = ui.available_rect_before_wrap();
                nested_rect = egui::Grid::new("nested")
                    .show(ui, |ui| {
                        ui.label("a");
                        ui.label("b");
                        ui.end_row();
                    })
                    .response
                    .rect;
                ui.end_row();
            });
        });
        output.drop_without_applying_deltas();
        nested_rects.push(nested_rect);
    }

    assert_eq!(nested_rect.min, cell_rect.min);
    assert!(
        nested_rects.windows(2).skip(1).all(|w| w[0] == w[1]),
        "{nested_rects:?}"
    );
}
