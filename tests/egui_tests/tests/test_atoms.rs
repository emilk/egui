use egui::{
    Align, Atom, AtomExt as _, AtomLayout, Button, Direction, Frame, Layout, TextWrapMode, Ui, Vec2,
};
use egui_kittest::{HarnessBuilder, SnapshotResult, SnapshotResults};

#[test]
fn test_atoms() {
    let mut results = SnapshotResults::new();

    results.add(single_test("max_width", |ui| {
        ui.add(Button::new((
            "max width not grow".atom_max_width(30.0),
            "other text",
        )));
    }));
    results.add(single_test("max_width_and_grow", |ui| {
        ui.add(Button::new((
            "max width and grow".atom_max_width(30.0).atom_grow(true),
            "other text",
        )));
    }));
    results.add(single_test("shrink_first_text", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new(("this should shrink", "this shouldn't")));
    }));
    results.add(single_test("shrink_last_text", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new((
            "this shouldn't shrink",
            "this should".atom_shrink(true),
        )));
    }));
    results.add(single_test("grow_all", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new((
            "I grow".atom_grow(true),
            "I also grow".atom_grow(true),
            "I grow as well".atom_grow(true),
        )));
    }));
    results.add(single_test("size_max_size", |ui| {
        ui.style_mut().wrap_mode = Some(TextWrapMode::Truncate);
        ui.add(Button::new((
            "size and max size"
                .atom_size(Vec2::new(80.0, 80.0))
                .atom_max_size(Vec2::new(20.0, 20.0)),
            "other text".atom_grow(true),
        )));
    }));
}

fn single_test(name: &str, mut f: impl FnMut(&mut Ui)) -> SnapshotResult {
    let mut harness = HarnessBuilder::default()
        .with_size(Vec2::new(400.0, 200.0))
        .build_ui(move |ui| {
            ui.label("Normal");
            let normal_width = ui.horizontal(&mut f).response.rect.width();

            ui.label("Justified");
            ui.with_layout(
                Layout::left_to_right(Align::Min).with_main_justify(true),
                &mut f,
            );

            ui.label("Shrunk");
            ui.scope(|ui| {
                ui.set_max_width(normal_width / 2.0);
                f(ui);
            });
        });

    harness.try_snapshot(name)
}

#[test]
fn test_intrinsic_size() {
    let widgets = [Ui::button, Ui::label];

    for widget in widgets {
        let mut intrinsic_size = None;
        for wrapping in [
            TextWrapMode::Extend,
            TextWrapMode::Wrap,
            TextWrapMode::Truncate,
        ] {
            _ = HarnessBuilder::default()
                .with_size(Vec2::new(100.0, 100.0))
                .build_ui(|ui| {
                    ui.style_mut().wrap_mode = Some(wrapping);
                    let response = widget(
                        ui,
                        "Hello world this is a long text that should be wrapped.",
                    );
                    if let Some(current_intrinsic_size) = intrinsic_size {
                        assert_eq!(
                            Some(current_intrinsic_size),
                            response.intrinsic_size(),
                            "For wrapping: {wrapping:?}"
                        );
                    }
                    assert!(
                        response.intrinsic_size().is_some(),
                        "intrinsic_size should be set for `Button`"
                    );
                    intrinsic_size = response.intrinsic_size();
                    if wrapping == TextWrapMode::Extend {
                        assert_eq!(Some(response.rect.size()), response.intrinsic_size());
                    }
                });
        }
    }
}

#[test]
fn test_button_shortcut_text() {
    let mut harness = HarnessBuilder::default().build_ui(|ui| {
        ui.add(egui::Button::new("Click me").shortcut_text(("1", "2", "3")));
    });
    harness.run();
    harness.fit_contents();

    harness.snapshot("button_shortcut");
}

