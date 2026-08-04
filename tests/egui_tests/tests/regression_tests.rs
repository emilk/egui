use std::sync::Arc;

use egui::accesskit::Role;
#[cfg(debug_assertions)]
use egui::epaint::Shape;
use egui::style::ScrollAnimation;
use egui::text::{LayoutJob, TextWrapping};
use egui::{
    Align, Button, Color32, FontFamily, FontId, Image, Label, Layout, Rect, RichText, Sense,
    TextBuffer, TextFormat, TextWrapMode, Ui, Vec2, include_image, vec2,
};
use egui::{Pos2, ScrollArea};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

#[test]
fn image_button_should_have_alt_text() {
    let harness = Harness::new_ui(|ui| {
        _ = ui.button(
            Image::new(include_image!("../../../crates/eframe/data/icon.png")).alt_text("Egui"),
        );
    });

    harness.get_by_label("Egui");
}

#[test]
fn button_selected_should_announce_toggled_state() {
    use egui::accesskit::Toggled;

    let harness = Harness::new_ui(|ui| {
        ui.add(Button::new("Plain"));
        ui.add(Button::new("Off").selected(false));
        ui.add(Button::new("On").selected(true));
    });

    assert_eq!(
        harness.get_by_label("Plain").accesskit_node().toggled(),
        None,
        "a plain Button must not be announced as a toggle",
    );
    assert_eq!(
        harness.get_by_label("Off").accesskit_node().toggled(),
        Some(Toggled::False),
    );
    assert_eq!(
        harness.get_by_label("On").accesskit_node().toggled(),
        Some(Toggled::True),
    );
}

#[test]
fn hovering_should_preserve_text_format() {
    let mut harness = Harness::builder().with_size((200.0, 70.0)).build_ui(|ui| {
        ui.add(
            Label::new(
                RichText::new("Long text that should be elided and has lots of styling and is long enough to have multiple lines.")
                    .italics()
                    .underline()
                    .color(Color32::LIGHT_BLUE),
            )
            .wrap_mode(TextWrapMode::Truncate),
        );
    });

    harness.get_by_label_contains("Long text").hover();

    harness.run_steps(5);

    harness.snapshot("hovering_should_preserve_text_format");
}

#[test]
fn text_edit_rtl() {
    let mut text = "hello ".to_owned();
    let mut harness = Harness::builder().with_size((200.0, 50.0)).build_ui(|ui| {
        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
            _ = ui.button("right");
            ui.add(
                egui::TextEdit::singleline(&mut text)
                    .desired_width(10.0)
                    .clip_text(false),
            );
            _ = ui.button("left");
        });
    });

    harness.get_by_role(Role::TextInput).focus();
    harness.step();
    harness.snapshot("text_edit_rtl_0");

    harness.get_by_role(Role::TextInput).type_text("world");

    for i in 1..3 {
        harness.step();
        harness.snapshot(format!("text_edit_rtl_{i}"));
    }
}

#[test]
fn text_edit_halign() {
    let mut harness = Harness::builder().with_size((212.0, 212.0)).build_ui(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);

        fn layouter(halign: Align) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<egui::Galley> {
            move |ui: &egui::Ui, buf: &dyn egui::TextBuffer, wrap_width: f32| {
                let mut job = LayoutJob {
                    wrap: TextWrapping {
                        max_rows: 4,
                        max_width: wrap_width,
                        ..Default::default()
                    },
                    halign,
                    ..Default::default()
                };
                job.append(
                    buf.as_str(),
                    0.0,
                    TextFormat::simple(FontId::new(13.0, FontFamily::Proportional), Color32::GRAY),
                );
                ui.fonts_mut(|f| f.layout_job(job))
            }
        }

        for widget_alignment in [Align::Min, Align::Center, Align::Max] {
            ui.horizontal(|ui| {
                for text_alignment in [Align::LEFT, Align::Center, Align::RIGHT] {
                    ui.add_sized(
                        vec2(64.0, 64.0),
                        egui::TextEdit::multiline(&mut format!(
                            "{widget_alignment:?}\n+\n{text_alignment:?}",
                        ))
                        .layouter(&mut layouter(text_alignment))
                        .vertical_align(widget_alignment)
                        .horizontal_align(widget_alignment),
                    );
                }
            });
        }
    });

    harness.get_by_value("Center\n+\nCenter").focus();
    harness.step();
    harness.snapshot("text_edit_halign");
}

