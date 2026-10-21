#![expect(clippy::unwrap_used)] // TODO(emilk): remove unwraps

use alloc::sync::Arc;
use core::{iter, ops::Range};

use emath::{Align, GuiRounding as _, NumExt as _, Pos2, Rect, Vec2, pos2, vec2};

use crate::{
    Color32, Mesh, Stroke, Vertex,
    stroke::PathStroke,
    text::{
        ByteIndex, ByteRange, FontPriority,
        face_store::FontFaceKey,
        fonts::FontsImpl,
        styled_metrics::StyledMetrics,
        unicode::{is_cjk, is_cjk_break_allowed, is_combining_mark},
    },
};

use super::{
    ByteRangeExt as _, Galley, Glyph, LayoutJob, LayoutSection, PlacedRow, Row, RowVisuals,
    TextDirection, VariationCoords,
    family::FamilyKey,
    font_face::{ClusterGlyph, FontFace, GlyphInfo, ShapedCluster, ShapedGlyph},
    glyph_atlas::{GlyphAllocation, OutlineGlyph, RasterGlyphAllocation},
};

// ----------------------------------------------------------------------------

/// Represents GUI scale and convenience methods for rounding to pixels.
#[derive(Clone, Copy)]
struct PointScale {
    pub pixels_per_point: f32,
}

impl PointScale {
    #[inline(always)]
    pub fn new(pixels_per_point: f32) -> Self {
        Self { pixels_per_point }
    }

    #[inline(always)]
    pub fn pixels_per_point(&self) -> f32 {
        self.pixels_per_point
    }

    #[inline(always)]
    pub fn round_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).round() / self.pixels_per_point
    }

    #[inline(always)]
    pub fn floor_to_pixel(&self, point: f32) -> f32 {
        (point * self.pixels_per_point).floor() / self.pixels_per_point
    }
}

// ----------------------------------------------------------------------------

/// Temporary storage before line-wrapping.
#[derive(Clone)]
struct Paragraph {
    /// Start of the next glyph to be added. In screen-space / physical pixels.
    pub cursor_x_px: f32,

    /// This is included in case there are no glyphs
    pub section_index_at_start: u32,

    pub glyphs: Vec<Glyph>,

    /// In case of an empty paragraph ("\n"), use this as height.
    pub empty_paragraph_height: f32,

    /// Is the base direction of this paragraph right-to-left?
    pub rtl: bool,
}

impl Paragraph {
    pub fn from_section_index(section_index_at_start: u32) -> Self {
        Self {
            cursor_x_px: 0.0,
            section_index_at_start,
            glyphs: vec![],
            empty_paragraph_height: 0.0,
            rtl: false,
        }
    }
}

/// Layout text into a [`Galley`].
///
/// In most cases you should use [`crate::FontsView::layout_job`] instead
/// since that memoizes the input, making subsequent layouting of the same text much faster.
pub(crate) fn layout(fonts: &mut FontsImpl, pixels_per_point: f32, job: Arc<LayoutJob>) -> Galley {
    profiling::function_scope!();

    job.debug_sanity_check();

    if job.wrap.max_rows == 0 {
        // Early-out: no text
        return Galley {
            job,
            rows: Default::default(),
            rect: Rect::ZERO,
            mesh_bounds: Rect::NOTHING,
            num_vertices: 0,
            num_indices: 0,
            pixels_per_point,
            elided: true,
            intrinsic_size: Vec2::ZERO,
        };
    }

    // For most of this we ignore the y coordinate:

    let bidi = BidiLevels::new(&job.text, job.direction);

    let mut paragraphs = vec![Paragraph::from_section_index(0)];
    paragraphs[0].rtl = bidi.is_rtl_paragraph(0);
    {
        let mut shape_buffer = fonts.take_shape_buffer();
        for section_index in 0..job.sections.len() {
            shape_buffer = layout_section(
                fonts,
                shape_buffer,
                pixels_per_point,
                &job,
                section_index as u32,
                &bidi,
                &mut paragraphs,
            );
        }
        fonts.return_shape_buffer(shape_buffer);
    }

    let point_scale = PointScale::new(pixels_per_point);

    let intrinsic_size = calculate_intrinsic_size(point_scale, &job, &paragraphs);

    let mut elided = false;
    let mut rows = rows_from_paragraphs(paragraphs, &job, pixels_per_point, &mut elided);
    if elided && let Some(last_placed) = rows.last_mut() {
        let last_row = Arc::make_mut(&mut last_placed.row);
        replace_last_glyph_with_overflow_character(fonts, pixels_per_point, &job, last_row);
        if let Some(last) = last_row.glyphs.last() {
            last_row.size.x = last.max_x();
        }
    }

    // Up to here every glyph sits at its logical position. Now place
    // right-to-left text where a reader expects it:
    if bidi.levels.is_some() {
        for placed_row in &mut rows {
            let row = Arc::make_mut(&mut placed_row.row);
            reset_whitespace_levels(row);
            if row.glyphs.iter().any(Glyph::is_rtl) {
                reorder_row_visually(point_scale, row);
            }
        }
    }

    let justify = job.justify && job.wrap.max_width.is_finite();

    let num_rows = rows.len();
    for (i, placed_row) in rows.iter_mut().enumerate() {
        // Aligned around x = 0 for now, see `align_rtl_rows_to_start`:
        let halign = if starts_on_the_right(&job, &placed_row.row) {
            Align::RIGHT
        } else {
            job.halign
        };
        let is_last_row = i + 1 == num_rows;
        let justify_row = justify && !placed_row.ends_with_newline && !is_last_row;
        if justify || halign != Align::LEFT {
            halign_and_justify_row(
                point_scale,
                placed_row,
                halign,
                job.wrap.max_width,
                justify_row,
                job.keep_trailing_whitespace,
            );
        }
    }
    align_rtl_rows_to_start(&job, &mut rows, pixels_per_point);

    // Calculate the Y positions and tessellate the text:
    galley_from_rows(point_scale, job, rows, elided, intrinsic_size)
}

/// Shared context for emitting shaped glyphs into a [`Paragraph`].
struct ShapingContext {
    /// The family of the section being shaped.
    family: FamilyKey,
    pixels_per_point: f32,
    font_size: f32,
    line_height: f32,
    extra_letter_spacing: f32,
    section_index: u32,
    font_metrics: StyledMetrics,
    is_first_glyph_in_section: bool,

    /// Bidi embedding level of the run being emitted.
    bidi_level: u8,
}

impl ShapingContext {
    fn glyph(
        &self,
        chr: char,
        physical_x: i32,
        advance_width_px: f32,
        face_metrics: &StyledMetrics,
        alloc: GlyphAllocation,
    ) -> Glyph {
        let GlyphAllocation { uv_rect, is_color } = alloc;
        Glyph {
            chr,
            pos: pos2(physical_x as f32 / self.pixels_per_point, f32::NAN),
            advance_width: advance_width_px / self.pixels_per_point,
            line_height: self.line_height,
            font_face_height: face_metrics.row_height,
            font_face_ascent: face_metrics.ascent,
            font_height: self.font_metrics.row_height,
            font_ascent: self.font_metrics.ascent,
            uv_rect,
            is_color,
            bidi_level: self.bidi_level,
            section_index: self.section_index,
            first_vertex: 0,
        }
    }
}

/// Produced by [`segment_into_runs`] for text shaping.
#[derive(Debug)]
struct TextRun {
    /// Which font face should shape this run.
    ///
    /// Ignored if `raster` is set.
    font_key: FontFaceKey,

    /// Byte range within the section text.
    byte_range: ByteRange,

    /// Bidi embedding level (UAX #9) shared by every char in the run.
    /// Odd means right-to-left.
    bidi_level: u8,

    /// Set if a priority [`GlyphRasterizer`](crate::text::GlyphRasterizer) rendered this run.
    ///
    /// Such a run is exactly one grapheme cluster, and is not shaped.
    raster: Option<RasterGlyphAllocation>,
}

/// Emit shaped glyphs from a [`harfrust::GlyphBuffer`] into a [`Paragraph`].
///
/// Every character gets exactly one [`Glyph`] (`glyphs.len() == char_count`),
/// an invariant all cursor and selection code relies on, whatever the shaper did:
///
/// * a cluster with fewer glyphs than characters (flag emojis, ligatures) gets
///   zero-width "continuation" glyphs for the extra characters;
/// * a cluster with more glyphs than characters (a letter the font composes
///   from a base and marks) is drawn as one bitmap, see [`emit_cluster_bitmap`].
///
/// The glyphs are emitted in _logical_ order with increasing x, whatever the
/// run's direction; a right-to-left row is put in visual order later, by
/// [`reorder_row_visually`], once the rows are known.
fn layout_shaped_run(
    fonts: &mut FontsImpl,
    run: &TextRun,
    run_text: &str,
    glyph_buffer: &harfrust::GlyphBuffer,
    face_metrics: &StyledMetrics,
    ctx: &mut ShapingContext,
    paragraph: &mut Paragraph,
) {
    let shaped = ShapedRun {
        run,
        text: run_text,
        face_metrics,
        infos: glyph_buffer.glyph_infos(),
        positions: glyph_buffer.glyph_positions(),
    };

    let mut clusters = logical_clusters(shaped.infos, run.bidi_level % 2 == 1).peekable();
    while let Some(glyph_range) = clusters.next() {
        let byte_range = shaped.cluster_bytes(&glyph_range, clusters.peek());
        let char_count = run_text
            .get(byte_range.clone())
            .map_or(0, |cluster_text| cluster_text.chars().count());

        // Apply extra_letter_spacing only at cluster boundaries,
        // never between glyphs within the same cluster (e.g. base + mark).
        if !ctx.is_first_glyph_in_section {
            paragraph.cursor_x_px += ctx.extra_letter_spacing * ctx.pixels_per_point;
        }
        ctx.is_first_glyph_in_section = false;

        let first_glyph = paragraph.glyphs.len();
        let as_one_bitmap = if char_count < glyph_range.len() {
            emit_cluster_bitmap(fonts, &shaped, glyph_range.clone(), ctx, paragraph)
        } else {
            None
        };
        let emitted = as_one_bitmap
            .unwrap_or_else(|| emit_cluster_glyphs(fonts, &shaped, glyph_range, ctx, paragraph));

        // Every glyph of a cluster was emitted for the cluster's first char.
        // Give each its own char, in logical order, so the row's chars are the text.
        if let Some(cluster_text) = run_text.get(byte_range.clone()) {
            for (glyph, chr) in paragraph.glyphs[first_glyph..]
                .iter_mut()
                .zip(cluster_text.chars())
            {
                glyph.chr = chr;
            }
        }

        emit_continuation_glyphs(ctx, paragraph, run_text, byte_range, emitted, face_metrics);
    }
}

/// A shaped run being emitted: what every glyph in it shares.
#[derive(Clone, Copy)]
struct ShapedRun<'a> {
    run: &'a TextRun,

    /// The run's text; [`harfrust::GlyphInfo::cluster`] is a byte offset into it.
    text: &'a str,

    face_metrics: &'a StyledMetrics,
    infos: &'a [harfrust::GlyphInfo],
    positions: &'a [harfrust::GlyphPosition],
}

impl ShapedRun<'_> {
    /// The bytes of the cluster shaped into `glyph_range`,
    /// given the cluster after it in logical order (if any).
    fn cluster_bytes(
        &self,
        glyph_range: &Range<usize>,
        next: Option<&Range<usize>>,
    ) -> Range<usize> {
        let start = self.infos[glyph_range.start].cluster as usize;
        let end = next.map_or(self.text.len(), |next| {
            self.infos[next.start].cluster as usize
        });
        start..end
    }

    /// The first character of the cluster starting at byte `cluster`.
    fn char_at(&self, cluster: u32) -> char {
        self.text
            .get(cluster as usize..)
            .and_then(|s| s.chars().next())
            .unwrap_or('\u{FFFD}') // Unicode Replacement Character
    }
}

/// Emit a cluster that has more glyphs than characters as one glyph carrying one bitmap.
///
/// Some fonts compose a letter from a base glyph and mark glyphs (Hack's `ǻ`,
/// Noto Naskh Arabic's `ب`), so one character shapes to several glyphs. Emitting
/// one [`Glyph`] per shaped glyph would break `glyphs.len() == char_count`, so
/// the cluster is rendered into one atlas entry ([`FontsImpl::allocate_cluster`])
/// carried by its first character, with the cluster's whole advance.
///
/// `None` if the cluster cannot be drawn that way (a `.notdef` or a color glyph
/// in it); the caller then falls back to one glyph per shaped glyph.
fn emit_cluster_bitmap(
    fonts: &mut FontsImpl,
    shaped: &ShapedRun<'_>,
    glyph_range: Range<usize>,
    ctx: &ShapingContext,
    paragraph: &mut Paragraph,
) -> Option<usize> {
    let px_scale = shaped.face_metrics.px_scale_factor;
    let chr = shaped.char_at(shaped.infos[glyph_range.start].cluster);

    let mut advance_px = 0.0;
    let mut glyphs = Vec::with_capacity(glyph_range.len());
    for (info, pos) in shaped.infos[glyph_range.clone()]
        .iter()
        .zip(&shaped.positions[glyph_range])
    {
        let glyph_id = skrifa::GlyphId::new(info.glyph_id);
        if glyph_id == skrifa::GlyphId::NOTDEF {
            return None;
        }
        glyphs.push(ClusterGlyph {
            glyph_id,
            offset_px: vec2(
                advance_px + pos.x_offset as f32 * px_scale,
                -(pos.y_offset as f32 * px_scale), // harfrust Y+ up → screen Y+ down
            ),
        });
        advance_px += pos.x_advance as f32 * px_scale;
    }

    let OutlineGlyph { allocation, x_px } = fonts.allocate_cluster(
        shaped.run.font_key,
        shaped.face_metrics,
        &ShapedCluster {
            glyphs: &glyphs,
            h_pos: paragraph.cursor_x_px,
            is_cjk: is_cjk(chr),
        },
    )?;

    paragraph.cursor_x_px += advance_px;
    paragraph
        .glyphs
        .push(ctx.glyph(chr, x_px, advance_px, shaped.face_metrics, allocation));
    Some(1)
}

/// Emit one [`Glyph`] per shaped glyph of a cluster, and return how many that was.
///
/// Fewer than the cluster's glyphs when the shaper could not map a character:
/// its combining marks and repeated `.notdef` glyphs are dropped.
fn emit_cluster_glyphs(
    fonts: &mut FontsImpl,
    shaped: &ShapedRun<'_>,
    glyph_range: Range<usize>,
    ctx: &ShapingContext,
    paragraph: &mut Paragraph,
) -> usize {
    let ShapedRun {
        run,
        text: run_text,
        face_metrics,
        infos,
        positions,
    } = *shaped;
    let px_scale = face_metrics.px_scale_factor;
    let mut emitted = 0;
    let first_glyph = paragraph.glyphs.len();

    // The glyph that stands for the cluster. Not always the first:
    // a right-to-left shaper puts a cluster's marks before its base.
    let base = if run.bidi_level % 2 == 1 {
        glyph_range
            .clone()
            .find(|&idx| 0 < positions[idx].x_advance)
            .unwrap_or(glyph_range.start)
    } else {
        glyph_range.start
    };

    for idx in glyph_range.clone() {
        let (info, pos) = (&infos[idx], &positions[idx]);
        let glyph_id = skrifa::GlyphId::new(info.glyph_id);
        let cluster = info.cluster;
        let mut advance_width_px = pos.x_advance as f32 * px_scale;
        let x_offset_px = pos.x_offset as f32 * px_scale;
        let y_offset_px = -(pos.y_offset as f32 * px_scale); // harfrust Y+ up → screen Y+ down

        let chr = shaped.char_at(cluster);

        // Tab is a layout concept, not a glyph — the shaper doesn't know about tab stops.
        // Override the advance width using the font's configured tab size.
        if chr == '\t' {
            let tweak = fonts.face(run.font_key).map(|face| face.tweak());
            let tab_size = tweak.map_or(4.0, |t| t.tab_size);
            let (_, space_info) = fonts.glyph_info(ctx.family, ' ', face_metrics);
            let space_width_px = space_info.advance_width_unscaled.0 * px_scale;
            advance_width_px = tab_size * space_width_px;
        }

        // Thin space (U+2009) and narrow no-break space (U+202F):
        // override the shaper's advance width with the configured fraction of a space.
        if chr == '\u{2009}' || chr == '\u{202F}' {
            let tweak = fonts.face(run.font_key).map(|face| face.tweak());
            let thin_space_width = tweak.map_or(0.5, |t| t.thin_space_width);
            let (_, space_info) = fonts.glyph_info(ctx.family, ' ', face_metrics);
            let space_width_px = space_info.advance_width_unscaled.0 * px_scale;
            advance_width_px = thin_space_width * space_width_px;
        }

        let glyph = if glyph_id == skrifa::GlyphId::NOTDEF {
            // The shaper couldn't map this character. Drop combining marks and duplicate
            // NOTDEF glyphs within the same cluster. The base character is rasterized,
            // or rendered as the `.notdef` glyph ("tofu") if no rasterizer can handle the cluster.
            if is_combining_mark(chr) || idx != base {
                continue;
            }

            let cluster_text = run_text
                .get(cluster as usize..)
                .and_then(|text| {
                    unicode_segmentation::UnicodeSegmentation::graphemes(text, true).next()
                })
                .unwrap_or_default();

            if let Some(raster) = fonts.rasterize_cluster(
                FontPriority::Lowest,
                ctx.family,
                cluster_text,
                ctx.pixels_per_point,
                ctx.font_size,
            ) {
                raster_glyph(ctx, paragraph, chr, &raster, face_metrics)
            } else {
                // Use the fallback font face (not run.font_key which returned NOTDEF).
                let fallback_key = fonts.resolve_face(ctx.family, chr);
                let fallback_metrics = fonts
                    .face(fallback_key)
                    .map(|face| {
                        face.styled_metrics(
                            ctx.pixels_per_point,
                            ctx.font_size,
                            &Default::default(),
                        )
                    })
                    .unwrap_or_default();
                let (_, glyph_info) = fonts.glyph_info(ctx.family, chr, &fallback_metrics);

                // The shaper had no glyph, but the face may still resolve the character
                // (e.g. `\t` and thin spaces become a space with a custom advance).
                // Only a genuine `.notdef` is a missing glyph.
                if glyph_info.id == Some(skrifa::GlyphId::NOTDEF) {
                    fonts.on_missing_glyph(ctx.family, cluster_text);
                }
                let advance_width_px =
                    glyph_info.advance_width_unscaled.0 * fallback_metrics.px_scale_factor;
                let OutlineGlyph { allocation, x_px } = allocate_glyph_info(
                    fonts,
                    fallback_key,
                    &fallback_metrics,
                    glyph_info,
                    paragraph.cursor_x_px,
                    chr,
                );

                paragraph.cursor_x_px += advance_width_px;

                ctx.glyph(chr, x_px, advance_width_px, &fallback_metrics, allocation)
            }
        } else {
            let OutlineGlyph {
                allocation: mut glyph_alloc,
                x_px,
            } = fonts.allocate_glyph(
                run.font_key,
                face_metrics,
                &ShapedGlyph {
                    glyph_id,
                    h_pos: paragraph.cursor_x_px + x_offset_px,
                    is_cjk: is_cjk(chr),
                },
            );

            // Apply shaper y_offset — this varies per glyph instance so it
            // is not part of the cached ShapedGlyph / GlyphAllocation.
            glyph_alloc.uv_rect.offset.y += y_offset_px / ctx.pixels_per_point;

            paragraph.cursor_x_px += advance_width_px;

            ctx.glyph(chr, x_px, advance_width_px, face_metrics, glyph_alloc)
        };
        paragraph.glyphs.push(glyph);
        emitted += 1;
    }

    // A right-to-left shaper puts a cluster's marks before its base. Put the base first,
    // so it carries the cluster's first char and the marks move with it in
    // `reorder_row_visually`. Positions are already final, so only the order changes.
    if run.bidi_level % 2 == 1
        && let Some(cluster) = paragraph.glyphs.get_mut(first_glyph..)
        && let Some(base) = cluster.iter().position(|glyph| 0.0 < glyph.advance_width)
    {
        cluster.rotate_left(base);
    }

    emitted
}

