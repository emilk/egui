//! Tests for [`egui::TabBar`] and [`egui::Tab`].

use egui::accesskit::{Role, Toggled};
use egui::{Atom, AtomExt as _, Image, ImageSource, Key, Tab, TabBar, Vec2};
use egui_kittest::{
    Harness,
    kittest::{NodeT as _, Queryable as _},
};

const EFRAME_ICON: ImageSource<'static> =
    egui::include_image!("../../../crates/eframe/data/icon.png");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Page {
    Home,
    Settings,
    About,
}

/// A tab bar with three tabs, and the name of the selected page below it.
fn harness() -> Harness<'static, Page> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 60.0))
        .build_ui_state(
            |ui, page: &mut Page| {
                TabBar::new().show(ui, |ui| {
                    ui.tab_value(page, Page::Home, "Home");
                    ui.tab_value(page, Page::Settings, "Settings");
                    ui.tab_value(page, Page::About, "About");
                });
                ui.label(format!("Page: {page:?}"));
            },
            Page::Home,
        );
    harness.run();
    harness
}

/// A tab bar with more tabs than fit in the given width. The state is the selected tab.
fn overflowing_harness(width: f32) -> Harness<'static, usize> {
    overflowing_harness_with_height(width, 40.0)
}

fn overflowing_harness_with_height(width: f32, height: f32) -> Harness<'static, usize> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, height))
        .build_ui_state(
            |ui, selected: &mut usize| {
                TabBar::new().show(ui, |ui| {
                    for i in 0..10 {
                        ui.tab_value(selected, i, format!("Tab {i}"));
                    }
                });
            },
            0,
        );
    harness.run();
    harness
}

fn is_selected(harness: &Harness<'_, impl Sized>, label: &str) -> bool {
    harness
        .get_by_role_and_label(Role::Tab, label)
        .accesskit_node()
        .toggled()
        == Some(Toggled::True)
}

#[test]
fn clicking_a_tab_selects_it() {
    let mut harness = harness();
    harness.get_by_label("Page: Home");

    harness.get_by_role_and_label(Role::Tab, "Settings").click();
    harness.run();

    assert_eq!(*harness.state(), Page::Settings);
    harness.get_by_label("Page: Settings");
    harness.snapshot("tab_bar/second_tab_selected");
}

#[test]
fn only_the_selected_tab_is_reported_as_selected() {
    let harness = harness();
    assert!(is_selected(&harness, "Home"));
    assert!(!is_selected(&harness, "Settings"));
    assert!(!is_selected(&harness, "About"));
}

#[test]
fn the_tabs_are_children_of_a_tab_list() {
    let harness = harness();
    let tab_list = harness.get_by_role(Role::TabList);
    assert_eq!(tab_list.query_all_by_role(Role::Tab).count(), 3);
}

#[test]
fn only_a_click_on_another_tab_marks_the_response_changed() {
    let mut selected = 0;
    let mut changed = [false; 2];
    let mut harness = Harness::new_ui(|ui| {
        TabBar::new().show(ui, |ui| {
            for (i, changed) in changed.iter_mut().enumerate() {
                *changed |= ui.tab_value(&mut selected, i, format!("Tab {i}")).changed();
            }
        });
    });
    harness.run();

    harness.get_by_label("Tab 0").click();
    harness.run();
    harness.get_by_label("Tab 1").click();
    harness.run();
    drop(harness);

    assert_eq!(
        changed,
        [false, true],
        "clicking the tab that is already selected changes nothing"
    );
    assert_eq!(selected, 1);
}

#[test]
fn arrow_keys_move_the_focus_and_enter_selects() {
    let mut harness = harness();
    harness.get_by_role_and_label(Role::Tab, "Home").focus();
    harness.run();

    harness.key_press(Key::ArrowRight);
    harness.run();

    assert!(
        harness
            .get_by_role_and_label(Role::Tab, "Settings")
            .is_focused()
    );
    assert_eq!(
        *harness.state(),
        Page::Home,
        "moving the focus does not select the tab"
    );
    harness.snapshot("tab_bar/second_tab_focused");

    harness.key_press(Key::Enter);
    harness.run();

    assert_eq!(*harness.state(), Page::Settings);
}

