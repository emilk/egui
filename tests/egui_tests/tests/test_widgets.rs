#![expect(clippy::unwrap_used)] // it's a test

use egui::accesskit::Role;
use egui::load::SizedTexture;
use egui::{
    Align, AtomExt as _, AtomLayout, Button, Color32, ColorImage, Direction, DragValue, Event,
    Grid, IntoAtoms as _, Layout, PointerButton, Response, Slider, Stroke, StrokeKind, TextEdit,
    TextWrapMode, TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, Widget as _, include_image,
};
use egui_kittest::kittest::{Queryable as _, by};
use egui_kittest::{Harness, Node, SnapshotResult, SnapshotResults};

#[test]
fn widget_tests() {
    let mut results = SnapshotResults::new();

    test_widget("button", |ui| ui.button("Button"), &mut results);
    test_widget(
        "button_image",
        |ui| {
            Button::image_and_text(
                include_image!("../../../crates/eframe/data/icon.png"),
                "Button",
            )
            .ui(ui)
        },
        &mut results,
    );
    test_widget(
        "button_image_shortcut",
        |ui| {
            Button::image_and_text(
                include_image!("../../../crates/eframe/data/icon.png"),
                "Open",
            )
            .shortcut_text("⌘O")
            .ui(ui)
        },
        &mut results,
    );
    results.add(VisualTests::test("button_image_shortcut_selected", |ui| {
        Button::image_and_text(
            include_image!("../../../crates/eframe/data/icon.png"),
            "Open",
        )
        .shortcut_text("⌘O")
        .selected(true)
        .ui(ui)
    }));

    test_widget(
        "selectable_value",
        |ui| ui.selectable_label(false, "Selectable"),
        &mut results,
    );
    test_widget(
        "selectable_value_selected",
        |ui| ui.selectable_label(true, "Selectable"),
        &mut results,
    );

    test_widget(
        "checkbox",
        |ui| ui.checkbox(&mut false, "Checkbox"),
        &mut results,
    );
    test_widget(
        "checkbox_checked",
        |ui| ui.checkbox(&mut true, "Checkbox"),
        &mut results,
    );
    test_widget("radio", |ui| ui.radio(false, "Radio"), &mut results);
    test_widget("radio_checked", |ui| ui.radio(true, "Radio"), &mut results);

    test_widget(
        "drag_value",
        |ui| DragValue::new(&mut 12.0).ui(ui),
        &mut results,
    );

    test_widget(
        "text_edit",
        |ui| {
            ui.spacing_mut().text_edit_width = 45.0;
            ui.text_edit_singleline(&mut "Hi!".to_owned())
        },
        &mut results,
    );
    test_widget(
        "text_edit_clip",
        |ui| {
            ui.spacing_mut().text_edit_width = 45.0;
            TextEdit::singleline(&mut "This is a very very long text".to_owned())
                .clip_text(true)
                .ui(ui)
        },
        &mut results,
    );
    test_widget(
        "text_edit_no_clip",
        |ui| {
            ui.spacing_mut().text_edit_width = 45.0;
            TextEdit::singleline(&mut "This is a very very long text".to_owned())
                .clip_text(false)
                .ui(ui)
        },
        &mut results,
    );
    test_widget(
        "text_edit_placeholder_clip",
        |ui| {
            ui.spacing_mut().text_edit_width = 45.0;
            TextEdit::singleline(&mut String::new())
                .hint_text("This is a very very long placeholder")
                .clip_text(true)
                .ui(ui)
        },
        &mut results,
    );

    test_widget(
        "slider",
        |ui| {
            ui.spacing_mut().slider_width = 45.0;
            Slider::new(&mut 12.0, 0.0..=100.0).ui(ui)
        },
        &mut results,
    );

    let source = include_image!("../../../crates/eframe/data/icon.png");
    let interesting_atoms = vec![
        ("minimal", ("Hello World!").into_atoms()),
        (
            "image",
            (source.clone().atom_max_height(12.0), "With Image").into_atoms(),
        ),
        (
            "multi_grow",
            ("g".atom_grow(true), "2", "g".atom_grow(true), "4").into_atoms(),
        ),
    ];

    for atoms in interesting_atoms {
        results.add(test_widget_layout(&format!("atoms_{}", atoms.0), |ui| {
            AtomLayout::new(atoms.1.clone()).ui(ui)
        }));
    }
}