#[test]
fn text_edit_delay() {
    let mut text = String::new();
    let mut harness = Harness::builder().with_size((200.0, 50.0)).build_ui(|ui| {
        ui.style_mut().scroll_animation = ScrollAnimation::none();
        ui.add(egui::TextEdit::singleline(&mut text).hint_text("Write something"));
    });

    harness.get_by_role(Role::TextInput).focus();
    harness.step();
    harness.snapshot("text_edit_delay_0_empty");

    harness.get_by_role(Role::TextInput).type_text("h");

    // When the text is empty, and we show the hint text, there is a frame delay.
    harness.step();
    harness.snapshot("text_edit_delay_1_h_invisible");

    // Now it should be visible
    harness.step();
    harness.snapshot("text_edit_delay_2_h_visible");

    harness.get_by_role(Role::TextInput).type_text("i");

    // The "i" should immediately be visible without a delay
    harness.step();
    harness.snapshot("text_edit_delay_3_i_visible");

    // The next frame should exactly match the previous one
    harness.step();
    harness.snapshot("text_edit_delay_4_i_visible");
}

#[test]
fn text_edit_scroll() {
    let mut text = "1\n2\n3\n4\n".to_owned();
    let mut harness = Harness::builder().build_ui(|ui| {
        ScrollArea::vertical().max_height(40.0).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut text)
                    .desired_rows(2)
                    .hint_text("Write something"),
            );
        });
    });

    harness.fit_contents();

    harness.get_by_role(Role::MultilineTextInput).focus();
    harness.step();
    harness.snapshot("text_edit_scroll_0_focus");

    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text("5\n");

    // When the text is empty, and we show the hint text, there is a frame delay.
    harness.run();
    harness.snapshot("text_edit_scroll_1_5");
}

#[test]
fn combobox_should_have_value() {
    let harness = Harness::new_ui(|ui| {
        egui::ComboBox::from_label("Select an option")
            .selected_text("Option 1")
            .show_ui(ui, |_ui| {});
    });

    assert_eq!(
        harness.get_by_label("Select an option").value().as_deref(),
        Some("Option 1")
    );
}

/// This test ensures that `ui.response().interact(...)` works correctly.
///
/// This was broken, because there was an optimization in [`egui::Response::interact`]
/// which caused the [`Sense`] of the original response to flip-flop between `click` and `hover`
/// between frames.
///
/// See <https://github.com/emilk/egui/pull/7713> for more details.
#[test]
fn interact_on_ui_response_should_be_stable() {
    let mut first_frame = true;
    let mut click_count = 0;
    let mut harness = Harness::new_ui(|ui| {
        let ui_response = ui.response();
        if !first_frame {
            assert!(
                ui_response.sense.contains(Sense::click()),
                "ui.response() didn't have click sense even though we called interact(Sense::click()) last frame"
            );
        }

        // Add a label so we have something to click with kittest
        ui.add(
            Label::new("senseless label")
                .sense(Sense::hover())
                .selectable(false),
        );

        let click_response = ui_response.interact(Sense::click());
        if click_response.clicked() {
            click_count += 1;
        }
        first_frame = false;
    });

    for i in 0..=10 {
        harness.run_steps(i);
        harness.get_by_label("senseless label").click();
    }

    drop(harness);
    assert_eq!(click_count, 10, "We missed some clicks!");
}

#[cfg(debug_assertions)]
fn has_red_warning_rect(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|clipped| {
        matches!(
            &clipped.shape,
            Shape::Rect(rect_shape)
                if rect_shape.stroke.color == Color32::RED
        )
    })
}

