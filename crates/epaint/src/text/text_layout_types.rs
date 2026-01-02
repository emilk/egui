use std::ops::Range;
use std::sync::Arc;

use super::{
    cursor::{CCursor, LayoutCursor},
    font::UvRect,
};
use crate::{Color32, FontId, Mesh, Stroke, text::FontsView};
use emath::{Align, GuiRounding as _, NumExt as _, OrderedFloat, Pos2, Rect, Vec2, pos2, vec2};

/// Describes the task of laying out text.
///
/// This supports mixing different fonts, color and formats (underline etc).
///
/// Pass this to [`crate::FontsView::layout_job`] or [`crate::text::layout`].
///
/// ## Example:
/// ```
/// use epaint::{Color32, text::{LayoutJob, TextFormat}, FontFamily, FontId};
///
/// let mut job = LayoutJob::default();
/// job.append(
///     "Hello ",
///     0.0,
///     TextFormat {
///         font_id: FontId::new(14.0, FontFamily::Proportional),
///         color: Color32::WHITE,
///         ..Default::default()
///     },
/// );
/// job.append(
///     "World!",
///     0.0,
///     TextFormat {
///         font_id: FontId::new(14.0, FontFamily::Monospace),
///         color: Color32::BLACK,
///         ..Default::default()
///     },
/// );
/// ```
///
/// As you can see, constructing a [`LayoutJob`] is currently a lot of work.
/// It would be nice to have a helper macro for it!
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayoutJob {
    /// The complete text of this job, referenced by [`LayoutSection`].
    pub text: String,

    /// The different section, which can have different fonts, colors, etc.
    pub sections: Vec<LayoutSection>,

    /// Controls the text wrapping and elision.
    pub wrap: TextWrapping,

    /// The first row must be at least this high.
    /// This is in case we lay out text that is the continuation
    /// of some earlier text (sharing the same row),
    /// in which case this will be the height of the earlier text.
    /// In other cases, set this to `0.0`.
    pub first_row_min_height: f32,

    /// If `true`, all `\n` characters will result in a new _paragraph_,
    /// starting on a new row.
    ///
    /// If `false`, all `\n` characters will be ignored
    /// and show up as the replacement character.
    ///
    /// Default: `true`.
    pub break_on_newline: bool,

    /// How to horizontally align the text (`Align::LEFT`, `Align::Center`, `Align::RIGHT`).
    pub halign: Align,

    /// Justify text so that word-wrapped rows fill the whole [`TextWrapping::max_width`].
    pub justify: bool,

    /// Round output sizes using [`emath::GuiRounding`], to avoid rounding errors in layout code.
    pub round_output_to_gui: bool,
}

impl Default for LayoutJob {
    #[inline]
    fn default() -> Self {
        Self {
            text: Default::default(),
            sections: Default::default(),
            wrap: Default::default(),
            first_row_min_height: 0.0,
            break_on_newline: true,
            halign: Align::LEFT,
            justify: false,
            round_output_to_gui: true,
        }
    }
}

