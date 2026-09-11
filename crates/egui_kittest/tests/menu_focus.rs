use egui::containers::menu::{MenuBar, SubMenuButton};
use egui::{Key, Modifiers, Ui};
use egui_kittest::Harness;
use kittest::Queryable as _;

fn navigation_menu(context_menu: bool) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(500.0, 400.0))
        .build_ui(move |ui| {
            for row in 0..12 {
                for col in 0..5 {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(col as f32 * 100.0, row as f32 * 30.0),
                        egui::vec2(90.0, 20.0),
                    );
                    let _ = ui.put(rect, egui::Button::new(format!("Behind {row} {col}")));
                }
            }
            ui.scope_builder(
                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                    egui::pos2(150.0, 90.0),
                    egui::vec2(100.0, 20.0),
                )),
                |ui| {
                    let content = |ui: &mut Ui| {
                        let _ = ui.button("First item");
                        let _ = ui.button("Second item");
                        ui.menu_button("Submenu", |ui| {
                            let _ = ui.button("First child");
                            ui.menu_button("Nested submenu", |ui| {
                                let _ = ui.button("Nested item");
                            });
                            let _ = ui.button("Last child");
                        });
                        let _ = ui.button("Last item");
                    };
                    if context_menu {
                        ui.button("Menu").context_menu(content);
                    } else {
                        ui.menu_button("Menu", content);
                    }
                },
            );
        })
}

fn open_navigation_menu(harness: &mut Harness<'_>) {
    harness.get_by_label("Menu").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
}

fn focused_layer(harness: &Harness<'_>) -> egui::LayerId {
    let id = harness
        .ctx
        .memory(|mem| mem.focused())
        .expect("a focused widget");
    harness
        .ctx
        .read_response(id)
        .expect("the focused widget is still shown")
        .layer_id
}

#[test]
fn arrow_navigation_stays_in_menu() {
    for context_menu in [false, true] {
        let mut harness = navigation_menu(context_menu);
        if context_menu {
            harness.get_by_label("Menu").click_secondary();
            harness.run();
        } else {
            open_navigation_menu(&mut harness);
            harness.key_press(Key::ArrowDown);
            harness.run();
            assert!(harness.get_by_label("First item").is_focused());
        }
        for (from, key, to) in [
            ("First item", Key::ArrowDown, "Second item"),
            ("Second item", Key::ArrowUp, "First item"),
            ("First item", Key::ArrowUp, "First item"),
            ("First item", Key::ArrowLeft, "First item"),
            ("First item", Key::ArrowRight, "First item"),
            ("Last item", Key::ArrowDown, "Last item"),
        ] {
            harness.get_by_label(from).focus();
            harness.run();
            harness.key_press(key);
            harness.run();
            assert!(
                harness.get_by_label(to).is_focused(),
                "{from}, {key:?}, context_menu={context_menu}"
            );
        }
    }
}

#[test]
fn arrow_navigation_between_submenus() {
    let mut harness = navigation_menu(false);
    open_navigation_menu(&mut harness);
    harness.get_by_label_contains("Submenu").focus();
    harness.run();
    let parent_layer = focused_layer(&harness);
    harness.key_press(Key::Enter);
    harness.run();
    harness.get_by_label("First child").focus();
    harness.run();
    let child_layer = focused_layer(&harness);
    harness.get_by_label_contains("Submenu").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    // The popup container itself can receive focus, so check the destination layer.
    assert_eq!(focused_layer(&harness), child_layer);
    harness.get_by_label("First child").focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(focused_layer(&harness), parent_layer);

    harness.get_by_label_contains("Nested submenu").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.get_by_label("Nested item").focus();
    harness.run();
    let nested_layer = focused_layer(&harness);
    harness.get_by_label_contains("Nested submenu").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused_layer(&harness), nested_layer);
    harness.get_by_label("Nested item").focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(focused_layer(&harness), child_layer);
}

#[test]
fn menu_tab_escape_and_reopen() {
    let mut harness = navigation_menu(false);
    for _ in 0..2 {
        open_navigation_menu(&mut harness);
        harness.get_by_label("First item").focus();
        harness.run();
        harness.key_press(Key::Tab);
        harness.run();
        assert!(harness.get_by_label("Second item").is_focused());
        harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
        harness.run();
        assert!(harness.get_by_label("First item").is_focused());
        harness.key_press(Key::Escape);
        harness.run();
        assert!(harness.query_by_label("First item").is_none());
        harness.get_by_label("Behind 10 0").focus();
        harness.run();
        harness.key_press(Key::ArrowRight);
        harness.run();
        assert!(harness.get_by_label("Behind 10 1").is_focused());
    }
}