/// Put `glyph_info` in the atlas, or nothing at all if it is invisible.
fn allocate_glyph_info(
    fonts: &mut FontsImpl,
    face_key: FontFaceKey,
    metrics: &StyledMetrics,
    glyph_info: GlyphInfo,
    h_pos_px: f32,
    chr: char,
) -> OutlineGlyph {
    let Some(glyph_id) = glyph_info.id else {
        return OutlineGlyph {
            allocation: GlyphAllocation::default(),
            x_px: h_pos_px.round() as i32,
        };
    };
    fonts.allocate_glyph(
        face_key,
        metrics,
        &ShapedGlyph {
            glyph_id,
            h_pos: h_pos_px,
            is_cjk: is_cjk(chr),
        },
    )
}

/// Emit one glyph for a rasterized cluster and advance the cursor.
fn raster_glyph(
    ctx: &ShapingContext,
    paragraph: &mut Paragraph,
    chr: char,
    raster: &RasterGlyphAllocation,
    face_metrics: &StyledMetrics,
) -> Glyph {
    let physical_x = paragraph.cursor_x_px.round() as i32;
    paragraph.cursor_x_px += raster.advance_px;
    ctx.glyph(
        chr,
        physical_x,
        raster.advance_px,
        face_metrics,
        raster.allocation,
    )
}

/// Emit the glyphs of a run that a [`GlyphRasterizer`](crate::text::GlyphRasterizer) rendered,
/// or that stands in for a cluster in a family without fonts (see [`FontsImpl::fontless_cluster`]).
///
/// The run is one grapheme cluster: its first char gets the bitmap,
/// and the rest zero-width continuation glyphs.
fn layout_raster_run(
    ctx: &mut ShapingContext,
    paragraph: &mut Paragraph,
    run_text: &str,
    raster: &RasterGlyphAllocation,
) {
    let Some(chr) = run_text.chars().next() else {
        return;
    };
    if !ctx.is_first_glyph_in_section {
        paragraph.cursor_x_px += ctx.extra_letter_spacing * ctx.pixels_per_point;
    }
    ctx.is_first_glyph_in_section = false;

    let face_metrics = ctx.font_metrics.clone();
    let glyph = raster_glyph(ctx, paragraph, chr, raster, &face_metrics);
    paragraph.glyphs.push(glyph);
    emit_continuation_glyphs(
        ctx,
        paragraph,
        run_text,
        0..run_text.len(),
        1,
        &face_metrics,
    );
}

/// Emit zero-width continuation glyphs when a cluster has more characters than
/// shaped glyphs.
///
/// This preserves the invariant `glyphs.len() == char_count` that all cursor
/// and text-selection code depends on. Continuation glyphs have
/// [`GlyphAllocation::default()`] so [`tessellate_glyphs`] skips them entirely.
fn emit_continuation_glyphs(
    ctx: &ShapingContext,
    paragraph: &mut Paragraph,
    run_text: &str,
    cluster_bytes: Range<usize>,
    cluster_glyph_count: usize,
    face_metrics: &StyledMetrics,
) {
    let Some(cluster_text) = run_text.get(cluster_bytes) else {
        return;
    };
    let char_count = cluster_text.chars().count();
    if char_count <= cluster_glyph_count {
        return;
    }

    let physical_x = paragraph.cursor_x_px.round() as i32;

    for chr in cluster_text.chars().skip(cluster_glyph_count) {
        paragraph.glyphs.push(ctx.glyph(
            chr,
            physical_x,
            0.0,
            face_metrics,
            GlyphAllocation::default(),
        ));
    }
}

// Ignores the Y coordinate.
#[must_use]
fn layout_section(
    fonts: &mut FontsImpl,
    mut shape_buffer: harfrust::UnicodeBuffer,
    pixels_per_point: f32,
    job: &LayoutJob,
    section_index: u32,
    bidi: &BidiLevels,
    out_paragraphs: &mut Vec<Paragraph>,
) -> harfrust::UnicodeBuffer {
    let section = &job.sections[section_index as usize];
    let LayoutSection {
        leading_space,
        byte_range,
        format,
    } = section;

    let family = fonts.family_key(&format.font_id.family);
    let font_size = format.font_id.size;
    let font_metrics = fonts.family_metrics(family, pixels_per_point, font_size, &format.coords);
    let line_height = section
        .format
        .line_height
        .unwrap_or(font_metrics.row_height);
    let extra_letter_spacing = section.format.extra_letter_spacing;

    let mut paragraph = out_paragraphs.last_mut().unwrap();
    if paragraph.glyphs.is_empty() {
        paragraph.empty_paragraph_height = line_height;
    }
    paragraph.cursor_x_px += leading_space * pixels_per_point;

    let section_text = &job.text[byte_range.as_usize()];
    let mut ctx = ShapingContext {
        family,
        pixels_per_point,
        font_size,
        line_height,
        extra_letter_spacing,
        section_index,
        font_metrics,
        is_first_glyph_in_section: paragraph.glyphs.is_empty(),
        bidi_level: 0,
    };
    let mut runs = Vec::new();

    // Byte offset of the current segment within `job.text`, to look up bidi levels:
    let mut segment_start = byte_range.start.0;

    // Process each paragraph segment (split on newlines — the shaper can't handle them).
    for (seg_idx, segment) in SplitOrWhole::new(section_text, job.break_on_newline).enumerate() {
        let segment_levels = bidi
            .levels
            .as_deref()
            .map(|levels| &levels[segment_start..segment_start + segment.len()]);

        if 0 < seg_idx {
            paragraph = out_paragraphs.push_mut(Paragraph::from_section_index(section_index));
            paragraph.empty_paragraph_height = line_height;
            paragraph.rtl = bidi.is_rtl_paragraph(segment_start);
            ctx.is_first_glyph_in_section = true;
        }
        segment_start += segment.len() + 1; // skip the `\n` we split on

        if segment.is_empty() {
            continue;
        }

        segment_into_runs(fonts, &ctx, segment, segment_levels, &mut runs);

        let num_runs = runs.len();
        for (run_idx, run) in runs.iter().enumerate() {
            let run_text = &segment[run.byte_range.as_usize()];
            ctx.bidi_level = run.bidi_level;
            if let Some(raster) = &run.raster {
                layout_raster_run(&mut ctx, paragraph, run_text, raster);
                continue;
            }
            let Some(font_face) = fonts.face(run.font_key) else {
                debug_assert!(
                    false,
                    "`segment_into_runs` should have turned this fontless cluster into a raster run"
                );
                continue;
            };

            let face_metrics =
                font_face.styled_metrics(pixels_per_point, font_size, &format.coords);

            // Set buffer flags for paragraph boundary context.
            let mut flags = harfrust::BufferFlags::empty();
            if run_idx == 0 {
                flags |= harfrust::BufferFlags::BEGINNING_OF_TEXT;
            }
            if run_idx + 1 == num_runs {
                flags |= harfrust::BufferFlags::END_OF_TEXT;
            }

            let glyph_buffer = shape_text(
                font_face,
                run_text,
                run.bidi_level % 2 == 1,
                &format.coords,
                shape_buffer,
                flags,
            );

            layout_shaped_run(
                fonts,
                run,
                run_text,
                &glyph_buffer,
                &face_metrics,
                &mut ctx,
                paragraph,
            );

            shape_buffer = glyph_buffer.clear();
        }
    }

    shape_buffer
}

/// Iterator that either splits on `'\n'` or yields the whole string once.
/// Avoids `Box<dyn Iterator>` and `Vec<&str>` allocation.
enum SplitOrWhole<'a> {
    Split(core::str::Split<'a, char>),
    Whole(iter::Once<&'a str>),
}

impl<'a> SplitOrWhole<'a> {
    fn new(text: &'a str, split: bool) -> Self {
        if split {
            Self::Split(text.split('\n'))
        } else {
            Self::Whole(iter::once(text))
        }
    }
}

impl<'a> Iterator for SplitOrWhole<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        match self {
            Self::Split(iter) => iter.next(),
            Self::Whole(iter) => iter.next(),
        }
    }
}

/// Calculate the intrinsic size of the text.
///
/// The result is eventually passed to `Response::intrinsic_size`.
/// This works by calculating the size of each `Paragraph` (instead of each `Row`).
fn calculate_intrinsic_size(
    point_scale: PointScale,
    job: &LayoutJob,
    paragraphs: &[Paragraph],
) -> Vec2 {
    let mut intrinsic_size = Vec2::ZERO;
    for (idx, paragraph) in paragraphs.iter().enumerate() {
        // Use the precise cursor position instead of `last_glyph.max_x()`,
        // because glyph positions are pixel-snapped but the cursor tracks
        // the exact subpixel advance. This ensures that when two galleys are
        // placed side-by-side, the gap matches what it would be within a
        // single galley.
        let width = paragraph.cursor_x_px / point_scale.pixels_per_point;
        intrinsic_size.x = f32::max(intrinsic_size.x, width);

        let mut height = paragraph
            .glyphs
            .iter()
            .map(|g| g.line_height)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal))
            .unwrap_or(paragraph.empty_paragraph_height);
        if idx == 0 {
            height = f32::max(height, job.first_row_min_height);
        }
        intrinsic_size.y += point_scale.round_to_pixel(height);
    }
    intrinsic_size
}

// Ignores the Y coordinate.
fn rows_from_paragraphs(
    paragraphs: Vec<Paragraph>,
    job: &LayoutJob,
    pixels_per_point: f32,
    elided: &mut bool,
) -> Vec<PlacedRow> {
    let num_paragraphs = paragraphs.len();

    let mut rows = vec![];

    for (i, paragraph) in paragraphs.into_iter().enumerate() {
        if job.wrap.max_rows <= rows.len() {
            *elided = true;
            break;
        }

        let is_last_paragraph = (i + 1) == num_paragraphs;

        if paragraph.glyphs.is_empty() {
            rows.push(PlacedRow {
                pos: pos2(0.0, f32::NAN),
                row: Arc::new(Row {
                    section_index_at_start: paragraph.section_index_at_start,
                    glyphs: vec![],
                    rtl_paragraph: paragraph.rtl,
                    visual_order: Vec::new(),
                    visuals: Default::default(),
                    size: vec2(0.0, paragraph.empty_paragraph_height),
                }),
                ends_with_newline: !is_last_paragraph,
            });
        } else {
            // Use precise cursor position for width instead of pixel-snapped
            // `last_glyph.max_x()`, so that side-by-side galleys have the same
            // spacing as characters within a single galley.
            let paragraph_width = paragraph.cursor_x_px / pixels_per_point;
            if paragraph_width <= job.effective_wrap_width() {
                // Early-out optimization: the whole paragraph fits on one row.
                rows.push(PlacedRow {
                    pos: pos2(0.0, f32::NAN),
                    row: Arc::new(Row {
                        section_index_at_start: paragraph.section_index_at_start,
                        glyphs: paragraph.glyphs,
                        rtl_paragraph: paragraph.rtl,
                        visual_order: Vec::new(),
                        visuals: Default::default(),
                        size: vec2(paragraph_width, 0.0),
                    }),
                    ends_with_newline: !is_last_paragraph,
                });
            } else {
                line_break(&paragraph, job, &mut rows, elided);
                let placed_row = rows.last_mut().unwrap();
                placed_row.ends_with_newline = !is_last_paragraph;
            }
        }
    }

    rows
}

fn line_break(
    paragraph: &Paragraph,
    job: &LayoutJob,
    out_rows: &mut Vec<PlacedRow>,
    elided: &mut bool,
) {
    let wrap_width = job.effective_wrap_width();

    // Keeps track of good places to insert row break if we exceed `wrap_width`.
    let mut row_break_candidates = RowBreakCandidates::default();

    let mut first_row_indentation = paragraph.glyphs[0].pos.x;
    let mut row_start_x = 0.0;
    let mut row_start_idx = 0;

    for i in 0..paragraph.glyphs.len() {
        if job.wrap.max_rows <= out_rows.len() {
            *elided = true;
            break;
        }

        let potential_row_width = paragraph.glyphs[i].max_x() - row_start_x;

        if wrap_width < potential_row_width {
            // Row break:

            if first_row_indentation > 0.0
                && !row_break_candidates.has_good_candidate(job.wrap.break_anywhere)
            {
                // Allow the first row to be completely empty, because we know there will be more space on the next row:
                // TODO(emilk): this records the height of this first row as zero, though that is probably fine since first_row_indentation usually comes with a first_row_min_height.
                out_rows.push(PlacedRow {
                    pos: pos2(0.0, f32::NAN),
                    row: Arc::new(Row {
                        section_index_at_start: paragraph.section_index_at_start,
                        glyphs: vec![],
                        rtl_paragraph: paragraph.rtl,
                        visual_order: Vec::new(),
                        visuals: Default::default(),
                        size: Vec2::ZERO,
                    }),
                    ends_with_newline: false,
                });
                row_start_x += first_row_indentation;
                first_row_indentation = 0.0;
            } else if let Some(last_kept_index) = row_break_candidates.get(job.wrap.break_anywhere)
            {
                let glyphs: Vec<Glyph> = paragraph.glyphs[row_start_idx..=last_kept_index]
                    .iter()
                    .copied()
                    .map(|mut glyph| {
                        glyph.pos.x -= row_start_x;
                        glyph
                    })
                    .collect();

                let section_index_at_start = glyphs[0].section_index;
                let paragraph_max_x = glyphs.last().unwrap().max_x();

                out_rows.push(PlacedRow {
                    pos: pos2(0.0, f32::NAN),
                    row: Arc::new(Row {
                        section_index_at_start,
                        glyphs,
                        rtl_paragraph: paragraph.rtl,
                        visual_order: Vec::new(),
                        visuals: Default::default(),
                        size: vec2(paragraph_max_x, 0.0),
                    }),
                    ends_with_newline: false,
                });

                // Start a new row:
                row_start_idx = last_kept_index + 1;
                row_start_x = paragraph.glyphs[row_start_idx].pos.x;
                row_break_candidates.forget_before_idx(row_start_idx);
            } else {
                // Found no place to break, so we have to overrun wrap_width.
            }
        }

        row_break_candidates.add(i, &paragraph.glyphs[i..]);
    }

    if row_start_idx < paragraph.glyphs.len() {
        // Final row of text:

        if job.wrap.max_rows <= out_rows.len() {
            *elided = true; // can't fit another row
        } else {
            let paragraph_min_x = paragraph.glyphs[row_start_idx].pos.x - row_start_x;
            let paragraph_max_x = paragraph.glyphs.last().unwrap().max_x() - row_start_x;

            let glyphs: Vec<Glyph> = paragraph.glyphs[row_start_idx..]
                .iter()
                .copied()
                .map(|mut glyph| {
                    glyph.pos.x -= row_start_x + paragraph_min_x;
                    glyph
                })
                .collect();

            let section_index_at_start = glyphs[0].section_index;

            out_rows.push(PlacedRow {
                pos: pos2(paragraph_min_x, 0.0),
                row: Arc::new(Row {
                    section_index_at_start,
                    glyphs,
                    rtl_paragraph: paragraph.rtl,
                    visual_order: Vec::new(),
                    visuals: Default::default(),
                    size: vec2(paragraph_max_x - paragraph_min_x, 0.0),
                }),
                ends_with_newline: false,
            });
        }
    }
}

