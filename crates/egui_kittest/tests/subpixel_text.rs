#![cfg(feature = "wgpu")]

//! Subpixel (LCD) text: `TextOptions::subpixel_order` with the wgpu renderer.

use egui::{
    Color32, FontId, Pos2, Theme, Vec2, emath::TSTransform, epaint::SubpixelOrder,
    epaint::TextShape,
};
use egui_kittest::{Harness, wgpu::WgpuTestRenderer};
use egui_wgpu::{WgpuSetup, wgpu};

#[derive(Clone, Copy, PartialEq)]
enum Draw {
    Plain,
    Rotated,
    Scaled,
}

/// A wgpu renderer on a device with or without dual-source blending.
fn renderer(dual_source: bool) -> WgpuTestRenderer {
    let mut setup = egui_kittest::wgpu::default_wgpu_setup();
    if !dual_source && let WgpuSetup::CreateNew(create) = &mut setup {
        let base = std::sync::Arc::clone(&create.device_descriptor);
        create.device_descriptor = std::sync::Arc::new(move |adapter| wgpu::DeviceDescriptor {
            required_features: wgpu::Features::empty(),
            ..base(adapter)
        });
    }
    WgpuTestRenderer::from_setup(setup)
}

/// White text on black, drawn as `draw`, with the given subpixel order, on a device with or
/// without dual-source blending.
fn render(order: Option<SubpixelOrder>, dual_source: bool, draw: Draw) -> image::RgbaImage {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(240.0, 60.0))
        .with_pixels_per_point(1.0)
        .renderer(renderer(dual_source))
        .build_ui(move |ui| {
            ui.ctx().set_theme(Theme::Dark);
            ui.ctx().style_mut_of(Theme::Dark, |s| {
                s.visuals.text_options.subpixel_order = order;
            });
            ui.painter()
                .rect_filled(ui.ctx().content_rect(), 0.0, Color32::BLACK);
            let galley = ui.painter().layout_no_wrap(
                "Hamburgefonstiv".to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            let mut shape = TextShape::new(Pos2::new(10.0, 20.0), galley, Color32::WHITE);
            if draw == Draw::Rotated {
                shape = shape.with_angle(0.2);
            }
            ui.painter().add(shape);
            if draw == Draw::Scaled {
                ui.ctx()
                    .set_transform_layer(ui.layer_id(), TSTransform::from_scaling(1.37));
            }
        });
    // The test renderer is made on the first render and doesn't report support, so say it here.
    harness.input_mut().subpixel_text = Some(true);
    harness.run();
    harness.render().expect("render")
}

/// Pixels whose channels differ: color fringes.
fn chromatic(image: &image::RgbaImage) -> usize {
    image
        .pixels()
        .filter(|p| {
            let [r, g, b, _] = p.0;
            r.max(g).max(b) - r.min(g).min(b) > 24
        })
        .count()
}

/// How much text is drawn: the sum of the green channel, the middle of the three stripes.
fn ink(image: &image::RgbaImage) -> u64 {
    image.pixels().map(|p| p.0[1] as u64).sum()
}

#[test]
fn subpixel_text_has_color_fringes() {
    let lcd = render(Some(SubpixelOrder::Rgb), true, Draw::Plain);
    let gray = render(None, true, Draw::Plain);
    assert_eq!(chromatic(&gray), 0);
    assert!(chromatic(&lcd) > 50, "{}", chromatic(&lcd));
    // About as much ink as grayscale text.
    let (a, b) = (ink(&lcd) as f64, ink(&gray) as f64);
    assert!((a / b - 1.0).abs() < 0.15, "{a} vs {b}");
}

#[test]
fn bgr_mirrors_rgb() {
    let rgb = render(Some(SubpixelOrder::Rgb), true, Draw::Plain);
    let bgr = render(Some(SubpixelOrder::Bgr), true, Draw::Plain);
    let red: u64 = rgb.pixels().map(|p| p.0[0] as u64).sum();
    let blue_of_bgr: u64 = bgr.pixels().map(|p| p.0[2] as u64).sum();
    assert_eq!(red, blue_of_bgr);
}

/// Grayscale text draws the same through the subpixel pipeline as without dual-source blending.
#[test]
fn grayscale_text_is_unchanged_by_the_lcd_pipeline() {
    let with = render(None, true, Draw::Plain);
    let without = render(None, false, Draw::Plain);
    assert_eq!(with.as_raw(), without.as_raw());
}

/// Text that doesn't land one texel per pixel loses its fringes instead of smearing them.
#[test]
fn transformed_subpixel_text_is_grayscale() {
    for draw in [Draw::Rotated, Draw::Scaled] {
        let image = render(Some(SubpixelOrder::Rgb), true, draw);
        assert_eq!(chromatic(&image), 0);
        assert!(ink(&image) > 0);
    }
}

/// Color glyphs (emoji) share the font atlas with subpixel coverage, and are drawn exactly as
/// without dual-source blending, where the font texture goes through the plain pipeline.
#[test]
fn color_glyphs_are_unchanged_by_subpixel_text() {
    use egui::epaint::text::{
        FontPriority, GlyphBitmap, GlyphRasterizer, GlyphRasterizerRequest, RasterizedGlyph,
    };

    // A 12 × 12 color glyph for "★", with soft edges.
    fn star(request: &GlyphRasterizerRequest<'_>) -> Option<RasterizedGlyph> {
        (request.cluster == "★").then(|| {
            let pixels = (0..144)
                .map(|i| {
                    let (x, y) = (i % 12, i / 12);
                    let alpha = if x == 0 || y == 0 || x == 11 || y == 11 {
                        128
                    } else {
                        255
                    };
                    Color32::from_rgba_unmultiplied((20 * x) as u8, 200, (20 * y) as u8, alpha)
                })
                .collect();
            RasterizedGlyph {
                bitmap: GlyphBitmap {
                    image: egui::ColorImage::new([12, 12], pixels),
                    offset_px: Vec2::new(0.0, -10.0),
                    is_color: true,
                },
                advance_px: 13.0,
            }
        })
    }

    let render_star = |order: Option<SubpixelOrder>, dual_source: bool| {
        let mut harness = Harness::builder()
            .with_size(Vec2::new(60.0, 30.0))
            .with_pixels_per_point(1.0)
            .renderer(renderer(dual_source))
            .build_ui(move |ui| {
                ui.ctx().set_theme(Theme::Dark);
                ui.ctx().add_glyph_rasterizer(
                    GlyphRasterizer::new("subpixel_text::star", star)
                        .with_priority(FontPriority::Highest),
                );
                ui.ctx().style_mut_of(Theme::Dark, |s| {
                    s.visuals.text_options.subpixel_order = order;
                });
                // Not black: over black, a wrong blend weight wouldn't show.
                ui.painter()
                    .rect_filled(ui.ctx().content_rect(), 0.0, Color32::from_gray(200));
                ui.label("★");
            });
        harness.input_mut().subpixel_text = Some(true);
        harness.run();
        harness.render().expect("render")
    };
    let reference = render_star(None, false);
    assert!(
        chromatic(&reference) > 50,
        "the star should be drawn in color"
    );
    assert_eq!(
        render_star(Some(SubpixelOrder::Rgb), true).as_raw(),
        reference.as_raw()
    );
    assert_eq!(render_star(None, true).as_raw(), reference.as_raw());
}