impl LayoutJob {
    /// Break on `\n` and at the given wrap width.
    #[inline]
    pub fn simple(text: String, font_id: FontId, color: Color32, wrap_width: f32) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format: TextFormat::simple(font_id, color),
            }],
            text,
            wrap: TextWrapping {
                max_width: wrap_width,
                ..Default::default()
            },
            break_on_newline: true,
            ..Default::default()
        }
    }

    /// Break on `\n`
    #[inline]
    pub fn simple_format(text: String, format: TextFormat) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format,
            }],
            text,
            break_on_newline: true,
            ..Default::default()
        }
    }

    /// Does not break on `\n`, but shows the replacement character instead.
    #[inline]
    pub fn simple_singleline(text: String, font_id: FontId, color: Color32) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format: TextFormat::simple(font_id, color),
            }],
            text,
            wrap: Default::default(),
            break_on_newline: false,
            ..Default::default()
        }
    }

    #[inline]
    pub fn single_section(text: String, format: TextFormat) -> Self {
        Self {
            sections: vec![LayoutSection {
                leading_space: 0.0,
                byte_range: 0..text.len(),
                format,
            }],
            text,
            wrap: Default::default(),
            break_on_newline: true,
            ..Default::default()
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    /// Helper for adding a new section when building a [`LayoutJob`].
    pub fn append(&mut self, text: &str, leading_space: f32, format: TextFormat) {
        let start = self.text.len();
        self.text += text;
        let byte_range = start..self.text.len();
        self.sections.push(LayoutSection {
            leading_space,
            byte_range,
            format,
        });
    }

    /// The height of the tallest font used in the job.
    ///
    /// Returns a value rounded to [`emath::GUI_ROUNDING`].
    pub fn font_height(&self, fonts: &mut FontsView<'_>) -> f32 {
        let mut max_height = 0.0_f32;
        for section in &self.sections {
            max_height = max_height.max(fonts.row_height(&section.format.font_id));
        }
        max_height
    }

    /// The wrap with, with a small margin in some cases.
    pub fn effective_wrap_width(&self) -> f32 {
        if self.round_output_to_gui {
            // On a previous pass we may have rounded down by at most 0.5 and reported that as a width.
            // egui may then set that width as the max width for subsequent frames, and it is important
            // that we then don't wrap earlier.
            self.wrap.max_width + 0.5
        } else {
            self.wrap.max_width
        }
    }
}

impl std::hash::Hash for LayoutJob {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            text,
            sections,
            wrap,
            first_row_min_height,
            break_on_newline,
            halign,
            justify,
            round_output_to_gui,
        } = self;

        text.hash(state);
        sections.hash(state);
        wrap.hash(state);
        emath::OrderedFloat(*first_row_min_height).hash(state);
        break_on_newline.hash(state);
        halign.hash(state);
        justify.hash(state);
        round_output_to_gui.hash(state);
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct LayoutSection {
    /// Can be used for first row indentation.
    pub leading_space: f32,

    /// Range into the galley text
    pub byte_range: Range<usize>,

    pub format: TextFormat,
}

impl std::hash::Hash for LayoutSection {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            leading_space,
            byte_range,
            format,
        } = self;
        OrderedFloat(*leading_space).hash(state);
        byte_range.hash(state);
        format.hash(state);
    }
}

// ----------------------------------------------------------------------------

/// Formatting option for a section of text.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextFormat {
    pub font_id: FontId,

    /// Extra spacing between letters, in points.
    ///
    /// Default: 0.0.
    pub extra_letter_spacing: f32,

    /// Explicit line height of the text in points.
    ///
    /// This is the distance between the bottom row of two subsequent lines of text.
    ///
    /// If `None` (the default), the line height is determined by the font.
    ///
    /// For even text it is recommended you round this to an even number of _pixels_.
    pub line_height: Option<f32>,

    /// Text color
    pub color: Color32,

    pub background: Color32,

    /// Amount to expand background fill by.
    ///
    /// Default: 1.0
    pub expand_bg: f32,

    pub italics: bool,

    pub underline: Stroke,

    pub strikethrough: Stroke,

    /// If you use a small font and [`Align::TOP`] you
    /// can get the effect of raised text.
    ///
    /// If you use a small font and [`Align::BOTTOM`]
    /// you get the effect of a subscript.
    ///
    /// If you use [`Align::Center`], you get text that is centered
    /// around a common center-line, which is nice when mixining emojis
    /// and normal text in e.g. a button.
    pub valign: Align,
}

impl Default for TextFormat {
    #[inline]
    fn default() -> Self {
        Self {
            font_id: FontId::default(),
            extra_letter_spacing: 0.0,
            line_height: None,
            color: Color32::GRAY,
            background: Color32::TRANSPARENT,
            expand_bg: 1.0,
            italics: false,
            underline: Stroke::NONE,
            strikethrough: Stroke::NONE,
            valign: Align::BOTTOM,
        }
    }
}

impl std::hash::Hash for TextFormat {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            font_id,
            extra_letter_spacing,
            line_height,
            color,
            background,
            expand_bg,
            italics,
            underline,
            strikethrough,
            valign,
        } = self;
        font_id.hash(state);
        emath::OrderedFloat(*extra_letter_spacing).hash(state);
        if let Some(line_height) = *line_height {
            emath::OrderedFloat(line_height).hash(state);
        }
        color.hash(state);
        background.hash(state);
        emath::OrderedFloat(*expand_bg).hash(state);
        italics.hash(state);
        underline.hash(state);
        strikethrough.hash(state);
        valign.hash(state);
    }
}

impl TextFormat {
    #[inline]
    pub fn simple(font_id: FontId, color: Color32) -> Self {
        Self {
            font_id,
            color,
            ..Default::default()
        }
    }
}

