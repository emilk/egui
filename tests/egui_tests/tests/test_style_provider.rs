use core::fmt::Debug;
use egui::theme::{DefaultStyle, StyleProvider};
use egui::widget_style::{
    AtomLayoutStyle, ButtonStyle, CheckboxStyle, SeparatorStyle, StyleArgs, TextEditStyle,
    TextVisuals,
};
use egui::{
    Align, Align2, Atom, AtomExt as _, Button, Checkbox, Color32, CornerRadius, FontFamily, FontId,
    Frame, Margin, Separator, Stroke, TextEdit, Ui, Vec2, include_image,
};
use egui_kittest::Harness;

struct VariantHandle {
    calls: usize,
    return_b_for: Option<usize>,
    fail_for: Option<usize>,
}

impl VariantHandle {
    #[track_caller]
    fn get<I: Debug>(&mut self, a: I, b: I) -> I {
        assert_ne!(
            self.fail_for,
            Some(self.calls),
            "Switching from {a:?} to {b:?} lead to no meaningful change."
        );
        let res = if self.return_b_for == Some(self.calls) {
            b
        } else {
            a
        };
        self.calls += 1;
        res
    }
}

fn test_variants<Variant: Clone, Comparison: PartialEq>(
    make_variant: impl Fn(&mut VariantHandle) -> Variant,
    mut render_variant: impl FnMut(Variant, bool) -> Comparison,
) {
    let mut init_handle = VariantHandle {
        calls: 0,
        return_b_for: None,
        fail_for: None,
    };
    let base_variant = make_variant(&mut init_handle);
    let variant_count = init_handle.calls;

    let base_image = render_variant(base_variant, false);

    for i in 0..variant_count {
        let mut test_handle = VariantHandle {
            calls: 0,
            fail_for: None,
            return_b_for: Some(i),
        };
        let test_variant = make_variant(&mut test_handle);
        let test_image = render_variant(test_variant.clone(), false);
        if test_image == base_image {
            // Open the snapshot so we can see what the variant looks like.
            render_variant(test_variant, true);

            let mut fail_handle = VariantHandle {
                calls: 0,
                fail_for: Some(i),
                return_b_for: None,
            };
            make_variant(&mut fail_handle);
        }
    }
}

/// Helper that walks all fields of a struct, swaping each value one after the other, ensuring
/// that any value change also results in a visual change of the UI.
fn test_harness_variants<Variant: Clone>(
    size: Vec2,
    make_variant: impl Fn(&mut VariantHandle) -> Variant,
    mut contents: impl FnMut(&mut Ui, Variant),
) {
    test_variants(make_variant, |variant, failure| {
        let mut harness = Harness::builder().with_size(size).build_ui(|ui| {
            contents(ui, variant.clone());
        });
        // Run a few frames so images have time to load.
        harness.run();
        if failure {
            // Helpful to see what's going on:
            // harness.debug_open_snapshot();
        }
        harness.render().expect("Failed to render the harness")
    });
}

struct FixedStyleProvider<T>(T);

impl<T: Clone> StyleProvider<T> for FixedStyleProvider<T> {
    fn style(&mut self, _modifiers: &StyleArgs<'_>) -> T {
        self.0.clone()
    }
}

/// A small image atom, so [`AtomLayoutStyle::image_tint`] has something to tint.
fn image_atom() -> Atom<'static> {
    include_image!("../../../crates/eframe/data/icon.png").atom_size(Vec2::splat(10.0))
}

fn frame_variants(variant: &mut VariantHandle) -> Frame {
    Frame {
        inner_margin: Margin::same(variant.get(0, 4)),
        fill: variant.get(Color32::GREEN, Color32::BLUE),
        stroke: Stroke::new(
            // Has to be more that 0.0 or color variant below will fail
            variant.get(1.0, 2.0),
            variant.get(Color32::RED, Color32::GREEN),
        ),

        corner_radius: variant.get(CornerRadius::same(0), CornerRadius::same(4)),
        outer_margin: Margin::same(variant.get(0, 4)),
        shadow: Default::default(),
    }
}

/// Which [`AtomLayoutStyle`] fields the widget under test can show.
///
/// Every `false` is a field the theme cannot reach in that widget, because the widget either
/// overrides it or leaves it no room. Each one has a comment at the call site saying why.
#[derive(Clone, Copy)]
struct AtomLayoutFields {
    align_x: bool,
    align_y: bool,
}

impl Default for AtomLayoutFields {
    fn default() -> Self {
        Self {
            align_x: true,
            align_y: true,
        }
    }
}

/// `min_size` is deliberately bigger than the content of every widget here, so `align2` has
/// room to move that content around.
fn atom_layout_variants(
    variant: &mut VariantHandle,
    fields: AtomLayoutFields,
    frame: Frame,
) -> AtomLayoutStyle {
    AtomLayoutStyle {
        align2: Some(Align2([
            if fields.align_x {
                variant.get(Align::Min, Align::Max)
            } else {
                Align::Min
            },
            if fields.align_y {
                variant.get(Align::Min, Align::Max)
            } else {
                Align::Min
            },
        ])),
        min_size: Vec2::new(variant.get(100.0, 200.0), variant.get(40.0, 80.0)),
        gap: variant.get(0.0, 10.0),
        frame,
        text_style: TextVisuals {
            font_id: variant.get(
                FontId::new(10.0, FontFamily::Proportional),
                FontId::new(12.0, FontFamily::Monospace),
            ),
            color: variant.get(Color32::WHITE, Color32::RED),
        },
        image_tint: variant.get(Color32::RED, Color32::GREEN),
    }
}

