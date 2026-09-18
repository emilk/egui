//! Render text glyphs from SVG files. See [`SvgGlyph`].

use std::sync::Arc;

use egui::{
    Color32, ColorImage, FontPriority, GlyphBitmap, GlyphRasterizer, GlyphRasterizerRequest,
    MAX_GLYPH_SIZE, RasterizedGlyph, Vec2, vec2,
};
use resvg::{
    tiny_skia::Pixmap,
    usvg::{Color, Group, Node, Paint, Transform, Tree},
};

/// A glyph rendered from an SVG, for a [`GlyphRasterizer`].
///
/// Use it to override how a character looks
/// or to add your own icons to a (private use) character.
///
/// The SVG is scaled so that its height is [`Self::with_height`] (default: one em),
/// and placed with its bottom edge [`Self::with_baseline_offset`] above the baseline (default: on it).
///
/// An SVG that only uses shades of gray is tinted with the text color, like a normal glyph.
/// One with other colors keeps them, like a color emoji.
/// Use [`Self::with_color`] to decide yourself.
///
/// ```no_run
/// # let ctx = egui::Context::default();
/// let svg = std::fs::read("ellipsis.svg").unwrap();
/// let ellipsis = egui_extras::SvgGlyph::from_bytes(&svg).unwrap();
/// ctx.add_glyph_rasterizer(ellipsis.into_rasterizer("…"));
/// ```
#[derive(Clone, Debug)]
pub struct SvgGlyph {
    tree: Arc<Tree>,

    /// Height of the rendered SVG, in em (fractions of the font size).
    height_em: f32,

    /// How far above the baseline the bottom of the SVG sits, in em.
    baseline_offset_em: f32,

    /// Horizontal advance in em. `None` means the width of the rendered SVG.
    advance_em: Option<f32>,

    /// Keep the colors of the SVG, instead of tinting it with the text color?
    ///
    /// Default: whether the SVG uses any color that is not a shade of gray.
    is_color: bool,
}

impl SvgGlyph {
    /// Parse an SVG with default [`resvg::usvg::Options`].
    ///
    /// # Errors
    /// On invalid SVG.
    pub fn from_bytes(svg_bytes: &[u8]) -> Result<Self, String> {
        let options = resvg::usvg::Options::default();
        let tree = Tree::from_data(svg_bytes, &options).map_err(|err| err.to_string())?;
        Ok(Self::from_tree(tree))
    }

    /// Use an already parsed SVG.
    pub fn from_tree(tree: Tree) -> Self {
        Self {
            is_color: has_color(tree.root()),
            tree: Arc::new(tree),
            height_em: 1.0,
            baseline_offset_em: 0.0,
            advance_em: None,
        }
    }

    /// Height of the rendered SVG, in em (fractions of the font size).
    ///
    /// Default: `1.0`. The width follows from the aspect ratio of the SVG.
    #[inline]
    pub fn with_height(mut self, height_em: f32) -> Self {
        self.height_em = height_em;
        self
    }

    /// How far above the baseline the bottom of the SVG sits, in em.
    ///
    /// Default: `0.0`, i.e. the SVG stands on the baseline.
    /// Negative values reach below the baseline.
    #[inline]
    pub fn with_baseline_offset(mut self, baseline_offset_em: f32) -> Self {
        self.baseline_offset_em = baseline_offset_em;
        self
    }

    /// Horizontal advance in em, i.e. how much room the glyph takes in the text.
    ///
    /// Default: the width of the rendered SVG.
    /// If wider than the SVG, the SVG is centered in the advance.
    #[inline]
    pub fn with_advance(mut self, advance_em: f32) -> Self {
        self.advance_em = Some(advance_em);
        self
    }

    /// Keep the colors of the SVG (like a color emoji),
    /// instead of tinting its shape with the text color?
    ///
    /// Default: `true` if the SVG uses any color that is not a shade of gray.
    #[inline]
    pub fn with_color(mut self, is_color: bool) -> Self {
        self.is_color = is_color;
        self
    }