// ----------------------------------------------------------------------------

/// How to wrap and elide text.
///
/// This enum is used in high-level APIs where providing a [`TextWrapping`] is too verbose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextWrapMode {
    /// The text should expand the `Ui` size when reaching its boundary.
    Extend,

    /// The text should wrap to the next line when reaching the `Ui` boundary.
    Wrap,

    /// The text should be elided using "…" when reaching the `Ui` boundary.
    ///
    /// Note that using [`TextWrapping`] and [`LayoutJob`] offers more control over the elision.
    Truncate,
}

/// Controls the text wrapping and elision of a [`LayoutJob`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TextWrapping {
    /// Wrap text so that no row is wider than this.
    ///
    /// If you would rather truncate text that doesn't fit, set [`Self::max_rows`] to `1`.
    ///
    /// Set `max_width` to [`f32::INFINITY`] to turn off wrapping and elision.
    ///
    /// Note that `\n` always produces a new row
    /// if [`LayoutJob::break_on_newline`] is `true`.
    pub max_width: f32,

    /// Maximum amount of rows the text galley should have.
    ///
    /// If this limit is reached, text will be truncated
    /// and [`Self::overflow_character`] appended to the final row.
    /// You can detect this by checking [`Galley::elided`].
    ///
    /// If set to `0`, no text will be outputted.
    ///
    /// If set to `1`, a single row will be outputted,
    /// eliding the text after [`Self::max_width`] is reached.
    /// When you set `max_rows = 1`, it is recommended you also set [`Self::break_anywhere`] to `true`.
    ///
    /// Default value: `usize::MAX`.
    pub max_rows: usize,

    /// If `true`: Allow breaking between any characters.
    /// If `false` (default): prefer breaking between words, etc.
    ///
    /// NOTE: Due to limitations in the current implementation,
    /// when truncating text using [`Self::max_rows`] the text may be truncated
    /// in the middle of a word even if [`Self::break_anywhere`] is `false`.
    /// Therefore it is recommended to set [`Self::break_anywhere`] to `true`
    /// whenever [`Self::max_rows`] is set to `1`.
    pub break_anywhere: bool,

    /// Character to use to represent elided text.
    ///
    /// The default is `…`.
    ///
    /// If not set, no character will be used (but the text will still be elided).
    pub overflow_character: Option<char>,
}

impl std::hash::Hash for TextWrapping {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self {
            max_width,
            max_rows,
            break_anywhere,
            overflow_character,
        } = self;
        emath::OrderedFloat(*max_width).hash(state);
        max_rows.hash(state);
        break_anywhere.hash(state);
        overflow_character.hash(state);
    }
}

impl Default for TextWrapping {
    fn default() -> Self {
        Self {
            max_width: f32::INFINITY,
            max_rows: usize::MAX,
            break_anywhere: false,
            overflow_character: Some('…'),
        }
    }
}

impl TextWrapping {
    /// Create a [`TextWrapping`] from a [`TextWrapMode`] and an available width.
    pub fn from_wrap_mode_and_width(mode: TextWrapMode, max_width: f32) -> Self {
        match mode {
            TextWrapMode::Extend => Self::no_max_width(),
            TextWrapMode::Wrap => Self::wrap_at_width(max_width),
            TextWrapMode::Truncate => Self::truncate_at_width(max_width),
        }
    }

    /// A row can be as long as it need to be.
    pub fn no_max_width() -> Self {
        Self {
            max_width: f32::INFINITY,
            ..Default::default()
        }
    }

    /// A row can be at most `max_width` wide but can wrap in any number of lines.
    pub fn wrap_at_width(max_width: f32) -> Self {
        Self {
            max_width,
            ..Default::default()
        }
    }

    /// Elide text that doesn't fit within the given width, replaced with `…`.
    pub fn truncate_at_width(max_width: f32) -> Self {
        Self {
            max_width,
            max_rows: 1,
            break_anywhere: true,
            ..Default::default()
        }
    }
}

// ----------------------------------------------------------------------------