/// Trims the last glyphs in the row and replaces it with an overflow character (e.g. `…`).
///
/// Called before we have any Y coordinates.
fn replace_last_glyph_with_overflow_character(
    fonts: &mut FontsImpl,
    pixels_per_point: f32,
    job: &LayoutJob,
    row: &mut Row,
) {
    let Some(overflow_character) = job.wrap.overflow_character else {
        return;
    };

    let mut section_index = row
        .glyphs
        .last()
        .map(|g| g.section_index)
        .unwrap_or(row.section_index_at_start);
    loop {
        let section = &job.sections[section_index as usize];
        let extra_letter_spacing = section.format.extra_letter_spacing;
        let family = fonts.family_key(&section.format.font_id.family);
        let font_size = section.format.font_id.size;

        let font_metrics =
            fonts.family_metrics(family, pixels_per_point, font_size, &section.format.coords);

        // A priority rasterizer may override the overflow character, just like any other glyph:
        let mut utf8 = [0_u8; 4];
        let raster = fonts.rasterize_cluster(
            FontPriority::Highest,
            family,
            overflow_character.encode_utf8(&mut utf8),
            pixels_per_point,
            font_size,
        );

        let (face_key, font_face_metrics, glyph_info) = if raster.is_some() {
            (
                FontFaceKey::INVALID,
                font_metrics.clone(),
                GlyphInfo::INVISIBLE,
            )
        } else {
            let face_key = fonts.resolve_face(family, overflow_character);
            let font_face_metrics = fonts
                .face(face_key)
                .map(|face| {
                    face.styled_metrics(pixels_per_point, font_size, &section.format.coords)
                })
                .unwrap_or_default();
            let (_, glyph_info) = fonts.glyph_info(family, overflow_character, &font_face_metrics);
            (face_key, font_face_metrics, glyph_info)
        };

        let overflow_glyph_x = if let Some(prev_glyph) = row.glyphs.last() {
            prev_glyph.max_x() + extra_letter_spacing
        } else {
            0.0 // TODO(emilk): heed paragraph leading_space 😬
        };
        // At the paragraph level the character goes to the visual end of the row
        // (the left end of a right-to-left row), whatever direction the text before it has:
        let bidi_level = u8::from(row.rtl_paragraph);

        let advance_width_px = match &raster {
            Some(raster) => raster.advance_px,
            None => glyph_info.advance_width_unscaled.0 * font_face_metrics.px_scale_factor,
        };
        let replacement_glyph_width = advance_width_px / pixels_per_point;

        // Check if we're within width budget:
        if overflow_glyph_x + replacement_glyph_width <= job.effective_wrap_width()
            || row.glyphs.is_empty()
        {
            // we are done

            let OutlineGlyph {
                allocation: replacement_glyph_alloc,
                x_px,
            } = match &raster {
                Some(raster) => OutlineGlyph {
                    allocation: raster.allocation,
                    x_px: (overflow_glyph_x * pixels_per_point).round() as i32,
                },
                None => allocate_glyph_info(
                    fonts,
                    face_key,
                    &font_face_metrics,
                    glyph_info,
                    overflow_glyph_x * pixels_per_point,
                    overflow_character,
                ),
            };

            let line_height = section
                .format
                .line_height
                .unwrap_or(font_metrics.row_height);

            row.glyphs.push(Glyph {
                chr: overflow_character,
                pos: pos2(x_px as f32 / pixels_per_point, f32::NAN),
                advance_width: advance_width_px / pixels_per_point,
                line_height,
                font_face_height: font_face_metrics.row_height,
                font_face_ascent: font_face_metrics.ascent,
                font_height: font_metrics.row_height,
                font_ascent: font_metrics.ascent,
                uv_rect: replacement_glyph_alloc.uv_rect,
                is_color: replacement_glyph_alloc.is_color,
                bidi_level,
                section_index,
                first_vertex: 0, // filled in later
            });
            return;
        }

        // We didn't fit - pop the last glyph and try again.
        if let Some(last_glyph) = row.glyphs.pop() {
            section_index = last_glyph.section_index;
        } else {
            section_index = row.section_index_at_start;
        }
    }
}

/// Horizontally aligned the text on a row.
///
/// Ignores the Y coordinate.
fn halign_and_justify_row(
    point_scale: PointScale,
    placed_row: &mut PlacedRow,
    halign: Align,
    wrap_width: f32,
    justify: bool,
    keep_trailing_whitespace: bool,
) {
    #![expect(clippy::useless_let_if_seq)] // False positive

    let row = Arc::make_mut(&mut placed_row.row);

    if row.glyphs.is_empty() {
        return;
    }

    let num_leading_spaces = row
        .glyphs
        .iter()
        .take_while(|glyph| glyph.chr.is_whitespace())
        .count();

    let glyph_range = if num_leading_spaces == row.glyphs.len() {
        // There is only whitespace
        (0, row.glyphs.len())
    } else if keep_trailing_whitespace {
        (num_leading_spaces, row.glyphs.len())
    } else {
        let num_trailing_spaces = row
            .glyphs
            .iter()
            .rev()
            .take_while(|glyph| glyph.chr.is_whitespace())
            .count();

        (num_leading_spaces, row.glyphs.len() - num_trailing_spaces)
    };
    let num_glyphs_in_range = glyph_range.1 - glyph_range.0;
    assert!(num_glyphs_in_range > 0, "Should have at least one glyph");

    // Glyphs are indexed in logical order but placed in visual order
    // (see `reorder_row_visually`), so measure and shift them by x:
    let Row {
        glyphs,
        visual_order,
        ..
    } = &mut *row;
    let num_glyphs = glyphs.len();
    let order = || visual_indices(visual_order, num_glyphs);
    let kept = |i: usize| glyph_range.0 <= i && i < glyph_range.1;
    let first_kept = order().position(kept).unwrap_or(0);
    let last_kept = order().rev().find(|&i| kept(i)).unwrap_or(0);

    let original_min_x = glyphs[order().nth(first_kept).unwrap_or(0)]
        .logical_rect()
        .min
        .x;
    let original_max_x = glyphs[last_kept].logical_rect().max.x;
    let original_width = original_max_x - original_min_x;

    let target_width = if justify && num_glyphs_in_range > 1 {
        wrap_width
    } else {
        original_width
    };

    let (target_min_x, target_max_x) = match halign {
        Align::LEFT => (0.0, target_width),
        Align::Center => (-target_width * 0.5, target_width * 0.5),
        Align::RIGHT => (-target_width, 0.0),
    };

    let num_spaces_in_range = glyphs[glyph_range.0..glyph_range.1]
        .iter()
        .filter(|glyph| glyph.chr.is_whitespace())
        .count();

    let mut extra_x_per_glyph = if num_glyphs_in_range == 1 {
        0.0
    } else {
        (target_width - original_width) / (num_glyphs_in_range as f32 - 1.0)
    };
    extra_x_per_glyph = extra_x_per_glyph.at_least(0.0); // Don't contract

    let mut extra_x_per_space = 0.0;
    if 0 < num_spaces_in_range && num_spaces_in_range < num_glyphs_in_range {
        // Add an integral number of pixels between each glyph,
        // and add the balance to the spaces:

        extra_x_per_glyph = point_scale.floor_to_pixel(extra_x_per_glyph);

        extra_x_per_space = (target_width
            - original_width
            - extra_x_per_glyph * (num_glyphs_in_range as f32 - 1.0))
            / (num_spaces_in_range as f32);
    }

    placed_row.pos.x = point_scale.round_to_pixel(target_min_x);
    let mut translate_x = -original_min_x - extra_x_per_glyph * first_kept as f32;

    for i in order() {
        let glyph = &mut glyphs[i];
        glyph.pos.x += translate_x;
        glyph.pos.x = point_scale.round_to_pixel(glyph.pos.x);
        translate_x += extra_x_per_glyph;
        if glyph.chr.is_whitespace() {
            translate_x += extra_x_per_space;
        }
    }

    // Note we ignore the leading/trailing whitespace here!
    row.size.x = target_max_x - target_min_x;
}

/// Is `row` aligned to the right because its start is there?
///
/// See [`LayoutJob::halign_min_is_start`].
fn starts_on_the_right(job: &LayoutJob, row: &Row) -> bool {
    job.halign == Align::Min
        && job.halign_min_is_start
        && row.rtl_paragraph
        && !row.glyphs.is_empty()
}

/// Puts each row that [`starts_on_the_right`] flush with the right edge of the widest row.
///
/// Such a row has been aligned as `Align::RIGHT`, with its size in `row.size.x`;
/// the other rows stay where they are. Returns `true` if any row moved.
pub(crate) fn align_rtl_rows_to_start(
    job: &LayoutJob,
    rows: &mut [PlacedRow],
    pixels_per_point: f32,
) -> bool {
    if !rows
        .iter()
        .any(|placed_row| starts_on_the_right(job, &placed_row.row))
    {
        return false;
    }
    let width = rows
        .iter()
        .map(|placed_row| {
            if starts_on_the_right(job, &placed_row.row) {
                placed_row.size.x
            } else {
                placed_row.pos.x + placed_row.size.x
            }
        })
        .fold(0.0, f32::max);
    let point_scale = PointScale::new(pixels_per_point);
    for placed_row in rows {
        if starts_on_the_right(job, &placed_row.row) {
            placed_row.pos.x = point_scale.round_to_pixel(width - placed_row.size.x);
        }
    }
    true
}

/// Calculate the Y positions and tessellate the text.
fn galley_from_rows(
    point_scale: PointScale,
    job: Arc<LayoutJob>,
    mut rows: Vec<PlacedRow>,
    elided: bool,
    intrinsic_size: Vec2,
) -> Galley {
    let mut first_row_min_height = job.first_row_min_height;
    let mut cursor_y = 0.0;

    for placed_row in &mut rows {
        let mut max_row_height = first_row_min_height.at_least(placed_row.height());
        let row = Arc::make_mut(&mut placed_row.row);

        first_row_min_height = 0.0;
        for glyph in &row.glyphs {
            max_row_height = max_row_height.at_least(glyph.line_height);
        }
        max_row_height = point_scale.round_to_pixel(max_row_height);

        // Now position each glyph vertically:
        for glyph in &mut row.glyphs {
            let format = &job.sections[glyph.section_index as usize].format;

            glyph.pos.y = glyph.font_face_ascent

                // Apply valign to the difference in height of the entire row,
                // and the line height of this glyph (which is the font height, unless explicitly set):
                + format.valign.to_factor() * (max_row_height - glyph.line_height)

                // If the line height was set explicitly (e.g. taller than the font),
                // we always center the text within that line height, like CSS does.
                // When `line_height` is not set, this is zero.
                + 0.5 * (glyph.line_height - glyph.font_height)

                // When mixing different `FontImpl` (e.g. latin and emojis),
                // we always center the difference:
                + 0.5 * (glyph.font_height - glyph.font_face_height);

            glyph.pos.y = point_scale.round_to_pixel(glyph.pos.y);
        }

        placed_row.pos.y = cursor_y;
        row.size.y = max_row_height;

        cursor_y += max_row_height;
        cursor_y = point_scale.round_to_pixel(cursor_y); // TODO(emilk): it would be better to do the calculations in pixels instead.
    }

    let format_summary = format_summary(&job);

    let mut rect = Rect::ZERO;
    let mut mesh_bounds = Rect::NOTHING;
    let mut num_vertices = 0;
    let mut num_indices = 0;

    for placed_row in &mut rows {
        rect |= placed_row.rect();

        let row = Arc::make_mut(&mut placed_row.row);
        row.visuals = tessellate_row(point_scale, &job, &format_summary, row);

        mesh_bounds |= row.visuals.mesh_bounds.translate(placed_row.pos.to_vec2());
        num_vertices += row.visuals.mesh.vertices.len();
        num_indices += row.visuals.mesh.indices.len();

        row.section_index_at_start = u32::MAX; // No longer in use.
        for glyph in &mut row.glyphs {
            glyph.section_index = u32::MAX; // No longer in use.
        }
    }

    let mut galley = Galley {
        job,
        rows,
        elided,
        rect,
        mesh_bounds,
        num_vertices,
        num_indices,
        pixels_per_point: point_scale.pixels_per_point,
        intrinsic_size,
    };

    if galley.job.round_output_to_gui {
        galley.round_output_to_gui();
    }

    galley
}

#[derive(Default)]
struct FormatSummary {
    any_background: bool,
    any_underline: bool,
    any_strikethrough: bool,
}

fn format_summary(job: &LayoutJob) -> FormatSummary {
    let mut format_summary = FormatSummary::default();
    for section in &job.sections {
        format_summary.any_background |= section.format.background != Color32::TRANSPARENT;
        format_summary.any_underline |= section.format.underline != Stroke::NONE;
        format_summary.any_strikethrough |= section.format.strikethrough != Stroke::NONE;
    }
    format_summary
}

fn tessellate_row(
    point_scale: PointScale,
    job: &LayoutJob,
    format_summary: &FormatSummary,
    row: &mut Row,
) -> RowVisuals {
    if row.glyphs.is_empty() {
        return Default::default();
    }

    let mut mesh = Mesh::default();

    mesh.reserve_triangles(row.glyphs.len() * 2);
    mesh.reserve_vertices(row.glyphs.len() * 4);

    if format_summary.any_background {
        add_row_backgrounds(point_scale, job, row, &mut mesh);
    }

    let glyph_index_start = mesh.indices.len();
    let glyph_vertex_start = mesh.vertices.len();
    tessellate_glyphs(point_scale, job, row, &mut mesh);
    let glyph_vertex_end = mesh.vertices.len();

    if format_summary.any_underline {
        add_row_hline(point_scale, row, &mut mesh, |glyph| {
            let format = &job.sections[glyph.section_index as usize].format;
            let stroke = format.underline;
            let y = glyph.logical_rect().bottom();
            (stroke, y)
        });
    }

    if format_summary.any_strikethrough {
        add_row_hline(point_scale, row, &mut mesh, |glyph| {
            let format = &job.sections[glyph.section_index as usize].format;
            let stroke = format.strikethrough;
            let y = glyph.logical_rect().center().y;
            (stroke, y)
        });
    }

    let mesh_bounds = mesh.calc_bounds();

    RowVisuals {
        mesh,
        mesh_bounds,
        glyph_index_start,
        glyph_vertex_range: glyph_vertex_start..glyph_vertex_end,
    }
}

/// Create background for glyphs that have them.
/// Creates as few rectangular regions as possible.
fn add_row_backgrounds(point_scale: PointScale, job: &LayoutJob, row: &Row, mesh: &mut Mesh) {
    if row.glyphs.is_empty() {
        return;
    }

    let mut end_run = |start: Option<(Color32, Rect, f32)>, stop_x: f32| {
        if let Some((color, start_rect, expand)) = start {
            let rect = Rect::from_min_max(start_rect.left_top(), pos2(stop_x, start_rect.bottom()));
            let rect = rect.expand(expand);
            let rect = rect.round_to_pixels(point_scale.pixels_per_point());
            mesh.add_colored_rect(rect, color);
        }
    };

    let mut run_start = None;
    let mut last_rect = Rect::NAN;

    for i in row.visual_order() {
        let glyph = &row.glyphs[i];
        let format = &job.sections[glyph.section_index as usize].format;
        let color = format.background;
        let rect = glyph.logical_rect();

        if color == Color32::TRANSPARENT {
            end_run(run_start.take(), last_rect.right());
        } else if let Some((existing_color, start, expand)) = run_start {
            if existing_color == color
                && start.top() == rect.top()
                && start.bottom() == rect.bottom()
                && format.expand_bg == expand
            {
                // continue the same background rectangle
            } else {
                end_run(run_start.take(), last_rect.right());
                run_start = Some((color, rect, format.expand_bg));
            }
        } else {
            run_start = Some((color, rect, format.expand_bg));
        }

        last_rect = rect;
    }

    end_run(run_start.take(), last_rect.right());
}

