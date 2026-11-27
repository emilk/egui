/// A grid in a cell of another grid keeps its size from frame to frame.
#[test]
fn nested_grid_does_not_grow() {
    let ctx = egui::Context::default();
    let mut nested_rect = egui::Rect::NOTHING;
    let mut nested_rects = vec![];
    for _ in 0..4 {
        let output = ctx.run_ui(Default::default(), |ui| {
            egui::Grid::new("parent").show(ui, |ui| {
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

    assert!(
        nested_rects.windows(2).skip(1).all(|w| w[0] == w[1]),
        "{nested_rects:?}"
    );
}

/// The outer cell margins only take up space on the axes where `edge_margin` is on.
#[test]
fn grid_edge_margin() {
    let size = |edge_margin: Option<egui::Vec2b>| {
        let ctx = egui::Context::default();
        let mut size = egui::Vec2::ZERO;
        for _ in 0..2 {
            let output = ctx.run_ui(Default::default(), |ui| {
                let top_left = ui.cursor().min;
                let mut grid = egui::Grid::new("grid")
                    .min_col_width(0.0)
                    .min_row_height(0.0);
                if let Some(edge_margin) = edge_margin {
                    grid = grid.edge_margin(edge_margin);
                }
                grid.show(ui, |ui| {
                    ui.allocate_space(egui::vec2(30.0, 10.0));
                    ui.allocate_space(egui::vec2(30.0, 10.0));
                    ui.end_row();
                });
                size = ui.min_rect().max - top_left;
            });
            output.drop_without_applying_deltas();
        }
        size
    };

    let margin = egui::style::TableStyle::default().cell_margin.sum();
    let cells = egui::vec2(60.0, 10.0);
    let inner = egui::vec2(margin.x, 0.0); // Between the two columns
    let default = size(None);
    assert_eq!(default, cells + egui::vec2(inner.x, margin.y));
    assert_eq!(size(Some(egui::Vec2b::new(false, true))), default);
    assert_eq!(size(Some(egui::Vec2b::TRUE)), cells + inner + margin);
    assert_eq!(size(Some(egui::Vec2b::FALSE)), cells + inner);
}