/// Text that has been laid out, ready for painting.
///
/// You can create a [`Galley`] using [`crate::FontsView::layout_job`];
///
/// Needs to be recreated if the underlying font atlas texture changes, which
/// happens under the following conditions:
/// - `pixels_per_point` or `max_texture_size` change. These parameters are set
///   in [`crate::text::Fonts::begin_pass`]. When using `egui` they are set
///   from `egui::InputState` and can change at any time.
/// - The atlas has become full. This can happen any time a new glyph is added
///   to the atlas, which in turn can happen any time new text is laid out.
///
/// The name comes from typography, where a "galley" is a metal tray
/// containing a column of set type, usually the size of a page of text.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Galley {
    /// The job that this galley is the result of.
    /// Contains the original string and style sections.
    pub job: Arc<LayoutJob>,

    /// Rows of text, from top to bottom, and their offsets.
    ///
    /// The number of characters in all rows sum up to `job.text.chars().count()`
    /// unless [`Self::elided`] is `true`.
    ///
    /// Note that a paragraph (a piece of text separated with `\n`)
    /// can be split up into multiple rows.
    pub rows: Vec<PlacedRow>,

    /// Set to true the text was truncated due to [`TextWrapping::max_rows`].
    pub elided: bool,

    /// Bounding rect.
    ///
    /// `rect.top()` is always 0.0.
    ///
    /// With [`LayoutJob::halign`]:
    /// * [`Align::LEFT`]: `rect.left() == 0.0`
    /// * [`Align::Center`]: `rect.center() == 0.0`
    /// * [`Align::RIGHT`]: `rect.right() == 0.0`
    pub rect: Rect,

    /// Tight bounding box around all the meshes in all the rows.
    /// Can be used for culling.
    pub mesh_bounds: Rect,

    /// Total number of vertices in all the row meshes.
    pub num_vertices: usize,

    /// Total number of indices in all the row meshes.
    pub num_indices: usize,

    /// The number of physical pixels for each logical point.
    /// Since this affects the layout, we keep track of it
    /// so that we can warn if this has changed once we get to
    /// tessellation.
    pub pixels_per_point: f32,

    pub(crate) intrinsic_size: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PlacedRow {
    /// The position of this [`Row`] relative to the galley.
    ///
    /// This is rounded to the closest _pixel_ in order to produce crisp, pixel-perfect text.
    pub pos: Pos2,

    /// The underlying unpositioned [`Row`].
    pub row: Arc<Row>,

    /// If true, this [`PlacedRow`] came from a paragraph ending with a `\n`.
    /// The `\n` itself is omitted from row's [`Row::glyphs`].
    /// A `\n` in the input text always creates a new [`PlacedRow`] below it,
    /// so that text that ends with `\n` has an empty [`PlacedRow`] last.
    /// This also implies that the last [`PlacedRow`] in a [`Galley`] always has `ends_with_newline == false`.
    pub ends_with_newline: bool,
}

impl PlacedRow {
    /// Logical bounding rectangle on font heights etc.
    ///
    /// This ignores / includes the `LayoutSection::leading_space`.
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.row.size)
    }

    /// Same as [`Self::rect`] but excluding the `LayoutSection::leading_space`.
    pub fn rect_without_leading_space(&self) -> Rect {
        let x = self.glyphs.first().map_or(self.pos.x, |g| g.pos.x);
        let size_x = self.size.x - x;
        Rect::from_min_size(Pos2::new(x, self.pos.y), Vec2::new(size_x, self.size.y))
    }
}

impl std::ops::Deref for PlacedRow {
    type Target = Row;

