use egui::{ComboBox, Popup, PopupCloseBehavior, Vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::Harness;

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
    harness.get_by_label("link").click();

    harness.run();

    assert!(harness.state().link_clicked);
}

#[test]
fn test_combobox_in_popup() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 200.0))
        .build_ui_state(
            |ui, state| {
                let response = ui.button("Open Popup");
                Popup::menu(&response)
                    .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                    .show(|ui| {
                        ui.heading("Popup");
                        ComboBox::new("combo", "")
                            .selected_text("Select an option")
                            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(state, 0, "Option 0");
                                ui.selectable_value(state, 1, "Option 1");
                            });
                    });
            },
            0,
        );

    harness.get_by_label("Open Popup").click();
    harness.run();
    harness.get_by_value("Select an option").click();
    harness.run();
    harness.get_by_label("Option 1").click();
    harness.run();
    assert_eq!(*harness.state(), 1);

    // The parent popup should not close when clicking on the child popup
    harness.get_by_label("Option 0").click();
    harness.run();
    assert_eq!(*harness.state(), 0);

    harness.snapshot("combobox_in_popup");

    // Clicking the parent popup should close the child popup
    harness.get_by_label("Popup").click();
    harness.run();
    assert_eq!(harness.query_by_label("Option 0"), None);

    harness.get_by_value("Select an option").click();
    harness.run();

    assert_eq!(harness.query_by_label("Option 0").is_some(), true);

    // Clicking outside should close both popups
    harness.get_by_label("Open Popup").click();
    harness.run();

    assert_eq!(harness.query_by_label("Popup"), None);
}
