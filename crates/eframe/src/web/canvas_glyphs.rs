//! Browser glyph rasterization for egui's unsupported-glyph fallback.
//!
//! This draws unsupported grapheme clusters with Canvas 2D, then puts the resulting pixels in
//! egui's font atlas.

use core::cell::RefCell;
use std::sync::Arc;

use egui::{
    ColorImage, GlyphRasterizer, GlyphRasterizerRequest, MAX_GLYPH_SIZE, RasterizedGlyph, vec2,
};
use wasm_bindgen::JsCast as _;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

struct CanvasGlyphs {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

impl CanvasGlyphs {
    fn new() -> Option<Self> {
        let document = web_sys::window()?.document()?;
        let canvas = document
            .create_element("canvas")
            .ok()?
            .dyn_into::<HtmlCanvasElement>()
            .ok()?;
        let context = canvas
            .get_context("2d")
            .ok()??
            .dyn_into::<CanvasRenderingContext2d>()
            .ok()?;
        Some(Self { canvas, context })
    }

    fn rasterize(&self, request: &GlyphRasterizerRequest<'_>) -> Option<RasterizedGlyph> {
        let GlyphRasterizerRequest {
            cluster,
            family,
            font_size_px,
            subpixel_offset_px,
        } = request;

        if cluster.trim().is_empty() || !font_size_px.is_finite() || *font_size_px <= 0.0 {
            return None;
        }

        let family = match family {
            egui::FontFamily::Monospace => "monospace",
            _ => {
                "system-ui, sans-serif, \"Apple Color Emoji\", \"Segoe UI Emoji\", \"Noto Color Emoji\""
            }
        };
        self.context.set_font(&format!("{font_size_px}px {family}"));
        self.context.set_text_baseline("alphabetic");
        let metrics = self.context.measure_text(request.cluster).ok()?;
        let left = metrics.actual_bounding_box_left();
        let ascent = metrics.actual_bounding_box_ascent();
        let right = metrics.actual_bounding_box_right();
        let descent = metrics.actual_bounding_box_descent();

        // Leave room for anti-aliased pixels at the glyph bounds.
        let padding = 2.0;

        let width = (left + right + 2.0 * padding).ceil().max(1.0) as u32;
        let height = (ascent + descent + 2.0 * padding).ceil().max(1.0) as u32;
        if MAX_GLYPH_SIZE < width as usize || MAX_GLYPH_SIZE < height as usize {
            return None;
        }

        self.canvas.set_width(width);
        self.canvas.set_height(height);
        // Resizing resets Canvas state.
        self.context.set_font(&format!("{font_size_px}px {family}"));
        self.context.set_text_baseline("alphabetic");
        // White, like egui's own glyph rasterizer: the atlas stores coverage,
        // and the tessellator multiplies it with the text color.
        // Color emoji ignore the fill style.
        self.context.set_fill_style_str("white");
        // Canvas positions text by its baseline, while the atlas image starts at its top-left.
        self.context
            .fill_text(
                cluster,
                padding + left + *subpixel_offset_px as f64,
                padding + ascent,
            )
            .ok()?;
        let rgba = self
            .context
            .get_image_data(0.0, 0.0, width as f64, height as f64)
            .ok()?
            .data()
            .0;
        let (pixels, []) = rgba.as_chunks::<4>() else {
            return None;
        };
        let is_color = pixels
            .iter()
            .any(|&[r, g, b, a]| a != 0 && (r != g || g != b));

        Some(RasterizedGlyph {
            image: ColorImage::from_rgba_unmultiplied([width as _, height as _], &rgba),
            offset_px: vec2((-left + padding) as f32, (-ascent + padding) as f32),
            advance_px: metrics.width() as f32,
            is_color,
        })
    }
}

thread_local! {
    // `Context` and `GlyphRasterizer` are `Send + Sync`, but browser Canvas handles are not.
    // Keeping the handles here lets the callback stay capture-free and usable from `egui::Context`.
    static CANVAS_GLYPHS: RefCell<Option<CanvasGlyphs>> = const { RefCell::new(None) };
}

pub(super) fn glyph_rasterizer() -> GlyphRasterizer {
    Arc::new(|request| {
        CANVAS_GLYPHS.with(|glyphs| {
            let mut glyphs = glyphs.borrow_mut();
            if glyphs.is_none() {
                *glyphs = CanvasGlyphs::new();
            }
            glyphs.as_mut()?.rasterize(request)
        })
    })
}