    fn deref(&self) -> &Self::Target {
        &self.row
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Row {
    /// This is included in case there are no glyphs.
    ///
    /// Only used during layout, then set to an invalid value in order to
    /// enable the paragraph-concat optimization path without having to
    /// adjust `section_index` when concatting.
    pub(crate) section_index_at_start: u32,

    /// One for each `char`.
    pub glyphs: Vec<Glyph>,

    /// Logical size based on font heights etc.
    /// Includes leading and trailing whitespace.
    pub size: Vec2,

    /// The mesh, ready to be rendered.
    pub visuals: RowVisuals,
}

/// The tessellated output of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RowVisuals {
    /// The tessellated text, using non-normalized (texel) UV coordinates.
    /// That is, you need to divide the uv coordinates by the texture size.
    pub mesh: Mesh,

    /// Bounds of the mesh, and can be used for culling.
    /// Does NOT include leading or trailing whitespace glyphs!!
    pub mesh_bounds: Rect,

    /// The number of triangle indices added before the first glyph triangle.
    ///
    /// This can be used to insert more triangles after the background but before the glyphs,
    /// i.e. for text selection visualization.
    pub glyph_index_start: usize,

    /// The range of vertices in the mesh that contain glyphs (as opposed to background, underlines, strikethorugh, etc).
    ///
    /// The glyph vertices comes after backgrounds (if any), but before any underlines and strikethrough.
    pub glyph_vertex_range: Range<usize>,
}

impl Default for RowVisuals {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            mesh_bounds: Rect::NOTHING,
            glyph_index_start: 0,
            glyph_vertex_range: 0..0,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Glyph {
    /// The character this glyph represents.
    pub chr: char,

    /// Baseline position, relative to the row.
    /// Logical position: pos.y is the same for all chars of the same [`TextFormat`].
    pub pos: Pos2,

    /// Logical width of the glyph.
    pub advance_width: f32,

    /// Height of this row of text.
    ///
    /// Usually same as [`Self::font_height`],
    /// unless explicitly overridden by [`TextFormat::line_height`].
    pub line_height: f32,

    /// The ascent of this font.
    pub font_ascent: f32,

    /// The row/line height of this font.
    pub font_height: f32,

    /// The ascent of the sub-font within the font (`FontFace`).
    pub font_face_ascent: f32,

    /// The row/line height of the sub-font within the font (`FontFace`).
    pub font_face_height: f32,

    /// Position and size of the glyph in the font texture, in texels.
    pub uv_rect: UvRect,

    /// Index into [`LayoutJob::sections`]. Decides color etc.
    ///
    /// Only used during layout, then set to an invalid value in order to
    /// enable the paragraph-concat optimization path without having to
    /// adjust `section_index` when concatting.
    pub(crate) section_index: u32,

    /// Which is our first vertex in [`RowVisuals::mesh`].
    pub first_vertex: u32,
}

impl Glyph {
    #[inline]
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.advance_width, self.line_height)
    }

    #[inline]
    pub fn max_x(&self) -> f32 {
        self.pos.x + self.advance_width
    }

    /// Same y range for all characters with the same [`TextFormat`].
    #[inline]
    pub fn logical_rect(&self) -> Rect {
        Rect::from_min_size(self.pos - vec2(0.0, self.font_ascent), self.size())
    }
}

// ----------------------------------------------------------------------------

impl Row {
    /// The text on this row, excluding the implicit `\n` if any.
    pub fn text(&self) -> String {
        self.glyphs.iter().map(|g| g.chr).collect()
    }

    /// Excludes the implicit `\n` after the [`Row`], if any.
    #[inline]
    pub fn char_count_excluding_newline(&self) -> usize {
        self.glyphs.len()
    }

    /// Closest char at the desired x coordinate in row-relative coordinates.
    /// Returns something in the range `[0, char_count_excluding_newline()]`.
    pub fn char_at(&self, desired_x: f32) -> usize {
        for (i, glyph) in self.glyphs.iter().enumerate() {
            if desired_x < glyph.logical_rect().center().x {
                return i;
            }
        }
        self.char_count_excluding_newline()
    }

    pub fn x_offset(&self, column: usize) -> f32 {
        if let Some(glyph) = self.glyphs.get(column) {
            glyph.pos.x
        } else {
            self.size.x
        }
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.size.y
    }
}

impl PlacedRow {
    #[inline]
    pub fn min_y(&self) -> f32 {
        self.rect().top()
    }

    #[inline]
    pub fn max_y(&self) -> f32 {
        self.rect().bottom()
    }

    /// Includes the implicit `\n` after the [`PlacedRow`], if any.
    #[inline]
    pub fn char_count_including_newline(&self) -> usize {
        self.row.glyphs.len() + (self.ends_with_newline as usize)
    }
}