    /// Render the SVG for this font size.
    ///
    /// Returns `None` for sizes that are not positive and finite,
    /// or would exceed [`MAX_GLYPH_SIZE`].
    pub fn rasterize(&self, font_size_px: f32) -> Option<RasterizedGlyph> {
        profiling::function_scope!();

        let Self {
            tree,
            height_em,
            baseline_offset_em,
            advance_em,
            is_color,
        } = self;

        if !font_size_px.is_finite() || font_size_px <= 0.0 {
            return None;
        }

        let source_size = Vec2::new(tree.size().width(), tree.size().height());
        if source_size.x <= 0.0 || source_size.y <= 0.0 {
            return None;
        }
        let height_px = height_em * font_size_px;
        let size_px = (source_size * (height_px / source_size.y))
            .round()
            .max(Vec2::splat(1.0));
        let [width, height] = [size_px.x as usize, size_px.y as usize];
        if MAX_GLYPH_SIZE < width || MAX_GLYPH_SIZE < height {
            log::warn!("SVG glyph of {width}x{height} px is too big for the font atlas");
            return None;
        }

        let mut pixmap = Pixmap::new(width as u32, height as u32)?;
        resvg::render(
            tree,
            Transform::from_scale(size_px.x / source_size.x, size_px.y / source_size.y),
            &mut pixmap.as_mut(),
        );

        let mut image = ColorImage::from_rgba_premultiplied([width, height], pixmap.data());
        if !is_color {
            // The atlas stores coverage for monochrome glyphs, and the tessellator tints them:
            for pixel in &mut image.pixels {
                *pixel = Color32::from_white_alpha(pixel.a());
            }
        }

        let advance_px = advance_em.map_or(size_px.x, |advance_em| advance_em * font_size_px);
        let offset_px = vec2(
            (advance_px - size_px.x) / 2.0,
            -(baseline_offset_em * font_size_px + size_px.y),
        );

        Some(RasterizedGlyph {
            bitmap: GlyphBitmap {
                image,
                offset_px,
                is_color: *is_color,
            },
            advance_px,
        })
    }

    /// A [`GlyphRasterizer`] that renders this SVG for the given grapheme cluster
    /// (usually a single character), in every font family.
    ///
    /// It has [`FontPriority::Highest`], so it overrides the installed fonts.
    /// Use [`GlyphRasterizer::with_priority`] to make it a fallback instead.
    pub fn into_rasterizer(self, cluster: impl Into<String>) -> GlyphRasterizer {
        let cluster: String = cluster.into();
        GlyphRasterizer::new(move |request: &GlyphRasterizerRequest<'_>| {
            if request.cluster == cluster {
                self.rasterize(request.font_size_px)
            } else {
                None
            }
        })
        .with_priority(FontPriority::Highest)
    }
}

/// Does anything in this group use a color that is not a shade of gray?
fn has_color(group: &Group) -> bool {
    group.children().iter().any(|node| match node {
        Node::Group(group) => has_color(group),
        Node::Path(path) => {
            path.fill()
                .is_some_and(|fill| paint_has_color(fill.paint()))
                || path
                    .stroke()
                    .is_some_and(|stroke| paint_has_color(stroke.paint()))
        }
        Node::Image(_) => true,
        Node::Text(text) => has_color(text.flattened()),
    })
}

fn paint_has_color(paint: &Paint) -> bool {
    match paint {
        Paint::Color(color) => !is_gray(*color),
        Paint::LinearGradient(gradient) => {
            gradient.stops().iter().any(|stop| !is_gray(stop.color()))
        }
        Paint::RadialGradient(gradient) => {
            gradient.stops().iter().any(|stop| !is_gray(stop.color()))
        }
        Paint::Pattern(pattern) => has_color(pattern.root()),
    }
}