#[test]
fn ensure_all_button_style_args_used() {
    test_harness_variants(
        Vec2::new(300.0, 150.0),
        |variant| {
            let frame = frame_variants(variant);
            ButtonStyle {
                atom_layout: atom_layout_variants(variant, AtomLayoutFields::default(), frame),
            }
        },
        |ui, variant| {
            egui_extras::install_image_loaders(ui.ctx());
            ui.replace_widget_theme(FixedStyleProvider(variant));
            ui.add(Button::new((image_atom(), "Image Button")));
        },
    );
}

/// The [`Checkbox`] paints its box as one rectangle, so only these [`Frame`] fields reach the
/// screen. `outer_margin`, `shadow` and three of the four `inner_margin` sides are dropped.
fn checkbox_frame_variants(variant: &mut VariantHandle) -> Frame {
    Frame {
        inner_margin: Margin::same(variant.get(0, 4)),
        fill: variant.get(Color32::BLACK, Color32::YELLOW),
        stroke: Stroke::new(
            // Has to be more that 0.0 or the color variant below will fail
            variant.get(1.0, 3.0),
            variant.get(Color32::WHITE, Color32::RED),
        ),
        corner_radius: variant.get(CornerRadius::same(0), CornerRadius::same(5)),
        outer_margin: Margin::ZERO,
        shadow: Default::default(),
    }
}

#[test]
fn ensure_all_checkbox_style_args_used() {
    test_harness_variants(
        Vec2::new(300.0, 150.0),
        |variant| {
            let frame = frame_variants(variant);
            let fields = AtomLayoutFields {
                // The `Checkbox` grows its box atom to `min_size.y`, so the content is always
                // exactly as tall as the space it gets aligned in. There is never any slack.
                align_y: false,
                ..Default::default()
            };
            CheckboxStyle {
                atom_layout: atom_layout_variants(variant, fields, frame),
                checkbox_size: variant.get(14.0, 24.0),
                check_size: variant.get(8.0, 13.0),
                checkbox_frame: checkbox_frame_variants(variant),
                check_stroke: Stroke::new(
                    // Has to be more that 0.0 or the color variant below will fail
                    variant.get(1.5, 3.0),
                    variant.get(Color32::WHITE, Color32::RED),
                ),
            }
        },
        |ui, variant| {
            egui_extras::install_image_loaders(ui.ctx());
            ui.replace_widget_theme(FixedStyleProvider(variant));
            // Checked, so `check_size` and `check_stroke` paint something.
            let mut checked = true;
            ui.add(Checkbox::new(&mut checked, (image_atom(), "On")));
        },
    );
}

#[test]
fn ensure_all_separator_style_args_used() {
    test_harness_variants(
        Vec2::new(300.0, 100.0),
        |variant| SeparatorStyle {
            spacing: variant.get(6.0, 30.0),
            stroke: Stroke::new(
                // Has to be more than 0.0 or the color variant below will fail
                variant.get(1.0, 4.0),
                variant.get(Color32::RED, Color32::GREEN),
            ),
        },
        |ui, variant| {
            ui.replace_widget_theme(FixedStyleProvider(variant));
            ui.add(Separator::default());
        },
    );
}

#[test]
fn ensure_all_text_edit_style_args_used() {
    test_harness_variants(
        Vec2::new(320.0, 260.0),
        |variant| {
            let frame = frame_variants(variant);
            TextEditStyle {
                atom_layout: atom_layout_variants(variant, AtomLayoutFields::default(), frame),
                hint_text_color: variant.get(Color32::GRAY, Color32::YELLOW),
                prefix_suffix_color: variant.get(Color32::BLACK, Color32::BLUE),
            }
        },
        |ui, variant| {
            egui_extras::install_image_loaders(ui.ctx());
            ui.replace_widget_theme(FixedStyleProvider(variant));

            // Make sure min_size can exceed this:
            ui.spacing_mut().text_edit_width = 100.0;

            // An empty field shows the hint text...
            let mut empty = String::new();
            ui.add(
                TextEdit::singleline(&mut empty)
                    .hint_text("Hint")
                    .prefix((image_atom(), "$"))
                    .suffix(".00"),
            );

            // ...and a filled one shows `text_style.color`.
            let mut filled = String::from("Text");
            ui.add(
                TextEdit::singleline(&mut filled)
                    .prefix((image_atom(), "$"))
                    .suffix(".00"),
            );
        },
    );
}

struct CustomStyleProvider;

impl StyleProvider<TextEditStyle> for CustomStyleProvider {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> TextEditStyle {
        let mut default: TextEditStyle = DefaultStyle.style(modifiers);

        default.hint_text_color = Color32::BLUE;
        default.prefix_suffix_color = Color32::GREEN;
        default.atom_layout.text_style.color = Color32::RED;

        default
    }
}

#[test]
fn text_edit_colors() {
    let mut harness = Harness::new_ui(|ui| {
        ui.add_widget_theme::<TextEditStyle>(CustomStyleProvider);

        ui.label("The text should match the colors:");

        ui.add(
            TextEdit::singleline(&mut String::new())
                .prefix("green")
                .suffix("green")
                .hint_text("blue"),
        );

        ui.add(
            TextEdit::singleline(&mut "Red".to_owned())
                .prefix("green")
                .suffix("green")
                .hint_text("blue"),
        );
    });

    harness.fit_contents();
    harness.snapshot("text_edit_colors");
}
