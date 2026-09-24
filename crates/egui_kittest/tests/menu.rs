use egui::containers::menu::{MenuBar, MenuConfig, SubMenuButton};
use egui::{PopupCloseBehavior, Ui, include_image};
use egui_kittest::Harness;
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
            MenuBar::new().config(self.config.clone()).ui(ui, |ui| {
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
                                        }
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

    harness.get_by_label("Menu A").click();
    harness.run();

    harness
        .get_by_label_contains("Submenu C (CloseOnClickOutside)")
        .hover();
    harness.run();

    // We should be able to check the checkbox without closing the menu
    // Click a couple of times, just in case
    for expect_checked in [true, false, true, false] {
        harness.get_by_label("Checkbox in Submenu C").click();
        harness.run();
        assert_eq!(expect_checked, harness.state().checked);
    }

    // Hovering outside should not close the menu
    harness.get_by_label("Some other label").hover();
    harness.run();
    assert!(harness.query_by_label("Checkbox in Submenu C").is_some());

    // Clicking outside should close the menu
    harness.get_by_label("Some other label").click();
    harness.run();
    assert!(harness.query_by_label("Checkbox in Submenu C").is_none());
}

#[test]
fn menu_close_on_click() {
    let mut harness =
        TestMenu::new(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClick))
            .into_harness();

    harness.get_by_label("Menu A").click();
    harness.run();

    harness.get_by_label_contains("Submenu B with icon").hover();
    harness.run();

    // Clicking the button should close the menu (even if ui.close() is not called by the button)
    harness.get_by_label("Button in Submenu B").click();
    harness.run();
    assert!(harness.query_by_label("Button in Submenu B").is_none());
}

#[test]
fn clicking_submenu_button_should_never_close_menu() {
    // We test for this since otherwise the menu wouldn't work on touch devices
    // The other tests use .hover to open submenus, but this test explicitly uses .click
    let mut harness =
        TestMenu::new(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClick))
            .into_harness();

    harness.get_by_label("Menu A").click();
    harness.run();

    // Clicking the submenu button should not close the menu
    harness.get_by_label_contains("Submenu B with icon").click();
    harness.run();

    harness.get_by_label("Button in Submenu B").click();
    harness.run();
    assert!(harness.query_by_label("Button in Submenu B").is_none());
}

#[cfg(feature = "snapshot")]
#[test]
fn menu_snapshots() {
    let mut harness = TestMenu::new(MenuConfig::new()).into_harness();

    let mut results = egui_kittest::SnapshotResults::new();

    harness.get_by_label("Menu A").hover();
    harness.run();
    results.add(harness.try_snapshot("menu/closed_hovered"));

    harness.get_by_label("Menu A").click();
    harness.run();
    results.add(harness.try_snapshot("menu/opened"));

    harness
        .get_by_label_contains("Submenu C (CloseOnClickOutside)")
        .hover();
    harness.run();
    results.add(harness.try_snapshot("menu/submenu"));

    harness.get_by_label_contains("Submenu D").hover();
    harness.run();
    results.add(harness.try_snapshot("menu/subsubmenu"));
}

#[test]
fn submenu_respects_custom_style() {
    const FILL: egui::Color32 = egui::Color32::from_rgb(123, 45, 67);
    const CORNER_RADIUS: egui::CornerRadius = egui::CornerRadius::same(13);
    const STROKE: egui::Stroke = egui::Stroke {
        width: 3.0,
        color: egui::Color32::GREEN,
    };

    let mut harness = Harness::builder()
        .with_size(egui::vec2(500.0, 300.0))
        .build_ui(|ui| {
            MenuBar::new().ui(ui, |ui| {
                ui.menu_button("Menu", |ui| {
                    SubMenuButton::new("Styled submenu")
                        .config(MenuConfig::new().style(|style: &mut egui::Style| {
                            style.visuals.window_fill = FILL;
                            style.visuals.window_stroke = STROKE;
                            style.visuals.menu_corner_radius = CORNER_RADIUS;
                        }))
                        .ui(ui, |ui| {
                            assert_eq!(ui.visuals().window_fill(), FILL);
                            ui.label("I should have a thick green outline and red fill");
                            ui.menu_button("Sub-submenu", |ui| {
                                assert_eq!(ui.visuals().window_fill(), FILL);
                                ui.label("I should inherit the style");
                            });
                        });
                });
            });
        });

    harness.get_by_label("Menu").click();
    harness.run();
    harness.get_by_label_contains("Styled submenu").hover();
    harness.run();
    assert!(
        harness
            .query_by_label("I should have a thick green outline and red fill")
            .is_some()
    );
    harness.get_by_label_contains("Sub-submenu").hover();
    harness.run();
    assert!(harness.query_by_label("I should inherit the style").is_some());

    harness.fit_contents();

    harness.snapshot("submenu_style");
}