fn tessellate_glyphs(point_scale: PointScale, job: &LayoutJob, row: &mut Row, mesh: &mut Mesh) {
    for glyph in &mut row.glyphs {
        glyph.first_vertex = mesh.vertices.len() as u32;
        let uv_rect = glyph.uv_rect;
        if !uv_rect.is_nothing() {
            let mut left_top = glyph.pos + uv_rect.offset;
            left_top.x = point_scale.round_to_pixel(left_top.x);
            left_top.y = point_scale.round_to_pixel(left_top.y);

            let rect = Rect::from_min_max(left_top, left_top + uv_rect.size);
            let uv = Rect::from_min_max(
                pos2(uv_rect.min[0] as f32, uv_rect.min[1] as f32),
                pos2(uv_rect.max[0] as f32, uv_rect.max[1] as f32),
            );

            let format = &job.sections[glyph.section_index as usize].format;

            // Color glyphs (e.g. color emoji) are untinted by the tessellator,
            // which is also where `Color32::PLACEHOLDER` and `override_text_color` are resolved.
            let color = format.color;

            if format.italics {
                let idx = mesh.vertices.len() as u32;
                mesh.add_triangle(idx, idx + 1, idx + 2);
                mesh.add_triangle(idx + 2, idx + 1, idx + 3);

                let top_offset = rect.height() * 0.25 * Vec2::X;

                mesh.vertices.push(Vertex {
                    pos: rect.left_top() + top_offset,
                    uv: uv.left_top(),
                    color,
                });
                mesh.vertices.push(Vertex {
                    pos: rect.right_top() + top_offset,
                    uv: uv.right_top(),
                    color,
                });
                mesh.vertices.push(Vertex {
                    pos: rect.left_bottom(),
                    uv: uv.left_bottom(),
                    color,
                });
                mesh.vertices.push(Vertex {
                    pos: rect.right_bottom(),
                    uv: uv.right_bottom(),
                    color,
                });
            } else {
                mesh.add_rect_with_uv(rect, uv, color);
            }
        }
    }
}

/// Add a horizontal line over a row of glyphs with a stroke and y decided by a callback.
fn add_row_hline(
    point_scale: PointScale,
    row: &Row,
    mesh: &mut Mesh,
    stroke_and_y: impl Fn(&Glyph) -> (Stroke, f32),
) {
    let mut path = crate::tessellator::Path::default(); // reusing path to avoid re-allocations.

    let mut end_line = |start: Option<(Stroke, Pos2)>, stop_x: f32| {
        if let Some((stroke, start)) = start {
            let stop = pos2(stop_x, start.y);
            path.clear();
            path.add_line_segment([start, stop]);
            let feathering = 1.0 / point_scale.pixels_per_point();
            path.stroke_open(feathering, &PathStroke::from(stroke), mesh);
        }
    };

    let mut line_start = None;
    let mut last_right_x = f32::NAN;

    for i in row.visual_order() {
        let glyph = &row.glyphs[i];
        let (stroke, mut y) = stroke_and_y(glyph);
        stroke.round_center_to_pixel(point_scale.pixels_per_point, &mut y);

        if stroke.is_empty() {
            end_line(line_start.take(), last_right_x);
        } else if let Some((existing_stroke, start)) = line_start {
            if existing_stroke == stroke && start.y == y {
                // continue the same line
            } else {
                end_line(line_start.take(), last_right_x);
                line_start = Some((stroke, pos2(glyph.pos.x, y)));
            }
        } else {
            line_start = Some((stroke, pos2(glyph.pos.x, y)));
        }

        last_right_x = glyph.max_x();
    }

    end_line(line_start.take(), last_right_x);
}

// ----------------------------------------------------------------------------

/// Keeps track of good places to break a long row of text.
/// Will focus primarily on spaces, secondarily on things like `-`
#[derive(Clone, Copy, Default)]
struct RowBreakCandidates {
    /// Breaking at ` ` or other whitespace
    /// is always the primary candidate.
    space: Option<usize>,

    /// Logograms (single character representing a whole word) or kana (Japanese hiragana and katakana) are good candidates for line break.
    cjk: Option<usize>,

    /// Breaking anywhere before a CJK character is acceptable too.
    pre_cjk: Option<usize>,

    /// Breaking at a dash is a super-
    /// good idea.
    dash: Option<usize>,

    /// This is nicer for things like URLs, e.g. www.
    /// example.com.
    punctuation: Option<usize>,

    /// Breaking after just random character is some
    /// times necessary.
    any: Option<usize>,
}

impl RowBreakCandidates {
    fn add(&mut self, index: usize, glyphs: &[Glyph]) {
        let chr = glyphs[0].chr;
        const NON_BREAKING_SPACE: char = '\u{A0}';
        if chr.is_whitespace() && chr != NON_BREAKING_SPACE {
            self.space = Some(index);
        } else if is_cjk(chr) && (glyphs.len() == 1 || is_cjk_break_allowed(glyphs[1].chr)) {
            self.cjk = Some(index);
        } else if chr == '-' {
            self.dash = Some(index);
        } else if chr.is_ascii_punctuation() {
            self.punctuation = Some(index);
        } else if glyphs.len() > 1 && is_cjk(glyphs[1].chr) {
            self.pre_cjk = Some(index);
        }
        self.any = Some(index);
    }

    fn word_boundary(&self) -> Option<usize> {
        [self.space, self.cjk, self.pre_cjk]
            .into_iter()
            .max()
            .flatten()
    }

    fn has_good_candidate(&self, break_anywhere: bool) -> bool {
        if break_anywhere {
            self.any.is_some()
        } else {
            self.word_boundary().is_some()
        }
    }

    fn get(&self, break_anywhere: bool) -> Option<usize> {
        if break_anywhere {
            self.any
        } else {
            self.word_boundary()
                .or(self.dash)
                .or(self.punctuation)
                .or(self.any)
        }
    }

    fn forget_before_idx(&mut self, index: usize) {
        let Self {
            space,
            cjk,
            pre_cjk,
            dash,
            punctuation,
            any,
        } = self;
        if space.is_some_and(|s| s < index) {
            *space = None;
        }
        if cjk.is_some_and(|s| s < index) {
            *cjk = None;
        }
        if pre_cjk.is_some_and(|s| s < index) {
            *pre_cjk = None;
        }
        if dash.is_some_and(|s| s < index) {
            *dash = None;
        }
        if punctuation.is_some_and(|s| s < index) {
            *punctuation = None;
        }
        if any.is_some_and(|s| s < index) {
            *any = None;
        }
    }
}

// ----------------------------------------------------------------------------

/// Segment text into runs where each run uses a single font face.
///
/// Grapheme clusters are never split across runs: if a combining mark
/// falls back to a different font than its base character, it stays
/// with the base character's font (the shaper will handle it).
///
/// Clusters that a priority [`GlyphRasterizer`](crate::text::GlyphRasterizer)
/// handles get a run of their own, with [`TextRun::raster`] set.
///
/// A run also never crosses a change of bidi embedding level (`levels`, one
/// per byte of `text`, or `None` for text without right-to-left characters),
/// so each run is shaped in one direction.
///
/// NOTE: Segmentation is by font face and direction, not by Unicode script.
/// A run may mix scripts (e.g. Latin + Cyrillic) when they share the same
/// font, which is acceptable for scripts with similar shaping rules.
///
/// Results are appended to `out` (which is cleared first) to allow
/// the caller to reuse the allocation across calls.
fn segment_into_runs(
    fonts: &mut FontsImpl,
    ctx: &ShapingContext,
    text: &str,
    levels: Option<&[u8]>,
    out: &mut Vec<TextRun>,
) {
    use unicode_segmentation::UnicodeSegmentation as _;

    out.clear();

    for (byte_offset, grapheme_str) in text.grapheme_indices(true) {
        let bidi_level = levels.map_or(0, |levels| levels[byte_offset]);
        let byte_offset = ByteIndex(byte_offset);
        let byte_end = byte_offset + grapheme_str.len();

        if let Some(raster) = fonts.rasterize_cluster(
            FontPriority::Highest,
            ctx.family,
            grapheme_str,
            ctx.pixels_per_point,
            ctx.font_size,
        ) {
            out.push(TextRun {
                font_key: FontFaceKey::INVALID,
                byte_range: byte_offset..byte_end,
                bidi_level,
                raster: Some(raster),
            });
            continue;
        }

        let font_key = fonts.resolve_cluster_face(ctx.family, grapheme_str);

        if fonts.face(font_key).is_none() {
            // The family has no font at all, so there is nothing to shape with.
            // Still emit a glyph per cluster, or cursors and selections break.
            let raster = fonts.fontless_cluster(
                ctx.family,
                grapheme_str,
                ctx.pixels_per_point,
                ctx.font_size,
            );
            out.push(TextRun {
                font_key,
                byte_range: byte_offset..byte_end,
                bidi_level,
                raster: Some(raster),
            });
            continue;
        }

        if let Some(last_run) = out.last_mut()
            && last_run.raster.is_none()
            && last_run.font_key == font_key
            && last_run.bidi_level == bidi_level
        {
            last_run.byte_range.end = byte_end;
            continue;
        }
        out.push(TextRun {
            font_key,
            byte_range: byte_offset..byte_end,
            bidi_level,
            raster: None,
        });
    }
}

/// The result of the bidi algorithm (UAX #9) on the text of a job, up to rule I2.
struct BidiLevels {
    /// The resolved embedding level of every byte of the text, or `None`
    /// when the text has no right-to-left character and every paragraph is
    /// left-to-right, which keeps the common case free of any bidi work.
    levels: Option<Vec<u8>>,

    /// The byte ranges of the right-to-left paragraphs, in order.
    rtl_paragraphs: Vec<Range<usize>>,

    direction: TextDirection,
}

impl BidiLevels {
    fn new(text: &str, direction: TextDirection) -> Self {
        let mut bidi = Self {
            levels: None,
            rtl_paragraphs: Vec::new(),
            direction,
        };
        let paragraph_level = match direction {
            TextDirection::Auto | TextDirection::LeftToRight if text.is_ascii() => return bidi,
            TextDirection::Auto => None,
            TextDirection::LeftToRight => Some(unicode_bidi::Level::ltr()),
            TextDirection::RightToLeft => Some(unicode_bidi::Level::rtl()),
        };
        let info = unicode_bidi::BidiInfo::new(text, paragraph_level);
        bidi.rtl_paragraphs = info
            .paragraphs
            .iter()
            .filter(|paragraph| paragraph.level.is_rtl())
            .map(|paragraph| paragraph.range.clone())
            .collect();
        // A right-to-left paragraph needs its levels even when they are all even,
        // e.g. for trailing whitespace (rule L1).
        if info.has_rtl() || !bidi.rtl_paragraphs.is_empty() {
            bidi.levels = Some(info.levels.iter().map(|level| level.number()).collect());
        }
        bidi
    }

    /// Is the paragraph that contains the byte at `byte_offset` right-to-left?
    ///
    /// With [`TextDirection::Auto`], the empty paragraph after a final `\n` is left-to-right.
    fn is_rtl_paragraph(&self, byte_offset: usize) -> bool {
        match self.direction {
            TextDirection::LeftToRight => false,
            TextDirection::RightToLeft => true,
            TextDirection::Auto => {
                let i = self
                    .rtl_paragraphs
                    .partition_point(|range| range.end <= byte_offset);
                self.rtl_paragraphs
                    .get(i)
                    .is_some_and(|range| range.contains(&byte_offset))
            }
        }
    }
}

/// UAX #9 rule L1: segment separators (tab), paragraph separators, and any sequence
/// of whitespace before one of them or at the end of the row take the paragraph level.
///
/// So the spaces where a right-to-left row wraps go to its left end,
/// not next to the last word wherever that is on the row.
fn reset_whitespace_levels(row: &mut Row) {
    use unicode_bidi::BidiClass::{B, BN, FSI, LRE, LRI, LRO, PDF, PDI, RLE, RLI, RLO, S, WS};

    let paragraph_level = u8::from(row.rtl_paragraph);
    // Walking backwards: is everything after this glyph, up to a separator
    // or the end of the row, whitespace?
    let mut in_trailing_whitespace = true;
    for glyph in row.glyphs.iter_mut().rev() {
        match unicode_bidi::bidi_class(glyph.chr) {
            B | S => {
                glyph.bidi_level = paragraph_level;
                in_trailing_whitespace = true;
            }
            // Whitespace, isolate formatting characters, and the characters
            // that rule X9 removes:
            WS | FSI | LRI | RLI | PDI | LRE | RLE | LRO | RLO | PDF | BN => {
                if in_trailing_whitespace {
                    glyph.bidi_level = paragraph_level;
                }
            }
            _ => in_trailing_whitespace = false,
        }
    }
}

/// The clusters of a shaped buffer, as ranges of glyph indices, in _logical_ order.
///
/// The shaper returns glyphs in visual order, which for a right-to-left run
/// is cluster-by-cluster reversed. Walking the clusters backwards restores
/// logical order while keeping the glyphs inside a cluster (a base and its
/// marks) in shaper order, because the marks' offsets are relative to the base.
fn logical_clusters(infos: &[harfrust::GlyphInfo], rtl: bool) -> LogicalClusters<'_> {
    LogicalClusters {
        infos,
        remaining: 0..infos.len(),
        rtl,
    }
}

/// See [`logical_clusters`].
struct LogicalClusters<'a> {
    infos: &'a [harfrust::GlyphInfo],

    /// The glyphs not yet yielded, consumed from the front (left-to-right)
    /// or from the back (right-to-left).
    remaining: Range<usize>,

    rtl: bool,
}

impl Iterator for LogicalClusters<'_> {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Range<usize>> {
        let infos = self.infos;
        let unyielded = &infos[self.remaining.clone()];
        let (first, last) = (unyielded.first()?, unyielded.last()?);
        let offset = self.remaining.start;

        if self.rtl {
            let start = unyielded
                .iter()
                .rposition(|info| info.cluster != last.cluster)
                .map_or(0, |i| i + 1);
            self.remaining.end = offset + start;
            Some(offset + start..offset + unyielded.len())
        } else {
            let end = unyielded
                .iter()
                .position(|info| info.cluster != first.cluster)
                .unwrap_or(unyielded.len());
            self.remaining.start = offset + end;
            Some(offset..offset + end)
        }
    }
}

#[cfg(test)]
std::thread_local! {
    /// Counts the steps [`reorder_row_visually`] takes, for a test that keeps it linear.
    static REORDER_STEPS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

#[inline]
fn count_reorder_steps(steps: usize) {
    #[cfg(test)]
    REORDER_STEPS.with(|count| count.set(count.get() + steps));
    #[cfg(not(test))]
    let _ = steps;
}

/// Place the glyphs of a row in visual order (UAX #9 rule L2) by rewriting their x,
/// and store that order in [`Row::visual_order`].
///
/// On entry `row.glyphs` is in logical order with increasing x, as
/// [`layout_shaped_run`] left it. It stays in logical order, so a glyph's
/// index is still its char index; only the positions move.
///
/// A block is one advancing glyph and the zero-width glyphs after it
/// (combining marks, the continuation glyphs of a cluster). Blocks move as
/// units so a mark stays over its base.
///
/// Takes time linear in the number of glyphs: each pass of L2 reverses level runs
/// (maximal sequences of blocks at one level), not single blocks, and there are
/// at most 126 levels.
fn reorder_row_visually(point_scale: PointScale, row: &mut Row) {
    let glyphs = &mut row.glyphs;
    if glyphs.len() < 2 {
        return;
    }

    let mut blocks: Vec<Range<usize>> = Vec::new();
    for (i, glyph) in glyphs.iter().enumerate() {
        match blocks.last_mut() {
            Some(last) if glyph.advance_width <= 0.0 => last.end = i + 1,
            _ => blocks.push(i..i + 1),
        }
    }
    count_reorder_steps(glyphs.len());
    let origins: Vec<f32> = blocks
        .iter()
        .map(|block| glyphs[block.start].pos.x)
        .collect();
    // A block's width is the distance to the next one, which includes any
    // extra letter spacing; the last block ends where its glyphs end.
    let widths: Vec<f32> = blocks
        .iter()
        .enumerate()
        .map(|(b, block)| match origins.get(b + 1) {
            Some(next_origin) => next_origin - origins[b],
            None => {
                glyphs[block.clone()]
                    .iter()
                    .map(Glyph::max_x)
                    .fold(origins[b], f32::max)
                    - origins[b]
            }
        })
        .collect();

    // Level runs, as ranges of block indices:
    let mut runs: Vec<Range<usize>> = Vec::new();
    let mut run_levels: Vec<u8> = Vec::new();
    for (b, block) in blocks.iter().enumerate() {
        let level = glyphs[block.start].bidi_level;
        match runs.last_mut() {
            Some(last) if run_levels.last() == Some(&level) => last.end = b + 1,
            _ => {
                runs.push(b..b + 1);
                run_levels.push(level);
            }
        }
    }
    count_reorder_steps(blocks.len());

    // L2: from the highest level down to the lowest odd level,
    // reverse every maximal sequence of runs at that level or higher.
    let Some(lowest_odd) = run_levels.iter().copied().filter(|l| l % 2 == 1).min() else {
        return;
    };
    let highest = run_levels.iter().copied().max().unwrap_or(0);
    let mut order: Vec<usize> = (0..runs.len()).collect();
    for level in (lowest_odd..=highest).rev() {
        let mut i = 0;
        while i < order.len() {
            if run_levels[order[i]] < level {
                i += 1;
                continue;
            }
            let start = i;
            while i < order.len() && level <= run_levels[order[i]] {
                i += 1;
            }
            order[start..i].reverse();
        }
        count_reorder_steps(order.len());
    }

    // Each pass at or below a run's level reversed it as a whole, so its blocks
    // are in reverse order if that happened an odd number of times:
    let mut visual_order = Vec::with_capacity(glyphs.len());
    let mut pen = origins[0];
    for &r in &order {
        let reversed =
            lowest_odd <= run_levels[r] && (run_levels[r] - lowest_odd).is_multiple_of(2);
        let run = runs[r].clone();
        let mut place = |b: usize| {
            for i in blocks[b].clone() {
                let glyph = &mut glyphs[i];
                glyph.pos.x = point_scale.round_to_pixel(pen + (glyph.pos.x - origins[b]));
                visual_order.push(i as u32);
            }
            pen += widths[b];
        };
        if reversed {
            run.rev().for_each(&mut place);
        } else {
            run.for_each(&mut place);
        }
    }
    count_reorder_steps(glyphs.len());
    row.visual_order = visual_order;
}

/// The indices of `len` glyphs from left to right, given a [`Row::visual_order`].
pub(crate) fn visual_indices(
    visual_order: &[u32],
    len: usize,
) -> impl DoubleEndedIterator<Item = usize> + Clone + '_ {
    // Rows that need no reordering have no `visual_order`; a row whose glyphs were
    // changed after layout gets the logical order rather than a panic.
    let identity = if visual_order.len() == len { 0 } else { len };
    let reordered = if identity == 0 { visual_order } else { &[] };
    (0..identity).chain(reordered.iter().map(|&i| i as usize))
}

