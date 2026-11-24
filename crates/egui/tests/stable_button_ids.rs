use egui::{
    Button, Context, Id, Popup, RawInput,
    accesskit::{Action, ActionRequest, TreeId},
    menu::MenuButton,
};

fn click_request(id: Id) -> egui::Event {
    egui::Event::AccessKitActionRequest(ActionRequest {
        action: Action::Click,
        target_tree: TreeId::ROOT,
        target_node: id.accesskit_id(),
        data: None,
    })
}

#[test]
fn stable_button_ids_keep_accessibility_actions_on_the_right_button() {
    let ctx = Context::default();
    let mut previous_confirm_id = None;

    for frame in 0..2 {
        let events = if frame == 1 {
            vec![click_request(previous_confirm_id.expect("first frame must assign an ID"))]
        } else {
            Vec::new()
        };

        ctx.run_ui(
            RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| {
                    if frame == 0 {
                        ui.spinner();
                    }

                    let confirm = ui.add(Button::new("Confirm").id_salt("confirm"));
                    let cancel = ui.add(Button::new("Cancel").id_salt("cancel"));

                    assert_eq!(confirm.id, ui.make_persistent_id("confirm"));
                    assert_eq!(cancel.id, ui.make_persistent_id("cancel"));
                    assert_ne!(confirm.id, cancel.id);
                    if frame == 0 {
                        previous_confirm_id = Some(confirm.id);
                    } else {
                        assert_eq!(Some(confirm.id), previous_confirm_id);
                        assert!(confirm.clicked(), "the Confirm action must reach Confirm");
                        assert!(!cancel.clicked(), "the Confirm action must not reach Cancel");
                    }
                });
            },
        )
        .drop_without_applying_deltas();
    }
}

#[test]
fn menu_button_id_and_popup_state_survive_preceding_widgets_disappearing() {
    let ctx = Context::default();
    let mut menu_id = None;
    let mut popup_id = None;

    for frame in 0..3 {
        let events = if frame == 1 {
            vec![click_request(menu_id.expect("first frame must assign an ID"))]
        } else {
            Vec::new()
        };

        ctx.run_ui(
            RawInput {
                events,
                ..Default::default()
            },
            |ui| {
                ui.horizontal(|ui| {
                    if frame != 2 {
                        ui.spinner();
                    }

                    let (menu, _) = MenuButton::new("Actions")
                        .id_salt("actions")
                        .ui(ui, |ui| {
                            ui.label("Action");
                        });

                    assert_eq!(menu.id, ui.make_persistent_id("actions"));
                    let id = Popup::default_response_id(&menu);
                    if frame == 0 {
                        menu_id = Some(menu.id);
                        popup_id = Some(id);
                    } else {
                        assert_eq!(Some(menu.id), menu_id);
                        assert_eq!(Some(id), popup_id);
                    }
                });
            },
        )
        .drop_without_applying_deltas();

        if frame > 0 {
            assert!(
                Popup::is_id_open(&ctx, popup_id.expect("first frame must assign popup ID")),
                "the menu should stay open after its preceding spinner disappears"
            );
        }
    }
}