/// A button that changes its text on hover, with the Id derived from the text.
/// This is a plausible bug: the widget keeps the same rect, but its Id changes
/// between frames because the label (and thus the Id salt) changes on hover.
/// The `warn_if_rect_changes_id` debug check should catch this.
#[test]
#[cfg(debug_assertions)]
fn warn_if_rect_changes_id() {
    let button_rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(100.0, 30.0));

    let mut harness = Harness::builder().with_size((200.0, 50.0)).build_ui(|ui| {
        ui.global_style_mut(|style| style.debug.warn_if_rect_changes_id = true);

        // Simulate a buggy widget whose Id depends on its label text,
        // and the label changes on hover:
        let is_hovered = ui.rect_contains_pointer(button_rect);
        let label = if is_hovered { "Hovering!" } else { "Click me" };
        let id = ui.id().with(label);
        let _response = ui.interact(button_rect, id, Sense::click());
    });

    // no hover — establishes stable prev_pass
    harness.step();
    assert!(
        !has_red_warning_rect(harness.output()),
        "Should not warn without hover"
    );

    // Move the pointer over the button
    harness.hover_at(button_rect.center());

    harness.step();
    assert!(
        has_red_warning_rect(harness.output()),
        "Should warn when a widget rect changes Id between passes"
    );
}

/// When a parent Ui's id changes (e.g. via `push_id` with a dynamic value),
/// all child widget ids shift too. This should NOT trigger `warn_if_rect_changes_id` because the
/// `parent_id` also changed — it's a cascading id shift, not a widget bug.
#[test]
#[cfg(debug_assertions)]
fn warn_if_rect_changes_id_false_positive_parent_shift() {
    use std::cell::Cell;

    let counter = Cell::new(0);
    let button_rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(100.0, 30.0));

    let mut harness = Harness::builder().with_size((200.0, 100.0)).build_ui(|ui| {
        ui.global_style_mut(|style| style.debug.warn_if_rect_changes_id = true);

        // push_id with a changing value causes the child Ui's id to shift,
        // which in turn shifts all widget ids inside it.
        ui.push_id(counter.get(), |ui| {
            let id = ui.id().with("my_widget");
            let _response = ui.interact(button_rect, id, Sense::click());
        });
    });

    // Frame 1: counter=0 — establishes prev_pass
    harness.step();
    assert!(
        !has_red_warning_rect(harness.output()),
        "Should not warn on first frame"
    );

    // Frame 2: counter=0 — prev_pass == this_pass
    harness.step();
    assert!(
        !has_red_warning_rect(harness.output()),
        "Should not warn when nothing changed"
    );

    // Now change the parent id, shifting all child widget ids
    counter.set(1);
    harness.step();

    assert!(
        !has_red_warning_rect(harness.output()),
        "Should NOT warn when parent Ui's id shifted (cascading id change)"
    );
}

#[test]
fn horizontal_wrapped_multiline_row_height() {
    let mut harness = Harness::builder().with_size((350.0, 300.0)).build_ui(|ui| {
        ui.style_mut().interaction.tooltip_delay = 0.0;
        ui.style_mut().interaction.show_tooltips_only_when_still = false;

        let mut string = String::new();

        ui.horizontal_wrapped(|ui| {
            ui.monospace("| ");
            let _ = ui.button("A");
            let _ = ui.button("B");
            ui.end_row();

            ui.monospace("| ");
            let _ = ui.button("C");
            let _ = ui.button("D");
            let _ = ui.button("E");
            ui.end_row();

            ui.monospace("| ");
            ui.text_edit_multiline(&mut string);
            ui.end_row();

            ui.monospace("| ");
            let _ = ui.button("F");
            let _ = ui.button("G");
            ui.end_row();

            ui.monospace("| ");
            let _ = ui.button("H");
            let _ = ui.button("I");
            let _ = ui.button("K");
            ui.end_row();
        });
    });

    harness.snapshot("horizontal_wrapped_multiline_row_height");
}