#[test]
fn generic_popup_with_submenu_keeps_earlier_popup_reachable() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 400.0))
        .build_ui(|ui| {
            let _ = ui.put(
                egui::Rect::from_min_size(egui::pos2(700.0, 50.0), egui::vec2(90.0, 20.0)),
                egui::Button::new("Background"),
            );
            let response = ui.button("Host popup");
            egui::Popup::from_response(&response).show(|ui| {
                let response = ui.button("Host first");
                // This popup is built before the submenu makes its parent a menu scope.
                egui::Popup::from_response(&response)
                    .id(response.id.with("earlier popup"))
                    .align(egui::RectAlign::RIGHT_START)
                    .show(|ui| {
                        let _ = ui.button("Earlier popup child");
                    });
                ui.menu_button("Hosted submenu", |ui| {
                    let _ = ui.button("Hosted child");
                });
            });
        });
    harness.get_by_label_contains("Hosted submenu").focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.get_by_label("Hosted child").focus();
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert!(harness.get_by_label_contains("Hosted submenu").is_focused());

    harness.get_by_label("Earlier popup child").focus();
    harness.run();
    let earlier_layer = focused_layer(&harness);
    harness.get_by_label("Host first").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused_layer(&harness), earlier_layer);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused_layer(&harness), earlier_layer);
}

#[test]
fn generic_popup_does_not_contain_arrow_navigation() {
    let mut harness = Harness::new_ui(|ui| {
        let _ = ui.put(
            egui::Rect::from_min_size(egui::pos2(400.0, 50.0), egui::vec2(90.0, 20.0)),
            egui::Button::new("Background"),
        );
        let response = ui.button("Popup");
        egui::Popup::from_response(&response).show(|ui| {
            let _ = ui.button("Popup item");
        });
    });
    harness.get_by_label("Popup item").focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(harness.get_by_label("Background").is_focused());
}

#[test]
fn menu_in_modal_keeps_arrow_navigation() {
    let mut harness = Harness::new_ui(|ui| {
        let _ = ui.button("Background");
        egui::Modal::new(egui::Id::unique("modal")).show(ui.ctx(), |ui| {
            ui.menu_button("Menu", |ui| {
                let _ = ui.button("First item");
                let _ = ui.button("Last item");
            });
            let _ = ui.button("Other modal item");
        });
    });
    open_navigation_menu(&mut harness);
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.get_by_label("First item").is_focused());
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.get_by_label("Last item").is_focused());
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.get_by_label("Last item").is_focused());
    harness.key_press(Key::Escape);
    harness.run();
    harness.get_by_label("Menu").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.get_by_label("Other modal item").is_focused());
}

#[test]
fn standalone_submenu_in_menu_bar_excludes_background() {
    let mut harness = Harness::new_ui(|ui| {
        MenuBar::new().ui(ui, |ui| {
            SubMenuButton::new("Standalone submenu").ui(ui, |ui| {
                let _ = ui.button("Standalone item");
            });
        });
        let _ = ui.put(
            egui::Rect::from_min_size(egui::pos2(500.0, 10.0), egui::vec2(90.0, 20.0)),
            egui::Button::new("Background"),
        );
    });
    harness.get_by_label_contains("Standalone submenu").hover();
    harness.run();
    harness.get_by_label("Standalone item").focus();
    harness.run();
    let submenu_layer = focused_layer(&harness);
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(focused_layer(&harness), submenu_layer);
}

#[test]
fn blocked_menu_does_not_trap_modal_navigation() {
    let mut harness = Harness::new_ui_state(
        |ui, show_modal| {
            let response = ui.button("Underlying menu");
            egui::Popup::menu(&response).open(true).show(|ui| {
                let _ = ui.button("Underlying item");
            });
            if *show_modal {
                egui::Modal::new(egui::Id::unique("modal")).show(ui.ctx(), |ui| {
                    let _ = ui.button("First modal item");
                    let _ = ui.button("Last modal item");
                });
            }
        },
        false,
    );
    *harness.state_mut() = true;
    harness.run();
    harness.get_by_label("First modal item").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.get_by_label("Last modal item").is_focused());
}

#[test]
fn combo_box_keyboard_selection() {
    let mut harness = Harness::new_ui_state(
        |ui, selected| {
            egui::ComboBox::from_label("Choice")
                .show_index(ui, selected, 3, |i| format!("Choice {i}"));
            let _ = ui.button("Background");
        },
        0,
    );
    harness.get_by_role(egui::accesskit::Role::ComboBox).focus();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.get_by_label("Choice 0").focus();
    harness.run();
    harness.key_press(Key::ArrowDown);
    harness.run();
    assert!(harness.get_by_label("Choice 1").is_focused());
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(*harness.state(), 1);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.query_by_label("Choice 2").is_none());
}

#[test]
fn color_popup_keeps_pointer_editing() {
    let mut harness = Harness::new_ui_state(
        |ui, color| {
            ui.color_edit_button_srgba(color);
        },
        egui::Color32::from_rgb(100, 150, 200),
    );
    harness
        .get_by_role(egui::accesskit::Role::ColorWell)
        .click();
    harness.run();
    let original = *harness.state();
    let grab = harness
        .query_all_by_role(egui::accesskit::Role::SpinButton)
        .next()
        .unwrap()
        .rect()
        .center();
    let target = grab + egui::vec2(20.0, 0.0);
    harness.hover_at(grab);
    harness.run();
    harness.drag_at(grab);
    harness.run();
    harness.hover_at(target);
    harness.run();
    harness.drop_at(target);
    harness.run();
    assert_ne!(*harness.state(), original);
    harness.key_press(Key::Escape);
    harness.run();
    assert!(
        harness
            .query_by_role(egui::accesskit::Role::SpinButton)
            .is_none()
    );
}