#[test]
fn a_disabled_tab_cannot_be_selected() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 40.0))
        .build_ui_state(
            |ui, page: &mut Page| {
                TabBar::new().show(ui, |ui| {
                    ui.tab_value(page, Page::Home, "Home");
                    ui.add_enabled_ui(false, |ui| {
                        ui.tab_value(page, Page::Settings, "Settings");
                    });
                });
            },
            Page::Home,
        );
    harness.run();

    harness.get_by_role_and_label(Role::Tab, "Settings").click();
    harness.run();

    assert_eq!(*harness.state(), Page::Home);
    harness.snapshot("tab_bar/disabled_tab");
}

/// A row of tabs lines up with a row of buttons.
#[test]
fn a_tab_is_as_tall_as_a_button() {
    let mut harness = Harness::new_ui(|ui| {
        ui.horizontal(|ui| {
            _ = ui.add(Tab::new(true, "Selected tab"));
            _ = ui.add(Tab::new(false, "Tab"));
            _ = ui.button("Button");
        });
    });
    harness.run();

    let button_height = harness.get_by_label("Button").rect().height();
    assert_eq!(
        harness.get_by_label("Selected tab").rect().height(),
        button_height
    );
    assert_eq!(harness.get_by_label("Tab").rect().height(), button_height);
}

/// Hovering or focusing a tab paints a frame, but must not move its neighbors.
#[test]
fn interacting_with_a_tab_does_not_shift_the_layout() {
    let mut harness = harness();
    let rects = |harness: &Harness<'_, Page>| {
        ["Home", "Settings", "About"].map(|label| harness.get_by_label(label).rect())
    };
    let at_rest = rects(&harness);

    harness.get_by_label("Settings").hover();
    harness.run();
    assert_eq!(rects(&harness), at_rest, "hovering shifted the tabs");

    harness.get_by_label("Settings").focus();
    harness.run();
    assert_eq!(rects(&harness), at_rest, "focusing shifted the tabs");
}

/// The line below the tabs spans the whole parent, not only the tabs.
#[test]
fn the_bar_fills_the_available_width() {
    let mut width = 0.0;
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 40.0))
        .build_ui(|ui| {
            let available_width = ui.available_width();
            let response = TabBar::new()
                .show(ui, |ui| ui.add(Tab::new(true, "Only")))
                .response;
            assert_eq!(response.rect.width(), available_width);
            width = response.rect.width();
        });
    harness.run();
    drop(harness);

    assert!(250.0 < width, "the bar is only {width} wide");
}

#[test]
fn too_many_tabs_do_not_widen_the_parent() {
    let mut harness = overflowing_harness(200.0);
    assert!(
        200.0 < harness.get_by_label("Tab 9").rect().right(),
        "the tabs should not fit, or this test tests nothing"
    );
    assert!(
        harness.ctx.globally_used_rect().right() <= 200.0,
        "the bar overflows: {:?}",
        harness.ctx.globally_used_rect()
    );
    harness.snapshot("tab_bar/overflow");
}

/// Walking the tabs with the arrow keys brings each of them into view.
#[test]
fn focusing_a_hidden_tab_scrolls_it_into_view() {
    let mut harness = overflowing_harness(200.0);
    assert!(
        200.0 < harness.get_by_label("Tab 9").rect().left(),
        "the last tab should start out hidden"
    );

    harness.get_by_label("Tab 0").focus();
    harness.run();
    for _ in 0..9 {
        harness.key_press(Key::ArrowRight);
        harness.run();
    }

    let last_tab = harness.get_by_label("Tab 9");
    assert!(last_tab.is_focused());
    assert!(
        0.0 <= last_tab.rect().left() && last_tab.rect().right() <= 200.0,
        "the focused tab is not in view: {:?}",
        last_tab.rect()
    );

    harness.key_press(Key::Enter);
    harness.run();

    assert_eq!(*harness.state(), 9);
    harness.snapshot("tab_bar/overflow_scrolled_to_last_tab");
}

#[test]
fn the_list_button_only_shows_when_the_tabs_do_not_fit() {
    let harness = harness();
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Show all tabs")
            .is_none()
    );

    let harness = overflowing_harness(200.0);
    let button = harness.get_by_role_and_label(Role::Button, "Show all tabs");
    assert!(
        button.rect().right() <= 200.0,
        "the button is outside the bar: {:?}",
        button.rect()
    );
}