#[test]
fn horizontal_wrapped_multiline_row_height_reference() {
    let mut harness = Harness::builder().with_size((350.0, 300.0)).build_ui(|ui| {
        ui.style_mut().interaction.tooltip_delay = 0.0;
        ui.style_mut().interaction.show_tooltips_only_when_still = false;

        let mut string = String::new();

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.monospace("| ");
                let _ = ui.button("A");
                let _ = ui.button("B");
            });

            ui.horizontal(|ui| {
                ui.monospace("| ");
                let _ = ui.button("C");
                let _ = ui.button("D");
                let _ = ui.button("E");
            });

            ui.horizontal(|ui| {
                ui.monospace("| ");
                ui.text_edit_multiline(&mut string);
            });

            ui.horizontal(|ui| {
                ui.monospace("| ");
                let _ = ui.button("F");
                let _ = ui.button("G");
            });

            ui.horizontal(|ui| {
                ui.monospace("| ");
                let _ = ui.button("H");
                let _ = ui.button("I");
                let _ = ui.button("K");
            });
        });
    });

    harness.snapshot("horizontal_wrapped_multiline_row_height_reference");
}

#[test]
fn animated_scroll_beats_sticky_bottom() {
    let mut harness = Harness::builder()
        .with_size((200.0, 120.0))
        .with_max_steps(8)
        .build_ui_state(
            |ui, state: &mut (bool, f32, f32)| {
                ui.style_mut().scroll_animation = ScrollAnimation::duration(0.5);

                let output = ScrollArea::vertical()
                    .max_height(60.0)
                    .stick_to_bottom(true)
                    .animated(true)
                    .show(ui, |ui| {
                        for row in 0..40 {
                            let response = ui.label(format!("Row {row}"));
                            if state.0 && row == 0 {
                                response.scroll_to_me(Some(Align::TOP));
                                state.0 = false;
                            }
                        }
                    });

                state.1 = output.state.offset.y;
                state.2 = (output.content_size.y - output.inner_rect.height()).max(0.0);
            },
            (false, 0.0, 0.0),
        );

    assert!((harness.state().1 - harness.state().2).abs() <= 1.0);

    harness.state_mut().0 = true;
    harness.step();
    harness.run();

    assert!(
        harness.state().1 + 1.0 < harness.state().2,
        "animated explicit scroll should leave the sticky bottom"
    );
}

/// Tests that tooltips are shown correctly for buttons that are only shown on hover.
///
/// Basically, this tests that a tooltip overlapping the mouse cursor does not interfere with a
/// buttons hover state.
#[test]
fn tooltip_should_work_for_hover_button() {
    let button_rect = Rect::from_min_size(Pos2::new(4.0, 4.0), Vec2::new(80.0, 20.0));
    let mut harness = Harness::builder().with_size((320.0, 80.0)).build_ui(|ui| {
        if ui.rect_contains_pointer(button_rect) {
            ui.button("A tooltip should be shown")
                .on_hover_text("My tooltip");
        }
    });

    harness.hover_at(button_rect.center());

    harness.run();

    harness.snapshot("test_tooltip_hover_regression");
}

/// Ensure that hovering close to a widget doesn't cause a tooltip feedback loop (due to a
/// difference between `hovered` and `contains_pointer` caused by the interact radius).
#[test]
fn tooltip_covering_button_should_not_cause_feedback_loop() {
    let mut harness = Harness::builder().with_size((200.0, 30.0)).build_ui(|ui| {
        ui.button("A tooltip should be shown")
            .on_hover_text("This tooltip is larger than the button");
    });

    harness.hover_at(
        harness
            .get_by_label("A tooltip should be shown")
            .rect()
            .left_center()
            - Vec2::X,
    );

    harness.run();

    harness.snapshot("tooltip_covering_button_should_not_cause_feedback_loop");
}

