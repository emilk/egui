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
fn open_popup_resizes_after_explicit_sizing_pass() {
    const POPUP_BUTTON: &str = "Growing popup";
    const MAX_HEIGHT: f32 = 100.0;

    struct State {
        item_count: usize,
        needs_sizing_pass: bool,
        popup_height: f32,
        viewport_height: f32,
        content_height: f32,
    }

    let mut harness = Harness::builder()
        .with_size(egui::Vec2::new(500.0, 300.0))
        .build_ui_state(
            |ui, state| {
                let response = ui.button(POPUP_BUTTON);
                let needs_sizing_pass = core::mem::take(&mut state.needs_sizing_pass);
                let item_count = state.item_count;

                if let Some(popup) = Popup::from_response(&response)
                    .sizing_pass(needs_sizing_pass)
                    .show(|ui| {
                        egui::ScrollArea::vertical()
                            .max_height(MAX_HEIGHT)
                            .show(ui, |ui| {
                                for index in 0..item_count {
                                    ui.label(format!("Item {index}"));
                                }
                            })
                    })
                {
                    state.popup_height = popup.response.rect.height();
                    state.viewport_height = popup.inner.inner_rect.height();
                    state.content_height = popup.inner.content_size.y;
                }
            },
            State {
                item_count: 2,
                needs_sizing_pass: false,
                popup_height: 0.0,
                viewport_height: 0.0,
                content_height: 0.0,
            },
        );

    harness.run();
    let initial_popup_height = harness.state().popup_height;

    harness.state_mut().item_count = 20;
    harness.run();
    let stale_viewport_height = harness.state().viewport_height;
    assert!(
        stale_viewport_height < MAX_HEIGHT,
        "viewport unexpectedly reached its maximum without a sizing pass"
    );

    harness.state_mut().needs_sizing_pass = true;
    harness.run();

    assert!(
        harness.state().popup_height > initial_popup_height,
        "popup did not grow after an explicit sizing pass"
    );
    assert!(
        harness.state().viewport_height > stale_viewport_height,
        "scroll viewport did not grow after an explicit sizing pass"
    );
    assert!(
        (harness.state().viewport_height - MAX_HEIGHT).abs() <= 0.5,
        "scroll viewport did not stop at its maximum height"
    );
    assert!(
        harness.state().content_height > harness.state().viewport_height,
        "popup contents did not remain scrollable at the maximum height"
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
