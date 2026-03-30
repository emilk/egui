use std::cell::RefCell;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

use crate::text::font::ScaledMetrics;

/// Data for a glyph rendered via HTML5 canvas
#[derive(Debug, Clone)]
pub struct CanvasGlyphData {
    /// RGBA pixel data from canvas ImageData
    pub image_data: Vec<u8>,
    /// Width of the glyph in pixels
    pub width: u32,
    /// Height of the glyph in pixels
    pub height: u32,
    /// Advance width for horizontal text layout
    pub advance_width: f32,
    /// Horizontal offset from origin
    pub offset_x: f32,
    /// Vertical offset from origin (baseline)
    pub offset_y: f32,
}

/// Renders glyphs using HTML5 canvas (WASM only)
///
/// This renderer uses the browser's native text rendering capabilities
/// to rasterize glyphs that are not available in the bundled fonts.
pub struct CanvasGlyphRenderer {
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
}

impl CanvasGlyphRenderer {
    /// Create a new canvas glyph renderer
    ///
    /// Returns an error if canvas creation or context acquisition fails
    pub fn new() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or("No window object")?;
        let document = window.document().ok_or("No document object")?;

        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;

        // Start with a small canvas, will resize as needed
        canvas.set_width(128);
        canvas.set_height(128);

        let context = canvas
            .get_context("2d")?
            .ok_or("Failed to get 2d context")?
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(Self { canvas, context })
    }

    /// Render a glyph using canvas
    ///
    /// Tries each font family in order until one renders the character successfully.
    /// Returns None if the character cannot be rendered or has zero width.
    pub fn render_glyph(
        &mut self,
        chr: char,
        metrics: &ScaledMetrics,
        font_families: &[String],
        bin: crate::text::font::SubpixelBin,
    ) -> Option<CanvasGlyphData> {
        // metrics.scale is the absolute font size in pixels (includes DPI and zoom)
        let font_size_px = metrics.scale;
        let subpixel_offset = bin.as_float();

        // Try each font family in the fallback chain
        for family in font_families {
            if let Some(data) = self.try_render_with_font(chr, font_size_px, subpixel_offset, family) {
                return Some(data);
            }
        }

        // Try with generic sans-serif as last resort
        self.try_render_with_font(chr, font_size_px, subpixel_offset, "sans-serif")
    }

    /// Try to render a glyph with a specific font family
    fn try_render_with_font(
        &mut self,
        chr: char,
        font_size_px: f32,
        subpixel_offset: f32,
        font_family: &str,
    ) -> Option<CanvasGlyphData> {
        let font_string = format!("{}px {}", font_size_px, font_family);
        self.context.set_font(&font_string);

        let text = chr.to_string();

        // Measure the text to get metrics
        let text_metrics = self.context.measure_text(&text).ok()?;
        let advance_width = text_metrics.width() as f32;

        // Skip if character is not supported (zero width)
        if advance_width < 0.1 {
            return None;
        }

        // Get bounding box metrics
        let ascent = text_metrics.actual_bounding_box_ascent();
        let descent = text_metrics.actual_bounding_box_descent();
        let left = text_metrics.actual_bounding_box_left();
        let right = text_metrics.actual_bounding_box_right();

        // Canvas measureText returns values at the specified font size
        // We render at font_size_px which is already in the right scale
        let width = (right + left).ceil() as u32;
        let height = (ascent + descent).ceil() as u32;

        // Skip zero-size glyphs
        if width == 0 || height == 0 {
            return None;
        }

        // Limit maximum glyph size to prevent excessive memory usage
        const MAX_GLYPH_SIZE: u32 = 256;
        if width > MAX_GLYPH_SIZE || height > MAX_GLYPH_SIZE {
            log::warn!(
                "Glyph '{}' too large ({}x{}), max size is {}x{}",
                chr,
                width,
                height,
                MAX_GLYPH_SIZE,
                MAX_GLYPH_SIZE
            );
            return None;
        }

        // Resize canvas if needed
        if width > self.canvas.width() || height > self.canvas.height() {
            self.canvas.set_width(width.max(128));
            self.canvas.set_height(height.max(128));
        }

        // Clear the canvas
        self.context.clear_rect(
            0.0,
            0.0,
            self.canvas.width() as f64,
            self.canvas.height() as f64,
        );

        // Set up rendering
        self.context.set_fill_style_str("white");
        self.context.set_text_baseline("alphabetic");

        // Render the text at the correct position
        // The baseline is at y = ascent, and we offset x by left bearing + subpixel offset
        if let Err(e) = self.context.fill_text(&text, left + subpixel_offset as f64, ascent) {
            log::debug!("Failed to render '{}': {:?}", chr, e);
            return None;
        }

        // Extract image data (now at device pixel resolution)
        let image_data = self
            .context
            .get_image_data(0.0, 0.0, width as f64, height as f64)
            .ok()?;

        let rgba_data = image_data.data().0;

        log::debug!(
            "Canvas glyph '{}': advance={}, size={}x{}, offset=({}, {}), font_size={}px",
            chr, advance_width, width, height, -left as f32, -ascent as f32, font_size_px
        );

        Some(CanvasGlyphData {
            image_data: rgba_data,
            width,
            height,
            advance_width,
            offset_x: -left as f32,
            offset_y: -ascent as f32,
        })
    }
}

thread_local! {
    static CANVAS_RENDERER: RefCell<Option<CanvasGlyphRenderer>> = RefCell::new(None);
}

/// Initialize the canvas renderer if not already initialized
fn ensure_canvas_renderer() -> Result<(), JsValue> {
    CANVAS_RENDERER.with(|renderer_cell| {
        if renderer_cell.borrow().is_none() {
            match CanvasGlyphRenderer::new() {
                Ok(renderer) => {
                    *renderer_cell.borrow_mut() = Some(renderer);
                    Ok(())
                }
                Err(e) => {
                    log::warn!("Failed to create canvas renderer: {:?}", e);
                    Err(e)
                }
            }
        } else {
            Ok(())
        }
    })
}

/// Render a glyph using the thread-local canvas renderer
pub fn render_glyph_with_canvas(
    chr: char,
    metrics: &ScaledMetrics,
    font_families: &[String],
    bin: crate::text::font::SubpixelBin,
) -> Option<CanvasGlyphData> {
    ensure_canvas_renderer().ok()?;

    CANVAS_RENDERER.with(|renderer_cell| {
        renderer_cell
            .borrow_mut()
            .as_mut()?
            .render_glyph(chr, metrics, font_families, bin)
    })
}
