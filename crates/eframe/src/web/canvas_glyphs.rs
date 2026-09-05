//! Browser glyph rasterization for egui's unsupported-glyph fallback.
//!
//! This draws unsupported grapheme clusters with Canvas 2D, then puts the resulting pixels in
//! egui's font atlas. Clusters the browser cannot render either (tofu) are rejected,
//! so egui draws its own replacement glyph.

use core::cell::RefCell;
use std::collections::HashMap;

use egui::{
    ColorImage, GlyphRasterizer, GlyphRasterizerRequest, MAX_GLYPH_SIZE, RasterizedGlyph,
    has_emoji_presentation, vec2,
};
use wasm_bindgen::JsCast as _;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

/// Room for anti-aliased pixels at the glyph bounds, in physical pixels.
const PADDING: f64 = 2.0;

/// Result of drawing some text on the canvas.
struct Drawn {
    /// Straight (unpremultiplied) RGBA.
    rgba: Vec<u8>,
    width: u32,
    height: u32,

    /// Distance from the pen to the left edge of the ink.
    left: f64,

    /// Distance from the baseline to the top of the ink.
    ascent: f64,

    /// Horizontal advance.
    advance: f64,
}

struct CanvasGlyphs {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,

    /// What the browser draws for a missing glyph, per `(font, subpixel offset)`.
    ///
    /// `None` if the browser draws nothing.
    tofu_cache: HashMap<(String, u32), Option<Drawn>>,
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
        Some(Self {
            canvas,
            context,
            tofu_cache: Default::default(),
        })
    }

    fn rasterize(&mut self, request: &GlyphRasterizerRequest<'_>) -> Option<RasterizedGlyph> {
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
        let font = format!("{font_size_px}px {family}");

        // White, like egui's own glyph rasterizer: the atlas stores coverage,
        // and the tessellator multiplies it with the text color.
        let white = self.draw(cluster, &font, "white", *subpixel_offset_px)?;

        if self.is_tofu(&white, &font, *subpixel_offset_px) {
            return None; // Let egui draw its own replacement glyph instead.
        }

        // Is this a monochrome glyph (to be tinted by the text color) or a color emoji (not)?
        // Monochrome glyphs take the fill color, color glyphs ignore it,
        // so we draw twice and see if the fill color made a difference.
        let black = self.draw(cluster, &font, "black", *subpixel_offset_px)?;
        let is_color = is_color(cluster, &white, &black);

        let Drawn {
            rgba,
            width,
            height,
            left,
            ascent,
            advance,
        } = white;

        Some(RasterizedGlyph {
            image: ColorImage::from_rgba_unmultiplied([width as _, height as _], &rgba),
            // The pen sits at `(PADDING + left, PADDING + ascent)` in the image,
            // so the image's top-left is this far from the pen:
            offset_px: vec2(-(left + PADDING) as f32, -(ascent + PADDING) as f32),
            advance_px: advance as f32,
            is_color,
        })
    }

    /// Did the browser draw its "missing glyph" box (tofu)?
    ///
    /// Browsers draw a box for glyphs they lack, and `measureText` reports
    /// a width for it, so the only way to tell is to compare pixels.
    fn is_tofu(&mut self, drawn: &Drawn, font: &str, subpixel_offset_px: f32) -> bool {
        /// An unassigned code point, so no font has a glyph for it.
        const MISSING_GLYPH: &str = "\u{0378}";

        let key = (font.to_owned(), subpixel_offset_px.to_bits());
        if !self.tofu_cache.contains_key(&key) {
            let tofu = self.draw(MISSING_GLYPH, font, "white", subpixel_offset_px);
            self.tofu_cache.insert(key.clone(), tofu);
        }

        self.tofu_cache
            .get(&key)
            .and_then(Option::as_ref)
            .is_some_and(|tofu| {
                tofu.width == drawn.width && tofu.height == drawn.height && tofu.rgba == drawn.rgba
            })
    }

    /// Draw `text` with the given CSS `font` and `fill_style` and read back the pixels.
    fn draw(
        &self,
        text: &str,
        font: &str,
        fill_style: &str,
        subpixel_offset_px: f32,
    ) -> Option<Drawn> {
        self.context.set_font(font);
        self.context.set_text_baseline("alphabetic");
        let metrics = self.context.measure_text(text).ok()?;
        let left = metrics.actual_bounding_box_left();
        let ascent = metrics.actual_bounding_box_ascent();
        let right = metrics.actual_bounding_box_right();
        let descent = metrics.actual_bounding_box_descent();

        let width = (left + right + 2.0 * PADDING).ceil().max(1.0) as u32;
        let height = (ascent + descent + 2.0 * PADDING).ceil().max(1.0) as u32;
        if MAX_GLYPH_SIZE < width as usize || MAX_GLYPH_SIZE < height as usize {
            return None;
        }

        // Only grow: resizing reallocates the backing store and resets all canvas state.
        if self.canvas.width() < width {
            self.canvas.set_width(width);
        }
        if self.canvas.height() < height {
            self.canvas.set_height(height);
        }
        self.context
            .clear_rect(0.0, 0.0, width as f64, height as f64);
        self.context.set_font(font);
        self.context.set_text_baseline("alphabetic");
        self.context.set_fill_style_str(fill_style);
        // Canvas positions text by its baseline, while the image starts at its top-left.
        self.context
            .fill_text(
                text,
                PADDING + left + subpixel_offset_px as f64,
                PADDING + ascent,
            )
            .ok()?;
        let rgba = self
            .context
            .get_image_data(0.0, 0.0, width as f64, height as f64)
            .ok()?
            .data()
            .0;

        Some(Drawn {
            rgba,
            width,
            height,
            left,
            ascent,
            advance: metrics.width(),
        })
    }
}