/// The tabs only just fit, so making room for the button would make them not fit.
/// The button must not come and go because of that.
#[test]
fn the_list_button_does_not_flicker_when_the_tabs_only_just_fit() {
    for width in 150..260 {
        let mut harness = Harness::builder()
            .with_size(Vec2::new(width as f32, 40.0))
            .build_ui_state(
                |ui, page: &mut Page| {
                    TabBar::new().show(ui, |ui| {
                        ui.tab_value(page, Page::Home, "Home");
                        ui.tab_value(page, Page::Settings, "Settings");
                        ui.tab_value(page, Page::About, "About");
                    });
                },
                Page::Home,
            );
        harness.run();

        let shown = |harness: &Harness<'_, Page>| {
            harness
                .query_by_role_and_label(Role::Button, "Show all tabs")
                .is_some()
        };
        let before = shown(&harness);
        for _ in 0..4 {
            harness.step();
            assert_eq!(shown(&harness), before, "flickering at width {width}");
        }
    }
}

#[test]
fn picking_a_tab_from_the_list_selects_it_and_scrolls_to_it() {
    // Tall enough for the list:
    let mut harness = overflowing_harness_with_height(200.0, 260.0);

    harness
        .get_by_role_and_label(Role::Button, "Show all tabs")
        .click();
    harness.run();
    harness.snapshot("tab_bar/list_of_all_tabs");

    // The tab itself is also called "Tab 7", so pick the one that is a button:
    harness.get_by_role_and_label(Role::Button, "Tab 7").click();
    harness.run();

    assert_eq!(*harness.state(), 7);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Tab 7")
            .is_none(),
        "the list should close after picking a tab"
    );
    let tab = harness.get_by_role_and_label(Role::Tab, "Tab 7");
    assert!(
        0.0 <= tab.rect().left() && tab.rect().right() <= 200.0,
        "the picked tab is not in view: {:?}",
        tab.rect()
    );
}

#[test]
fn a_disabled_tab_cannot_be_picked_from_the_list() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(160.0, 200.0))
        .build_ui_state(
            |ui, selected: &mut usize| {
                TabBar::new().show(ui, |ui| {
                    for i in 0..6 {
                        ui.add_enabled_ui(i != 4, |ui| {
                            ui.tab_value(selected, i, format!("Tab {i}"));
                        });
                    }
                });
            },
            0,
        );
    harness.run();

    harness
        .get_by_role_and_label(Role::Button, "Show all tabs")
        .click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Tab 4").click();
    harness.run();

    assert_eq!(*harness.state(), 0);
}

/// Images and text are laid out like on a button, and a hovered tab gets a frame.
#[test]
fn tabs_with_atoms() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(340.0, 40.0))
        .build_ui(|ui| {
            egui_extras::install_image_loaders(ui.ctx());
            let image = Image::new(EFRAME_ICON)
                .alt_text("eframe")
                .atom_size(Vec2::splat(14.0));

            TabBar::new().show(ui, |ui| {
                ui.add(Tab::new(true, (image, "Image and text")));
                ui.add(Tab::new(false, ("Left", Atom::grow(), "Right")));
                ui.add(Tab::new(false, "Hovered"));
            });
        });
    harness.run();

    harness.get_by_role_and_label(Role::Tab, "Image and text");
    harness.get_by_role_and_label(Role::Tab, "Left Right");

    harness.get_by_label("Hovered").hover();
    harness.run();
    harness.snapshot("tab_bar/atoms");
}

/// Each bar keeps its own scroll offset, and its own selection.
#[test]
fn two_tab_bars_are_independent() {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(200.0, 70.0))
        .build_ui_state(
            |ui, selected: &mut [usize; 2]| {
                for (bar, selected) in selected.iter_mut().enumerate() {
                    TabBar::new().id_salt(bar).show(ui, |ui| {
                        for i in 0..10 {
                            ui.tab_value(selected, i, format!("Tab {bar}.{i}"));
                        }
                    });
                }
            },
            [0, 0],
        );
    harness.run();

    harness.get_by_label("Tab 1.0").focus();
    harness.run();
    for _ in 0..9 {
        harness.key_press(Key::ArrowRight);
        harness.run();
    }
    harness.key_press(Key::Enter);
    harness.run();

    assert_eq!(*harness.state(), [0, 9]);
    assert!(
        harness.get_by_label("Tab 0.0").rect().left() >= 0.0,
        "the first bar should not have scrolled"
    );
    assert!(
        harness.get_by_label("Tab 1.9").rect().right() <= 200.0,
        "the second bar should have scrolled"
    );
    harness.snapshot("tab_bar/two_bars");
}