/// Shape a text run and return the raw [`harfrust::GlyphBuffer`].
///
/// The caller should iterate `glyph_infos()` / `glyph_positions()` (both
/// `Copy` slices) and convert font units to pixels using `metrics.px_scale_factor`.
/// After iteration, recycle the buffer via `glyph_buffer.clear()`.
fn shape_text(
    font_face: &FontFace,
    text: &str,
    rtl: bool,
    coords: &VariationCoords,
    mut buffer: harfrust::UnicodeBuffer,
    flags: harfrust::BufferFlags,
) -> harfrust::GlyphBuffer {
    let font_ref = font_face.skrifa_font_ref();
    let tweak = font_face.tweak();

    // Build shaper with variable font instance if variation coordinates are set.
    let variations: Vec<harfrust::Variation> = iter::chain(tweak.coords.as_ref(), coords.as_ref())
        .map(|&(tag, value)| harfrust::Variation { tag, value })
        .collect();

    let instance = if variations.is_empty() {
        None
    } else {
        Some(harfrust::ShaperInstance::from_variations(
            font_ref, variations,
        ))
    };

    let shaper = font_face
        .shaper_data()
        .shaper(font_ref)
        .instance(instance.as_ref())
        .build();

    buffer.set_flags(flags);
    buffer.push_str(text);
    // The direction is decided by the bidi algorithm, per run;
    // the shaper only guesses the script (and would guess the direction wrong
    // for digits inside right-to-left text).
    buffer.set_direction(if rtl {
        harfrust::Direction::RightToLeft
    } else {
        harfrust::Direction::LeftToRight
    });
    buffer.guess_segment_properties();

    shaper.shape(buffer, harfrust::ShapeOptions::new())
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::iter;

    use super::{super::*, *};
    use crate::{ColorImage, text::cursor::CCursor};
    use emath::Rangef;

    fn test_fonts() -> FontsImpl {
        FontsImpl::new(TextOptions::default(), FontDefinitions::default())
    }

    fn layout_without_fonts(text: &str, policy: MissingGlyphPolicy) -> Arc<Galley> {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(policy);
        fonts.with_pixels_per_point(2.0).layout_no_wrap(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
        )
    }

    /// Chinese closes a clause with full-width marks (，：；？！）), and those must never start a row:
    /// <https://en.wikipedia.org/wiki/Line_breaking_rules_in_East_Asian_languages>
    #[test]
    fn cjk_row_never_starts_with_full_width_closing_punctuation() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let text = "一二三，四五六：七八九？十一二！三四五）六七八";
        let one = fonts
            .layout_no_wrap("一".to_owned(), font_id.clone(), Color32::WHITE)
            .size()
            .x;
        // Every width that fits a few glyphs, so some row would end just before each mark.
        for glyphs in 2..12 {
            let width = one * glyphs as f32 + 0.5 * one;
            let galley = fonts.layout(text.to_owned(), font_id.clone(), Color32::WHITE, width);
            for placed in galley.rows.iter().skip(1) {
                let first = placed.row.glyphs.first().map(|g| g.chr);
                assert!(
                    !matches!(first, Some('，' | '：' | '？' | '！' | '）')),
                    "a row starts with {first:?} at {glyphs} glyphs wide"
                );
            }
        }
    }

    /// With no font at all we still need one glyph per character and rows with height,
    /// or text cursors get clamped to 0 and text edits lose their contents.
    #[test]
    fn no_font_still_gives_one_glyph_per_char() {
        let galley = layout_without_fonts("ab c", MissingGlyphPolicy::Tofu);

        assert_eq!(galley.rows.len(), 1);
        let row = &galley.rows[0].row;
        assert_eq!(row.glyphs.len(), 4);
        assert!(0.0 < row.height(), "row should have height: {row:?}");
        assert!(0.0 < row.size.x, "row should have width: {row:?}");

        // Every character gets a tofu box, whitespace included:
        for (i, glyph) in row.glyphs.iter().enumerate() {
            assert!(!glyph.uv_rect.is_nothing(), "glyph {i}: {glyph:?}");
            assert!(0.0 < glyph.advance_width, "glyph {i}: {glyph:?}");
            assert_eq!(galley.clamp_cursor(&CCursor::new(i)).index.0, i);
            assert!(
                glyph.pos.x.is_finite() && glyph.pos.y.is_finite(),
                "{glyph:?}"
            );
        }
        assert_eq!(galley.clamp_cursor(&CCursor::new(4)).index.0, 4);
        assert_eq!(galley.end().index.0, 4);
    }

    #[test]
    fn no_font_tofu_is_cached_across_layouts() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty());
        let mut view = fonts.with_pixels_per_point(1.0);
        let a = view.layout_no_wrap("a".into(), FontId::proportional(14.0), Color32::WHITE);
        let b = view.layout_no_wrap("b".into(), FontId::proportional(14.0), Color32::WHITE);
        assert_eq!(
            a.rows[0].row.glyphs[0].uv_rect.min, b.rows[0].row.glyphs[0].uv_rect.min,
            "the same synthetic tofu box should be reused"
        );
    }

    #[test]
    #[should_panic(expected = "No glyph for \"a\" (U+0061) in Proportional")]
    fn missing_glyph_policy_panic_without_fonts() {
        let _ = layout_without_fonts("a", MissingGlyphPolicy::Panic);
    }

    #[test]
    #[should_panic(expected = "No glyph for \" \" (U+0020) in Proportional")]
    fn missing_glyph_policy_panic_without_fonts_includes_whitespace() {
        let _ = layout_without_fonts(" ", MissingGlyphPolicy::Panic);
    }

    #[test]
    fn no_font_newline_still_breaks_rows() {
        let galley = layout_without_fonts(" \t\n ", MissingGlyphPolicy::Tofu);
        assert_eq!(galley.rows.len(), 2);
        assert_eq!(galley.rows[0].row.glyphs.len(), 2);
        assert_eq!(galley.rows[1].row.glyphs.len(), 1);
    }

    #[test]
    #[cfg(feature = "default_fonts")] // Needs `hack_only`
    #[should_panic(
        expected = "No glyph for \"😀\" (U+1F600) in Proportional. Installed fonts: [\"Hack\"]"
    )]
    fn missing_glyph_policy_panic_with_fonts() {
        let mut fonts = Fonts::new(TextOptions::default(), hack_only())
            .with_missing_glyph_policy(MissingGlyphPolicy::Panic);
        let _ = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            "a😀".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
    }

    /// Fonts have no glyphs for control characters like `\t`; that is not a missing glyph.
    #[test]
    #[cfg(feature = "default_fonts")] // Needs `hack_only`
    fn missing_glyph_policy_panic_ignores_tab_with_fonts() {
        let mut fonts = Fonts::new(TextOptions::default(), hack_only())
            .with_missing_glyph_policy(MissingGlyphPolicy::Panic);
        let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            "a\tb".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
        assert_eq!(galley.rows[0].row.glyphs.len(), 3);
    }

    #[test]
    #[cfg(feature = "default_fonts")] // Needs `hack_only`
    fn missing_glyph_policy_panic_is_satisfied_by_a_fallback_rasterizer() {
        let rasterizer = GlyphRasterizer::new("test", |_: &GlyphRasterizerRequest<'_>| {
            Some(white_raster_glyph())
        });
        let mut fonts = Fonts::new(TextOptions::default(), hack_only())
            .with_glyph_rasterizer(rasterizer)
            .with_missing_glyph_policy(MissingGlyphPolicy::Panic);
        let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            "a😀".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
        );
        assert_eq!(galley.rows[0].row.glyphs.len(), 2);
    }

    /// Only `Hack`, which has no emoji.
    #[cfg(feature = "default_fonts")]
    fn hack_only() -> FontDefinitions {
        let mut definitions = FontDefinitions::empty();
        definitions.font_data.insert(
            "Hack".to_owned(),
            Arc::new(FontData::from_static(epaint_default_fonts::HACK_REGULAR)),
        );
        definitions
            .families
            .insert(FontFamily::Proportional, vec!["Hack".to_owned()]);
        definitions
    }

    /// Hack composes `ǻ` from `å` plus an acute: two glyphs for one character.
    #[test]
    #[cfg(feature = "default_fonts")]
    fn a_letter_the_font_composes_from_several_glyphs_is_one_glyph() {
        let mut fonts = FontsImpl::new(TextOptions::default(), hack_only());
        let composed = layout_simple(&mut fonts, "aǻb");
        let plain = layout_simple(&mut fonts, "aåb");

        let glyphs = &composed.rows[0].row.glyphs;
        let chars: Vec<char> = glyphs.iter().map(|glyph| glyph.chr).collect();
        assert_eq!(chars, ['a', 'ǻ', 'b']);

        let letter = &glyphs[1];
        assert!(!letter.uv_rect.is_nothing(), "the composed letter is drawn");
        assert!(
            plain.rows[0].row.glyphs[1].uv_rect.size.y < letter.uv_rect.size.y,
            "its bitmap covers the acute above the ring"
        );
        assert_eq!(
            composed.size(),
            plain.size(),
            "the cluster keeps the shaper's advance"
        );

        for i in 0..=3 {
            let cursor = CCursor {
                index: CharIndex(i),
                prefer_next_row: false,
            };
            let rect = composed.pos_from_cursor(cursor);
            let back = composed.cursor_from_pos(rect.center().to_vec2());
            assert_eq!(back.index.0, i, "cursor round-trip at {i}");
        }
    }

    #[test]
    fn color_raster_glyph_is_not_tinted() {
        let rasterizer = GlyphRasterizer::new("test", |request: &GlyphRasterizerRequest<'_>| {
            (request.cluster == "한").then(color_raster_glyph)
        });
        let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default())
            .with_glyph_rasterizer(rasterizer);

        // Returns the tessellated vertex color of 'a' (a normal glyph) and '한' (a color glyph).
        let mut glyph_colors = |text_color: Color32, override_text_color: Option<Color32>| {
            let job = LayoutJob::simple(
                "a한".into(),
                FontId::proportional(14.0),
                text_color,
                f32::INFINITY,
            );
            let galley = layout(&mut fonts, 1.0, Arc::new(job));
            let row = &galley.rows[0].row;
            assert!(!row.glyphs[0].is_color);
            assert!(row.glyphs[1].is_color);
            let first_vertices = [
                row.glyphs[0].first_vertex as usize,
                row.glyphs[1].first_vertex as usize,
            ];

            let mut text_shape =
                crate::TextShape::new(Pos2::ZERO, Arc::new(galley), Color32::GREEN);
            text_shape.override_text_color = override_text_color;
            let mut mesh = crate::Mesh::default();
            crate::Tessellator::new(1.0, Default::default(), [1024, 1024], vec![])
                .tessellate_text(&text_shape, &mut mesh);
            first_vertices.map(|i| mesh.vertices[i].color)
        };

        let half_alpha = Color32::from_white_alpha(128);

        // Explicit text color:
        assert_eq!(
            glyph_colors(Color32::BLUE, None),
            [Color32::BLUE, Color32::WHITE]
        );

        // Delayed text color, resolved by the tessellator (used by most widgets):
        assert_eq!(
            glyph_colors(Color32::PLACEHOLDER, None),
            [Color32::GREEN, Color32::WHITE]
        );

        // Override, e.g. for disabled widgets. The alpha should still apply:
        assert_eq!(
            glyph_colors(Color32::BLUE, Some(Color32::from_black_alpha(128))),
            [Color32::from_black_alpha(128), half_alpha]
        );
    }

    #[test]
    fn color_raster_glyph_keeps_color_in_atlas() {
        let rasterizer = GlyphRasterizer::new("test", |_: &GlyphRasterizerRequest<'_>| {
            Some(color_raster_glyph())
        });
        // The default transfer function discards color for regular (white) glyphs:
        let options = TextOptions {
            color_transfer_function: crate::FontColorTransferFunction::TwoCoverageMinusCoverageSq,
            ..Default::default()
        };
        let mut fonts =
            Fonts::new(options, FontDefinitions::default()).with_glyph_rasterizer(rasterizer);
        let galley = fonts.with_pixels_per_point(1.0).layout_no_wrap(
            "한".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
        );

        let [x, y] = galley.rows[0].row.glyphs[0].uv_rect.min;
        assert_eq!(fonts.image()[(x as usize, y as usize)], Color32::RED);
    }

    /// A rasterizer that records the requests it gets and returns a fixed glyph.
    fn recording_rasterizer(
        requests: &Arc<crate::mutex::Mutex<Vec<(String, FontFamily)>>>,
        result: Option<RasterizedGlyph>,
    ) -> GlyphRasterizer {
        let requests = Arc::clone(requests);
        GlyphRasterizer::new(
            unique_test_key(),
            move |request: &GlyphRasterizerRequest<'_>| {
                requests
                    .lock()
                    .push((request.cluster.to_owned(), request.family.clone()));
                result.clone()
            },
        )
    }

    fn white_raster_glyph() -> RasterizedGlyph {
        RasterizedGlyph {
            bitmap: GlyphBitmap {
                image: ColorImage::new([1, 1], vec![Color32::WHITE]),
                offset_px: Vec2::ZERO,
                is_color: false,
            },
            advance_px: 10.0,
        }
    }

    /// Each test rasterizer needs its own [`GlyphRasterizer::key`], or they would replace each other.
    fn unique_test_key() -> String {
        static COUNTER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        format!("test rasterizer {n}")
    }

    fn color_raster_glyph() -> RasterizedGlyph {
        RasterizedGlyph {
            bitmap: GlyphBitmap {
                image: ColorImage::new([1, 1], vec![Color32::RED]),
                is_color: true,
                ..white_raster_glyph().bitmap
            },
            ..white_raster_glyph()
        }
    }

    /// `(chr, is_color, has_pixels)` for each glyph.
    #[cfg(feature = "default_fonts")]
    fn glyph_summary(galley: &Galley) -> Vec<(char, bool, bool)> {
        galley
            .rows
            .iter()
            .flat_map(|row| &row.row.glyphs)
            .map(|glyph| (glyph.chr, glyph.is_color, !glyph.uv_rect.is_nothing()))
            .collect()
    }

    #[test]
    #[cfg(feature = "default_fonts")] // Needs `hack_only`
    fn rasterized_sequence_keeps_one_glyph_per_char() {
        let requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let rasterizer = recording_rasterizer(&requests, Some(color_raster_glyph()));
        // No font has the emoji, so the rasterizer gets the whole cluster:
        let mut fonts =
            FontsImpl::new(TextOptions::default(), hack_only()).with_glyph_rasterizer(rasterizer);
        let family = "👨\u{200D}👩\u{200D}👧";
        let job = LayoutJob::simple(
            family.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(&mut fonts, 1.0, Arc::new(job));

        // One visible glyph for the whole cluster, plus zero-width continuation glyphs:
        let summary = glyph_summary(&galley);
        assert_eq!(summary.len(), family.chars().count());
        assert_eq!(summary[0], ('👨', true, true));
        assert!(summary[1..].iter().all(|(_, _, has_pixels)| !has_pixels));
        assert_eq!(
            *requests.lock(),
            [(family.to_owned(), FontFamily::Proportional)]
        );
    }

    #[test]
    fn raster_glyph_cache_is_keyed_by_family() {
        let requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let rasterizer = recording_rasterizer(&requests, Some(white_raster_glyph()));
        let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default())
            .with_glyph_rasterizer(rasterizer);

        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let job = LayoutJob::simple(
                "한".into(),
                FontId::new(14.0, family),
                Color32::WHITE,
                f32::INFINITY,
            );
            layout(&mut fonts, 1.0, Arc::new(job));
        }

        assert_eq!(
            *requests.lock(),
            vec![
                ("한".to_owned(), FontFamily::Proportional),
                ("한".to_owned(), FontFamily::Monospace),
            ]
        );
    }

    #[test]
    fn raster_glyph_failure_is_cached_and_falls_back_to_replacement() {
        let requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let rasterizer = recording_rasterizer(&requests, None);
        let mut fonts = FontsImpl::new(TextOptions::default(), FontDefinitions::default())
            .with_glyph_rasterizer(rasterizer);
        let job = Arc::new(LayoutJob::simple(
            "한".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        ));

        let galley = layout(&mut fonts, 1.0, Arc::clone(&job));
        layout(&mut fonts, 1.0, job);

        assert_eq!(requests.lock().len(), 1, "Failures should be cached");
        let glyph = &galley.rows[0].row.glyphs[0];
        assert_eq!(glyph.chr, '한');
        assert!(
            !glyph.uv_rect.is_nothing(),
            "Should render the `.notdef` glyph"
        );
    }

    /// A priority rasterizer that only handles `cluster`, and records every request.
    fn priority_rasterizer_for(
        cluster: &'static str,
        requests: &Arc<crate::mutex::Mutex<Vec<(String, FontFamily)>>>,
    ) -> GlyphRasterizer {
        let requests = Arc::clone(requests);
        GlyphRasterizer::new(
            unique_test_key(),
            move |request: &GlyphRasterizerRequest<'_>| {
                requests
                    .lock()
                    .push((request.cluster.to_owned(), request.family.clone()));
                (request.cluster == cluster).then(color_raster_glyph)
            },
        )
        .with_priority(FontPriority::Highest)
    }

    fn layout_simple(fonts: &mut FontsImpl, text: &str) -> Galley {
        let job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        layout(fonts, 1.0, Arc::new(job))
    }

    #[test]
    #[cfg(feature = "default_fonts")] // The font must have `…` for the override to mean anything
    fn priority_rasterizer_overrides_installed_glyph() {
        let requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let mut fonts = test_fonts().with_glyph_rasterizer(priority_rasterizer_for("…", &requests));
        assert!(fonts.has_glyph(&FontFamily::Proportional, '…'));

        let galley = layout_simple(&mut fonts, "a…b");

        assert_eq!(
            glyph_summary(&galley),
            [('a', false, true), ('…', true, true), ('b', false, true)]
        );
        let clusters: Vec<_> = requests
            .lock()
            .iter()
            .map(|(cluster, _)| cluster.clone())
            .collect();
        assert_eq!(clusters, ["a", "…", "b"], "Asked about every cluster");
    }

    #[test]
    #[cfg(feature = "default_fonts")] // Needs `glyph_summary`
    fn priority_rasterizer_miss_falls_through_to_font_before_fallback() {
        let priority_requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let fallback_requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let mut fonts = test_fonts()
            .with_glyph_rasterizer(priority_rasterizer_for("never", &priority_requests))
            .with_glyph_rasterizer(recording_rasterizer(
                &fallback_requests,
                Some(color_raster_glyph()),
            ));

        let galley = layout_simple(&mut fonts, "a한");

        // 'a' comes from the font, '한' from the fallback:
        assert_eq!(
            glyph_summary(&galley),
            [('a', false, true), ('한', true, true)]
        );
        assert_eq!(priority_requests.lock().len(), 2);
        assert_eq!(
            *fallback_requests.lock(),
            [("한".to_owned(), FontFamily::Proportional)],
            "The fallback should only see what the fonts lack"
        );
    }

    #[test]
    fn rasterizers_are_asked_in_order_until_one_succeeds() {
        let first_requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let second_requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let third_requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let mut fonts = test_fonts()
            .with_glyph_rasterizer(recording_rasterizer(&first_requests, None))
            .with_glyph_rasterizer(recording_rasterizer(
                &second_requests,
                Some(white_raster_glyph()),
            ))
            .with_glyph_rasterizer(recording_rasterizer(
                &third_requests,
                Some(white_raster_glyph()),
            ));

        let galley = layout_simple(&mut fonts, "한");

        assert!(!galley.rows[0].row.glyphs[0].uv_rect.is_nothing());
        assert_eq!(first_requests.lock().len(), 1);
        assert_eq!(second_requests.lock().len(), 1);
        assert_eq!(
            third_requests.lock().len(),
            0,
            "The second one already succeeded"
        );
    }

    #[test]
    #[cfg(feature = "default_fonts")] // Needs `glyph_summary`
    fn priority_rasterizer_keeps_one_glyph_per_char() {
        let requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let family = "👨\u{200D}👩\u{200D}👧";
        let mut fonts =
            test_fonts().with_glyph_rasterizer(priority_rasterizer_for(family, &requests));

        let galley = layout_simple(&mut fonts, family);

        let summary = glyph_summary(&galley);
        assert_eq!(summary.len(), family.chars().count());
        assert_eq!(summary[0], ('👨', true, true));
        assert!(summary[1..].iter().all(|(_, _, has_pixels)| !has_pixels));
    }

    #[test]
    fn overflow_character_uses_priority_rasterizer() {
        let requests = Arc::new(crate::mutex::Mutex::new(Vec::new()));
        let mut fonts = test_fonts().with_glyph_rasterizer(priority_rasterizer_for("…", &requests));

        let mut job = LayoutJob::simple(
            "Hello wide world".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        job.wrap = TextWrapping::truncate_at_width(40.0);
        let galley = layout(&mut fonts, 1.0, Arc::new(job));

        let glyphs = &galley.rows[0].row.glyphs;
        let last = &glyphs[glyphs.len() - 1];
        assert_eq!(last.chr, '…');
        assert!(last.is_color, "Should come from the priority rasterizer");
        assert!(!last.uv_rect.is_nothing());
    }

    #[test]
    fn test_zero_max_width() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let mut layout_job = LayoutJob::single_section("W".into(), TextFormat::default());
        layout_job.wrap.max_width = 0.0;
        let galley = layout(&mut fonts, pixels_per_point, layout_job.into());
        assert_eq!(galley.rows.len(), 1);
    }

    #[test]
    fn test_truncate_with_newline() {
        // No matter where we wrap, we should be appending the newline character.

        let pixels_per_point = 1.0;

        let mut fonts = test_fonts();
        let text_format = TextFormat {
            font_id: FontId::monospace(12.0),
            ..Default::default()
        };

        for text in ["Hello\nworld", "\nfoo"] {
            for break_anywhere in [false, true] {
                for max_width in [0.0, 5.0, 10.0, 20.0, f32::INFINITY] {
                    let mut layout_job =
                        LayoutJob::single_section(text.into(), text_format.clone());
                    layout_job.wrap.max_width = max_width;
                    layout_job.wrap.max_rows = 1;
                    layout_job.wrap.break_anywhere = break_anywhere;

                    let galley = layout(&mut fonts, pixels_per_point, layout_job.into());

                    assert!(galley.elided);
                    assert_eq!(galley.rows.len(), 1);
                    let row_text = galley.rows[0].text();
                    assert!(
                        row_text.ends_with('…'),
                        "Expected row to end with `…`, got {row_text:?} when line-breaking the text {text:?} with max_width {max_width} and break_anywhere {break_anywhere}.",
                    );
                }
            }
        }

        {
            let mut layout_job = LayoutJob::single_section("Hello\nworld".into(), text_format);
            layout_job.wrap.max_width = 50.0;
            layout_job.wrap.max_rows = 1;
            layout_job.wrap.break_anywhere = false;

            let galley = layout(&mut fonts, pixels_per_point, layout_job.into());

            assert!(galley.elided);
            assert_eq!(galley.rows.len(), 1);
            let row_text = galley.rows[0].text();
            assert_eq!(row_text, "Hello…");
        }
    }

    #[test]
    #[cfg(feature = "default_fonts")]
    // No bundled font has CJK glyphs, so the line breaks depend on
    // the width of the `.notdef` glyph of the primary font.
    fn test_cjk() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let mut layout_job = LayoutJob::single_section(
            "日本語とEnglishの混在した文章".into(),
            TextFormat::default(),
        );
        layout_job.wrap.max_width = 90.0;
        let galley = layout(&mut fonts, pixels_per_point, layout_job.into());
        assert_eq!(
            galley.rows.iter().map(|row| row.text()).collect::<Vec<_>>(),
            vec!["日本語とEnglishの混", "在した文章"]
        );
    }

    #[test]
    #[cfg(feature = "default_fonts")]
    // No bundled font has CJK glyphs, so the line breaks depend on
    // the width of the `.notdef` glyph of the primary font.
    fn test_pre_cjk() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let mut layout_job = LayoutJob::single_section(
            "日本語とEnglishの混在した文章".into(),
            TextFormat::default(),
        );
        layout_job.wrap.max_width = 110.0;
        let galley = layout(&mut fonts, pixels_per_point, layout_job.into());
        assert_eq!(
            galley.rows.iter().map(|row| row.text()).collect::<Vec<_>>(),
            vec!["日本語とEnglishの混在した", "文章"]
        );
    }

    #[test]
    fn test_truncate_width() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let mut layout_job =
            LayoutJob::single_section("# DNA\nMore text".into(), TextFormat::default());
        layout_job.wrap.max_width = f32::INFINITY;
        layout_job.wrap.max_rows = 1;
        layout_job.round_output_to_gui = false;
        let galley = layout(&mut fonts, pixels_per_point, layout_job.into());
        assert!(galley.elided);
        assert_eq!(
            galley.rows.iter().map(|row| row.text()).collect::<Vec<_>>(),
            vec!["# DNA…"]
        );
        let row = &galley.rows[0];
        assert_eq!(row.pos, Pos2::ZERO);
        assert_eq!(row.rect().max.x, row.glyphs.last().unwrap().max_x());
    }

    #[test]
    fn test_truncate_with_pixels_per_point() {
        let mut fonts = test_fonts();

        for pixels_per_point in [
            0.33, 0.5, 0.67, 1.0, 1.25, 1.33, 1.5, 1.75, 2.0, 3.0, 4.0, 5.0,
        ] {
            for ch in ['W', 'A', 'n', 't', 'i'] {
                let target_width = 50.0;
                let text = (0..20).map(|_| ch).collect::<String>();

                let mut job = LayoutJob::single_section(text, TextFormat::default());
                job.wrap.max_width = target_width;
                job.wrap.max_rows = 1;
                let elided_galley = layout(&mut fonts, pixels_per_point, job.into());
                assert!(elided_galley.elided);

                let test_galley = layout(
                    &mut fonts,
                    pixels_per_point,
                    Arc::new(LayoutJob::single_section(
                        iter::chain(
                            (0..elided_galley.rows[0].char_count_excluding_newline().0).map(|_| ch),
                            iter::once('…'),
                        )
                        .collect::<String>(),
                        TextFormat::default(),
                    )),
                );

                assert!(elided_galley.size().x >= 0.0);
                assert!(elided_galley.size().x <= target_width);
                assert!(test_galley.size().x > target_width);
            }
        }
    }

    #[test]
    fn test_empty_row() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let font_id = FontId::default();
        let family = fonts.family_key(&font_id.family);
        let font_height = fonts
            .family_metrics(
                family,
                pixels_per_point,
                font_id.size,
                &VariationCoords::default(),
            )
            .row_height;

        let job = LayoutJob::simple(String::new(), font_id, Color32::WHITE, f32::INFINITY);

        let galley = layout(&mut fonts, pixels_per_point, job.into());

        assert_eq!(galley.rows.len(), 1, "Expected one row");
        assert_eq!(
            galley.rows[0].row.glyphs.len(),
            0,
            "Expected no glyphs in the empty row"
        );
        assert_eq!(
            galley.size(),
            Vec2::new(0.0, font_height.round()),
            "Unexpected galley size"
        );
        assert_eq!(
            galley.intrinsic_size(),
            Vec2::new(0.0, font_height.round()),
            "Unexpected intrinsic size"
        );
    }

    #[test]
    #[cfg(feature = "default_fonts")]
    fn test_end_with_newline() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let font_id = FontId::default();
        let family = fonts.family_key(&font_id.family);
        let font_height = fonts
            .family_metrics(
                family,
                pixels_per_point,
                font_id.size,
                &VariationCoords::default(),
            )
            .row_height;

        let job = LayoutJob::simple("Hi!\n".to_owned(), font_id, Color32::WHITE, f32::INFINITY);

        let galley = layout(&mut fonts, pixels_per_point, job.into());

        assert_eq!(galley.rows.len(), 2, "Expected two rows");
        assert_eq!(
            galley.rows[1].row.glyphs.len(),
            0,
            "Expected no glyphs in the empty row"
        );
        assert_eq!(
            galley.size().round(),
            Vec2::new(17.0, font_height.round() * 2.0),
            "Unexpected galley size"
        );
        assert_eq!(
            galley.intrinsic_size().round(),
            Vec2::new(17.0, font_height.round() * 2.0),
            "Unexpected intrinsic size"
        );
    }

    #[test]
    fn test_valign_with_custom_line_height() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let font_id = FontId::default();
        let family = fonts.family_key(&font_id.family);
        let font_height = fonts
            .family_metrics(
                family,
                pixels_per_point,
                font_id.size,
                &VariationCoords::default(),
            )
            .row_height;
        let extra_height = 20.0;
        let line_height = (font_height + extra_height).round();
        let free_space = line_height - font_height;

        // Row height, and y-position of the last glyph (in the last section):
        let mut glyph_y = |sections: &[(Option<f32>, Align)]| {
            let mut job = LayoutJob::default();
            for &(line_height, valign) in sections {
                job.append(
                    "Hello",
                    0.0,
                    TextFormat {
                        font_id: font_id.clone(),
                        line_height,
                        valign,
                        ..Default::default()
                    },
                );
            }
            let galley = layout(&mut fonts, pixels_per_point, job.into());
            assert_eq!(galley.rows.len(), 1);
            let glyph = galley.rows[0].row.glyphs.last().unwrap();
            (galley.size().y, glyph.pos.y)
        };

        let (default_height, baseline_y) = glyph_y(&[(None, Align::default())]);
        assert_eq!(default_height, font_height.round());

        // A custom line height always centers the text, regardless of `valign`
        // (including the default `valign`):
        for valign in [
            TextFormat::default().valign,
            Align::TOP,
            Align::Center,
            Align::BOTTOM,
        ] {
            let (height, y) = glyph_y(&[(Some(line_height), valign)]);
            assert_eq!(
                height, line_height,
                "Row should have the custom line height"
            );
            assert!(
                (y - baseline_y - 0.5 * free_space).abs() <= 1.0,
                "{valign:?}: text should be centered in the line: baseline_y={baseline_y}, y={y}, free_space={free_space}"
            );
        }

        // `valign` still governs how a section without a custom line height
        // is placed within a taller row:
        let tall = (Some(line_height), Align::Center);
        let (_, top) = glyph_y(&[tall, (None, Align::TOP)]);
        let (_, center) = glyph_y(&[tall, (None, Align::Center)]);
        let (_, bottom) = glyph_y(&[tall, (None, Align::BOTTOM)]);
        assert!((top - baseline_y).abs() <= 1.0, "top={top}");
        assert!(
            (center - top - 0.5 * free_space).abs() <= 1.0,
            "top={top}, center={center}, free_space={free_space}"
        );
        assert!(
            (bottom - top - free_space).abs() <= 1.0,
            "top={top}, bottom={bottom}, free_space={free_space}"
        );
    }

    #[test]
    fn test_combining_diacritics() {
        // ɔ̃ = U+0254 (LATIN SMALL LETTER OPEN O) + U+0303 (COMBINING TILDE)
        // With text shaping, the combining tilde should NOT produce a separate
        // advance — it should be positioned above ɔ via GPOS anchors.
        // Note: the default fonts don't contain U+0254, so the `.notdef` glyph
        // is used. The key test is that the combining mark does NOT add extra width.
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let job_combined = LayoutJob::simple(
            "ɔ\u{0303}".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley_combined = layout(&mut fonts, pixels_per_point, job_combined.into());

        let job_base = LayoutJob::simple(
            "ɔ".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley_base = layout(&mut fonts, pixels_per_point, job_base.into());

        let width_combined = galley_combined.size().x;
        let width_base = galley_base.size().x;

        assert!(
            (width_combined - width_base).abs() < 2.0,
            "Combining diacritic should not add significant width. \
             Base width: {width_base}, Combined width: {width_combined}"
        );

        let glyphs = &galley_combined.rows[0].row.glyphs;
        assert!(!glyphs.is_empty(), "Expected at least 1 glyph for ɔ̃");
    }

    #[test]
    fn test_shaping_basic_latin() {
        // Basic test: shaped Latin text should produce the same number of glyphs as characters.
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let job = LayoutJob::simple(
            "Hello".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(&mut fonts, pixels_per_point, job.into());

        assert_eq!(galley.rows.len(), 1);
        assert_eq!(galley.rows[0].row.glyphs.len(), 5);
        assert!(galley.size().x > 0.0);
    }

    #[test]
    fn test_shaping_empty_string() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let job = LayoutJob::simple(
            String::new(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(&mut fonts, pixels_per_point, job.into());

        assert_eq!(galley.rows.len(), 1);
        assert_eq!(galley.rows[0].row.glyphs.len(), 0);
    }

    #[test]
    fn test_shaping_multiple_newlines() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let job = LayoutJob::simple(
            "A\n\nB".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(&mut fonts, pixels_per_point, job.into());

        assert_eq!(galley.rows.len(), 3, "Expected 3 rows for 'A\\n\\nB'");
        assert_eq!(galley.rows[0].row.glyphs.len(), 1); // "A"
        assert_eq!(galley.rows[1].row.glyphs.len(), 0); // empty line
        assert_eq!(galley.rows[2].row.glyphs.len(), 1); // "B"
    }

    #[test]
    fn test_shaping_mixed_font_fallback() {
        // Text with both Latin and emoji should work without panicking,
        // even though they use different font faces.
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();

        let job = LayoutJob::simple(
            "Hi 🎉 bye".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(&mut fonts, pixels_per_point, job.into());

        assert_eq!(galley.rows.len(), 1);
        // "Hi " (3) + "🎉" (1) + " bye" (4) = at least 8 glyphs
        assert!(
            galley.rows[0].row.glyphs.len() >= 8,
            "Expected >= 8 glyphs, got {}",
            galley.rows[0].row.glyphs.len()
        );
    }

    #[test]
    #[cfg(feature = "default_fonts")]
    fn test_gpos_kerning() {
        // GPOS kerning: pairs like "AV", "VA", "AT" should be tighter than
        // the sum of individual character widths. Without text shaping, egui
        // only uses the legacy `kern` table, so these pairs had diff ≈ 0.
        // With harfrust, GPOS kerning applies proper negative adjustments.
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let font_id = FontId::proportional(14.0);

        for pair in ["AV", "VA", "AT"] {
            let (pair_w, _, _) = measure_text(&mut fonts, pair, &font_id, pixels_per_point);
            let chars: Vec<char> = pair.chars().collect();
            let (w1, _, _) = measure_text(
                &mut fonts,
                &chars[0].to_string(),
                &font_id,
                pixels_per_point,
            );
            let (w2, _, _) = measure_text(
                &mut fonts,
                &chars[1].to_string(),
                &font_id,
                pixels_per_point,
            );
            let sum = w1 + w2;
            let kern_adjustment = sum - pair_w;

            assert!(
                kern_adjustment > 0.5,
                "GPOS kerning for '{pair}': expected pair to be noticeably tighter \
                 than sum of individuals. pair_width={pair_w:.2}, sum={sum:.2}, \
                 kern_adjustment={kern_adjustment:.2} (should be > 0.5)",
            );
        }
    }

    /// Regression test for <https://github.com/emilk/egui/issues/8087>.
    ///
    /// Multi-codepoint grapheme clusters (flag emojis, combining marks) must
    /// produce exactly as many glyphs as characters so that cursor positioning
    /// and text selection remain correct.
    #[test]
    fn test_grapheme_cluster_glyph_count() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let font_id = FontId::default();

        // Each test case: (input text, expected char count)
        let cases: &[(&str, usize)] = &[
            // Flag emoji: two Regional Indicator codepoints → one visual glyph
            ("\u{1F1EF}\u{1F1F5}", 2), // 🇯🇵
            // Flag surrounded by ASCII
            ("A\u{1F1EB}\u{1F1F7}B", 4), // A🇫🇷B
            // Base char + combining acute accent
            ("e\u{0301}", 2), // é as decomposed
            // Multiple combining marks
            ("o\u{0302}\u{0323}", 3), // ộ
            // Plain ASCII (sanity check)
            ("Hello", 5),
        ];

        for &(text, expected_chars) in cases {
            let job = LayoutJob::simple(
                text.to_owned(),
                font_id.clone(),
                Color32::WHITE,
                f32::INFINITY,
            );
            let galley = layout(&mut fonts, pixels_per_point, job.into());

            let total_glyphs: usize = galley.rows.iter().map(|r| r.row.glyphs.len()).sum();

            assert_eq!(
                total_glyphs,
                expected_chars,
                "Glyph count mismatch for {text:?}: \
                 expected {expected_chars} glyphs (one per char), got {total_glyphs}. \
                 Glyphs: {:?}",
                galley.rows[0]
                    .row
                    .glyphs
                    .iter()
                    .map(|g| (g.chr, g.advance_width))
                    .collect::<Vec<_>>(),
            );

            // Verify that Row::text() reconstructs the input text.
            let row_text: String = galley.rows.iter().map(|r| r.text()).collect();
            assert_eq!(row_text, text, "Row::text() mismatch for {text:?}");

            // Verify cursor round-trip: end cursor index == char count.
            assert_eq!(
                galley.end().index.0,
                expected_chars,
                "Galley::end().index mismatch for {text:?}",
            );
        }
    }

    /// Verify that cursor positioning round-trips correctly for text
    /// containing multi-codepoint grapheme clusters (regression test for #8087).
    #[test]
    fn test_grapheme_cluster_cursor_roundtrip() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let font_id = FontId::default();

        // "A" + flag emoji (2 codepoints) + "B" = 4 chars
        let text = "A\u{1F1EF}\u{1F1F5}B";
        let job = LayoutJob::simple(
            text.to_owned(),
            font_id.clone(),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(&mut fonts, pixels_per_point, job.into());

        // Walking through every cursor index should produce valid positions.
        for i in 0..=galley.end().index.0 {
            let cursor = CCursor {
                index: CharIndex(i),
                prefer_next_row: false,
            };
            let rect = galley.pos_from_cursor(cursor);
            assert!(
                rect.is_finite(),
                "pos_from_cursor returned non-finite rect for index {i}",
            );

            // Round-trip: position → cursor → position should be stable.
            let cursor2 = galley.cursor_from_pos(Vec2::new(rect.center().x, rect.center().y));
            let rect2 = galley.pos_from_cursor(cursor2);
            assert!(
                (rect.min.x - rect2.min.x).abs() < 1.0,
                "Cursor round-trip unstable at index {i}: \
                 first={}, second={}, cursor2.index={}",
                rect.min.x,
                rect2.min.x,
                cursor2.index,
            );
        }
    }

    /// The bundled fonts have no Hebrew, so these glyphs are `.notdef`;
    /// levels, order, glyph count and cursor geometry do not depend on that.
    #[test]
    fn bidi_text_keeps_one_glyph_per_char_in_logical_order() {
        let mut fonts = test_fonts();
        for text in ["שלום 42 עולם", "abc אבג 12", "אב", "١٢٣ עברית"] {
            let galley = layout_simple(&mut fonts, text);
            let row = &galley.rows[0].row;
            assert_eq!(row.glyphs.len(), text.chars().count(), "{text:?}");
            assert_eq!(row.text(), text);
            assert_eq!(galley.end().index.0, text.chars().count());
        }
    }

    #[test]
    fn rtl_rows_are_placed_in_visual_order() {
        let mut fonts = test_fonts();
        let x_of = |galley: &Galley, chr: char| {
            galley.rows[0]
                .row
                .glyphs
                .iter()
                .find(|g| g.chr == chr)
                .map(|g| g.pos.x)
                .unwrap_or_else(|| panic!("no glyph for {chr:?}"))
        };

        // A right-to-left paragraph: the first word ends up on the right,
        // and the number inside it still reads left to right.
        let galley = layout_simple(&mut fonts, "אב 12");
        let xs = ['1', '2', ' ', 'ב', 'א'].map(|c| x_of(&galley, c));
        assert!(xs.is_sorted(), "visual order of \"אב 12\": {xs:?}");
        // The space takes the paragraph direction (UAX #9 N1/N2); the digits do not.
        for glyph in &galley.rows[0].row.glyphs {
            assert_eq!(
                glyph.is_rtl(),
                matches!(glyph.chr, 'א' | 'ב' | ' '),
                "{:?}",
                glyph.chr
            );
        }

        // A left-to-right paragraph with a Hebrew word in it:
        // the word is mirrored, the line is not.
        let galley = layout_simple(&mut fonts, "ab אב");
        let xs = ['a', 'b', ' ', 'ב', 'א'].map(|c| x_of(&galley, c));
        assert!(xs.is_sorted(), "visual order of \"ab אב\": {xs:?}");

        // Plain text is untouched:
        let galley = layout_simple(&mut fonts, "ab 12");
        let xs = ['a', 'b', ' ', '1', '2'].map(|c| x_of(&galley, c));
        assert!(xs.is_sorted(), "{xs:?}");
    }

    #[test]
    fn rtl_cursor_sits_on_the_right_side_of_its_glyph() {
        let mut fonts = test_fonts();
        let galley = layout_simple(&mut fonts, "אב");
        let row = &galley.rows[0].row;
        let [alef, bet] = [&row.glyphs[0], &row.glyphs[1]];
        assert!(bet.pos.x < alef.pos.x, "bet is drawn left of alef");
        assert_eq!(row.x_offset(CharIndex(0)), alef.max_x());
        assert_eq!(row.x_offset(CharIndex(1)), bet.max_x());
        assert_eq!(row.x_offset(CharIndex(2)), bet.pos.x);

        for i in 0..=2 {
            let cursor = CCursor {
                index: CharIndex(i),
                prefer_next_row: false,
            };
            let rect = galley.pos_from_cursor(cursor);
            let back = galley.cursor_from_pos(rect.center().to_vec2());
            assert_eq!(back.index.0, i, "cursor round-trip at {i}");
        }
    }

    /// Regression test for <https://github.com/emilk/egui/issues/8576>.
    ///
    /// The shaper returns a right-to-left run in visual order, with decreasing
    /// cluster offsets. Such a run must still produce one glyph per character.
    #[test]
    fn test_right_to_left_run_glyph_count() {
        let pixels_per_point = 1.0;
        let mut fonts = test_fonts();
        let font_id = FontId::default();

        let cases = [
            "ب",
            "بب",
            "ببب",
            "אב",
            "a אבג b",
            "שלום 42 עולם",
            // Base char + combining mark inside a right-to-left run
            "ב\u{05BC}א",
        ];

        for text in cases {
            let job = LayoutJob::simple(
                text.to_owned(),
                font_id.clone(),
                Color32::WHITE,
                f32::INFINITY,
            );
            let galley = layout(&mut fonts, pixels_per_point, job.into());

            let total_glyphs: usize = galley.rows.iter().map(|r| r.row.glyphs.len()).sum();
            let num_chars = text.chars().count();

            assert_eq!(total_glyphs, num_chars, "Glyph count mismatch for {text:?}");
            assert_eq!(
                galley.end().index.0,
                num_chars,
                "Galley::end().index mismatch for {text:?}",
            );
        }
    }

    /// Arabic-Indic digits belong to the Arabic script, so a shaper left to guess the direction
    /// shapes them right to left and "٤٥" comes out as "٥٤". Outside right-to-left text the bidi
    /// algorithm keeps them left to right (<https://github.com/crmne/zapfast/issues/184>).
    #[test]
    fn arabic_indic_digits_without_rtl_text_stay_left_to_right() {
        let mut fonts = test_fonts();
        for text in ["\u{664}\u{665}", "\u{664}\u{665} messages"] {
            let galley = layout_simple(&mut fonts, text);
            let glyphs = &galley.rows[0].row.glyphs;
            assert_eq!(glyphs.len(), text.chars().count(), "{text:?}");
            assert_eq!(galley.rows[0].row.text(), text);
            assert!(
                glyphs[0].pos.x < glyphs[1].pos.x,
                "{text:?}: four is drawn left of five"
            );
        }
    }

    fn layout_with_direction(
        fonts: &mut FontsImpl,
        text: &str,
        direction: TextDirection,
    ) -> Galley {
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        job.direction = direction;
        layout(fonts, 1.0, Arc::new(job))
    }

    #[test]
    fn paragraph_direction_is_auto_or_forced() {
        let mut fonts = test_fonts();

        // Each paragraph takes the direction of its first strong character:
        let galley =
            layout_with_direction(&mut fonts, "abc אבג\nאבג abc\n123", TextDirection::Auto);
        let rtl: Vec<bool> = galley.rows.iter().map(|row| row.rtl_paragraph).collect();
        assert_eq!(rtl, [false, true, false]);
        assert_visual_order(&galley.rows[0].row, &[0, 1, 2, 6, 5, 4]);
        assert_visual_order(&galley.rows[1].row, &[4, 5, 6, 2, 1, 0]);

        // A forced direction overrides the first strong character:
        let galley = layout_with_direction(&mut fonts, "abc אבג", TextDirection::RightToLeft);
        assert!(galley.rows[0].rtl_paragraph);
        assert_visual_order(&galley.rows[0].row, &[6, 5, 4, 0, 1, 2]);

        let galley = layout_with_direction(&mut fonts, "אבג abc", TextDirection::LeftToRight);
        assert!(!galley.rows[0].rtl_paragraph);
        assert_visual_order(&galley.rows[0].row, &[2, 1, 0, 4, 5, 6]);

        // Text without a right-to-left character can still be in a right-to-left paragraph:
        let galley = layout_with_direction(&mut fonts, "abc", TextDirection::RightToLeft);
        assert!(galley.rows[0].rtl_paragraph);
        assert!(
            galley.rows[0]
                .glyphs
                .iter()
                .all(|glyph| glyph.bidi_level == 2)
        );
        assert_visual_order(&galley.rows[0].row, &[0, 1, 2]);
    }

    /// `Fonts` lays out each paragraph of a multi-paragraph job on its own;
    /// every one of them must keep the job's direction.
    #[test]
    fn paragraph_direction_survives_the_paragraph_cache() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        let mut job = LayoutJob::simple(
            "abc\ndef\n".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        job.direction = TextDirection::RightToLeft;
        let galley = fonts.with_pixels_per_point(1.0).layout_job(job);
        assert_eq!(galley.rows.len(), 3);
        assert!(galley.rows.iter().all(|row| row.rtl_paragraph));
    }

    /// The left and right edge of the visible glyphs of a row, in galley coordinates.
    fn visible_x_range(placed_row: &PlacedRow) -> Rangef {
        let visible = placed_row
            .glyphs
            .iter()
            .filter(|glyph| !glyph.chr.is_whitespace());
        let min = visible
            .clone()
            .map(|g| g.pos.x)
            .fold(f32::INFINITY, f32::min);
        let max = visible.map(Glyph::max_x).fold(f32::NEG_INFINITY, f32::max);
        Rangef::new(placed_row.pos.x + min, placed_row.pos.x + max)
    }

    #[test]
    fn rtl_paragraphs_start_on_the_right() {
        let text = "abcdefghij\nאב גד\nאב";
        let job = |halign: Align, halign_min_is_start: bool| {
            let mut job = LayoutJob::simple(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                f32::INFINITY,
            );
            job.halign = halign;
            job.halign_min_is_start = halign_min_is_start;
            job
        };
        let mut fonts = test_fonts();
        let close = |a: f32, b: f32| (a - b).abs() <= 0.5;

        // The start of a right-to-left paragraph is on the right,
        // flush with the widest row; the galley still starts at x = 0:
        let galley = layout(&mut fonts, 1.0, Arc::new(job(Align::LEFT, true)));
        let ranges: Vec<Rangef> = galley.rows.iter().map(visible_x_range).collect();
        assert_eq!(galley.rect.min.x, 0.0);
        assert!(close(ranges[0].min, 0.0), "{ranges:?}");
        for range in &ranges[1..] {
            assert!(close(range.max, ranges[0].max), "{ranges:?}");
            assert!(0.0 < range.min, "{ranges:?}");
        }

        // `Fonts` lays out each paragraph on its own, then joins them:
        let joined = Fonts::new(TextOptions::default(), FontDefinitions::default())
            .with_pixels_per_point(1.0)
            .layout_job(job(Align::LEFT, true));
        let joined_ranges: Vec<Rangef> = joined.rows.iter().map(visible_x_range).collect();
        assert_eq!(joined_ranges, ranges);
        assert_eq!(joined.rect.min.x, 0.0);

        // An explicit left alignment:
        let galley = layout(&mut fonts, 1.0, Arc::new(job(Align::LEFT, false)));
        for placed_row in &galley.rows {
            assert!(close(visible_x_range(placed_row).min, 0.0));
        }

        // Right and center alignment do not depend on the direction:
        let galley = layout(&mut fonts, 1.0, Arc::new(job(Align::RIGHT, true)));
        for placed_row in &galley.rows {
            assert!(close(visible_x_range(placed_row).max, 0.0));
        }
        let galley = layout(&mut fonts, 1.0, Arc::new(job(Align::Center, true)));
        for placed_row in &galley.rows {
            let range = visible_x_range(placed_row);
            assert!((range.min + range.max).abs() <= 1.0, "{range:?}");
        }
    }

    #[test]
    fn elision_is_at_the_visual_end_of_the_row() {
        let mut fonts = test_fonts();
        for (text, rtl) in [
            ("אבגד הוזח טיכל מנסע", true),
            ("שלום hello world again", true),
            ("ab אבגד הוזח טיכל מנסע", false),
        ] {
            let full_width = layout_simple(&mut fonts, text).size().x;
            let mut job = LayoutJob::simple_singleline(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
            );
            job.wrap = TextWrapping::truncate_at_width(full_width / 2.0);
            let galley = layout(&mut fonts, 1.0, Arc::new(job));
            assert!(galley.elided);
            let row = &galley.rows[0].row;
            assert_eq!(row.rtl_paragraph, rtl);

            // The logically first text is kept, followed by the overflow character:
            let kept = row.text();
            assert!(kept.ends_with('…'), "{kept:?}");
            assert!(text.starts_with(kept.trim_end_matches('…')), "{kept:?}");

            let ellipsis = row.glyphs.last().unwrap();
            let others = &row.glyphs[..row.glyphs.len() - 1];
            // Without fonts the overflow character has no width, and goes along with
            // the glyph before it:
            let visible = 0.0 < ellipsis.advance_width;
            assert_eq!(visible, cfg!(feature = "default_fonts"));
            if visible {
                let at_visual_end = |g: &Glyph| {
                    if rtl {
                        ellipsis.max_x() <= g.pos.x
                    } else {
                        g.max_x() <= ellipsis.pos.x
                    }
                };
                assert!(others.iter().all(at_visual_end), "{kept:?}");
            }
        }
    }

    #[test]
    fn whitespace_at_the_end_of_a_row_takes_the_paragraph_level() {
        let mut fonts = test_fonts();
        let mut layout_left = |text: &str, max_width: f32| {
            let mut job = LayoutJob::simple(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                max_width,
            );
            job.halign_min_is_start = false;
            layout(&mut fonts, 1.0, Arc::new(job))
        };
        for (first_row, rest, rtl) in [("אבג abc ", "def", true), ("abc אבג ", "דהו", false)]
        {
            let width = layout_left(first_row, f32::INFINITY).size().x;
            let galley = layout_left(&format!("{first_row}{rest}"), width + 1.0);
            assert_eq!(galley.rows.len(), 2);
            let row = &galley.rows[0].row;
            assert_eq!(row.text(), first_row);
            assert_eq!(row.rtl_paragraph, rtl);

            // Inside the paragraph this space is between two words of the other direction,
            // but where the row wraps it goes to the visual end of the row:
            let space = row.glyphs.last().unwrap();
            assert_eq!(space.bidi_level, u8::from(rtl));
            let others = &row.glyphs[..row.glyphs.len() - 1];
            if rtl {
                assert!(others.iter().all(|g| space.pos.x < g.pos.x));
            } else {
                assert!(others.iter().all(|g| g.pos.x < space.pos.x));
            }
        }

        // A tab is a segment separator and also takes the paragraph level:
        let galley = layout_with_direction(&mut fonts, "abc \tdef", TextDirection::RightToLeft);
        let levels: Vec<u8> = galley.rows[0].glyphs.iter().map(|g| g.bidi_level).collect();
        assert_eq!(levels, [2, 2, 2, 1, 1, 2, 2, 2]);
    }

    #[test]
    fn reordering_a_long_mixed_paragraph_takes_linear_steps() {
        let mut fonts = test_fonts();
        let text: String = iter::repeat_n("abc אבג 123 (דה) x: ", 500).collect();
        let num_chars = text.chars().count();
        assert!(10_000 <= num_chars);

        for max_width in [f32::INFINITY, 300.0] {
            REORDER_STEPS.with(|steps| steps.set(0));
            let job = LayoutJob::simple(
                text.clone(),
                FontId::proportional(14.0),
                Color32::WHITE,
                max_width,
            );
            let galley = layout(&mut fonts, 1.0, Arc::new(job));
            let steps = REORDER_STEPS.with(core::cell::Cell::get);
            assert!(
                0 < steps && steps <= 6 * num_chars,
                "{steps} steps for {num_chars} chars"
            );

            // The cached order is a permutation that matches where the glyphs are:
            for placed_row in &galley.rows {
                let row = &placed_row.row;
                let mut order: Vec<usize> = row.visual_order().collect();
                let xs: Vec<f32> = order
                    .iter()
                    .filter(|&&i| 0.0 < row.glyphs[i].advance_width)
                    .map(|&i| row.glyphs[i].pos.x)
                    .collect();
                assert!(xs.is_sorted(), "{:?}", row.text());
                order.sort_unstable();
                assert!(order.into_iter().eq(0..row.glyphs.len()));
            }
        }
    }

    /// Asserts that the glyphs at these char indices are placed left to right.
    #[track_caller]
    fn assert_visual_order(row: &Row, left_to_right: &[usize]) {
        let xs: Vec<f32> = left_to_right.iter().map(|&i| row.glyphs[i].pos.x).collect();
        assert!(
            xs.is_sorted_by(|a, b| a < b),
            "{:?}: glyphs {left_to_right:?} are at {xs:?}",
            row.text()
        );
    }

    #[test]
    #[cfg(feature = "default_fonts")]
    fn arabic_with_digits_and_brackets() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::default());
        let mut layout = |text: &str| {
            fonts.with_pixels_per_point(1.0).layout_no_wrap(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
            )
        };
        let text = "عدد (123) هنا";
        let galley = layout(text);
        let row = &galley.rows[0].row;
        assert_eq!(row.text(), text);

        // The number reads left to right, the words and the brackets right to left:
        assert_visual_order(row, &[12, 11, 10, 9, 8, 5, 6, 7, 4, 3, 2, 1, 0]);

        // The shaper mirrors a bracket in right-to-left text,
        // so the logical "(" is drawn as ")" on the right of the number:
        let ltr = layout("(x)");
        let image = fonts.image();
        // Which bracket a bitmap shows, by the side its middle row is inked on.
        // (Comparing bitmaps directly fails on their different sub-pixel offsets.)
        let shape = |glyph: &Glyph| {
            let [x0, y0] = glyph.uv_rect.min.map(usize::from);
            let [x1, y1] = glyph.uv_rect.max.map(usize::from);
            let y = usize::midpoint(y0, y1);
            let ink: Vec<f32> = (x0..x1).map(|x| f32::from(image[(x, y)].a())).collect();
            let weighted: f32 = ink.iter().enumerate().map(|(i, a)| i as f32 * a).sum();
            let centroid = weighted / ink.iter().sum::<f32>();
            if centroid < (ink.len() - 1) as f32 / 2.0 {
                '('
            } else {
                ')'
            }
        };
        assert_eq!(shape(&ltr.rows[0].row.glyphs[0]), '(');
        assert_eq!(shape(&ltr.rows[0].row.glyphs[2]), ')');
        assert_eq!(shape(&row.glyphs[4]), ')');
        assert_eq!(shape(&row.glyphs[8]), '(');
    }

    #[test]
    fn mixed_hebrew_and_latin_in_one_paragraph() {
        let mut fonts = test_fonts();

        // Left-to-right paragraph: only the Hebrew word is reversed.
        let galley = layout_simple(&mut fonts, "Hello שלום world");
        let ltr: Vec<usize> = (0..=5).chain([9, 8, 7, 6]).chain(10..=15).collect();
        assert_visual_order(&galley.rows[0].row, &ltr);

        // Right-to-left paragraph: the words run right to left, the Latin word inside reads left to right.
        let galley = layout_simple(&mut fonts, "שלום Hello עולם");
        let rtl: Vec<usize> = (10..=14).rev().chain(5..=9).chain((0..=4).rev()).collect();
        assert_visual_order(&galley.rows[0].row, &rtl);

        // Each wrapped row is reordered on its own, and keeps its logical chars:
        let width = layout_simple(&mut fonts, "Hello ").size().x;
        let job = LayoutJob::simple(
            "שלום Hello עולם".to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            width + 1.0,
        );
        let galley = layout(&mut fonts, 1.0, Arc::new(job));
        let texts: Vec<String> = galley.rows.iter().map(|row| row.row.text()).collect();
        assert_eq!(texts, ["שלום ", "Hello ", "עולם"]);
        assert_visual_order(&galley.rows[0].row, &[4, 3, 2, 1, 0]);
        // The space after "Hello" is right to left, like the paragraph, so it ends up on the left:
        assert_visual_order(&galley.rows[1].row, &[5, 0, 1, 2, 3, 4]);
        assert_visual_order(&galley.rows[2].row, &[3, 2, 1, 0]);
    }

    /// A click on the left or right half of a glyph puts the cursor on that side of it,
    /// in the glyph's own direction, also where the direction changes.
    #[test]
    fn cursor_hit_testing_across_a_direction_boundary() {
        let mut fonts = test_fonts();
        for text in ["abc אבג def", "אבג abc דהו"] {
            let galley = layout_simple(&mut fonts, text);
            let row = &galley.rows[0].row;
            let y = row.size.y / 2.0;
            for (i, glyph) in row.glyphs.iter().enumerate() {
                let rect = glyph.logical_rect();
                let left = galley.cursor_from_pos(vec2(rect.lerp_inside([0.25, 0.5]).x, y));
                let right = galley.cursor_from_pos(vec2(rect.lerp_inside([0.75, 0.5]).x, y));
                let (before, after) = if glyph.is_rtl() {
                    (right, left)
                } else {
                    (left, right)
                };
                assert_eq!(before.index.0, i, "{text:?}: before {:?}", glyph.chr);
                assert_eq!(after.index.0, i + 1, "{text:?}: after {:?}", glyph.chr);
            }
        }
    }

    /// A selection that is contiguous in logical order can be split on screen.
    #[test]
    fn selection_across_a_direction_boundary() {
        let mut fonts = test_fonts();
        let galley = layout_simple(&mut fonts, "abc אבג def");
        let row = &galley.rows[0].row;
        let span = |chars: Range<usize>| {
            let glyphs = &row.glyphs[chars];
            let min = glyphs.iter().map(|g| g.pos.x).fold(f32::INFINITY, f32::min);
            let max = glyphs
                .iter()
                .map(Glyph::max_x)
                .fold(f32::NEG_INFINITY, f32::max);
            Rangef::new(min, max)
        };
        let ranges =
            |chars: Range<usize>| row.x_ranges(CharIndex(chars.start)..CharIndex(chars.end));

        // "abc א" is "abc " on the left and "א" at the right end of the Hebrew word:
        assert_eq!(ranges(0..5).as_slice(), [span(0..4), span(4..5)]);
        // The Hebrew word alone:
        assert_eq!(ranges(4..7).as_slice(), [span(4..7)]);
        // "בג def" is the left of the Hebrew word, and everything right of it:
        assert_eq!(ranges(5..11).as_slice(), [span(5..7), span(7..11)]);

        // Left-to-right text: one range, from cursor to cursor.
        let galley = layout_simple(&mut fonts, "abc def");
        let row = &galley.rows[0].row;
        assert_eq!(
            row.x_ranges(CharIndex(1)..CharIndex(5)).as_slice(),
            [Rangef::new(
                row.x_offset(CharIndex(1)),
                row.x_offset(CharIndex(5))
            )]
        );
    }

    /// The end of a right-to-left row that ends in left-to-right text is after that text.
    #[test]
    fn rtl_row_ending_in_ltr_text_has_its_end_cursor_after_it() {
        let mut fonts = test_fonts();
        for text in ["ملف PDF", "صفحة 12 من 340"] {
            let galley = layout_simple(&mut fonts, text);
            let row = &galley.rows[0].row;
            let last = row.glyphs.last().unwrap();
            assert!(!last.is_rtl());
            assert_eq!(
                row.x_offset(row.char_count_excluding_newline()),
                last.max_x(),
                "{text:?}"
            );
        }
    }

    /// A right-to-left shaper puts a mark before its base. The base must still be drawn
    /// and carry the char index, and the mark move with it when the row is reordered.
    #[test]
    #[cfg(feature = "default_fonts")]
    fn rtl_mark_stays_on_its_base() {
        // Hack has the combining acute but no Hebrew, so alef is drawn as `.notdef`.
        let mut fonts = FontsImpl::new(TextOptions::default(), hack_only());
        let galley = layout_simple(&mut fonts, "\u{5D0}\u{301}\u{5D1}");
        let row = &galley.rows[0].row;
        let [alef, acute, bet] = [0, 1, 2].map(|i| &row.glyphs[i]);
        assert!(0.0 < alef.advance_width, "alef carries its own index");
        assert!(!alef.uv_rect.is_nothing(), "alef is drawn");
        assert_eq!(acute.advance_width, 0.0);
        assert!(!acute.uv_rect.is_nothing(), "the acute is drawn");
        assert!(bet.pos.x < alef.pos.x, "bet is drawn left of alef");
        let acute_x = acute.pos.x + acute.uv_rect.offset.x;
        assert!(
            alef.pos.x <= acute_x && acute_x <= alef.max_x(),
            "the acute at {acute_x} is over alef at {}..{}",
            alef.pos.x,
            alef.max_x()
        );
    }

    /// A base and a mark that the font draws as two glyphs of one cluster
    /// each keep their own char.
    #[test]
    #[cfg(feature = "default_fonts")]
    fn multi_glyph_cluster_keeps_its_chars() {
        // Hack has the combining acute, and no precomposed form of either cluster.
        let mut fonts = FontsImpl::new(TextOptions::default(), hack_only());
        for text in ["q\u{301}x", "\u{5D0}\u{301}\u{5D1}"] {
            let galley = layout_simple(&mut fonts, text);
            assert_eq!(galley.rows[0].row.text(), text);
        }
    }

    /// Left and Right step over whole grapheme clusters.
    #[test]
    fn cursor_moves_by_grapheme_cluster() {
        let mut fonts = test_fonts();
        let text = "e\u{301}👨\u{200D}👩\u{200D}👧🇸🇪1\u{FE0F}\u{20E3}क्षกี่\u{627}\u{651}";
        let boundaries = [0, 2, 7, 9, 12, 15, 18, 20];
        let galley = layout_simple(&mut fonts, text);

        let mut cursor = galley.begin();
        let mut right = vec![cursor.index.0];
        while cursor != galley.end() {
            cursor = galley.cursor_right_one_character(&cursor);
            right.push(cursor.index.0);
        }
        assert_eq!(right, boundaries);

        let mut left = vec![cursor.index.0];
        while cursor != galley.begin() {
            cursor = galley.cursor_left_one_character(&cursor);
            left.push(cursor.index.0);
        }
        left.reverse();
        assert_eq!(left, boundaries);
    }

    /// Hit testing never puts the cursor inside a grapheme cluster,
    /// in left-to-right and in right-to-left rows.
    #[test]
    fn hit_testing_lands_on_cluster_boundaries() {
        let mut font_sets = vec![test_fonts()];
        #[cfg(feature = "default_fonts")]
        font_sets.push(FontsImpl::new(TextOptions::default(), hack_only()));

        let texts = [
            "\u{628}\u{651}\u{627}\u{64B} \u{5D0}\u{301}\u{5D1}\u{302}\u{303}", // right-to-left
            "\u{5D0}\u{301} q\u{301}e\u{301} 🇸🇪 \u{5D1}\u{302}",                // mixed directions
            "q\u{301}e\u{301}🇸🇪1\u{FE0F}\u{20E3}कि",                            // left-to-right
        ];
        for fonts in &mut font_sets {
            for text in texts {
                let galley = layout_simple(fonts, text);
                let row = &galley.rows[0];
                let mut x = row.rect().left() - 5.0;
                while x < row.rect().right() + 5.0 {
                    let cursor = galley.cursor_from_pos(vec2(x, row.rect().center().y));
                    assert!(
                        grapheme::cluster_around(text, cursor.index).is_none(),
                        "{text:?}: the cursor at x = {x} is at {}, inside a cluster",
                        cursor.index
                    );
                    x += 0.25;
                }
            }
        }
    }

    /// Glyph and row positions of left-to-right text: no wrapping, centered and justified.
    #[test]
    #[cfg(feature = "default_fonts")]
    fn ltr_layout_is_unchanged_by_bidi() {
        let mut fonts = test_fonts();
        let text = "Hello, wörld! fi ffi 123 (x) — ok\tdone";
        let mut lines = Vec::new();
        for (max_width, halign, justify) in [
            (f32::INFINITY, Align::LEFT, false),
            (80.0, Align::Center, false),
            (80.0, Align::LEFT, true),
        ] {
            let mut job = LayoutJob::simple(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                max_width,
            );
            job.halign = halign;
            job.justify = justify;
            let galley = layout(&mut fonts, 1.0, Arc::new(job));
            for placed_row in &galley.rows {
                assert!(placed_row.row.glyphs.iter().all(|g| g.bidi_level == 0));
                let xs: Vec<String> = placed_row
                    .row
                    .glyphs
                    .iter()
                    .map(|glyph| format!("{:.2}", glyph.pos.x))
                    .collect();
                lines.push(format!(
                    "{:.2},{:.2} {:.2}x{:.2}: {}",
                    placed_row.pos.x,
                    placed_row.pos.y,
                    placed_row.row.size.x,
                    placed_row.row.size.y,
                    xs.join(" ")
                ));
            }
        }
        // As laid out by egui 0.37.0, before bidi:
        let expected = [
            "0.00,0.00 233.75x16.00: 0.00 9.00 17.00 21.00 24.00 32.00 36.00 39.00 50.00 58.00 63.00 67.00 75.00 79.00 82.00 91.00 90.00 94.00 108.00 108.00 108.00 111.00 119.00 127.00 134.00 138.00 142.00 149.00 153.00 156.00 170.00 174.00 182.00 189.00 201.00 210.00 218.00 226.00",
            "-18.00,0.00 35.44x16.00: 0.00 9.00 17.00 21.00 24.00 32.00 36.00",
            "-35.00,16.00 69.00x16.00: 0.00 11.00 19.00 24.00 28.00 36.00 40.00 43.00 52.00 51.00 55.00 69.00 69.00 69.00",
            "-29.00,32.00 58.94x16.00: 0.00 8.00 16.00 23.00 27.00 31.00 38.00 42.00 45.00 59.00",
            "-30.00,48.00 59.69x16.00: 0.00 8.00 15.00 27.00 36.00 44.00 52.00",
            "0.00,0.00 80.00x16.00: 0.00 18.00 35.00 48.00 60.00 77.00 89.00",
            "0.00,16.00 80.00x16.00: 0.00 11.00 19.00 24.00 28.00 36.00 40.00 49.00 58.00 57.00 66.00 80.00 80.00 80.00",
            "0.00,32.00 80.00x16.00: 0.00 10.00 20.00 29.00 38.00 44.00 53.00 59.00 66.00 82.00",
            "0.00,48.00 59.69x16.00: 0.00 8.00 15.00 27.00 36.00 44.00 52.00",
        ];
        assert_eq!(lines, expected);
    }

    #[cfg(feature = "default_fonts")]
    fn measure_text(
        fonts: &mut FontsImpl,
        text: &str,
        font_id: &FontId,
        pixels_per_point: f32,
    ) -> (f32, usize, Vec<(char, f32)>) {
        let job = LayoutJob::simple(
            text.to_owned(),
            font_id.clone(),
            Color32::WHITE,
            f32::INFINITY,
        );
        let galley = layout(fonts, pixels_per_point, job.into());
        let glyphs = &galley.rows[0].row.glyphs;
        let details: Vec<_> = glyphs.iter().map(|g| (g.chr, g.advance_width)).collect();
        (galley.size().x, glyphs.len(), details)
    }
}
