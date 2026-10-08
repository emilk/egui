use egui::containers::menu::{MenuBar, SubMenuButton};
use egui::{Key, Ui};
use egui_kittest::Harness;
use kittest::Queryable as _;

#[test]
fn menu_linear_navigation() {
    for context_menu in [false, true] {
        let mut harness = navigation_menu(context_menu);
        if context_menu {
            harness.get_by_label("Menu").click_secondary();
            harness.run();
        } else {
            open_navigation_menu(&mut harness);
        }
        for (from, key, to) in [
            ("Menu", Key::ArrowDown, "First item"),
            ("Menu", Key::ArrowUp, "Last item"),
            // Skips the disabled entry.
            ("First item", Key::ArrowDown, "Second item"),
            ("Second item", Key::ArrowUp, "First item"),
            ("First item", Key::ArrowUp, "Last item"),
            ("Last item", Key::ArrowDown, "First item"),
            ("First item", Key::ArrowLeft, "Menu"),
            ("First item", Key::ArrowRight, "First item"),
        ] {
            focus(&mut harness, from);
            harness.state_mut().clear();
            press(&mut harness, key);
            let step = format!("{from}, {key:?}, context_menu={context_menu}");
            assert!(is_focused(&harness, to), "{step}");
            if to.ends_with("item") && to != from {
                assert_eq!(*harness.state(), [to], "gained_focus: {step}");
            }
        }
    }
}

#[test]
fn submenu_navigation() {
    let mut harness = navigation_menu(false);
    open_navigation_menu(&mut harness);

    // Arrowing onto a submenu button opens the submenu but keeps focus on the button.
    focus(&mut harness, "Second item");
    press(&mut harness, Key::ArrowDown);
    assert!(is_focused(&harness, "Submenu"));
    assert!(is_shown(&harness, "First child"));
    press(&mut harness, Key::ArrowDown);
    assert!(is_focused(&harness, "Last item"));
    assert!(!is_shown(&harness, "First child"));
    press(&mut harness, Key::ArrowUp);
    assert!(is_shown(&harness, "First child"));

    press(&mut harness, Key::ArrowRight);
    assert!(is_focused(&harness, "First child"));
    press(&mut harness, Key::ArrowDown);
    assert!(is_focused(&harness, "Nested submenu"));
    assert!(is_shown(&harness, "Nested item"));
    press(&mut harness, Key::ArrowRight);
    assert!(is_focused(&harness, "Nested item"));

    // Left returns to the submenu button and closes the submenu.
    press(&mut harness, Key::ArrowLeft);
    assert!(is_focused(&harness, "Nested submenu"));
    assert!(!is_shown(&harness, "Nested item"));
    press(&mut harness, Key::ArrowLeft);
    assert!(is_focused(&harness, "Submenu"));
    assert!(!is_shown(&harness, "First child"));

    press(&mut harness, Key::Enter);
    assert!(is_focused(&harness, "First child"));
    press(&mut harness, Key::ArrowLeft);
    assert!(is_focused(&harness, "Submenu"));

    // Tab moves focus without opening the submenu.
    focus(&mut harness, "Second item");
    press(&mut harness, Key::Tab);
    assert!(is_focused(&harness, "Submenu"));
    assert!(!is_shown(&harness, "First child"));

    // A stationary pointer does not close a submenu opened with the keyboard, but moving it does.
    harness.get_by_label("First item").hover();
    harness.run();
    focus(&mut harness, "Second item");
    press(&mut harness, Key::ArrowDown);
    assert!(is_shown(&harness, "First child"));
    harness.get_by_label("Second item").hover();
    harness.run();
    assert!(!is_shown(&harness, "First child"));
}

#[test]
fn menu_controls_keep_their_arrow_keys() {
    let mut harness = Harness::new_ui_state(
        |ui, (text, value)| {
            ui.menu_button("Menu", |ui| {
                let label = ui.label("Text");
                ui.text_edit_multiline(text).labelled_by(label.id);
                ui.add(egui::Slider::new(value, 0.0..=10.0).text("Value"));
                let _ = ui.button("Last item");
            });
        },
        (String::from("first\nsecond"), 5.0),
    );
    open_navigation_menu(&mut harness);
    harness
        .get_by_role(egui::accesskit::Role::MultilineTextInput)
        .focus();
    harness.run();
    for key in [
        Key::ArrowLeft,
        Key::ArrowRight,
        Key::ArrowUp,
        Key::ArrowDown,
    ] {
        press(&mut harness, key);
        assert!(
            harness
                .get_by_role(egui::accesskit::Role::MultilineTextInput)
                .is_focused()
        );
    }
    harness.get_by_role(egui::accesskit::Role::Slider).focus();
    harness.run();
    press(&mut harness, Key::ArrowRight);
    assert!(
        harness
            .get_by_role(egui::accesskit::Role::Slider)
            .is_focused()
    );
    assert!(harness.state().1 > 5.0);
}