impl Galley {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.job.is_empty()
    }

    /// The full, non-elided text of the input job.
    #[inline]
    pub fn text(&self) -> &str {
        &self.job.text
    }

    #[inline]
    pub fn size(&self) -> Vec2 {
        self.rect.size()
    }

    /// This is the size that a non-wrapped, non-truncated, non-justified version of the text
    /// would have.
    ///
    /// Useful for advanced layouting.
    #[inline]
    pub fn intrinsic_size(&self) -> Vec2 {
        // We do the rounding here instead of in `round_output_to_gui` so that rounding
        // errors don't accumulate when concatenating multiple galleys.
        if self.job.round_output_to_gui {
            self.intrinsic_size.round_ui()
        } else {
            self.intrinsic_size
        }
    }

    pub(crate) fn round_output_to_gui(&mut self) {
        for placed_row in &mut self.rows {
            // Optimization: only call `make_mut` if necessary (can cause a deep clone)
            let rounded_size = placed_row.row.size.round_ui();
            if placed_row.row.size != rounded_size {
                Arc::make_mut(&mut placed_row.row).size = rounded_size;
            }
        }

        let rect = &mut self.rect;

        let did_exceed_wrap_width_by_a_lot = rect.width() > self.job.wrap.max_width + 1.0;

        *rect = rect.round_ui();

        if did_exceed_wrap_width_by_a_lot {
            // If the user picked a too aggressive wrap width (e.g. more narrow than any individual glyph),
            // we should let the user know by reporting that our width is wider than the wrap width.
        } else {
            // Make sure we don't report being wider than the wrap width the user picked:
            rect.max.x = rect
                .max
                .x
                .at_most(rect.min.x + self.job.wrap.max_width)
                .floor_ui();
        }
    }

    /// Append each galley under the previous one.
    pub fn concat(job: Arc<LayoutJob>, galleys: &[Arc<Self>], pixels_per_point: f32) -> Self {
        profiling::function_scope!();

        let mut merged_galley = Self {
            job,
            rows: Vec::new(),
            elided: false,
            rect: Rect::ZERO,
            mesh_bounds: Rect::NOTHING,
            num_vertices: 0,
            num_indices: 0,
            pixels_per_point,
            intrinsic_size: Vec2::ZERO,
        };

        for (i, galley) in galleys.iter().enumerate() {
            let current_y_offset = merged_galley.rect.height();
            let is_last_galley = i + 1 == galleys.len();

            merged_galley
                .rows
                .extend(galley.rows.iter().enumerate().map(|(row_idx, placed_row)| {
                    let new_pos = placed_row.pos + current_y_offset * Vec2::Y;
                    let new_pos = new_pos.round_to_pixels(pixels_per_point);
                    merged_galley.mesh_bounds |=
                        placed_row.visuals.mesh_bounds.translate(new_pos.to_vec2());
                    merged_galley.rect |= Rect::from_min_size(new_pos, placed_row.size);

                    let mut ends_with_newline = placed_row.ends_with_newline;
                    let is_last_row_in_galley = row_idx + 1 == galley.rows.len();
                    // Since we remove the `\n` when splitting rows, we need to add it back here
                    ends_with_newline |= !is_last_galley && is_last_row_in_galley;
                    super::PlacedRow {
                        pos: new_pos,
                        row: Arc::clone(&placed_row.row),
                        ends_with_newline,
                    }
                }));

            merged_galley.num_vertices += galley.num_vertices;
            merged_galley.num_indices += galley.num_indices;
            // Note that if `galley.elided` is true this will be the last `Galley` in
            // the vector and the loop will end.
            merged_galley.elided |= galley.elided;
            merged_galley.intrinsic_size.x =
                f32::max(merged_galley.intrinsic_size.x, galley.intrinsic_size.x);
            merged_galley.intrinsic_size.y += galley.intrinsic_size.y;
        }

        if merged_galley.job.round_output_to_gui {
            merged_galley.round_output_to_gui();
        }

        merged_galley
    }
}

impl AsRef<str> for Galley {
    #[inline]
    fn as_ref(&self) -> &str {
        self.text()
    }
}

impl std::borrow::Borrow<str> for Galley {
    #[inline]
    fn borrow(&self) -> &str {
        self.text()
    }
}

impl std::ops::Deref for Galley {
    type Target = str;
    #[inline]
    fn deref(&self) -> &str {
        self.text()
    }
}

// ----------------------------------------------------------------------------

/// ## Physical positions
impl Galley {
    /// Zero-width rect past the last character.
    fn end_pos(&self) -> Rect {
        if let Some(row) = self.rows.last() {
            let x = row.rect().right();
            Rect::from_min_max(pos2(x, row.min_y()), pos2(x, row.max_y()))
        } else {
            // Empty galley
            Rect::from_min_max(pos2(0.0, 0.0), pos2(0.0, 0.0))
        }
    }