fn test_widget(name: &str, mut w: impl FnMut(&mut Ui) -> Response, results: &mut SnapshotResults) {
    results.add(test_widget_layout(name, &mut w));
    results.add(VisualTests::test(name, &mut w));
}

fn test_widget_layout(name: &str, mut w: impl FnMut(&mut Ui) -> Response) -> SnapshotResult {
    let test_size = Vec2::new(110.0, 45.0);

    struct Row {
        main_dir: Direction,
        main_align: Align,
        main_justify: bool,
    }

    struct Col {
        cross_align: Align,
        cross_justify: bool,
    }

    let mut rows = Vec::new();
    let mut cols = Vec::new();

    for main_justify in [false, true] {
        for main_dir in [
            Direction::LeftToRight,
            Direction::TopDown,
            Direction::RightToLeft,
            Direction::BottomUp,
        ] {
            for main_align in [Align::Min, Align::Center, Align::Max] {
                rows.push(Row {
                    main_dir,
                    main_align,
                    main_justify,
                });
            }
        }
    }

    for cross_justify in [false, true] {
        for cross_align in [Align::Min, Align::Center, Align::Max] {
            cols.push(Col {
                cross_align,
                cross_justify,
            });
        }
    }

    let mut harness = Harness::builder().build_ui(|ui| {
        egui_extras::install_image_loaders(ui.ctx());

        {
            let mut wrap_test_size = test_size;
            wrap_test_size.x /= 3.0;
            ui.heading("Wrapping");

            let modes = [
                TextWrapMode::Extend,
                TextWrapMode::Truncate,
                TextWrapMode::Wrap,
            ];
            Grid::new("wrapping")
                .spacing(Vec2::new(test_size.x / 2.0, 4.0))
                .show(ui, |ui| {
                    for mode in &modes {
                        ui.label(format!("{mode:?}"));
                    }
                    ui.end_row();

                    for mode in &modes {
                        let (_, rect) = ui.allocate_space(wrap_test_size);

                        let mut child_ui = ui.new_child(UiBuilder::new().max_rect(rect));
                        child_ui.style_mut().wrap_mode = Some(*mode);
                        w(&mut child_ui);

                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            Stroke::new(1.0, Color32::WHITE),
                            StrokeKind::Outside,
                        );
                    }
                });
        }

        ui.heading("Layout");
        Grid::new("layout").striped(true).show(ui, |ui| {
            ui.label("");
            for col in &cols {
                ui.label(format!(
                    "cross_align: {:?}\ncross_justify:{:?}",
                    col.cross_align, col.cross_justify
                ));
            }
            ui.end_row();

            for row in &rows {
                ui.label(format!(
                    "main_dir: {:?}\nmain_align: {:?}\nmain_justify: {:?}",
                    row.main_dir, row.main_align, row.main_justify
                ));
                for col in &cols {
                    let layout = Layout {
                        main_dir: row.main_dir,
                        main_align: row.main_align,
                        main_justify: row.main_justify,
                        cross_align: col.cross_align,
                        cross_justify: col.cross_justify,
                        main_wrap: false,
                    };

                    let (_, rect) = ui.allocate_space(test_size);

                    let mut child_ui = ui.new_child(UiBuilder::new().layout(layout).max_rect(rect));
                    w(&mut child_ui);

                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.0, Color32::WHITE),
                        StrokeKind::Outside,
                    );
                }

                ui.end_row();
            }
        });
    });

    harness.fit_contents();
    harness.try_snapshot(format!("layout/{name}"))
}