#[test]
fn arrow_navigation_outside_menus() {
    // The background keeps spatial navigation while a menu is open and after it closes.
    let mut harness = navigation_menu(false);
    open_navigation_menu(&mut harness);
    for _ in 0..2 {
        focus(&mut harness, "Behind 10 0");
        press(&mut harness, Key::ArrowRight);
        assert!(is_focused(&harness, "Behind 10 1"));
        press(&mut harness, Key::Escape);
        assert!(!is_shown(&harness, "First item"));
    }

    // A generic popup keeps spatial navigation.
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
    focus(&mut harness, "Popup item");
    press(&mut harness, Key::ArrowRight);
    assert!(is_focused(&harness, "Background"));

    // A menu in a modal navigates its entries, and the modal navigates once the menu closes.
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
    for to in ["First item", "Last item", "First item"] {
        press(&mut harness, Key::ArrowDown);
        assert!(is_focused(&harness, to));
    }
    press(&mut harness, Key::Escape);
    focus(&mut harness, "Menu");
    press(&mut harness, Key::ArrowDown);
    assert!(is_focused(&harness, "Other modal item"));

    // A menu below a modal does not take the modal's arrow keys.
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
    focus(&mut harness, "First modal item");
    press(&mut harness, Key::ArrowDown);
    assert!(is_focused(&harness, "Last modal item"));
}

#[test]
fn submenu_in_other_containers() {
    // A submenu in a generic popup returns to its button.
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
                // A sibling popup must not change which parent Left returns to.
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
    focus(&mut harness, "Hosted submenu");
    press(&mut harness, Key::Enter);
    assert!(is_focused(&harness, "Hosted child"));
    press(&mut harness, Key::ArrowLeft);
    assert!(is_focused(&harness, "Hosted submenu"));
    assert!(!is_shown(&harness, "Hosted child"));

    focus(&mut harness, "Earlier popup child");
    let earlier_layer = focused_layer(&harness);
    focus(&mut harness, "Host first");
    press(&mut harness, Key::ArrowRight);
    assert_eq!(focused_layer(&harness), earlier_layer);

    // A submenu directly in a menu bar keeps navigation inside the submenu.
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
    focus(&mut harness, "Standalone submenu");
    press(&mut harness, Key::ArrowRight);
    assert!(is_focused(&harness, "Standalone item"));
    press(&mut harness, Key::ArrowRight);
    assert!(is_focused(&harness, "Standalone item"));
    press(&mut harness, Key::ArrowLeft);
    assert!(is_focused(&harness, "Standalone submenu"));
}

/// A menu over a grid of background buttons. The state collects entries that gained focus.
fn navigation_menu(context_menu: bool) -> Harness<'static, Vec<String>> {
    Harness::builder()
        .with_size(egui::vec2(500.0, 400.0))
        .build_ui_state(
            move |ui, gained: &mut Vec<String>| {
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
                            let mut item = |ui: &mut Ui, label: &str| {
                                if ui.button(label).gained_focus() {
                                    gained.push(label.to_owned());
                                }
                            };
                            item(ui, "First item");
                            let _ = ui.add_enabled(false, egui::Button::new("Disabled item"));
                            item(ui, "Second item");
                            ui.menu_button("Submenu", |ui| {
                                let _ = ui.button("First child");
                                ui.menu_button("Nested submenu", |ui| {
                                    let _ = ui.button("Nested item");
                                });
                                let _ = ui.button("Last child");
                            });
                            item(ui, "Last item");
                        };
                        if context_menu {
                            ui.button("Menu").context_menu(content);
                        } else {
                            ui.menu_button("Menu", content);
                        }
                    },
                );
            },
            Vec::new(),
        )
}

fn open_navigation_menu<State>(harness: &mut Harness<'_, State>) {
    focus(harness, "Menu");
    press(harness, Key::Enter);
}

fn focus<State>(harness: &mut Harness<'_, State>, label: &str) {
    harness.get_by_label_contains(label).focus();
    harness.run();
}

fn press<State>(harness: &mut Harness<'_, State>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn is_focused<State>(harness: &Harness<'_, State>, label: &str) -> bool {
    harness.get_by_label_contains(label).is_focused()
}

fn is_shown<State>(harness: &Harness<'_, State>, label: &str) -> bool {
    harness.query_by_label(label).is_some()
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