fn is_gray(color: Color) -> bool {
    let Color { red, green, blue } = color;
    red == green && green == blue
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10x20 rectangle.
    fn rect_svg(fill: &str) -> Vec<u8> {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="20" fill="{fill}"/></svg>"#
        )
        .into_bytes()
    }

    fn red_rect() -> SvgGlyph {
        SvgGlyph::from_bytes(&rect_svg("red")).unwrap()
    }

    fn gray_rect() -> SvgGlyph {
        SvgGlyph::from_bytes(&rect_svg("#808080")).unwrap()
    }

    #[test]
    fn gray_is_tinted() {
        let glyph = gray_rect().rasterize(40.0).unwrap();
        assert_eq!(glyph.bitmap.image.size, [20, 40]);
        assert!(!glyph.bitmap.is_color);
        assert!(
            glyph
                .bitmap
                .image
                .pixels
                .iter()
                .all(|&p| p == Color32::WHITE)
        );
        assert_eq!(glyph.bitmap.offset_px, vec2(0.0, -40.0));
        assert_eq!(glyph.advance_px, 20.0);
    }

    #[test]
    fn color_is_kept() {
        let glyph = red_rect().rasterize(40.0).unwrap();
        assert!(glyph.bitmap.is_color);
        assert!(glyph.bitmap.image.pixels.iter().all(|&p| p == Color32::RED));
    }

    #[test]
    fn color_detection_can_be_overridden() {
        assert!(
            !red_rect()
                .with_color(false)
                .rasterize(40.0)
                .unwrap()
                .bitmap
                .is_color
        );
        assert!(
            gray_rect()
                .with_color(true)
                .rasterize(40.0)
                .unwrap()
                .bitmap
                .is_color
        );
    }

    #[test]
    fn detects_color_in_gradients_and_nested_groups() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
            <defs><linearGradient id="g"><stop offset="0" stop-color="black"/><stop offset="1" stop-color="blue"/></linearGradient></defs>
            <g><g><rect width="10" height="10" fill="url(#g)"/></g></g>
        </svg>"#;
        assert!(SvgGlyph::from_bytes(svg).unwrap().is_color);

        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
            <g><circle cx="5" cy="5" r="4" fill="none" stroke="white"/></g>
        </svg>"#;
        assert!(!SvgGlyph::from_bytes(svg).unwrap().is_color);
    }

    #[test]
    fn height_advance_and_baseline_offset() {
        let glyph = red_rect()
            .with_height(0.5)
            .with_advance(1.0)
            .with_baseline_offset(0.25)
            .rasterize(40.0)
            .unwrap();
        assert_eq!(glyph.bitmap.image.size, [10, 20]);
        assert_eq!(glyph.advance_px, 40.0);
        assert_eq!(
            glyph.bitmap.offset_px,
            vec2(15.0, -30.0),
            "Centered in the advance, 10 px above the baseline"
        );
    }

    #[test]
    fn rejects_bad_sizes() {
        assert!(red_rect().rasterize(0.0).is_none());
        assert!(red_rect().rasterize(f32::NAN).is_none());
        assert!(red_rect().rasterize(1e6).is_none());
    }

    #[test]
    fn rasterizer_only_handles_its_cluster() {
        let rasterizer = red_rect().into_rasterizer("…");
        assert_eq!(rasterizer.priority, FontPriority::Highest);
        let request = |cluster| GlyphRasterizerRequest {
            cluster,
            family: &egui::FontFamily::Proportional,
            font_size_px: 20.0,
            subpixel_offset_px: 0.0,
        };
        assert!((rasterizer.rasterize)(&request("…")).is_some());
        assert!((rasterizer.rasterize)(&request("a")).is_none());
    }

    #[test]
    fn overrides_glyph_in_layout() {
        use egui::epaint::text::{FontDefinitions, FontId, Fonts, TextOptions};

        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default())
            .with_glyph_rasterizer(red_rect().into_rasterizer("…"));
        let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            "…".to_owned(),
            FontId::proportional(20.0),
            Color32::WHITE,
        );
        let glyph = galley.rows[0].row.glyphs[0];
        assert_eq!(glyph.chr, '…');
        assert_eq!(glyph.advance_width, 10.0);
        assert!(!glyph.uv_rect.is_nothing());
    }
}