/// Test atom nesting and [`egui::AtomLayout::direction`].
#[test]
fn test_atom_layout_nesting_and_direction() {
    let mut harness = HarnessBuilder::default().build_ui(|ui| {
        let style = ui.style();
        let canvas_frame = Frame::canvas(style);

        let button_frame = style
            .button_style(
                &egui::widget_style::Classes::default(),
                egui::widget_style::WidgetState::Inactive,
            )
            .frame;

        let row = |direction: Direction| {
            Atom::layout(
                AtomLayout::new(("one", "two", "three"))
                    .direction(direction)
                    .frame(button_frame),
            )
        };

        AtomLayout::new((
            Atom::layout(
                AtomLayout::new((
                    row(Direction::LeftToRight).atom_grow(true),
                    row(Direction::RightToLeft).atom_grow(true),
                ))
                .direction(Direction::TopDown),
            )
            .atom_grow(true),
            row(Direction::TopDown),
            row(Direction::BottomUp),
        ))
        .direction(Direction::LeftToRight)
        .frame(canvas_frame)
        .show(ui);
    });

    harness.fit_contents();

    harness.snapshot("atom_layout_nesting");
}

/// Tests the spacing between galleys.
/// All of these should look the same.
#[test]
fn test_atom_letter_spacing() {
    use egui::AtomLayout;

    let mut harness = HarnessBuilder::default().build_ui(|ui| {
        ui.add(AtomLayout::new("1.00x").gap(0.0));
        ui.add(AtomLayout::new(("1.00", "x")).gap(0.0));
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label("1.00");
            ui.label("x");
        });
    });
    harness.run();
    harness.fit_contents();

    harness.snapshot("atom_letter_spacing");
}

/// `AtomLayout::selectable(true)` should opt the layout into click+drag sensing
/// so its text can be selected, while the default layout stays inert.
/// See <https://github.com/emilk/egui/issues/8217>.
#[test]
fn test_atom_selectable_senses_click_and_drag() {
    use egui::{AtomLayout, Sense};

    let mut captured = (Sense::hover(), Sense::hover());
    {
        let mut harness = HarnessBuilder::default().build_ui(|ui| {
            let selectable = AtomLayout::new("selectable").selectable(true).show(ui);
            let default = AtomLayout::new("default").show(ui);
            captured = (selectable.response.sense, default.response.sense);
        });
        harness.run();
    }

    let (selectable_sense, default_sense) = captured;
    assert!(
        selectable_sense.senses_click() && selectable_sense.senses_drag(),
        "a selectable AtomLayout should sense clicks and drags"
    );
    assert!(
        !default_sense.senses_drag(),
        "a non-selectable AtomLayout should stay inert"
    );
}

/// Selecting the text of a `selectable` [`egui::AtomLayout`] and copying it should
/// yield the text, while a non-selectable one yields nothing.
/// See <https://github.com/emilk/egui/issues/8217>.
#[test]
fn test_atom_selectable_text_can_be_copied() {
    use egui::{AtomLayout, Event, Modifiers, OutputCommand, PointerButton, Pos2, Rect};
    use std::cell::Cell;

    fn copied_text(selectable: bool) -> Option<String> {
        let rect_cell = Cell::new(Rect::NOTHING);
        let mut harness = HarnessBuilder::default()
            .with_size(Vec2::new(400.0, 100.0))
            .build_ui(|ui| {
                let response = AtomLayout::new("selectable atoms")
                    .selectable(selectable)
                    .show(ui);
                rect_cell.set(response.response.rect);
            });
        harness.run();

        let rect = rect_cell.get();
        let left = Pos2::new(rect.left() + 1.0, rect.center().y);
        let right = Pos2::new(rect.right() - 1.0, rect.center().y);

        // Press at the start of the text and drag to the end to select all of it.
        harness.event(Event::PointerMoved(left));
        harness.event(Event::PointerButton {
            pos: left,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        });
        harness.run();
        harness.event(Event::PointerMoved(right));
        harness.run();

        // Copy, then read back the clipboard command produced by this frame.
        harness.event(Event::Copy);
        harness.step();

        harness
            .output()
            .platform_output
            .commands
            .iter()
            .find_map(|cmd| match cmd {
                OutputCommand::CopyText(text) => Some(text.clone()),
                _ => None,
            })
    }

    assert_eq!(
        copied_text(true).as_deref(),
        Some("selectable atoms"),
        "selectable atom text should be copyable after selecting it"
    );
    assert_eq!(
        copied_text(false),
        None,
        "non-selectable atom text should not be selectable"
    );
}