/// Tests that a tooltip closes when the pointer moves onto a neighboring widget,
/// so that the neighbor can show its own tooltip.
///
/// The two buttons are only `item_spacing.y` (3 pt) apart, which is less than the
/// hit-test `interact_radius` (5 pt), so the first button is still close enough to
/// interact with when the pointer is on the second one.
#[test]
fn tooltip_should_hand_over_to_neighboring_widget() {
    let mut harness = Harness::builder().with_size((300.0, 200.0)).build_ui(|ui| {
        ui.button("Button A").on_hover_text("Tooltip A");
        ui.button("Button B").on_hover_text("Tooltip B");
    });

    let a_rect = harness.get_by_label("Button A").rect();
    let b_rect = harness.get_by_label("Button B").rect();

    harness.hover_at(a_rect.center_bottom() - Vec2::Y);
    harness.run();
    assert!(
        harness.query_by_label("Tooltip A").is_some(),
        "Tooltip A should be shown when hovering Button A"
    );

    harness.hover_at(b_rect.center_top() + Vec2::Y);
    harness.run();
    assert!(
        harness.query_by_label("Tooltip B").is_some(),
        "Tooltip B should be shown when hovering Button B"
    );
    assert!(
        harness.query_by_label("Tooltip A").is_none(),
        "Tooltip A should be hidden when hovering Button B"
    );
}

/// When a window is minimized or occluded, the integration runs no pass at all,
/// and instead ticks the app logic with [`egui::Context::run_logic`].
///
/// Such a tick must leave all ui state alone. Otherwise areas think they were hidden and
/// replay their fade-in, popups close, focus is lost, and child viewports pop back up.
/// See <https://github.com/emilk/egui/issues/8266>.
#[test]
fn run_logic_should_not_disturb_ui_state() {
    const MENU: &str = "My menu";
    const MENU_ITEM: &str = "Button in my menu";
    const FOCUSED_BUTTON: &str = "Click me";

    let child_viewport = egui::ViewportId::from_hash_of("My child viewport");
    let area_id = egui::Id::new("My area");
    let area_layer = egui::LayerId::new(egui::Order::Middle, area_id);

    let mut harness = Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        .build_ui(move |ui| {
            // A backend that can open real windows, like eframe:
            ui.ctx().set_embed_viewports(false);

            ui.ctx()
                .show_viewport_deferred(child_viewport, Default::default(), |_ui, _class| {});

            ui.menu_button(MENU, |ui| {
                _ = ui.button(MENU_ITEM);
            });

            egui::Area::new(area_id)
                .fixed_pos((150.0, 120.0))
                .show(ui.ctx(), |ui| {
                    _ = ui.button(FOCUSED_BUTTON);
                });
        });

    harness.get_by_label(MENU).click();
    harness.run();
    // Nothing asks for focus again, so the test fails if egui ever loses it:
    harness.get_by_label(FOCUSED_BUTTON).focus();
    harness.run();

    let assert_state = |harness: &Harness<'_>| {
        assert!(
            harness
                .get_by_label(FOCUSED_BUTTON)
                .accesskit_node()
                .is_focused(),
            "The button lost focus"
        );
        harness.get_by_label(MENU_ITEM); // Panics if the menu closed
        assert!(
            harness
                .ctx
                .memory(|m| m.areas().visible_last_frame(&area_layer)),
            "Area state was reset"
        );
        assert!(
            harness
                .ctx
                .viewport_for(child_viewport, |viewport| viewport.class)
                == egui::ViewportClass::Deferred,
            "The child viewport was closed"
        );
    };

    assert_state(&harness);

    // The window is now occluded, so the integration runs no pass,
    // and only ticks the app logic:
    for i in 0..2 {
        let time = 100.0 + f64::from(i);
        let mut raw_input = egui::RawInput {
            time: Some(time),
            ..Default::default()
        };
        raw_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .occluded = Some(true);

        let output = harness.ctx.run_logic(&raw_input, |ctx| {
            assert_eq!(
                ctx.input(|i| i.viewport().occluded),
                Some(true),
                "App logic should be able to tell that the window is occluded"
            );
            assert!(
                ctx.input(|i| i.time) != time,
                "The ui input should not be interpreted: it is for the next pass"
            );

            // The app asks to be shown again:
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        });

        assert_eq!(
            output
                .viewport_commands
                .into_values()
                .flatten()
                .collect::<Vec<_>>(),
            vec![egui::ViewportCommand::Focus],
            "The integration should receive the command, even though there was no pass"
        );

        assert_state(&harness);
    }

    // The window is visible again, and everything should be where we left it:
    harness.run();

    assert_state(&harness);
}