/// Utility to create a snapshot test of the different states of a egui widget.
/// This renders each state to a texture to work around the fact only a single widget can be
/// hovered / pressed / focused at a time.
struct VisualTests<'a> {
    name: String,
    w: &'a mut dyn FnMut(&mut Ui) -> Response,
    results: Vec<(String, ColorImage)>,
}

impl<'a> VisualTests<'a> {
    pub fn test(name: &str, mut w: impl FnMut(&mut Ui) -> Response) -> SnapshotResult {
        let mut vis = VisualTests::new(name, &mut w);
        vis.add_default_states();
        vis.render()
    }

    pub fn new(name: &str, w: &'a mut dyn FnMut(&mut Ui) -> Response) -> Self {
        Self {
            name: name.to_owned(),
            w,
            results: Vec::new(),
        }
    }

    fn add_default_states(&mut self) {
        self.add("idle", |_| {});
        self.add_node("hover", |node| {
            node.hover();
        });
        self.add("pressed", |harness| {
            harness.get_next_widget().hover();
            let rect = harness.get_next_widget().rect();
            harness.input_mut().events.push(Event::PointerButton {
                button: PointerButton::Primary,
                pos: rect.center(),
                pressed: true,
                modifiers: Default::default(),
            });
        });
        self.add_node("focussed", |node| {
            node.focus();
        });
        self.add_disabled();
    }

    fn single_test(&mut self, f: impl FnOnce(&mut Harness<'_>), enabled: bool) -> ColorImage {
        let mut harness = Harness::builder().with_step_dt(0.05).build_ui(|ui| {
            egui_extras::install_image_loaders(ui.ctx());
            ui.add_enabled_ui(enabled, |ui| {
                (self.w)(ui);
            });
        });

        harness.fit_contents();

        // Wait for images to load
        harness.try_run_realtime().ok();

        f(&mut harness);

        harness.run();

        let image = harness.render().expect("Failed to render harness");

        ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_ref(),
        )
    }

    pub fn add(&mut self, name: &str, test: impl FnOnce(&mut Harness<'_>)) {
        let image = self.single_test(test, true);
        self.results.push((name.to_owned(), image));
    }

    pub fn add_disabled(&mut self) {
        let image = self.single_test(|_| {}, false);
        self.results.push(("disabled".to_owned(), image));
    }

    pub fn add_node(&mut self, name: &str, test: impl FnOnce(&Node<'_>)) {
        self.add(name, |harness| {
            let node = harness.get_next_widget();
            test(&node);
        });
    }

    pub fn render(self) -> SnapshotResult {
        let mut results = Some(self.results);
        let mut images: Option<Vec<(String, TextureHandle, SizedTexture)>> = None;

        let mut harness = Harness::new_ui(|ui| {
            let results = images.get_or_insert_with(|| {
                results
                    .take()
                    .unwrap()
                    .into_iter()
                    .map(|(name, image)| {
                        let size = Vec2::new(image.width() as f32, image.height() as f32);
                        let texture_handle =
                            ui.ctx()
                                .load_texture(name.clone(), image, TextureOptions::default());
                        let texture = SizedTexture::new(texture_handle.id(), size);
                        (name.clone(), texture_handle, texture)
                    })
                    .collect()
            });

            Grid::new("results").show(ui, |ui| {
                for (name, _, image) in results {
                    ui.label(&*name);

                    ui.scope(|ui| {
                        ui.image(*image);
                    });

                    ui.end_row();
                }
            });
        });

        harness.fit_contents();

        harness.try_snapshot(format!("visuals/{}", self.name))
    }
}

trait HarnessExt {
    fn get_next_widget(&self) -> Node<'_>;
}

impl HarnessExt for Harness<'_> {
    fn get_next_widget(&self) -> Node<'_> {
        self.get_all(by().predicate(|node| node.role() != Role::GenericContainer))
            .next()
            .unwrap()
    }
}
