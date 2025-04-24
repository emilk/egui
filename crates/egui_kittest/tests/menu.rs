use egui::containers::menu::{Bar, MenuConfig, SubMenuButton};
use egui::{include_image, PopupCloseBehavior, Ui};
use egui_kittest::{Harness, SnapshotResults};
use kittest::Queryable as _;

struct TestMenu {
    config: MenuConfig,
    checked: bool,
}

impl TestMenu {
    fn new(config: MenuConfig) -> Self {
        Self {
            config,
            checked: false,
        }
    }

    fn ui(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            Bar::new().config(self.config.clone()).ui(ui, |ui| {
                egui::Sides::new().show(
                    ui,
                    |ui| {
                        ui.menu_button("Menu A", |ui| {
                            _ = ui.button("Button in Menu A");
                            ui.menu_button("Submenu A", |ui| {
                                for i in 0..4 {
                                    _ = ui.button(format!("Button {i} in Submenu A"));
                                }
                            });
                            ui.menu_image_text_button(
                                include_image!("../../eframe/data/icon.png"),
                                "Submenu B with icon",
                                |ui| {
                                    _ = ui.button("Button in Submenu B");
                                },
                            );
                            SubMenuButton::new("Submenu C (CloseOnClickOutside)")
                                .config(
                                    MenuConfig::new()
                                        .close_behavior(PopupCloseBehavior::CloseOnClickOutside),
                                )
                                .ui(ui, |ui| {
                                    _ = ui.button("Button in Submenu C");
                                    ui.checkbox(&mut self.checked, "Checkbox in Submenu C");
                                    ui.menu_button("Submenu D", |ui| {
                                        if ui
                                            .button("Button in Submenu D (close on click)")
                                            .clicked()
                                        {
                                            ui.close();
                                        };
                                    });
                                });
                        });
                        ui.menu_image_text_button(
                            include_image!("../../eframe/data/icon.png"),
                            "Menu B with icon",
                            |ui| {
                                _ = ui.button("Button in Menu B");
                            },
                        );
                        _ = ui.button("Menu Button");
                        ui.menu_button("Menu C", |ui| {
                            _ = ui.button("Button in Menu C");
                        });
                    },
                    |ui| {
                        ui.label("Some other label");
                    },
                );
            });
        });
    }

    fn into_harness(self) -> Harness<'static, Self> {
        Harness::builder()
            .with_size(egui::Vec2::new(500.0, 300.0))
            .build_ui_state(
                |ui, menu| {
                    egui_extras::install_image_loaders(ui.ctx());
                    menu.ui(ui);
                },
                self,
            )
    }
}

#[test]
fn menu_close_on_click_outside() {
    // We're intentionally setting CloseOnClick here so we can test if a submenu can override the
    // close behavior. (Note how Submenu C has CloseOnClickOutside set)
    let mut harness =
        TestMenu::new(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClick))
            .into_harness();

    harness.get_by_label("Menu A").simulate_click();
    harness.run();

    harness
        .get_by_label("Submenu C (CloseOnClickOutside)")
        .hover();
    harness.run();

    // We should be able to check the checkbox without closing the menu
    // Click a couple of times, just in case
    for expect_checked in [true, false, true, false] {
        harness
            .get_by_label("Checkbox in Submenu C")
            .simulate_click();
        harness.run();
        assert_eq!(expect_checked, harness.state().checked);
    }

    // Hovering outside should not close the menu
    harness.get_by_label("Some other label").hover();
    harness.run();
    assert!(harness.query_by_label("Checkbox in Submenu C").is_some());

    // Clicking outside should close the menu
    harness.get_by_label("Some other label").simulate_click();
    harness.run();
    assert!(harness.query_by_label("Checkbox in Submenu C").is_none());
}

#[test]
fn menu_close_on_click() {
    let mut harness =
        TestMenu::new(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClick))
            .into_harness();

    harness.get_by_label("Menu A").simulate_click();
    harness.run();

    harness.get_by_label("Submenu B with icon").hover();
    harness.run();

    // Clicking the button should close the menu (even if ui.close() is not called by the button)
    harness.get_by_label("Button in Submenu B").simulate_click();
    harness.run();
    assert!(harness.query_by_label("Button in Submenu B").is_none());
}

#[test]
fn clicking_submenu_button_should_never_close_menu() {
    // We test for this since otherwise the menu wouldn't work on touch devices
    // The other tests use .hover to open submenus, but this test explicitly uses .simulate_click
    let mut harness =
        TestMenu::new(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClick))
            .into_harness();

    harness.get_by_label("Menu A").simulate_click();
    harness.run();

    // Clicking the submenu button should not close the menu
    harness.get_by_label("Submenu B with icon").simulate_click();
    harness.run();

    harness.get_by_label("Button in Submenu B").simulate_click();
    harness.run();
    assert!(harness.query_by_label("Button in Submenu B").is_none());
}

#[test]
fn menu_snapshots() {
    let mut harness = TestMenu::new(MenuConfig::new()).into_harness();

    let mut results = SnapshotResults::new();

    harness.get_by_label("Menu A").hover();
    harness.run();
    results.add(harness.try_snapshot("menu/closed_hovered"));

    harness.get_by_label("Menu A").simulate_click();
    harness.run();
    results.add(harness.try_snapshot("menu/opened"));

    harness
        .get_by_label("Submenu C (CloseOnClickOutside)")
        .hover();
    harness.run();
    results.add(harness.try_snapshot("menu/submenu"));

    harness.get_by_label("Submenu D").hover();
    harness.run();
    results.add(harness.try_snapshot("menu/subsubmenu"));
}
