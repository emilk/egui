use kittest::Queryable as _;

#[test]
fn test_interactive_tooltip() {
    struct State {
        link_clicked: bool,
    }

    let mut harness = egui_kittest::Harness::new_ui_state(
        |ui, state| {
            ui.label("I have a tooltip").on_hover_ui(|ui| {
                if ui.link("link").clicked() {
                    state.link_clicked = true;
                }
            });
        },
        State {
            link_clicked: false,
        },
    );

    harness.get_by_label_contains("tooltip").hover();
    harness.run();
    harness.get_by_label("link").hover();
    harness.run();
    harness.get_by_label("link").simulate_click();

    harness.run();

    assert!(harness.state().link_clicked);
}
