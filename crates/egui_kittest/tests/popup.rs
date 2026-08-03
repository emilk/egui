use egui::{Align, Layout, Popup};
use egui_kittest::Harness;
use kittest::Queryable as _;

#[test]
fn reopened_popup_resizes_for_wider_items() {
    const POPUP_BUTTON: &str = "Dynamic popup";
    const SHORT_ITEM: &str = "Short item";
    const WIDE_ITEM: &str = "Newly added item with a much wider label";

    #[derive(Default)]
    struct State {
        open: bool,
        show_wide_item: bool,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(500.0, 300.0))
        .build_ui_state(
            |ui, state| {
                let response = ui.button(POPUP_BUTTON);
                if response.clicked() {
                    state.open = !state.open;
                }

                Popup::from_response(&response)
                    .open(state.open)
                    .layout(Layout::top_down_justified(Align::Min))
                    .show(|ui| {
                        _ = ui.selectable_label(false, SHORT_ITEM);
                        _ = ui.selectable_label(false, "Another short item");
                        if state.show_wide_item {
                            _ = ui.selectable_label(false, WIDE_ITEM);
                        }
                    });
            },
            State::default(),
        );

    harness.get_by_label(POPUP_BUTTON).click();
    harness.run();
    let initial_row_size = harness.get_by_label(SHORT_ITEM).rect().size();

    harness.get_by_label(POPUP_BUTTON).click();
    harness.run();
    assert!(harness.query_by_label(SHORT_ITEM).is_none());

    harness.state_mut().show_wide_item = true;
    harness.run();
    harness.get_by_label(POPUP_BUTTON).click();
    harness.run();

    let reopened_row_size = harness.get_by_label(SHORT_ITEM).rect().size();
    let wide_row_size = harness.get_by_label(WIDE_ITEM).rect().size();

    assert!(
        reopened_row_size.x > initial_row_size.x,
        "reopened row width ({}) did not grow beyond its initial width ({})",
        reopened_row_size.x,
        initial_row_size.x
    );
    assert!(
        wide_row_size.y <= initial_row_size.y + 0.5,
        "new row height ({}) exceeds the single-line row height ({})",
        wide_row_size.y,
        initial_row_size.y
    );
}

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