    /// Returns a 0-width Rect.
    fn pos_from_layout_cursor(&self, layout_cursor: &LayoutCursor) -> Rect {
        let Some(row) = self.rows.get(layout_cursor.row) else {
            return self.end_pos();
        };

        let x = row.x_offset(layout_cursor.column);
        Rect::from_min_max(pos2(x, row.min_y()), pos2(x, row.max_y()))
    }

    /// Returns a 0-width Rect.
    pub fn pos_from_cursor(&self, cursor: CCursor) -> Rect {
        self.pos_from_layout_cursor(&self.layout_from_cursor(cursor))
    }

    /// Cursor at the given position within the galley.
    ///
    /// A cursor above the galley is considered
    /// same as a cursor at the start,
    /// and a cursor below the galley is considered
    /// same as a cursor at the end.
    /// This allows implementing text-selection by dragging above/below the galley.
    pub fn cursor_from_pos(&self, pos: Vec2) -> CCursor {
        // Vertical margin around galley improves text selection UX
        const VMARGIN: f32 = 5.0;

        if let Some(first_row) = self.rows.first()
            && pos.y < first_row.min_y() - VMARGIN
        {
            return self.begin();
        }
        if let Some(last_row) = self.rows.last()
            && last_row.max_y() + VMARGIN < pos.y
        {
            return self.end();
        }

        let mut best_y_dist = f32::INFINITY;
        let mut cursor = CCursor::default();

        let mut ccursor_index = 0;

        for row in &self.rows {
            let min_y = row.min_y();
            let max_y = row.max_y();

            let is_pos_within_row = min_y <= pos.y && pos.y <= max_y;
            let y_dist = (min_y - pos.y).abs().min((max_y - pos.y).abs());
            if is_pos_within_row || y_dist < best_y_dist {
                best_y_dist = y_dist;
                // char_at is `Row` not `PlacedRow` relative which means we have to subtract the pos.
                let column = row.char_at(pos.x - row.pos.x);
                let prefer_next_row = column < row.char_count_excluding_newline();
                cursor = CCursor {
                    index: ccursor_index + column,
                    prefer_next_row,
                };

                if is_pos_within_row {
                    return cursor;
                }
            }
            ccursor_index += row.char_count_including_newline();
        }

        cursor
    }
}

/// ## Cursor positions
impl Galley {
    /// Cursor to the first character.
    ///
    /// This is the same as [`CCursor::default`].
    #[inline]
    #[expect(clippy::unused_self)]
    pub fn begin(&self) -> CCursor {
        CCursor::default()
    }

    /// Cursor to one-past last character.
    pub fn end(&self) -> CCursor {
        if self.rows.is_empty() {
            return Default::default();
        }
        let mut ccursor = CCursor {
            index: 0,
            prefer_next_row: true,
        };
        for row in &self.rows {
            let row_char_count = row.char_count_including_newline();
            ccursor.index += row_char_count;
        }
        ccursor
    }
}

/// ## Cursor conversions
impl Galley {
    // The returned cursor is clamped.
    pub fn layout_from_cursor(&self, cursor: CCursor) -> LayoutCursor {
        let prefer_next_row = cursor.prefer_next_row;
        let mut ccursor_it = CCursor {
            index: 0,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            let row_char_count = row.char_count_excluding_newline();

            if ccursor_it.index <= cursor.index && cursor.index <= ccursor_it.index + row_char_count
            {
                let column = cursor.index - ccursor_it.index;

                let select_next_row_instead = prefer_next_row
                    && !row.ends_with_newline
                    && column >= row.char_count_excluding_newline();
                if !select_next_row_instead {
                    return LayoutCursor {
                        row: row_nr,
                        column,
                    };
                }
            }
            ccursor_it.index += row.char_count_including_newline();
        }
        debug_assert!(ccursor_it == self.end(), "Cursor out of bounds");

        if let Some(last_row) = self.rows.last() {
            LayoutCursor {
                row: self.rows.len() - 1,
                column: last_row.char_count_including_newline(),
            }
        } else {
            Default::default()
        }
    }

    fn cursor_from_layout(&self, layout_cursor: LayoutCursor) -> CCursor {
        if layout_cursor.row >= self.rows.len() {
            return self.end();
        }

        let prefer_next_row =
            layout_cursor.column < self.rows[layout_cursor.row].char_count_excluding_newline();
        let mut cursor_it = CCursor {
            index: 0,
            prefer_next_row,
        };

        for (row_nr, row) in self.rows.iter().enumerate() {
            if row_nr == layout_cursor.row {
                cursor_it.index += layout_cursor
                    .column
                    .at_most(row.char_count_excluding_newline());

                return cursor_it;
            }
            cursor_it.index += row.char_count_including_newline();
        }
        cursor_it
    }
}

