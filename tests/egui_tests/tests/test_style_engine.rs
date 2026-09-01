use egui::theme::StyleProvider;
use egui::widget_style::{AtomLayoutStyle, ButtonStyle, StyleArgs, TextVisuals};
use egui::{
    Align, Align2, AtomExt, Button, Color32, CornerRadius, FontFamily, FontId, Frame, Margin,
    Stroke, Ui, Vec2, include_image,
};
use egui_kittest::Harness;
use std::fmt::Debug;

struct VariantHandle {
    calls: usize,
    return_b_for: Option<usize>,
    fail_for: Option<usize>,
}

impl VariantHandle {
    #[track_caller]
    fn get<I: Debug>(&mut self, a: I, b: I) -> I {
        if self.fail_for == Some(self.calls) {
            panic!("Switching from {a:?} to {b:?} lead to no meaningful change.");
        }
        let res = if self.return_b_for == Some(self.calls) {
            dbg!(&b);
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
        if test_image != base_image {
            let mut fail_handle = VariantHandle {
                calls: 0,
                fail_for: Some(i),
                return_b_for: None,
            };
            make_variant(&mut fail_handle);
        }
        render_variant(test_variant, true);
    }
}

fn test_harness_variants<Variant: Clone>(
    make_variant: impl Fn(&mut VariantHandle) -> Variant,
    mut contents: impl FnMut(&mut Ui, Variant),
) {
    test_variants(make_variant, |variant, failure| {
        let mut harness = Harness::builder()
            .with_size(Vec2::new(300.0, 100.0))
            .build_ui(|ui| {
                contents(ui, variant.clone());
            });
        if failure {
            harness.debug_open_snapshot();
        }
        harness.render().unwrap()
    });
}

struct FixedStyleProvider<T>(T);
impl<T: Clone> StyleProvider<T> for FixedStyleProvider<T> {
    fn style(&mut self, modifiers: &StyleArgs<'_>) -> T {
        self.0.clone()
    }
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

#[test]
fn ensure_all_button_style_args_used() {
    test_harness_variants(
        |variant| ButtonStyle {
            atom_layout: AtomLayoutStyle {
                align2: Some(Align2([
                    variant.get(Align::Min, Align::Max),
                    variant.get(Align::Min, Align::Max),
                ])),
                min_size: Vec2::new(variant.get(0.0, 200.0), variant.get(0.0, 50.0)),
                // min_size: Vec2::new(100.0, 100.0),
                gap: variant.get(0.0, 10.0),
                frame: frame_variants(variant),
                text_style: TextVisuals {
                    font_id: variant.get(
                        FontId::new(10.0, FontFamily::Proportional),
                        FontId::new(12.0, FontFamily::Monospace),
                    ),
                    color: variant.get(Color32::WHITE, Color32::RED),
                },
                image_tint: variant.get(Color32::RED, Color32::GREEN),
            },
        },
        |ui, variant| {
            ui.replace_widget_theme(FixedStyleProvider(variant));
            ui.add_sized(Vec2::new(200.0, 200.0), Button::new((
                include_image!("../../../crates/eframe/data/icon.png").atom_size(Vec2::splat(10.0)),
                "Image Button",
            )));
        },
    );
}