/// Did the fill color affect the pixels?
///
/// Compares a `white` and a `black` draw of the same cluster, pixel by pixel.
/// A monochrome glyph takes the fill color, so its pixels differ between the two.
/// A color glyph (emoji) ignores it, so its pixels stay the same.
///
/// This is a majority vote rather than an exact comparison:
/// some browsers (e.g. Firefox for Android) are not pixel-exact between two draws,
/// and some color glyphs have layers that take the fill color.
/// Unicode emoji presentation breaks ties, e.g. for glyphs with no opaque pixels.
///
/// Looking at the pixels of a single draw does not work:
/// a black-and-white emoji (⚫, 🖤) looks monochrome but must not be tinted,
/// and some platforms draw e.g. Thai and Korean with colored (subpixel-antialiased) fringes.
fn is_color(cluster: &str, white: &Drawn, black: &Drawn) -> bool {
    let mut monochrome_votes = 0_usize;
    let mut color_votes = 0_usize;

    for (&[white_r, white_g, white_b, white_a], &[black_r, black_g, black_b, black_a]) in
        core::iter::zip(white.rgba.as_chunks::<4>().0, black.rgba.as_chunks::<4>().0)
    {
        if white_a < 255 || black_a < 255 {
            continue; // let's only count opaque pixels
        }
        let diff_r = white_r.abs_diff(black_r);
        let diff_g = white_g.abs_diff(black_g);
        let diff_b = white_b.abs_diff(black_b);
        let max_diff = diff_r.max(diff_g).max(diff_b);
        let similar = max_diff < 128;

        if similar {
            color_votes += 1;
        } else {
            monochrome_votes += 1;
        }
    }

    if monochrome_votes == color_votes {
        has_emoji_presentation(cluster)
    } else {
        monochrome_votes < color_votes
    }
}

thread_local! {
    // `Context` and `GlyphRasterizer` are `Send + Sync`, but browser Canvas handles are not.
    // Keeping the handles here lets the callback stay capture-free and usable from `egui::Context`.
    static CANVAS_GLYPHS: RefCell<Option<CanvasGlyphs>> = const { RefCell::new(None) };
}

pub(super) fn glyph_rasterizer() -> GlyphRasterizer {
    GlyphRasterizer::new(|request| {
        CANVAS_GLYPHS.with(|glyphs| {
            let mut glyphs = glyphs.borrow_mut();
            if glyphs.is_none() {
                *glyphs = CanvasGlyphs::new();
            }
            glyphs.as_mut()?.rasterize(request)
        })
    })
}