/// ## Cursor positions
impl Galley {
    #[expect(clippy::unused_self)]
    pub fn cursor_left_one_character(&self, cursor: &CCursor) -> CCursor {
        if cursor.index == 0 {
            Default::default()
        } else {
            CCursor {
                index: cursor.index - 1,
                prefer_next_row: true, // default to this when navigating. It is more often useful to put cursor at the beginning of a row than at the end.
            }
        }
    }

    pub fn cursor_right_one_character(&self, cursor: &CCursor) -> CCursor {
        CCursor {
            index: (cursor.index + 1).min(self.end().index),
            prefer_next_row: true, // default to this when navigating. It is more often useful to put cursor at the beginning of a row than at the end.
        }
    }

    pub fn clamp_cursor(&self, cursor: &CCursor) -> CCursor {
        self.cursor_from_layout(self.layout_from_cursor(*cursor))
    }

    pub fn cursor_up_one_row(
        &self,
        cursor: &CCursor,
        h_pos: Option<f32>,
    ) -> (CCursor, Option<f32>) {
        let layout_cursor = self.layout_from_cursor(*cursor);
        let h_pos = h_pos.unwrap_or_else(|| self.pos_from_layout_cursor(&layout_cursor).center().x);
        if layout_cursor.row == 0 {
            (CCursor::default(), None)
        } else {
            let new_row = layout_cursor.row - 1;

            let new_layout_cursor = {
                // keep same X coord
                let column = self.rows[new_row].char_at(h_pos);
                LayoutCursor {
                    row: new_row,
                    column,
                }
            };
            (self.cursor_from_layout(new_layout_cursor), Some(h_pos))
        }
    }

    pub fn cursor_down_one_row(
        &self,
        cursor: &CCursor,
        h_pos: Option<f32>,
    ) -> (CCursor, Option<f32>) {
        let layout_cursor = self.layout_from_cursor(*cursor);
        let h_pos = h_pos.unwrap_or_else(|| self.pos_from_layout_cursor(&layout_cursor).center().x);
        if layout_cursor.row + 1 < self.rows.len() {
            let new_row = layout_cursor.row + 1;

            let new_layout_cursor = {
                // keep same X coord
                let column = self.rows[new_row].char_at(h_pos);
                LayoutCursor {
                    row: new_row,
                    column,
                }
            };

            (self.cursor_from_layout(new_layout_cursor), Some(h_pos))
        } else {
            (self.end(), None)
        }
    }

    pub fn cursor_begin_of_row(&self, cursor: &CCursor) -> CCursor {
        let layout_cursor = self.layout_from_cursor(*cursor);
        self.cursor_from_layout(LayoutCursor {
            row: layout_cursor.row,
            column: 0,
        })
    }

    pub fn cursor_end_of_row(&self, cursor: &CCursor) -> CCursor {
        let layout_cursor = self.layout_from_cursor(*cursor);
        self.cursor_from_layout(LayoutCursor {
            row: layout_cursor.row,
            column: self.rows[layout_cursor.row].char_count_excluding_newline(),
        })
    }

    pub fn cursor_begin_of_paragraph(&self, cursor: &CCursor) -> CCursor {
        let mut layout_cursor = self.layout_from_cursor(*cursor);
        layout_cursor.column = 0;

        loop {
            let prev_row = layout_cursor
                .row
                .checked_sub(1)
                .and_then(|row| self.rows.get(row));

            let Some(prev_row) = prev_row else {
                // This is the first row
                break;
            };

            if prev_row.ends_with_newline {
                break;
            }

            layout_cursor.row -= 1;
        }

        self.cursor_from_layout(layout_cursor)
    }

    pub fn cursor_end_of_paragraph(&self, cursor: &CCursor) -> CCursor {
        let mut layout_cursor = self.layout_from_cursor(*cursor);
        loop {
            let row = &self.rows[layout_cursor.row];
            if row.ends_with_newline || layout_cursor.row == self.rows.len() - 1 {
                layout_cursor.column = row.char_count_excluding_newline();
                break;
            }

            layout_cursor.row += 1;
        }

        self.cursor_from_layout(layout_cursor)
    }
}
