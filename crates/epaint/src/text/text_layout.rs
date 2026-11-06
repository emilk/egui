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
        unicode::{is_breaking_space, is_cjk, is_cjk_break_allowed, is_combining_mark},
    },
};

use super::{
    ByteRangeExt as _, Galley, Glyph, LayoutJob, LayoutSection, PlacedRow, Row, RowVisuals,
    TextDirection, VariationCoords,
    bidi::segment_into_bidi_runs,
    family::FamilyKey,
    font_face::{FontFace, GlyphInfo, ShapedGlyph},
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

/// One recorded UAX #9 directional run of a [`Paragraph`]: the glyph-index
/// range it occupies in `Paragraph::glyphs` (in emission = visual order) and
/// the job-text-absolute byte range it covers.
///
/// Recorded while emitting a paragraph segment that resolved to more than one
/// directional run or to a right-to-left run, and only when the job can wrap
/// (a finite wrap width) — the spans are consumed only by the wrapping path.
/// Runs accumulate across the `LayoutSection`s a paragraph spans; their
/// `bytes` stay absolute in `LayoutJob::text`, which makes that safe.
#[derive(Clone, Debug)]
struct RecordedBidiRun {
    /// Glyph index range within `Paragraph::glyphs`.
    pub glyphs: Range<usize>,

    /// Byte range within `LayoutJob::text`.
    pub bytes: Range<usize>,

    /// Whether the run resolves to an odd (right-to-left) embedding level.
    pub rtl: bool,
}

/// Temporary storage before line-wrapping.
#[derive(Clone)]
struct Paragraph {
    /// Start of the next glyph to be added. In screen-space / physical pixels.
    pub cursor_x_px: f32,

    /// This is included in case there are no glyphs
    pub section_index_at_start: u32,

    pub glyphs: Vec<Glyph>,

    /// Byte offset of the first char of each glyph's shaping cluster, in
    /// `LayoutJob::text`. Same length as `glyphs` while
    /// [`ShapingContext::track_cluster_bytes`] is set; empty (no allocation)
    /// otherwise.
    pub cluster_byte_starts: Vec<usize>,

    /// The segment's directional runs recorded during emission
    /// ([`RecordedBidiRun`]), in visual order. Empty (no allocation) for
    /// pure-LTR paragraphs and for jobs that cannot wrap.
    pub bidi_runs: Vec<RecordedBidiRun>,

    /// Byte range of the paragraph's characters in `LayoutJob::text`, stamped
    /// during emission (segments are contiguous by the `LayoutSection`
    /// invariant). The logical wrap path resolves UAX #9 over this span, so
    /// row byte ranges always stay inside it.
    pub byte_span: Range<usize>,

    /// In case of an empty paragraph ("\n"), use this as height.
    pub empty_paragraph_height: f32,

    /// The `LayoutSection::leading_space` advance consumed before this
    /// paragraph's first glyph, in logical pixels. Remembered so row
    /// anchoring can keep it in the ink of the row that carries the
    /// paragraph's first glyph.
    pub leading_advance: f32,
}

impl Paragraph {
    pub fn from_section_index(section_index_at_start: u32) -> Self {
        Self {
            cursor_x_px: 0.0,
            section_index_at_start,
            glyphs: vec![],
            cluster_byte_starts: vec![],
            bidi_runs: vec![],
            byte_span: 0..0,
            empty_paragraph_height: 0.0,
            leading_advance: 0.0,
        }
    }

    /// Push a glyph together with the byte offset of its cluster's first char
    /// in `LayoutJob::text`. The offset is only recorded when tracking
    /// (see [`ShapingContext::track_cluster_bytes`]).
    fn push_glyph(&mut self, ctx: &ShapingContext, glyph: Glyph, cluster_start: usize) {
        if ctx.track_cluster_bytes {
            self.cluster_byte_starts.push(cluster_start);
        }
        self.glyphs.push(glyph);
    }
}

/// Resolve the job's base direction once: explicit, else first-strong, else LTR.
fn resolved_direction(job: &LayoutJob) -> TextDirection {
    job.direction
        .unwrap_or_else(|| TextDirection::infer(&job.text).unwrap_or_default())
}

/// Pin the resolved base direction and make `halign` the reading-start anchor:
/// for a right-to-left paragraph the default `Align::LEFT` ("start") anchors the
/// row's RIGHT edge. `Align::Center`/`Align::RIGHT` keep their literal meaning.
/// Idempotent; called before caching/splitting so every consumer sees a coherent job.
pub(crate) fn normalize_reading_direction(job: &mut LayoutJob) {
    let direction = resolved_direction(job);
    job.direction = Some(direction);
    if direction == TextDirection::RightToLeft && job.halign == Align::LEFT {
        job.halign = Align::RIGHT;
    }
}

/// Layout text into a [`Galley`].
///
/// In most cases you should use [`crate::FontsView::layout_job`] instead
/// since that memoizes the input, making subsequent layouting of the same text much faster.
pub(crate) fn layout(fonts: &mut FontsImpl, pixels_per_point: f32, job: Arc<LayoutJob>) -> Galley {
    profiling::function_scope!();

    job.debug_sanity_check();

    let mut job = job;
    normalize_reading_direction(Arc::make_mut(&mut job));

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

    // Per-glyph cluster byte offsets serve two independent consumers, each
    // with its own engagement condition (both only matter when the job can
    // actually wrap — `line_break` and `line_break_logically` are only
    // called for overflowing paragraphs):
    // - the logical wrap path (`line_break_logically`) needs them for
    //   cluster-exact row byte ranges whenever the text can record
    //   directional runs (see `text_may_record_bidi_runs`);
    // - the breaker consult (`word_ranges`) only when someone can use the
    //   breaker: a custom breaker is installed, or the bundled dictionary is
    //   compiled in and the text may contain Southeast-Asian characters.
    //   That condition must stay exactly as it is: widening it would consult
    //   breakers on jobs they were never engaged for, changing break
    //   candidates.
    let wrap_finite = job.effective_wrap_width().is_finite();
    let may_consult_breaker = job.wrap.line_breaker.is_some() || may_have_sea_text(&job.text);
    let track_cluster_bytes =
        wrap_finite && (may_consult_breaker || text_may_record_bidi_runs(&job.text));

    let paragraphs = shape_paragraphs(fonts, pixels_per_point, &job, track_cluster_bytes);

    let point_scale = PointScale::new(pixels_per_point);

    let intrinsic_size = calculate_intrinsic_size(point_scale, &job, &paragraphs);

    // Consulted with the full job text; ranges are byte offsets into it.
    // Gated on the breaker condition only — NOT on the wider cluster
    // tracking — so the breaker's reach is unchanged.
    let word_ranges = if wrap_finite && may_consult_breaker {
        let breaker = job
            .wrap
            .line_breaker
            .clone()
            .unwrap_or_else(crate::text::line_breaker::default_breaker);
        breaker.word_boundaries(&job.text)
    } else {
        Vec::new()
    };

    let mut elided = false;
    let mut rows = rows_from_paragraphs(
        paragraphs,
        &job,
        pixels_per_point,
        &word_ranges,
        &mut elided,
    );
    if elided && let Some(last_placed) = rows.last_mut() {
        let last_row = Arc::make_mut(&mut last_placed.row);
        if resolved_direction(&job) == TextDirection::RightToLeft {
            replace_first_glyph_with_overflow_character(fonts, pixels_per_point, &job, last_row);
        } else {
            replace_last_glyph_with_overflow_character(fonts, pixels_per_point, &job, last_row);
        }
        if let Some(last) = last_row.glyphs.last() {
            last_row.size.x = last.max_x();
        }
    }

    let justify = job.justify && job.wrap.max_width.is_finite();

    if justify || job.halign != Align::LEFT {
        let num_rows = rows.len();
        for (i, placed_row) in rows.iter_mut().enumerate() {
            let is_last_row = i + 1 == num_rows;
            let justify_row = justify && !placed_row.ends_with_newline && !is_last_row;
            halign_and_justify_row(
                point_scale,
                placed_row,
                job.halign,
                job.wrap.max_width,
                justify_row,
                job.keep_trailing_whitespace,
            );
        }
    }

    // Calculate the Y positions and tessellate the text:
    galley_from_rows(point_scale, job, rows, elided, intrinsic_size)
}

/// Cheap pre-filter for the `sea_dictionary` default breaker: every
/// Southeast-Asian script block is above U+0800, so any candidate char's
/// UTF-8 encoding starts with a byte >= 0xE0. Constant-folded to `false`
/// without the feature.
fn may_have_sea_text(text: &str) -> bool {
    if cfg!(feature = "sea_dictionary") {
        text.bytes().any(|b| b >= 0xE0)
    } else {
        let _ = text;
        false
    }
}

/// Cheap pre-filter for the logical wrap path's cluster tracking: whether the
/// text contains any character that can make a segment record a directional
/// run ([`RecordedBidiRun`]) — a right-to-left character (the two blocks
/// below cover the R/AL/AN bidi classes), or a mark that can split a segment
/// into multiple runs (the RLM, the isolates, and the explicit
/// embedding/override marks — `RLE`/`RLO` force a right-to-left run even in
/// a left-to-right-base paragraph).
///
/// A single pass, same style as [`may_have_sea_text`]. This is deliberately
/// the common-case subset: a false negative (a rare modern RTL block the
/// scan misses — Adlam, Old Hungarian, Manichaean, Hanifi Rohingya, Yezidi)
/// degrades gracefully — such a paragraph wraps via the `Σ chr.len_utf8()`
/// attribution fallback and `place_row`'s guard, never a panic — and
/// extending the list is a one-line change with no safety impact.
fn text_may_record_bidi_runs(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(
            c,
            '\u{0590}'..='\u{08FF}'   // Hebrew, Arabic, Syriac, Thaana, N'Ko, Samaritan, …
            | '\u{200F}'              // RLM: right-to-left mark
            | '\u{202A}'..='\u{202E}' // LRE/RLE/PDF/LRO/RLO: embeddings and overrides
            | '\u{2066}'..='\u{2069}' // LRI/RLI/FSI/PDI: isolates
            | '\u{FB1D}'..='\u{FDFF}' // Hebrew presentation forms
            | '\u{FE70}'..='\u{FEFC}' // Arabic presentation forms
        )
    })
}

/// Shape the job's sections into [`Paragraph`]s (one per newline-split
/// segment), recording each qualifying segment's directional runs
/// ([`RecordedBidiRun`]) along the way.
fn shape_paragraphs(
    fonts: &mut FontsImpl,
    pixels_per_point: f32,
    job: &LayoutJob,
    track_cluster_bytes: bool,
) -> Vec<Paragraph> {
    let mut paragraphs = vec![Paragraph::from_section_index(0)];
    let mut shape_buffer = fonts.take_shape_buffer();
    for (section_index, section) in job.sections.iter().enumerate() {
        shape_buffer = layout_section(
            fonts,
            shape_buffer,
            pixels_per_point,
            job,
            track_cluster_bytes,
            section_index as u32,
            section,
            &mut paragraphs,
        );
    }
    fonts.return_shape_buffer(shape_buffer);
    paragraphs
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
    prev_cluster: Option<u32>,

    /// Record per-glyph cluster byte offsets (see `Paragraph`).
    track_cluster_bytes: bool,

    /// Byte offset of the current newline-split segment in `LayoutJob::text`.
    segment_base: usize,

    /// The UAX #9 embedding level of the bidi run being laid out (odd = RTL).
    /// Stamped onto every glyph the context creates, so continuation glyphs
    /// carry their cluster's level too.
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

    /// Set if a priority [`GlyphRasterizer`](crate::text::GlyphRasterizer) rendered this run.
    ///
    /// Such a run is exactly one grapheme cluster, and is not shaped.
    raster: Option<RasterGlyphAllocation>,
}

/// Emit shaped glyphs from a [`harfrust::GlyphBuffer`] into a [`Paragraph`].
///
/// When a cluster maps multiple characters to fewer glyphs (e.g. flag emojis,
/// ligatures), zero-width "continuation" glyphs are emitted for the extra
/// characters so that `glyphs.len() == char_count` — an invariant that all
/// cursor and selection code relies on.
fn layout_shaped_run(
    fonts: &mut FontsImpl,
    run: &TextRun,
    run_text: &str,
    glyph_buffer: &harfrust::GlyphBuffer,
    face_metrics: &StyledMetrics,
    ctx: &mut ShapingContext,
    paragraph: &mut Paragraph,
) {
    let px_scale = face_metrics.px_scale_factor;

    // Reset cluster tracking — cluster values are byte offsets within run_text,
    // so they are not comparable across runs.
    ctx.prev_cluster = None;

    // Track how many glyphs we emit per cluster so we can add zero-width
    // continuation glyphs when a cluster has more chars than glyphs. The byte
    // range of the outgoing cluster is derived from its neighbouring cluster
    // starts: for an ascending run the next cluster starts right after it,
    // while a right-to-left run emits its clusters in *descending* order, so
    // the outgoing cluster ends where the previous one began (the next larger
    // cluster start) — or, for the run's first transition, at the end of the
    // run text (there is no earlier cluster; the outgoing one is the logically
    // last). `previous_cluster_start` is therefore seeded only once a cluster
    // has actually been processed — never from the dummy `cluster_start_byte`
    // the run starts with:
    let mut cluster_start_byte: usize = 0;
    let mut previous_cluster_start: Option<usize> = None;
    let mut cluster_glyph_count: usize = 0;

    for (info, pos) in iter::zip(glyph_buffer.glyph_infos(), glyph_buffer.glyph_positions()) {
        let glyph_id = skrifa::GlyphId::new(info.glyph_id);
        let cluster = info.cluster;
        let mut advance_width_px = pos.x_advance as f32 * px_scale;
        let x_offset_px = pos.x_offset as f32 * px_scale;
        let y_offset_px = -(pos.y_offset as f32 * px_scale); // harfrust Y+ up → screen Y+ down

        let chr = run_text
            .get(cluster as usize..)
            .and_then(|s| s.chars().next())
            .unwrap_or('\u{FFFD}'); // Unicode Replacement Character

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

        // Apply extra_letter_spacing only at cluster boundaries,
        // never between glyphs within the same cluster (e.g. base + mark).
        let is_new_cluster = ctx.prev_cluster.is_none_or(|pc| pc != cluster);
        if is_new_cluster {
            if ctx.prev_cluster.is_some() {
                let outgoing_end = if cluster_start_byte < cluster as usize {
                    // Ascending clusters (a left-to-right run): the outgoing
                    // cluster ends where this one starts:
                    cluster as usize
                } else {
                    // Descending clusters (a right-to-left run): the outgoing
                    // cluster ends where the previous one began — the next
                    // larger cluster start:
                    previous_cluster_start.unwrap_or(run_text.len())
                };
                emit_continuation_glyphs(
                    ctx,
                    paragraph,
                    run_text,
                    cluster_start_byte..outgoing_end,
                    cluster_glyph_count,
                    face_metrics,
                    ctx.segment_base + run.byte_range.as_usize().start + cluster_start_byte,
                );
                // The outgoing cluster was really processed, so it may seed the
                // "previous cluster" for the next transition:
                previous_cluster_start = Some(cluster_start_byte);
            }
            if !ctx.is_first_glyph_in_section {
                paragraph.cursor_x_px += ctx.extra_letter_spacing * ctx.pixels_per_point;
            }
            cluster_start_byte = cluster as usize;
            cluster_glyph_count = 0;
            ctx.is_first_glyph_in_section = false;
        }
        ctx.prev_cluster = Some(cluster);

        let glyph = if glyph_id == skrifa::GlyphId::NOTDEF {
            // The shaper couldn't map this character. Drop combining marks and duplicate
            // NOTDEF glyphs within the same cluster. The first base character is rasterized,
            // or rendered as the `.notdef` glyph ("tofu") if no rasterizer can handle the cluster.
            if is_combining_mark(chr) || !is_new_cluster {
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
        // All glyphs of one cluster carry the cluster's start (see the
        // continuation glyphs below): a word boundary is then applied after
        // the whole cluster, so a cluster is never visually cut.
        paragraph.push_glyph(
            ctx,
            glyph,
            ctx.segment_base + run.byte_range.as_usize().start + cluster as usize,
        );
        cluster_glyph_count += 1;
    }

    // Emit continuation glyphs for the last cluster in the run. Its byte range
    // ends where the next larger cluster start begins (a right-to-left run
    // emits descending, so that is the previous cluster start); an ascending
    // run's last cluster runs to the end of the run text.
    if ctx.prev_cluster.is_some() {
        let last_end = match previous_cluster_start {
            Some(previous) if cluster_start_byte < previous => previous,
            _ => run_text.len(),
        };
        emit_continuation_glyphs(
            ctx,
            paragraph,
            run_text,
            cluster_start_byte..last_end,
            cluster_glyph_count,
            face_metrics,
            ctx.segment_base + run.byte_range.as_usize().start + cluster_start_byte,
        );
    }
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
    cluster_start: usize,
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
    paragraph.push_glyph(ctx, glyph, cluster_start);
    emit_continuation_glyphs(
        ctx,
        paragraph,
        run_text,
        0..run_text.len(),
        1,
        &face_metrics,
        cluster_start,
    );
}

/// Emit zero-width continuation glyphs when a cluster has more characters than
/// shaped glyphs.
///
/// This preserves the invariant `glyphs.len() == char_count` that all cursor
/// and text-selection code depends on. Continuation glyphs have
/// [`GlyphAllocation::default()`] so [`tessellate_glyphs`] skips them entirely.
///
/// Continuation glyphs belong to the same cluster as the glyphs before them,
/// so they carry its `cluster_start` too: a word boundary that would fall
/// between two glyphs of one cluster is applied after the whole cluster.
fn emit_continuation_glyphs(
    ctx: &ShapingContext,
    paragraph: &mut Paragraph,
    run_text: &str,
    cluster_bytes: Range<usize>,
    cluster_glyph_count: usize,
    face_metrics: &StyledMetrics,
    cluster_start: usize,
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
        paragraph.push_glyph(
            ctx,
            ctx.glyph(
                chr,
                physical_x,
                0.0,
                face_metrics,
                GlyphAllocation::default(),
            ),
            cluster_start,
        );
    }
}

// Ignores the Y coordinate.
#[must_use]
#[expect(clippy::too_many_arguments)]
fn layout_section(
    fonts: &mut FontsImpl,
    mut shape_buffer: harfrust::UnicodeBuffer,
    pixels_per_point: f32,
    job: &LayoutJob,
    track_cluster_bytes: bool,
    section_index: u32,
    section: &LayoutSection,
    out_paragraphs: &mut Vec<Paragraph>,
) -> harfrust::UnicodeBuffer {
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
        // The advance precedes this paragraph's first glyph (a section that
        // starts here): remember it so row anchoring can keep it in the ink.
        paragraph.leading_advance += leading_space;
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
        prev_cluster: None,
        track_cluster_bytes,
        segment_base: byte_range.as_usize().start,
        bidi_level: 0,
    };
    let mut runs = Vec::new();
    let mut bidi_runs = Vec::new();

    // The paragraph's base embedding level (UAX #9 P3): the job's resolved direction.
    let base_level = if resolved_direction(job) == TextDirection::RightToLeft {
        unicode_bidi::Level::rtl()
    } else {
        unicode_bidi::Level::ltr()
    };

    // Process each paragraph segment (split on newlines — the shaper can't handle them).
    // Directional runs are recorded for the wrapping path only when the job can
    // actually wrap (see `RecordedBidiRun`): an infinite wrap width can never
    // overflow, so its spans would never be consumed.
    let record_bidi_runs = job.effective_wrap_width().is_finite();
    let section_base = byte_range.as_usize().start;
    let mut segment_offset = 0usize;
    for (seg_idx, segment) in SplitOrWhole::new(section_text, job.break_on_newline).enumerate() {
        let segment_base = section_base + segment_offset;
        // The `+ 1` accounts for the `'\n'` after every yielded segment,
        // including the empty ones skipped below (consecutive newlines).
        segment_offset += segment.len() + 1;
        ctx.segment_base = segment_base;

        if 0 < seg_idx {
            paragraph = out_paragraphs.push_mut(Paragraph::from_section_index(section_index));
            paragraph.empty_paragraph_height = line_height;
            ctx.is_first_glyph_in_section = true;
        }

        if segment.is_empty() {
            continue;
        }

        // Stamp the segment's bytes onto its paragraph (contiguous by the
        // `LayoutSection` invariant; see `Paragraph::byte_span`):
        let segment_end = segment_base + segment.len();
        if paragraph.byte_span.is_empty() {
            paragraph.byte_span = segment_base..segment_end;
        } else {
            paragraph.byte_span.end = segment_end;
        }

        // UAX #9: split the segment into maximal directional runs and lay them out in
        // visual (left-to-right) order. Each run is shaped in its own direction, so an
        // RTL run's glyphs already come out of the shaper in visual order; within an
        // RTL run the font runs are therefore emitted in reverse (logical) order too.
        //
        // Wrapping needs the *logical* stream, so the runs are also recorded on
        // the paragraph (see `RecordedBidiRun`) and the wrap path
        // (`line_break_logically`) reorders each *produced row* per UAX #9
        // L1/L2, resolving the levels over the paragraph's whole byte span
        // (which can merge runs across a section boundary where the
        // per-segment emission split them — more UAX #9-conformant). The
        // earlier KNOWN LIMITATION — wrapped RTL text rendering on the wrong
        // rows — is fixed by that path; what remains approximate is mapping a
        // logical cursor column INSIDE a run that counteracts the base
        // direction (e.g. an embedded left-to-right word in a right-to-left
        // row), which needs a logical↔visual run map stored on the row (see
        // `Row::glyphs`). Single-row text (no wrap) is unaffected: it takes
        // the early-out and keeps this emission order as-is.
        segment_into_bidi_runs(segment, base_level, &mut bidi_runs);

        // Record only segments that resolved to more than one run or to a
        // right-to-left run: a single even-level run (pure LTR, or an
        // explicitly RTL-pinned base with no RTL characters — its double
        // reversal is the identity) emits its glyphs in logical order already
        // and needs no per-row reordering.
        let record_this_segment = record_bidi_runs
            && (bidi_runs.len() > 1 || bidi_runs.first().is_some_and(|run| run.is_rtl()));

        for bidi_run in &bidi_runs {
            let Some(bidi_text) = segment.get(bidi_run.byte_range.clone()) else {
                continue;
            };

            // First glyph the run will emit (its span in `Paragraph::glyphs`,
            // recorded below if this segment qualifies):
            let run_first_glyph = paragraph.glyphs.len();

            // Font runs are segmented per bidi run on the run's own substring, so
            // `run.byte_range` below is relative to `bidi_text`. Bias the context's
            // segment base by the bidi run's start: every cluster offset computed
            // from it (`segment_base + run start + cluster`) then lands on the
            // right byte in `LayoutJob::text`.
            ctx.segment_base = segment_base + bidi_run.byte_range.start;

            // Every glyph of this bidi run carries the run's embedding level
            // (odd = RTL), so consumers can split the visual glyph stream back
            // into directional runs:
            ctx.bidi_level = bidi_run.level.number();

            segment_into_runs(fonts, &ctx, bidi_text, &mut runs);

            let num_runs = runs.len();

            // Within an RTL run the font runs are emitted in reverse (logical) order,
            // because each run's shaper output is already in visual order.
            for i in 0..num_runs {
                let run_idx = if bidi_run.is_rtl() {
                    num_runs - 1 - i
                } else {
                    i
                };
                let Some(run) = runs.get(run_idx) else {
                    continue;
                };
                let run_text = &bidi_text[run.byte_range.as_usize()];
                if let Some(raster) = &run.raster {
                    let cluster_start = ctx.segment_base + run.byte_range.as_usize().start;
                    layout_raster_run(&mut ctx, paragraph, run_text, raster, cluster_start);
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

                // Set buffer flags for text-boundary context. These follow the
                // *logical* position of the font run within the bidi run (each bidi
                // run is shaped in isolation; the emission order above is the visual
                // order, not the logical one).
                let mut flags = harfrust::BufferFlags::empty();
                if run_idx == 0 {
                    flags |= harfrust::BufferFlags::BEGINNING_OF_TEXT;
                }
                if run_idx + 1 == num_runs {
                    flags |= harfrust::BufferFlags::END_OF_TEXT;
                }

                let direction = if bidi_run.is_rtl() {
                    harfrust::Direction::RightToLeft
                } else {
                    harfrust::Direction::LeftToRight
                };

                let glyph_buffer = shape_text(
                    font_face,
                    run_text,
                    &format.coords,
                    shape_buffer,
                    flags,
                    direction,
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

            // Record the run's span (glyph indices in emission = visual order,
            // bytes absolute in `LayoutJob::text`). Runs that emitted no glyphs
            // are not recorded, so recorded glyph ranges stay contiguous.
            if record_this_segment && paragraph.glyphs.len() > run_first_glyph {
                paragraph.bidi_runs.push(RecordedBidiRun {
                    glyphs: run_first_glyph..paragraph.glyphs.len(),
                    bytes: segment_base + bidi_run.byte_range.start
                        ..segment_base + bidi_run.byte_range.end,
                    rtl: bidi_run.is_rtl(),
                });
            }
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
    word_ranges: &[Range<usize>],
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
                    base_direction: resolved_direction(job),
                    leading_advance: 0.0,
                    glyphs: vec![],
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
                        base_direction: resolved_direction(job),
                        leading_advance: paragraph.leading_advance,
                        glyphs: paragraph.glyphs,
                        visuals: Default::default(),
                        size: vec2(paragraph_width, 0.0),
                    }),
                    ends_with_newline: !is_last_paragraph,
                });
            } else {
                if wraps_logically(&paragraph) {
                    // The KNOWN LIMITATION's fix: wrap the logical stream and
                    // reorder each produced row per UAX #9 L1/L2.
                    line_break_logically(&paragraph, job, word_ranges, &mut rows, elided);
                } else {
                    line_break(&paragraph, job, word_ranges, &mut rows, elided);
                }
                if let Some(placed_row) = rows.last_mut() {
                    placed_row.ends_with_newline = !is_last_paragraph;
                }
            }
        }
    }

    rows
}

fn line_break(
    paragraph: &Paragraph,
    job: &LayoutJob,
    word_ranges: &[Range<usize>],
    out_rows: &mut Vec<PlacedRow>,
    elided: &mut bool,
) {
    let wrap_width = job.effective_wrap_width();

    // Keeps track of good places to insert row break if we exceed `wrap_width`.
    let mut row_break_candidates = RowBreakCandidates::default();

    // Index into `word_ranges`: the next dictionary-word end not yet registered.
    let mut word_idx = 0;

    let mut first_row_indentation = paragraph.glyphs[0].pos.x;
    let mut row_start_x = 0.0;
    let mut row_start_idx = 0;

    // The `leading_space` advance the paragraph's first glyph carries: the row
    // holding that glyph keeps it (its glyphs carry it in their positions), so
    // row anchoring can re-inset it at the reading start; every later row has
    // it re-based out of its glyphs and gets 0.0.
    let mut leading_advance = paragraph.leading_advance;

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
                        base_direction: resolved_direction(job),
                        leading_advance: 0.0,
                        glyphs: vec![],
                        visuals: Default::default(),
                        size: Vec2::ZERO,
                    }),
                    ends_with_newline: false,
                });
                row_start_x += first_row_indentation;
                first_row_indentation = 0.0;
                // The advance was consumed with the indent (via `row_start_x`),
                // so no row keeps it:
                leading_advance = 0.0;
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
                        base_direction: resolved_direction(job),
                        leading_advance,
                        glyphs,
                        visuals: Default::default(),
                        size: vec2(paragraph_max_x, 0.0),
                    }),
                    ends_with_newline: false,
                });

                // Start a new row:
                row_start_idx = last_kept_index + 1;
                row_start_x = paragraph.glyphs[row_start_idx].pos.x;
                row_break_candidates.forget_before_idx(row_start_idx);
                // The advance rode the row that carried the paragraph's first
                // glyph; later rows' glyphs are re-based without it:
                leading_advance = 0.0;
            } else {
                // Found no place to break, so we have to overrun wrap_width.
            }
        }

        row_break_candidates.add(i, &paragraph.glyphs[i..]);

        // Translate breaker byte ranges into glyph indices with an ascending
        // pointer walk: no slicing by breaker-provided offsets, so hostile
        // ranges can degrade break quality but never panic.
        if !word_ranges.is_empty() {
            debug_assert_eq!(
                paragraph.cluster_byte_starts.len(),
                paragraph.glyphs.len(),
                "cluster tracking must cover every glyph when a breaker is engaged"
            );
            // `word_ranges` is only non-empty when cluster tracking was on, so
            // the indexed vector has an entry per glyph (guarded by the assert
            // above).
            while let Some(word_end) = word_ranges.get(word_idx).map(|word_range| word_range.end) {
                let this_start = paragraph.cluster_byte_starts[i];
                if this_start >= word_end {
                    // The word ended before this glyph (its last glyph may have
                    // been dropped by the shaper); move on to the next word.
                    word_idx += 1;
                    continue;
                }
                let next_start = paragraph.cluster_byte_starts.get(i + 1).copied();
                if next_start.is_none_or(|next| next >= word_end) {
                    // This glyph is the last one whose cluster starts inside
                    // the word: register "may break after it" and advance.
                    row_break_candidates.word = Some(i);
                    word_idx += 1;
                }
                break;
            }
        }
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
                    base_direction: resolved_direction(job),
                    leading_advance,
                    glyphs,
                    visuals: Default::default(),
                    size: vec2(paragraph_max_x - paragraph_min_x, 0.0),
                }),
                ends_with_newline: false,
            });
            // This is the last row: for the single-row overrun the advance has
            // moved from the glyphs into `paragraph_min_x`/`pos.x`, and the row
            // records it so row anchoring can re-inset it at the reading start.
        }
    }
}

// ----------------------------------------------------------------------------

/// One logical block of a paragraph's glyph stream: a base glyph plus the
/// zero-advance glyphs of its cluster (the same grouping the right-to-left
/// cursor mapping walks), with the byte range its characters cover and the
/// geometry needed to stack it.
///
/// Blocks are the unit the logical-stream wrap path partitions into rows: a
/// block is never split across rows, so a cluster is never visually cut.
struct LogicalBlock {
    /// Glyph index range within `Paragraph::glyphs`, in emitted (visual)
    /// stream order.
    glyphs: Range<usize>,

    /// Byte range within `LayoutJob::text` in LOGICAL character order.
    /// Cluster-exact while cluster tracking is on (see `logical_blocks`):
    /// the start is the block's leading glyph's cluster start and the end
    /// the next strictly greater distinct cluster start, or the paragraph
    /// span's end. Without tracking (a text the tracking scan missed) it
    /// falls back to summing the glyphs' `chr.len_utf8()`, which drifts on
    /// decomposed glyphs; `place_row`'s guard keeps the rows of such a
    /// paragraph.
    bytes: Range<usize>,

    /// x of the block's first (base) glyph in the emitted stream.
    origin: f32,

    /// The block's stacking width: the distance to the next block's origin in
    /// the emitted stream (the paragraph's ink end for the last one), clamped
    /// at zero for hostile geometry.
    width: f32,
}

/// The paragraph's glyphs as logical blocks, in logical order.
///
/// The block inventory covers ALL of the paragraph's glyphs: the recorded
/// directional runs ([`Paragraph::bidi_runs`], which carry exact byte
/// ranges) interleaved with the complement glyph intervals — whole segments
/// that resolved to a single even-level run and were therefore not recorded
/// (a qualifying segment records ALL of its runs, so a complement interval
/// never splits one). Both the complement glyph intervals and the
/// corresponding byte gaps (the complement of the recorded byte ranges in
/// the paragraph's span) are naturally ordered — glyph intervals in
/// emission (= section/segment) order, byte gaps in byte order — and under
/// the `glyphs.len() == char_count` invariant they match one to one, so the
/// k-th complement interval fills the k-th byte gap. An unrecorded stretch
/// is a single even-level run: its stream order IS its logical order, so
/// its blocks iterate in stream order; a recorded right-to-left run's
/// blocks iterate in reverse stream order. The stretches are then sorted by
/// byte start, which IS the logical order. While cluster tracking is on,
/// byte ranges are cluster-exact (see the assembly below); without it they
/// are assigned cumulatively from each stretch's own byte start by summing
/// the glyphs' `chr.len_utf8()` — a fallback that drifts on decomposed
/// glyphs.
fn logical_blocks(paragraph: &Paragraph) -> Vec<LogicalBlock> {
    let glyphs = &paragraph.glyphs;

    // Stream blocks paragraph-wide: a base glyph plus its trailing
    // zero-advance glyphs. Widths are measured in stream order (to the next
    // block's origin, across stretch boundaries — the emitted stream is one
    // ascending x sequence).
    let mut stream_blocks: Vec<(Range<usize>, f32)> = Vec::new();
    let mut i = 0;
    while i < glyphs.len() {
        let mut end = i + 1;
        while glyphs
            .get(end)
            .is_some_and(|glyph| glyph.advance_width <= 0.0)
        {
            end += 1;
        }
        stream_blocks.push((i..end, glyphs[i].pos.x));
        i = end;
    }
    let ink_end = glyphs.last().map_or(0.0, |glyph| glyph.max_x());
    // Glyph index -> stream block index (blocks are stream-contiguous):
    let mut block_of_glyph = vec![0usize; glyphs.len()];
    for (block_idx, (glyph_range, _)) in stream_blocks.iter().enumerate() {
        for g in glyph_range.clone() {
            block_of_glyph[g] = block_idx;
        }
    }

    // The byte gaps between the recorded runs (sorted by byte start), inside
    // the paragraph's span:
    let mut sorted_bytes: Vec<Range<usize>> = paragraph
        .bidi_runs
        .iter()
        .map(|run| run.bytes.clone())
        .collect();
    sorted_bytes.sort_by_key(|bytes| bytes.start);
    let mut byte_gaps: Vec<Range<usize>> = Vec::new();
    let mut byte_cursor = paragraph.byte_span.start;
    for bytes in &sorted_bytes {
        if bytes.start > byte_cursor {
            byte_gaps.push(byte_cursor..bytes.start);
        }
        byte_cursor = byte_cursor.max(bytes.end);
    }
    if byte_cursor < paragraph.byte_span.end {
        byte_gaps.push(byte_cursor..paragraph.byte_span.end);
    }

    // The complement glyph intervals (whole unrecorded segments):
    let mut glyph_gaps: Vec<Range<usize>> = Vec::new();
    let mut next_glyph = 0usize;
    for run in &paragraph.bidi_runs {
        if run.glyphs.start > next_glyph {
            glyph_gaps.push(next_glyph..run.glyphs.start);
        }
        next_glyph = run.glyphs.end;
    }
    if next_glyph < glyphs.len() {
        glyph_gaps.push(next_glyph..glyphs.len());
    }

    // The stretches: (glyph range, byte range, rtl). Hostile shapes can
    // desync the two gap lists; an unpaired glyph interval gets a degenerate
    // byte range and its row's byte ranges simply clamp downstream.
    let mut stretches: Vec<(Range<usize>, Range<usize>, bool)> = Vec::new();
    for run in &paragraph.bidi_runs {
        stretches.push((run.glyphs.clone(), run.bytes.clone(), run.rtl));
    }
    for (k, glyph_gap) in glyph_gaps.iter().enumerate() {
        let byte_gap = byte_gaps
            .get(k)
            .cloned()
            .unwrap_or(paragraph.byte_span.start..paragraph.byte_span.start);
        stretches.push((glyph_gap.clone(), byte_gap, false));
    }
    // The logical order is the byte order:
    stretches.sort_by_key(|(_, bytes, _)| bytes.start);

    // Assemble the blocks stretch by stretch, in logical order.
    //
    // Byte attribution is cluster-exact while cluster tracking is on (the
    // HarfBuzz cluster model: a cluster is a single, indivisible unit, and
    // its byte range comes from cluster boundaries — shaping may split one
    // character into several glyphs, e.g. decomposing a code point through
    // the ccmp feature, or merge/delete code points, leaving gapped cluster
    // values — never from per-glyph first-char sums;
    // https://harfbuzz.github.io/clusters.html ,
    // https://harfbuzz.github.io/working-with-harfbuzz-clusters.html):
    // - a block's `bytes.start` is its leading glyph's cluster start
    //   (absolute), clamped into the paragraph span;
    // - its `bytes.end` is the first strictly greater distinct cluster
    //   start found scanning forward over the subsequent blocks' leading
    //   glyphs, else the span's end. The blocks vector is in logical order
    //   (within an RTL stretch `order` is reversed, so cluster starts
    //   ascend — final visual-order buffers are monotonic, LTR ascending /
    //   RTL descending, and the reversal restores ascending starts), so the
    //   scan is a plain forward walk.
    //
    // Invariants (pinned by the `logical_blocks_byte_ranges_*` tests):
    // every block's `bytes ⊆ paragraph.byte_span`; starts are non-decreasing
    // in blocks-vector order; every well-formed block's range is non-empty;
    // the union of the block ranges covers `paragraph.byte_span`; a row's
    // byte range spans from its first block's `bytes.start` to its last
    // block's `bytes.end`. NOT pinned (it fails on a decomposition dot
    // absorbed into the neighbouring block): each glyph's cluster start
    // lying inside its own block's range.
    let cluster_starts: Option<&[usize]> = if paragraph.cluster_byte_starts.len() == glyphs.len() {
        Some(&paragraph.cluster_byte_starts)
    } else {
        None
    };
    let mut blocks: Vec<LogicalBlock> = Vec::with_capacity(stream_blocks.len());
    for (stretch_glyphs, stretch_bytes, rtl) in stretches {
        if stretch_glyphs.is_empty() {
            continue;
        }
        let first_block = block_of_glyph[stretch_glyphs.start];
        let last_block = block_of_glyph[stretch_glyphs.end - 1];
        let mut order: Vec<usize> = (first_block..=last_block).collect();
        if rtl {
            order.reverse();
        }
        let mut byte_cursor = stretch_bytes.start;
        for stream_idx in order {
            let Some((glyph_range, origin)) = stream_blocks.get(stream_idx) else {
                continue;
            };
            let width = match stream_blocks.get(stream_idx + 1) {
                Some((_, next_origin)) => (*next_origin - *origin).max(0.0),
                None => (ink_end - *origin).max(0.0),
            };
            let bytes = if let Some(cluster_starts) = cluster_starts {
                // The end is derived in a second pass below, over the
                // finished blocks vector.
                let start = cluster_starts[glyph_range.start]
                    .clamp(paragraph.byte_span.start, paragraph.byte_span.end);
                start..start
            } else {
                // Fallback (unreachable in the logical wrap path — see the
                // `records-runs-implies-tracked` assert in
                // `line_break_logically`): the historic cumulative walk,
                // which counts a decomposed char once per emitted glyph.
                let byte_len: usize = glyphs[glyph_range.clone()]
                    .iter()
                    .map(|glyph| glyph.chr.len_utf8())
                    .sum();
                let bytes = byte_cursor..byte_cursor + byte_len;
                byte_cursor += byte_len;
                bytes
            };
            blocks.push(LogicalBlock {
                bytes,
                glyphs: glyph_range.clone(),
                origin: *origin,
                width,
            });
        }
    }
    if cluster_starts.is_some() {
        for i in 0..blocks.len() {
            let start = blocks[i].bytes.start;
            let end = blocks[i + 1..]
                .iter()
                .map(|block| block.bytes.start)
                .find(|&next_start| next_start > start)
                .unwrap_or(paragraph.byte_span.end)
                .min(paragraph.byte_span.end)
                .max(start);
            blocks[i].bytes.end = end;
        }
    }
    blocks
}

/// Whether this overflowing paragraph takes the logical-stream wrap path: any
/// recorded directional run (a segment that resolved to more than one run or
/// to a right-to-left run). Pure-LTR paragraphs record nothing and never take
/// the path — every LTR galley keeps its exact pre-change code path.
fn wraps_logically(paragraph: &Paragraph) -> bool {
    !paragraph.bidi_runs.is_empty()
}

/// Per-row UAX #9 context for the logical wrap path, computed once per
/// overflowing paragraph: the paragraph's whole byte span (a paragraph can
/// span several `LayoutSection`s — contiguous byte ranges by the sections
/// invariant), so row-level runs can merge across a section boundary where
/// the per-segment emission split them.
struct RowReorder<'a> {
    /// The paragraph's byte-span text (empty if the span was hostile).
    text: &'a str,

    /// Byte offset of the span in `LayoutJob::text`.
    span_start: usize,

    /// The job's resolved base level.
    base_level: unicode_bidi::Level,

    /// UAX #9 resolution over the whole span.
    info: unicode_bidi::ParagraphBidiInfo<'a>,
}

impl RowReorder<'_> {
    /// Place one row's blocks in visual order per UAX #9 L1/L2 and push the
    /// row.
    ///
    /// The row's logical blocks ([`RowReorder::place_row`] receives their
    /// index range) cover a contiguous logical byte range; `visual_runs` on
    /// that range applies L1 (scoped to the row — the break candidate
    /// whitespace resets to the base level and renders at the row's visual
    /// reading end) and returns the row's runs in visual order. Blocks are
    /// then re-stacked with a pen: each run's blocks in logical order, or
    /// reversed for a right-to-left run, each glyph keeping its intra-block
    /// offset (marks and ligature geometry stay exact; only whole blocks
    /// move).
    ///
    /// `leading_advance` is the `LayoutSection::leading_space` advance the
    /// paragraph's first glyph carries when this row is the paragraph's
    /// reading-first row (0.0 otherwise): the stacking pen starts at it, so
    /// the advance stays in the row's glyphs and size exactly like on the
    /// legacy wrap path.
    fn place_row(
        &self,
        paragraph: &Paragraph,
        job: &LayoutJob,
        blocks: &[LogicalBlock],
        row_range: Range<usize>,
        leading_advance: f32,
        out_rows: &mut Vec<PlacedRow>,
    ) {
        if row_range.is_empty() {
            return;
        }

        // The row's byte range relative to the span, clamped and snapped to
        // char boundaries. Hostile shapes (a cluster standing in for a
        // replacement char) can drift the `chr.len_utf8()` sums of the
        // fallback attribution; the clamping below then converts that into
        // an empty or end-equal range, which may degrade reorder quality.
        let span_len = self.text.len();
        let mut rel_start = blocks[row_range.start]
            .bytes
            .start
            .saturating_sub(self.span_start)
            .min(span_len);
        let mut rel_end = blocks[row_range.end - 1]
            .bytes
            .end
            .saturating_sub(self.span_start)
            .min(span_len)
            .max(rel_start);
        while rel_start > 0 && !self.text.is_char_boundary(rel_start) {
            rel_start -= 1;
        }
        while rel_end > rel_start && !self.text.is_char_boundary(rel_end) {
            rel_end -= 1;
        }

        // A hostile shape can still leave an empty or end-equal range after
        // clamping and boundary snapping; unicode-bidi indexes
        // `levels[line.start]` unchecked after an `<=` assert, so resolve runs
        // only for a non-empty in-span range. Blocks no run claims are placed in
        // logical order by the fallback below — the row and its glyphs are
        // always kept. (This no-op semantics is the cross-framework convention,
        // not a local workaround: typst guards the same call caller-side with
        // "The bidi crate panics for empty lines" (typst-layout/src/inline/
        // line.rs), and FriBidi's fribidi_reorder_line treats len == 0 as a
        // success no-op — see github.com/fribidi/fribidi lib/fribidi-bidi.c.)
        let (levels, runs) = if rel_start < rel_end && rel_start < span_len {
            self.info.visual_runs(rel_start..rel_end)
        } else {
            (Vec::new(), Vec::new())
        };

        // Place the blocks in the row's visual run order. Blocks no run
        // claims (hostile byte drift, or an empty clamped range) are kept in
        // logical order at the visual end rather than dropped.
        let mut glyphs: Vec<Glyph> = Vec::new();
        let mut claimed = vec![false; row_range.len()];
        let mut pen = leading_advance;
        let place_block = |glyphs: &mut Vec<Glyph>, pen: &mut f32, block: &LogicalBlock| {
            for glyph in &paragraph.glyphs[block.glyphs.clone()] {
                let mut glyph = *glyph;
                glyph.pos.x = *pen + (glyph.pos.x - block.origin);
                glyphs.push(glyph);
            }
            *pen += block.width;
        };
        for run_rel in &runs {
            let run_level = levels
                .get(run_rel.start)
                .copied()
                .unwrap_or(self.base_level);
            let run_start = run_rel.start + self.span_start;
            let run_end = run_rel.end + self.span_start;
            let mut indices: Vec<usize> = row_range
                .clone()
                .filter(|&b| blocks[b].bytes.start >= run_start && blocks[b].bytes.start < run_end)
                .collect();
            if run_level.is_rtl() {
                indices.reverse();
            }
            for b in indices {
                claimed[b - row_range.start] = true;
                place_block(&mut glyphs, &mut pen, &blocks[b]);
            }
        }
        for (offset, b) in row_range.clone().enumerate() {
            if !claimed[offset] {
                place_block(&mut glyphs, &mut pen, &blocks[b]);
            }
        }

        if glyphs.is_empty() {
            return;
        }
        let section_index_at_start = glyphs[0].section_index;
        let paragraph_max_x = glyphs.last().map_or(0.0, |glyph| glyph.max_x());
        out_rows.push(PlacedRow {
            pos: pos2(0.0, f32::NAN),
            row: Arc::new(Row {
                section_index_at_start,
                base_direction: resolved_direction(job),
                leading_advance,
                glyphs,
                visuals: Default::default(),
                size: vec2(paragraph_max_x, 0.0),
            }),
            ends_with_newline: false,
        });
    }
}

/// Wrap an overflowing paragraph that contains right-to-left text on the
/// LOGICAL stream: blocks are walked in reading order, accumulating widths;
/// break candidates are registered after the overflow check at each block,
/// exactly like [`line_break`] registers them per glyph; and each produced
/// row is placed per UAX #9 L1/L2 over the paragraph's whole byte span
/// ([`RowReorder::place_row`]).
///
/// The rows come out in logical (reading) order: a wrapped RTL paragraph
/// reads top-down instead of bottom-up, and mixed-direction rows keep UAX #9
/// L1/L2 per rendered line — the fix for the KNOWN LIMITATION described in
/// [`layout_section`]. The break candidate whitespace block stays in its row
/// (as in `line_break`); per-line L1 resets it to the base level, so on
/// right-to-left rows it renders at the visual reading end (the left edge),
/// pinned by the wrapped oracles.
///
/// Mirrors `line_break`'s arithmetic: the leading indent
/// (`LayoutSection::leading_space`, carried by the first emitted glyph)
/// counts against the first row's budget; the empty-first-row escape hatch
/// and the `max_rows`/`elided` checks apply per produced row.
fn line_break_logically(
    paragraph: &Paragraph,
    job: &LayoutJob,
    word_ranges: &[Range<usize>],
    out_rows: &mut Vec<PlacedRow>,
    elided: &mut bool,
) {
    let wrap_width = job.effective_wrap_width();
    let blocks = logical_blocks(paragraph);

    // The cluster-exact attribution above is only sound when tracking was
    // on. The gate in `layout` guarantees it for every text the tracking
    // predicate recognizes, so the `Σ chr.len_utf8()` fallback in
    // `logical_blocks` is unreachable for such jobs; it exists for texts the
    // scan misses (documented graceful degradation — the drift is kept in
    // check by `place_row`'s guard), never a panic.
    debug_assert!(
        paragraph.cluster_byte_starts.len() == paragraph.glyphs.len()
            || !text_may_record_bidi_runs(&job.text),
        "records-runs-implies-tracked violated: a paragraph recorded bidi \
         runs while the tracking predicate recognized its text — the `Σ \
         chr.len_utf8()` attribution will drift on decomposed glyphs"
    );

    // Per-row UAX #9 context, computed once per overflowing paragraph:
    let span = paragraph.byte_span.clone();
    let span_text = job.text.get(span.clone()).unwrap_or("");
    let base_level = if resolved_direction(job) == TextDirection::RightToLeft {
        unicode_bidi::Level::rtl()
    } else {
        unicode_bidi::Level::ltr()
    };
    let reorder = RowReorder {
        text: span_text,
        span_start: span.start,
        base_level,
        info: unicode_bidi::ParagraphBidiInfo::new(span_text, Some(base_level)),
    };

    // Keeps track of good places to insert a row break if we exceed `wrap_width`.
    let mut row_break_candidates = RowBreakCandidates::default();

    // Index into `word_ranges`: the next dictionary-word end not yet registered.
    let mut word_idx = 0;

    // The pen: accumulated width of the current row's blocks. Starts at the
    // leading indent, mirroring `line_break`'s first row measured from x = 0
    // (whose glyph max_x values include the indent).
    let first_row_indentation = paragraph.glyphs[0].pos.x.max(0.0);
    let mut pen = first_row_indentation;
    let mut first_row_indentation_left = first_row_indentation > 0.0;

    // Logical block index where the current row starts.
    let mut row_start = 0usize;

    for b in 0..blocks.len() {
        if job.wrap.max_rows <= out_rows.len() {
            *elided = true;
            break;
        }

        pen += blocks[b].width;
        let potential_row_width = pen;

        if wrap_width < potential_row_width {
            // Row break:

            if first_row_indentation_left
                && !row_break_candidates.has_good_candidate(job.wrap.break_anywhere)
            {
                // Allow the first row to be completely empty, because we know
                // there will be more space on the next row (as in
                // `line_break`; the indent is spent on it and excluded from
                // the following budgets):
                out_rows.push(PlacedRow {
                    pos: pos2(0.0, f32::NAN),
                    row: Arc::new(Row {
                        section_index_at_start: paragraph.section_index_at_start,
                        base_direction: resolved_direction(job),
                        leading_advance: 0.0,
                        glyphs: vec![],
                        visuals: Default::default(),
                        size: Vec2::ZERO,
                    }),
                    ends_with_newline: false,
                });
                pen -= first_row_indentation;
                first_row_indentation_left = false;
            } else if let Some(last_kept) = row_break_candidates.get(job.wrap.break_anywhere) {
                reorder.place_row(
                    paragraph,
                    job,
                    &blocks,
                    row_start..last_kept + 1,
                    if row_start == 0 && first_row_indentation_left {
                        paragraph.leading_advance
                    } else {
                        0.0
                    },
                    out_rows,
                );

                // Start a new row:
                row_start = last_kept + 1;
                pen = 0.0;
                row_break_candidates.forget_before_idx(row_start);
            } else {
                // Found no place to break, so we have to overrun wrap_width.
            }
        }

        // Register the block as a break candidate — after the overflow check,
        // exactly as `line_break` registers each glyph (a block that itself
        // overflows is never a candidate):
        let base_glyph = &paragraph.glyphs[blocks[b].glyphs.start];
        let next_in_stream = paragraph.glyphs.get(blocks[b].glyphs.end);
        row_break_candidates.add_block(b, base_glyph, next_in_stream);

        // Translate breaker byte ranges into block indices with an ascending
        // pointer walk over the blocks' logical byte starts (hostile ranges
        // degrade break quality but never panic). `word_ranges` is only
        // non-empty when cluster tracking was on, so the indexed vector has
        // an entry per glyph (guarded by the assert below).
        if !word_ranges.is_empty() {
            debug_assert_eq!(
                paragraph.cluster_byte_starts.len(),
                paragraph.glyphs.len(),
                "cluster tracking must cover every glyph when a breaker is engaged"
            );
            while let Some(word_end) = word_ranges.get(word_idx).map(|word_range| word_range.end) {
                let this_start = blocks[b].bytes.start;
                if this_start >= word_end {
                    // The word ended before this block; move on to the next word.
                    word_idx += 1;
                    continue;
                }
                let next_start = blocks.get(b + 1).map(|block| block.bytes.start);
                if next_start.is_none_or(|next| next >= word_end) {
                    // This block is the last one whose characters start
                    // inside the word: register "may break after it".
                    row_break_candidates.word = Some(b);
                    word_idx += 1;
                }
                break;
            }
        }
    }

    if row_start < blocks.len() {
        // Final row of text (the logically-last blocks):

        if job.wrap.max_rows <= out_rows.len() {
            *elided = true; // can't fit another row
        } else {
            // As in `line_break`: the leading indent rides the first row's
            // glyphs (its `leading_advance` and its stream positions) only
            // when the whole paragraph is a single row; rows after a break
            // sit at zero.
            reorder.place_row(
                paragraph,
                job,
                &blocks,
                row_start..blocks.len(),
                if row_start == 0 && first_row_indentation_left {
                    paragraph.leading_advance
                } else {
                    0.0
                },
                out_rows,
            );
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
        let ResolvedOverflowGlyph {
            extra_letter_spacing,
            font_metrics,
            raster,
            face_key,
            font_face_metrics,
            glyph_info,
            advance_width_px,
        } = resolve_overflow_glyph(fonts, pixels_per_point, section, overflow_character);
        let replacement_glyph_width = advance_width_px / pixels_per_point;

        let overflow_glyph_x = if let Some(prev_glyph) = row.glyphs.last() {
            prev_glyph.max_x() + extra_letter_spacing
        } else {
            0.0 // TODO(emilk): heed paragraph leading_space 😬
        };

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
                // The ellipsis continues the row's reading direction, so it
                // takes the row's base level:
                bidi_level: u8::from(row.base_direction == TextDirection::RightToLeft),
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

/// The overflow character resolved against one section: the section's font lookups,
/// the priority-rasterizer probe (which may override the glyph), the plain-face
/// fallback, and the metrics and advance width needed to place it.
///
/// Shared by the LTR and RTL elision paths, which differ only in which end of the
/// row they trim and where they insert the replacement glyph.
struct ResolvedOverflowGlyph {
    extra_letter_spacing: f32,
    font_metrics: StyledMetrics,
    raster: Option<RasterGlyphAllocation>,
    face_key: FontFaceKey,
    font_face_metrics: StyledMetrics,
    glyph_info: GlyphInfo,
    advance_width_px: f32,
}

/// Resolves the replacement glyph for [`replace_last_glyph_with_overflow_character`]
/// and [`replace_first_glyph_with_overflow_character`].
fn resolve_overflow_glyph(
    fonts: &mut FontsImpl,
    pixels_per_point: f32,
    section: &LayoutSection,
    overflow_character: char,
) -> ResolvedOverflowGlyph {
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
            .map(|face| face.styled_metrics(pixels_per_point, font_size, &section.format.coords))
            .unwrap_or_default();
        let (_, glyph_info) = fonts.glyph_info(family, overflow_character, &font_face_metrics);
        (face_key, font_face_metrics, glyph_info)
    };

    let advance_width_px = match &raster {
        Some(raster) => raster.advance_px,
        None => glyph_info.advance_width_unscaled.0 * font_face_metrics.px_scale_factor,
    };

    ResolvedOverflowGlyph {
        extra_letter_spacing,
        font_metrics,
        raster,
        face_key,
        font_face_metrics,
        glyph_info,
        advance_width_px,
    }
}

/// Replaces the glyph run at the visual LEFT of the row with the overflow character (e.g. `…`).
///
/// The right-to-left twin of [`replace_last_glyph_with_overflow_character`].
/// For runs the shaper emits in visual (RTL) order the visual-left run is the logical end
/// of the text, so the text's beginning survives; for the synthetic tofu path (no fonts)
/// glyphs are in logical order — the contract is the visual one.
///
/// Called before we have any Y coordinates.
fn replace_first_glyph_with_overflow_character(
    fonts: &mut FontsImpl,
    pixels_per_point: f32,
    job: &LayoutJob,
    row: &mut Row,
) {
    let Some(overflow_character) = job.wrap.overflow_character else {
        return;
    };

    // How many glyphs to drop from the front of the row. We scan forward without
    // removing anything (the survivors keep their positions) and do a single
    // `drain` once the overflow glyph fits.
    let mut drop = 0_usize;

    loop {
        // The overflow glyph goes immediately before the first survivor, and inherits
        // its section (the glyph immediately right of the insertion point):
        let section_index = row
            .glyphs
            .get(drop)
            .map(|g| g.section_index)
            .unwrap_or(row.section_index_at_start);
        let section = &job.sections[section_index as usize];
        let ResolvedOverflowGlyph {
            extra_letter_spacing,
            font_metrics,
            raster,
            face_key,
            font_face_metrics,
            glyph_info,
            advance_width_px,
        } = resolve_overflow_glyph(fonts, pixels_per_point, section, overflow_character);
        let replacement_glyph_width = advance_width_px / pixels_per_point;

        // Where the overflow glyph would sit: immediately before the first survivor:
        let overflow_glyph_x = match row.glyphs.get(drop) {
            Some(first) => first.pos.x - extra_letter_spacing - replacement_glyph_width,
            None => 0.0, // TODO(emilk): heed paragraph leading_space 😬
        };

        // Check if we're within width budget - the whole row after placement:
        let row_width = match row.glyphs.last() {
            Some(last) => last.max_x() - overflow_glyph_x,
            None => replacement_glyph_width,
        };
        let fits = row_width <= job.effective_wrap_width() || drop >= row.glyphs.len();

        if fits {
            // We are done: trim the replaced run and insert the overflow glyph at the front.

            row.glyphs.drain(0..drop);

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

            row.glyphs.insert(
                0,
                Glyph {
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
                    // The ellipsis continues the row's reading direction, so it
                    // takes the row's base level:
                    bidi_level: u8::from(row.base_direction == TextDirection::RightToLeft),
                    section_index,
                    first_vertex: 0, // filled in later
                },
            );
            return;
        }

        // We didn't fit - drop the first glyph and try again.
        drop += 1;
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

    // The natural width of the row, whitespace included. Alignment trims the
    // ink below, but the row keeps this size so the space after (or before,
    // in right-to-left rows) the text is allocated like in a left-aligned row.
    let natural_width = row.size.x;

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

    // The ink span of the row, over ALL glyphs in the range — not just the
    // endpoints. Shaped complex scripts (e.g. Arabic ligature glyphs with
    // negative offsets, plus the zero-advance continuation placeholders at
    // both ends of the glyph vector) make the glyph vector non-monotonic in
    // x, so the first and last glyph do not bracket the ink. `f32::min`/`max`
    // ignore NaN, so a stray NaN position cannot poison the span.
    let original_min_x = row.glyphs[glyph_range.0..glyph_range.1]
        .iter()
        .map(|glyph| glyph.logical_rect().min.x)
        .fold(f32::INFINITY, f32::min);
    let original_max_x = row.glyphs[glyph_range.0..glyph_range.1]
        .iter()
        .map(|glyph| glyph.logical_rect().max.x)
        .fold(f32::NEG_INFINITY, f32::max);
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

    let num_spaces_in_range = row.glyphs[glyph_range.0..glyph_range.1]
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

    // The paragraph's leading advance sits before the first glyph. For a
    // left-anchored row it survives inside the ink positions; for a
    // right-anchored right-to-left row the reading start is the row's RIGHT
    // edge, so the anchored ink is inset by the advance instead of the
    // advance being absorbed into the row box.
    let reading_start_inset = if halign == Align::RIGHT
        && row.base_direction == TextDirection::RightToLeft
        && row.leading_advance != 0.0
    {
        row.leading_advance
    } else {
        0.0
    };
    placed_row.pos.x = point_scale.round_to_pixel(target_min_x) - reading_start_inset;
    let mut translate_x = -original_min_x - extra_x_per_glyph * glyph_range.0 as f32;

    for glyph in &mut row.glyphs {
        glyph.pos.x += translate_x;
        glyph.pos.x = point_scale.round_to_pixel(glyph.pos.x);
        translate_x += extra_x_per_glyph;
        if glyph.chr.is_whitespace() {
            translate_x += extra_x_per_space;
        }
    }

    // The ink range above is only for positioning: centered and right-anchored
    // rows align around their ink, so trailing whitespace must not skew them.
    // The row's allocated size keeps counting the whitespace (except for
    // justified rows, which fill the wrap width, and rows that
    // `keep_trailing_whitespace`, whose aligned span is their size), so
    // inter-widget spacing is the same in both reading directions.
    row.size.x = if justify || keep_trailing_whitespace {
        target_max_x - target_min_x
    } else {
        natural_width
    };
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

    for glyph in &row.glyphs {
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

    for glyph in &row.glyphs {
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

    /// End of a dictionary word in a script that is written without spaces
    /// (Thai, Lao, Khmer, Myanmar). Breaking after it is as good as after a
    /// space.
    word: Option<usize>,

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
        if is_breaking_space(chr) {
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
        [self.space, self.cjk, self.pre_cjk, self.word]
            .into_iter()
            .max()
            .flatten()
    }

    /// [`Self::add`] for a logical block: the block's base glyph plays the
    /// role of `glyphs[0]`, and the glyph that follows the block in the
    /// emitted stream plays `glyphs[1]` (`None` at the paragraph end).
    fn add_block(&mut self, index: usize, base: &Glyph, next_in_stream: Option<&Glyph>) {
        let chr = base.chr;
        const NON_BREAKING_SPACE: char = '\u{A0}';
        if chr.is_whitespace() && chr != NON_BREAKING_SPACE {
            self.space = Some(index);
        } else if is_cjk(chr) && next_in_stream.is_none_or(|glyph| is_cjk_break_allowed(glyph.chr))
        {
            self.cjk = Some(index);
        } else if chr == '-' {
            self.dash = Some(index);
        } else if chr.is_ascii_punctuation() {
            self.punctuation = Some(index);
        } else if next_in_stream.is_some_and(|glyph| is_cjk(glyph.chr)) {
            self.pre_cjk = Some(index);
        }
        self.any = Some(index);
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
            word,
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
        if word.is_some_and(|s| s < index) {
            *word = None;
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
/// NOTE: Segmentation is by font face, not by Unicode script. A run may
/// mix scripts (e.g. Latin + Cyrillic) when they share the same font.
/// Directional segmentation (UAX #9) happens one level up, in
/// [`segment_into_bidi_runs`]: the caller feeds this function one bidi run
/// at a time, so a run may mix scripts but never directions.
///
/// Results are appended to `out` (which is cleared first) to allow
/// the caller to reuse the allocation across calls.
fn segment_into_runs(
    fonts: &mut FontsImpl,
    ctx: &ShapingContext,
    text: &str,
    out: &mut Vec<TextRun>,
) {
    use unicode_segmentation::UnicodeSegmentation as _;

    out.clear();

    for (byte_offset, grapheme_str) in text.grapheme_indices(true) {
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
                raster: Some(raster),
            });
            continue;
        }

        if let Some(last_run) = out.last_mut()
            && last_run.raster.is_none()
            && last_run.font_key == font_key
        {
            last_run.byte_range.end = byte_end;
            continue;
        }
        out.push(TextRun {
            font_key,
            byte_range: byte_offset..byte_end,
            raster: None,
        });
    }
}

/// Shape a text run and return the raw [`harfrust::GlyphBuffer`].
///
/// The caller should iterate `glyph_infos()` / `glyph_positions()` (both
/// `Copy` slices) and convert font units to pixels using `metrics.px_scale_factor`.
/// After iteration, recycle the buffer via `glyph_buffer.clear()`.
///
/// `direction` is the run's resolved bidi direction (UAX #9): it is pinned on the
/// buffer so the shaper emits the glyphs in visual order for that run. The script
/// and language are still guessed from the run's content.
fn shape_text(
    font_face: &FontFace,
    text: &str,
    coords: &VariationCoords,
    mut buffer: harfrust::UnicodeBuffer,
    flags: harfrust::BufferFlags,
    direction: harfrust::Direction,
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
    buffer.set_direction(direction);
    buffer.push_str(text);
    buffer.guess_segment_properties();

    shaper.shape(buffer, harfrust::ShapeOptions::new())
}

// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use core::{hash::Hash as _, hash::Hasher as _, iter};

    use super::{super::*, *};
    use crate::{
        ColorImage,
        text::ByteRangeExt,
        text::cursor::{CCursor, LayoutCursor},
        text::galley_cache::GalleyCache,
    };

    fn test_fonts() -> FontsImpl {
        let mut definitions = FontDefinitions::default();
        if definitions.families.values().all(Vec::is_empty) {
            // The `default_fonts` feature is off (a plain `cargo test -p epaint`),
            // so `FontDefinitions::default` installed nothing. Install the bundled
            // fonts exactly as it does with the feature (see `font_definitions.rs`),
            // so these tests exercise real shaping either way. With the feature on
            // the fonts are already installed and this is a no-op.
            definitions.font_data.insert(
                "Hack".to_owned(),
                Arc::new(FontData::from_static(epaint_default_fonts::HACK_REGULAR)),
            );
            definitions.font_data.insert(
                "Ubuntu-Light".to_owned(),
                Arc::new(FontData::from_static(epaint_default_fonts::UBUNTU_LIGHT)),
            );
            definitions.font_data.insert(
                "egui-icons".to_owned(),
                Arc::new(
                    FontData::from_static(epaint_default_fonts::EGUI_ICONS).tweak(FontTweak {
                        scale: 0.90, // Make smaller
                        ..Default::default()
                    }),
                ),
            );
            definitions.families.insert(
                FontFamily::Monospace,
                vec![
                    "Hack".to_owned(),
                    "Ubuntu-Light".to_owned(), // fallback for √ etc
                    "egui-icons".to_owned(),
                ],
            );
            definitions.families.insert(
                FontFamily::Proportional,
                vec!["Ubuntu-Light".to_owned(), "egui-icons".to_owned()],
            );
        }
        FontsImpl::new(TextOptions::default(), definitions)
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

    /// An application-installed breaker that forces a word boundary at a fixed
    /// byte offset, to prove the hook works without any feature.
    struct ForceAt(usize);

    impl LineBreaker for ForceAt {
        fn word_boundaries(&self, text: &str) -> Vec<Range<usize>> {
            let word = 0..self.0.min(text.len());
            vec![word]
        }
    }

    /// A breaker returning canned ranges, hostile ones included
    /// (see `hostile_breaker_ranges_are_safe`).
    struct Ranges(Vec<Range<usize>>);

    impl LineBreaker for Ranges {
        fn word_boundaries(&self, _text: &str) -> Vec<Range<usize>> {
            self.0.clone()
        }
    }

    /// A custom [`LineBreaker`] drives row breaks for arbitrary scripts, not
    /// just Southeast-Asian ones (no feature required).
    #[test]
    fn custom_line_breaker_is_used() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let text = "abcdefghij";
        let one = fonts
            .layout_no_wrap("a".to_owned(), font_id.clone(), Color32::WHITE)
            .size()
            .x;
        // Every width that fits the forced word plus a fraction, so the first
        // overflow decision happens at or after the forced word end.
        for n in 4..8 {
            let width = one * n as f32 + 0.5 * one;
            let mut job =
                LayoutJob::simple(text.to_owned(), font_id.clone(), Color32::WHITE, width);
            job.wrap.line_breaker = Some(Arc::new(ForceAt(4)));
            let galley = fonts.layout_job(job);
            let row_texts: Vec<String> = galley.rows.iter().map(|p| p.row.text()).collect();
            assert!(
                row_texts.iter().any(|t| t == "abcd"),
                "no row is \"abcd\" at width {width}: {row_texts:?}"
            );
        }
    }

    /// Two layouts with the same breaker `Arc` hit the galley cache:
    /// the cache key includes breaker identity (and the cache pins it).
    #[test]
    fn same_breaker_arc_hits_the_cache() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let breaker: Arc<dyn LineBreaker> = Arc::new(ForceAt(4));

        let job = || {
            let mut job = LayoutJob::simple(
                "abcdefghij".to_owned(),
                font_id.clone(),
                Color32::WHITE,
                100.0,
            );
            job.wrap.line_breaker = Some(Arc::clone(&breaker));
            job
        };
        let a = fonts.layout_job(job());
        let b = fonts.layout_job(job());
        assert!(
            Arc::ptr_eq(&a, &b),
            "the same breaker Arc must hit the galley cache"
        );
    }

    /// Swapping in a different breaker must re-layout: the cache key includes
    /// breaker identity, so no stale galley can be served.
    #[test]
    fn swapping_the_breaker_relayouts() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let text = "abcdefghij";
        let one = fonts
            .layout_no_wrap("a".to_owned(), font_id.clone(), Color32::WHITE)
            .size()
            .x;
        let width = one * 8.0 + 0.5 * one;

        let job = |breaker: Arc<dyn LineBreaker>| {
            let mut job =
                LayoutJob::simple(text.to_owned(), font_id.clone(), Color32::WHITE, width);
            job.wrap.line_breaker = Some(breaker);
            job
        };
        let galley4 = fonts.layout_job(job(Arc::new(ForceAt(4))));
        let galley6 = fonts.layout_job(job(Arc::new(ForceAt(6))));

        let row_texts4: Vec<String> = galley4.rows.iter().map(|p| p.row.text()).collect();
        let row_texts6: Vec<String> = galley6.rows.iter().map(|p| p.row.text()).collect();
        assert_ne!(row_texts4, row_texts6);
    }

    /// A hostile breaker (empty, inverted, overlapping, unsorted, zero-length
    /// or out-of-bounds ranges) must never panic and never slice the text by
    /// attacker-controlled offsets.
    #[test]
    fn hostile_breaker_ranges_are_safe() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let text = "abcdefghij";
        let one = fonts
            .layout_no_wrap("a".to_owned(), font_id.clone(), Color32::WHITE)
            .size()
            .x;

        let cases: Vec<Vec<Range<usize>>> = vec![
            vec![],
            vec![0..0],
            vec![text.len()..0],                    // inverted
            vec![0..usize::MAX],                    // out of bounds
            vec![1..1],                             // empty
            vec![0..4, 2..6],                       // overlapping
            vec![Range { start: 6, end: 2 }, 0..4], // unsorted
        ];
        for ranges in cases {
            let mut job =
                LayoutJob::simple(text.to_owned(), font_id.clone(), Color32::WHITE, one * 4.0);
            job.wrap.line_breaker = Some(Arc::new(Ranges(ranges.clone())));
            let galley = fonts.layout_job(job);
            let total_glyphs: usize = galley.rows.iter().map(|p| p.row.glyphs.len()).sum();
            assert_eq!(total_glyphs, text.chars().count(), "ranges: {ranges:?}");
        }
    }

    /// The issue's repro: Thai text wraps at word boundaries once the rows are
    /// wide enough to fit a whole word, and no row overruns `max_width`.
    #[cfg(feature = "sea_dictionary")]
    #[test]
    fn sea_text_wraps_at_word_boundaries() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let text = "ภาษาไทยใช้การเขียนต่อเนื่องโดยไม่คั่นคำด้วยช่องว่าง";
        let one = fonts
            .layout_no_wrap("ก".to_owned(), font_id.clone(), Color32::WHITE)
            .size()
            .x;
        // Swept from 12: above the longest word in the split (`เนื่อง`, 6
        // code points), so every break lands on a registered word end.
        for glyphs in 12..30 {
            let width = one * glyphs as f32 + 0.5 * one;
            let galley = fonts.layout(text.to_owned(), font_id.clone(), Color32::WHITE, width);
            for placed in galley.rows.iter().skip(1) {
                let first = placed.row.glyphs.first().map(|g| g.chr);
                if let Some(first) = first {
                    assert!(
                        !crate::text::line_breaker::is_sea_extend(first)
                            && !matches!(first, '\u{0E30}' | '\u{0E32}' | '\u{0EB0}' | '\u{0EB2}'),
                        "a row starts with the stranded continuation char {first:?} at {glyphs} glyphs wide"
                    );
                }
                assert!(
                    placed.row.size.x <= width + one,
                    "row of width {} overruns max width {width} at {glyphs} glyphs wide",
                    placed.row.size.x
                );
            }
        }
    }

    /// Khmer kinsoku: closing punctuation (and a coeng) never start a row.
    #[cfg(feature = "sea_dictionary")]
    #[test]
    fn sea_closing_punctuation_never_starts_a_row() {
        let mut fonts = Fonts::new(TextOptions::default(), FontDefinitions::empty())
            .with_missing_glyph_policy(MissingGlyphPolicy::Tofu);
        let mut fonts = fonts.with_pixels_per_point(2.0);
        let font_id = FontId::proportional(14.0);
        let text = "កម្ពុជាមានប្រជាជនច្រើន។ខ្មែរជាភាសា។";
        let one = fonts
            .layout_no_wrap("ក".to_owned(), font_id.clone(), Color32::WHITE)
            .size()
            .x;
        // Swept from 12: above the longest words in the split (`កម្ពុជា` /
        // `ប្រជាជន`, 7 code points each), so every break lands on a word end.
        for glyphs in 12..30 {
            let width = one * glyphs as f32 + 0.5 * one;
            let galley = fonts.layout(text.to_owned(), font_id.clone(), Color32::WHITE, width);
            for placed in galley.rows.iter().skip(1) {
                let first = placed.row.glyphs.first().map(|g| g.chr);
                assert!(
                    !matches!(first, Some('។' | '\u{17D2}')),
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
    #[cfg(feature = "default_fonts")]
    fn test_no_break_spaces() {
        let row_texts = |text: &str| {
            let mut fonts = test_fonts();
            let mut layout_job = LayoutJob::single_section(text.into(), TextFormat::default());
            layout_job.wrap.max_width = 90.0;
            let galley = layout(&mut fonts, 1.0, layout_job.into());
            galley.rows.iter().map(|row| row.text()).collect::<Vec<_>>()
        };

        assert_eq!(
            row_texts("Sum: 123\u{2009}456\u{2009}789"),
            vec!["Sum: 123\u{2009}456\u{2009}", "789"]
        );
        for no_break_space in ['\u{00A0}', '\u{2007}', '\u{202F}'] {
            let number = format!("123{no_break_space}456{no_break_space}789");
            assert_eq!(
                row_texts(&format!("Sum: {number}")),
                vec!["Sum: ".to_owned(), number]
            );
        }
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

    #[test]
    fn first_strong_inference_matches_uax9_dir_auto() {
        // First strong character is RTL (digits are weak, class EN):
        assert_eq!(
            TextDirection::infer("12 نص"),
            Some(TextDirection::RightToLeft)
        );
        // First strong character is LTR:
        assert_eq!(
            TextDirection::infer("Layer نص"),
            Some(TextDirection::LeftToRight)
        );
        // No strong character at all (only weak/neutral ones):
        assert_eq!(TextDirection::infer("123"), None);
        assert_eq!(TextDirection::infer(""), None);
        assert_eq!(TextDirection::infer("..."), None);
    }

    fn tofu_fonts() -> FontsImpl {
        FontsImpl::new(TextOptions::default(), FontDefinitions::empty())
    }

    #[test]
    fn rtl_default_halign_anchors_the_reading_start() {
        let mut fonts = tofu_fonts();
        let pixels_per_point = 2.0;
        let font_id = FontId::proportional(14.0);

        // Inferred (direction: None) strong-RTL text right-anchors:
        let job = LayoutJob::simple_singleline("نص عربي…".into(), font_id.clone(), Color32::WHITE);
        assert_eq!(job.direction, None);
        let galley = layout(&mut fonts, pixels_per_point, job.into());
        assert_eq!(galley.rows.len(), 1);
        // The row grows leftward from the anchor:
        assert!(galley.rect.left() < 0.0, "left: {}", galley.rect.left());
        // The right edge sits at the anchor (tolerance of half a physical pixel):
        assert!(
            galley.rect.right() <= 1e-3,
            "right: {}",
            galley.rect.right()
        );

        // An explicit LTR pin keeps the old leftward-from-zero anchoring:
        let mut job =
            LayoutJob::simple_singleline("نص عربي…".into(), font_id.clone(), Color32::WHITE);
        job.direction = Some(TextDirection::LeftToRight);
        let galley = layout(&mut fonts, pixels_per_point, job.into());
        assert_eq!(galley.rows.len(), 1);
        assert_eq!(galley.rect.left(), 0.0);

        // Pinning works without inference: explicit RTL on Latin text right-anchors:
        let mut job = LayoutJob::simple_singleline("Hello".into(), font_id, Color32::WHITE);
        job.direction = Some(TextDirection::RightToLeft);
        let galley = layout(&mut fonts, pixels_per_point, job.into());
        assert_eq!(galley.rows.len(), 1);
        assert!(galley.rect.left() < 0.0, "left: {}", galley.rect.left());
        assert!(
            galley.rect.right() <= 1e-3,
            "right: {}",
            galley.rect.right()
        );
    }

    /// A row ending in whitespace allocates the same width in both reading
    /// directions: the ink is trimmed for alignment, but the row's size keeps
    /// counting the whitespace, so a widget following at zero item spacing
    /// keeps its gap under right-to-left layout too.
    #[test]
    fn rtl_row_allocates_trailing_whitespace_like_its_ltr_twin() {
        let mut fonts = tofu_fonts();
        let pixels_per_point = 2.0;
        let font_id = FontId::proportional(14.0);

        let mut ltr_job =
            LayoutJob::simple_singleline("abc ".into(), font_id.clone(), Color32::WHITE);
        ltr_job.direction = Some(TextDirection::LeftToRight);
        let ltr = layout(&mut fonts, pixels_per_point, ltr_job.into());

        let mut rtl_job = LayoutJob::simple_singleline("abc ".into(), font_id, Color32::WHITE);
        rtl_job.direction = Some(TextDirection::RightToLeft);
        let rtl = layout(&mut fonts, pixels_per_point, rtl_job.into());

        assert_eq!(ltr.rows.len(), 1);
        assert_eq!(rtl.rows.len(), 1);
        assert_eq!(
            rtl.size().x,
            ltr.size().x,
            "the rtl row must allocate the same width as its ltr twin"
        );
        // The ink still ends at the reading-start anchor, and the row rect
        // starts at the ink (the `rect.left()` compensation contract); only
        // the allocated width grows to cover the whitespace:
        let rtl_ink = ink_rect(&rtl);
        assert!(rtl_ink.right().abs() <= 0.5 / pixels_per_point);
        assert!((rtl_ink.left() - rtl.rect.left()).abs() <= 0.5 / pixels_per_point);
    }

    /// The bounding rect of the non-whitespace glyphs ("the ink"), in galley
    /// space.
    fn ink_rect(galley: &Galley) -> Rect {
        galley
            .rows
            .iter()
            .map(|row| {
                row.glyphs
                    .iter()
                    .filter(|glyph| !glyph.chr.is_whitespace())
                    .map(|glyph| glyph.logical_rect())
                    .fold(Rect::NOTHING, Rect::union)
                    .translate(row.pos.to_vec2())
            })
            .fold(Rect::NOTHING, Rect::union)
    }

    /// Trailing whitespace never skews the ink of centered or right-anchored
    /// rows: they align around their ink (whitespace excluded), while the
    /// allocated width still includes it.
    #[test]
    fn trailing_whitespace_does_not_skew_center_or_right_ink() {
        let mut fonts = tofu_fonts();
        let pixels_per_point = 2.0;
        let font_id = FontId::proportional(14.0);
        // Half a physical pixel: glyph positions are pixel-rounded.
        let tol = 0.5 / pixels_per_point;

        let mut left_job =
            LayoutJob::simple_singleline("abc ".into(), font_id.clone(), Color32::WHITE);
        left_job.direction = Some(TextDirection::LeftToRight);
        let ink_width = ink_rect(&layout(&mut fonts, pixels_per_point, left_job.into())).width();

        for halign in [Align::Center, Align::RIGHT] {
            let mut job =
                LayoutJob::simple_singleline("abc ".into(), font_id.clone(), Color32::WHITE);
            job.direction = Some(TextDirection::LeftToRight);
            job.halign = halign;
            let galley = layout(&mut fonts, pixels_per_point, job.into());
            assert_eq!(galley.rows.len(), 1);

            // The ink, in galley space: the non-whitespace glyphs, offset by
            // the row position.
            let ink = ink_rect(&galley);
            match halign {
                Align::Center => {
                    assert!(
                        (ink.center().x).abs() <= tol,
                        "center ink skewed: {:?}",
                        ink.center().x
                    );
                }
                Align::RIGHT => {
                    assert!(
                        ink.right().abs() <= tol,
                        "right ink skewed: {:?}",
                        ink.right()
                    );
                }
                Align::LEFT => unreachable!(),
            }
            assert!(
                (ink.width() - ink_width).abs() <= tol,
                "ink width changed: {:?} vs {ink_width:?}",
                ink.width()
            );
        }
    }

    #[test]
    fn rtl_elision_replaces_the_visual_left_run() {
        let mut fonts = tofu_fonts();
        let pixels_per_point = 2.0;
        let font_id = FontId::proportional(14.0);

        let mut job = LayoutJob::simple_singleline(
            "نص عربي طويل جدا".into(),
            font_id.clone(),
            Color32::WHITE,
        );
        job.wrap.max_width = 30.0;
        job.wrap.max_rows = 1;
        job.wrap.overflow_character = Some('…');
        let max_width = job.wrap.max_width;

        let galley = layout(&mut fonts, pixels_per_point, job.into());
        assert_eq!(galley.rows.len(), 1);
        let glyphs = &galley.rows[0].row.glyphs;
        // The overflow character is at the visual LEFT of the row:
        assert_eq!(glyphs.first().map(|g| g.chr), Some('…'));
        // Some of the content survives:
        assert!(glyphs.len() > 1);
        // The row stays within the wrap budget:
        assert!(galley.size().x <= max_width, "size: {:?}", galley.size());

        // LTR counterpart: the overflow character is the LAST glyph.
        let mut job =
            LayoutJob::simple_singleline("long latin text".into(), font_id, Color32::WHITE);
        job.wrap.max_width = 30.0;
        job.wrap.max_rows = 1;
        job.wrap.overflow_character = Some('…');

        let galley = layout(&mut fonts, pixels_per_point, job.into());
        assert_eq!(galley.rows.len(), 1);
        let glyphs = &galley.rows[0].row.glyphs;
        assert_eq!(glyphs.last().map(|g| g.chr), Some('…'));
        assert!(glyphs.len() > 1);
    }

    #[test]
    fn direction_none_ltr_content_is_backwards_compatible() {
        let mut fonts = tofu_fonts();
        let pixels_per_point = 2.0;

        for text in ["Hello world", "123", ""] {
            let mut job = LayoutJob::simple(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                100.0,
            );
            let none_galley = layout(&mut fonts, pixels_per_point, job.clone().into());

            job.direction = Some(TextDirection::LeftToRight);
            let ltr_galley = layout(&mut fonts, pixels_per_point, job.into());

            similar_asserts::assert_eq!(
                format!("{:#.1?}", none_galley),
                format!("{:#.1?}", ltr_galley)
            );
        }
    }

    #[test]
    fn layout_job_hash_and_eq_distinguish_direction_and_language() {
        fn hash(job: &LayoutJob) -> u64 {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            job.hash(&mut hasher);
            hasher.finish()
        }

        let mut a = LayoutJob::simple(
            "text".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            100.0,
        );
        let mut b = a.clone();
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));

        // Differing only in direction:
        b.direction = Some(TextDirection::RightToLeft);
        assert_ne!(a, b);
        assert_ne!(hash(&a), hash(&b));
        a.direction = Some(TextDirection::RightToLeft);
        b.direction = None;
        assert_ne!(a, b);
        assert_ne!(hash(&a), hash(&b));

        // Reset direction, test language:
        a.direction = None;
        b.language = Some("th".into());
        assert_ne!(a, b);
        assert_ne!(hash(&a), hash(&b));
        a.language = Some("th".into());
        assert_eq!(a, b);
        assert_eq!(hash(&a), hash(&b));
        b.language = Some("ja".into());
        assert_ne!(a, b);
        assert_ne!(hash(&a), hash(&b));
    }

    #[test]
    fn galley_cache_distinguishes_direction_and_language() {
        let mut fonts = tofu_fonts();

        let mut job = LayoutJob::simple(
            "text".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            100.0,
        );
        let mut cache = GalleyCache::default();
        let a = cache.layout(&mut fonts, 2.0, job.clone(), false);
        job.language = Some("th".into());
        let b = cache.layout(&mut fonts, 2.0, job.clone(), false);
        assert!(
            !Arc::ptr_eq(&a, &b),
            "language must invalidate the cache key"
        );

        let c = cache.layout(&mut fonts, 2.0, job, false);
        assert!(Arc::ptr_eq(&b, &c), "identical jobs should hit the cache");

        let mut job = LayoutJob::simple(
            "text".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            100.0,
        );
        let mut cache = GalleyCache::default();
        let a = cache.layout(&mut fonts, 2.0, job.clone(), false);
        job.direction = Some(TextDirection::LeftToRight);
        let b = cache.layout(&mut fonts, 2.0, job, false);
        assert!(
            !Arc::ptr_eq(&a, &b),
            "direction (None vs explicit LTR) must invalidate the cache key"
        );
    }

    // -------------------------------------------------------------------------
    // Bidi run segmentation + visual reordering (UAX #9)
    //
    // The glyphs of a bidi-reordered row are stored in VISUAL order (left to
    // right), matching what single RTL runs always did. Zero-advance
    // continuation glyphs (which keep `glyphs.len() == char_count` for
    // multi-glyph clusters) are filtered out in the order assertions below;
    // they draw nothing and follow their cluster's base glyphs.

    /// The visible (advance > 0) glyphs of a single-row galley as `(chr, x, advance)`.
    fn visible_glyphs(galley: &Galley) -> Vec<(char, f32, f32)> {
        assert_eq!(galley.rows.len(), 1, "expected a single-row galley");
        galley.rows[0]
            .row
            .glyphs
            .iter()
            .filter(|g| g.advance_width > 0.0)
            .map(|g| (g.chr, g.pos.x, g.advance_width))
            .collect()
    }

    fn assert_x_nondecreasing(glyphs: &[(char, f32, f32)]) {
        for pair in glyphs.windows(2) {
            assert!(
                pair[0].1 <= pair[1].1 + 1e-3,
                "glyph x positions must not decrease left to right: {:?} then {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    /// Asserts the visible glyphs match `expected` in order, with a small
    /// tolerance on the pixel values (the pre-change dumps were rounded to
    /// four decimals).
    fn assert_glyphs_close(glyphs: &[(char, f32, f32)], expected: &[(char, f32, f32)]) {
        assert_eq!(glyphs.len(), expected.len(), "glyph count: {glyphs:?}");
        for (got, want) in iter::zip(glyphs, expected) {
            assert_eq!(got.0, want.0, "glyph: {glyphs:?}");
            assert!(
                (got.1 - want.1).abs() < 1e-3 && (got.2 - want.2).abs() < 1e-3,
                "glyph {}: got ({:.4}, {:.4}), want ({:.4}, {:.4})",
                got.0,
                got.1,
                got.2,
                want.1,
                want.2
            );
        }
    }

    fn layout_with_direction(text: &str, direction: Option<TextDirection>) -> Galley {
        let mut fonts = test_fonts();
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        job.direction = direction;
        layout(&mut fonts, 1.0, job.into())
    }

    /// T1 — the rename-ar oracle: a first-strong RTL paragraph renders in visual
    /// order (`pdf.٢٠٢٦ تقرير`), not logical order. The latin file extension and
    /// the dot sit at the visual left; the Arabic word at the visual right; the
    /// Arabic-Indic digits keep their UAX #9 order (level 2, LTR) between them.
    #[test]
    fn bidi_oracle_first_strong_rtl_renders_visual_order() {
        let text = "تقرير ٢٠٢٦.pdf";

        // First-strong inference (direction: None):
        let glyphs = visible_glyphs(&layout_with_direction(text, None));
        // Glyphs are stored in visual (left-to-right) order: the latin extension
        // and dot at the left, the digits in LTR order, then the Arabic word,
        // whose glyph order runs right-to-left (its first logical letter ت is
        // the visually-rightmost glyph). I.e. the rendered line reads
        // `pdf.٢٠٢٦ تقرير` — the rename-ar oracle order.
        let expected: Vec<char> = [
            'p', 'd', 'f', '.', '٢', '٠', '٢', '٦', ' ', 'ر', 'ي', 'ر', 'ق', 'ت',
        ]
        .into();
        let rendered: Vec<char> = glyphs.iter().map(|g| g.0).collect();
        assert_eq!(rendered, expected, "glyphs: {glyphs:?}");
        assert_x_nondecreasing(&glyphs);
        // The extension is at the visual left, the Arabic word at the visual right:
        assert_eq!(glyphs.first().map(|g| g.0), Some('p'));
        assert_eq!(glyphs.last().map(|g| g.0), Some('ت'));

        // An explicit RTL pin produces the same visual order:
        let pinned = visible_glyphs(&layout_with_direction(
            text,
            Some(TextDirection::RightToLeft),
        ));
        assert_eq!(
            pinned.iter().map(|g| g.0).collect::<Vec<char>>(),
            rendered,
            "explicit RTL must match the first-strong rendering"
        );
    }

    /// T2 — LTR base with an embedded RTL run: the embedded run is reversed
    /// (its glyphs come out in visual order) while the surrounding LTR text
    /// stays in logical order.
    #[test]
    fn bidi_ltr_base_reverses_embedded_rtl_run() {
        for direction in [None, Some(TextDirection::LeftToRight)] {
            let glyphs = visible_glyphs(&layout_with_direction("Hello שלום world", direction));
            let rendered: String = glyphs.iter().map(|g| g.0).collect();
            assert_eq!(rendered, "Hello םולש world", "glyphs: {glyphs:?}");
            assert_x_nondecreasing(&glyphs);
        }
    }

    /// T3 — numbers in an RTL context keep their UAX #9 placement: European
    /// numbers sit at level 2 (LTR), so they render in logical order at the
    /// visual left of an RTL paragraph (they were reversed before bidi
    /// reordering landed).
    #[test]
    fn bidi_numbers_keep_uax9_placement_in_rtl_context() {
        let glyphs = visible_glyphs(&layout_with_direction("عربي 123", None));
        let rendered: String = glyphs.iter().map(|g| g.0).collect();
        assert_eq!(rendered, "123 يبرع", "glyphs: {glyphs:?}");
        assert_x_nondecreasing(&glyphs);
    }

    /// T4 — byte-identity pins: pure-LTR text lays out exactly as it did before
    /// bidi reordering landed. The values below were captured from the
    /// pre-bidi tree with the default fonts at ppp 1.0.
    #[test]
    fn bidi_pure_ltr_is_byte_identical_to_pre_change_output() {
        let glyphs = visible_glyphs(&layout_with_direction("Hello World 123", None));
        let expected: Vec<(char, f32, f32)> = [
            ('H', 0.0, 9.6880),
            ('e', 9.0, 7.6720),
            ('l', 17.0, 3.5700),
            ('l', 21.0, 3.5700),
            ('o', 24.0, 8.1480),
            (' ', 32.0, 3.1920),
            ('W', 35.0, 12.7400),
            ('o', 48.0, 8.1480),
            ('r', 56.0, 5.2360),
            ('l', 62.0, 3.5700),
            ('d', 65.0, 8.1620),
            (' ', 73.0, 3.1920),
            ('1', 77.0, 7.8960),
            ('2', 84.0, 7.8960),
            ('3', 92.0, 7.8960),
        ]
        .into();
        assert_glyphs_close(&glyphs, &expected);
    }

    /// T4 — byte-identity pins: pure-RTL text (one bidi run) is laid out exactly
    /// as before — its single run was always shaped in its own direction and
    /// emitted in visual order. The zero-advance glyphs at the end are the
    /// pre-existing continuation-glyph output of the shaper path (unchanged by
    /// bidi reordering); the helper filters them out, so they are NOT pinned
    /// by this test.
    #[test]
    fn bidi_pure_rtl_is_byte_identical_to_pre_change_output() {
        for direction in [None, Some(TextDirection::RightToLeft)] {
            let glyphs = visible_glyphs(&layout_with_direction("تقرير", direction));
            // The five visible glyphs in visual order (the four zero-advance
            // continuation glyphs that follow are filtered out by the helper and
            // therefore unpinned):
            let expected: Vec<(char, f32, f32)> = [
                ('ر', 0.0, 7.0),
                ('ي', 7.0, 7.0),
                ('ر', 14.0, 7.0),
                ('ق', 21.0, 7.0),
                ('ت', 28.0, 7.0),
            ]
            .into();
            assert_glyphs_close(&glyphs, &expected);
        }
    }

    /// T5 — the cursor layer maps *logical* columns over the visual-order glyphs
    /// of a bidi-reordered row: under a right-to-left base direction the
    /// text-start caret (logical column 0) sits at the visual RIGHT edge and the
    /// text-end caret at the visual LEFT edge, clicks map visual x back to the
    /// logical column (clicking the visual middle of the Arabic word lands
    /// between the right graphemes), and every column round-trips through its
    /// caret x.
    #[test]
    fn bidi_cursor_hit_test_follows_logical_order() {
        let galley = layout_with_direction("عربي 123", None);
        assert_eq!(galley.rows.len(), 1);
        let row = &galley.rows[0];
        let n = row.row.char_count_excluding_newline().0;
        assert_eq!(n, 8, "one cursor column per character");
        let mid_y = (row.min_y() + row.max_y()) * 0.5;

        // Text start (logical column 0) sits at the visual RIGHT edge, the text
        // end (logical column n) at the visual LEFT edge:
        let pos = |column: usize| {
            galley
                .pos_from_layout_cursor(&LayoutCursor {
                    row: 0,
                    column: CharIndex(column),
                })
                .min
                .x
        };
        assert!(
            pos(0) > galley.rect.center().x,
            "the text-start caret must sit in the visual right half, got {} (rect {:?})",
            pos(0),
            galley.rect
        );
        assert!(
            pos(n) < galley.rect.center().x,
            "the text-end caret must sit in the visual left half, got {} (rect {:?})",
            pos(n),
            galley.rect
        );

        // Caret x decreases as the logical column advances (reading right to
        // left):
        for column in 0..n {
            assert!(
                pos(column) > pos(column + 1),
                "caret x must decrease from column {column} to {}: {} vs {}",
                column + 1,
                pos(column),
                pos(column + 1)
            );
        }

        // Clicking the visual middle of the Arabic word: the center of the `ب`
        // glyph (visual slot 5) puts the caret between `ب` and `ي` (logical
        // columns 2 and 3), and a click at the word's visual midpoint (the
        // boundary between `ر` and `ب`) one column earlier:
        let ba = &row.row.glyphs[5];
        assert_eq!(ba.chr, 'ب');
        let ba_center = ba.pos.x + ba.advance_width * 0.5 + row.pos.x;
        let clicked = galley.cursor_from_pos(vec2(ba_center, mid_y));
        assert_eq!(
            clicked.index.0, 3,
            "clicking the visual center of `ب` must put the caret between the right graphemes"
        );

        // x round-trip: every caret position maps back to its own column:
        for column in 0..=n {
            let back = galley.cursor_from_pos(vec2(pos(column), mid_y));
            assert_eq!(
                back.index.0, column,
                "clicking the caret x of column {column} must return that column"
            );
        }
    }

    /// One glyph slot per character: the continuation-glyph bookkeeping of RTL
    /// runs (whose shaper output is in descending cluster order) used to
    /// over-emit zero-advance duplicates for every character but the first,
    /// breaking the `glyphs.len() == char_count` invariant that all cursor
    /// code relies on.
    #[test]
    fn rtl_rows_have_one_glyph_slot_per_char() {
        for text in ["تقرير", "عربي 123", "سلام"] {
            for direction in [None, Some(TextDirection::RightToLeft)] {
                let galley = layout_with_direction(text, direction);
                assert_eq!(galley.rows.len(), 1);
                let count = galley.rows[0].row.glyphs.len();
                assert_eq!(
                    count,
                    text.chars().count(),
                    "glyph slots must equal the char count for {text:?} (direction {direction:?})"
                );
            }
        }
    }

    /// Every glyph is stamped with its bidi run's UAX #9 embedding level, so a
    /// row's visual glyph stream can be split back into directional runs at
    /// level changes (consumers: the accessibility `TextRun` builder). LTR rows
    /// stay all-zero (pure metadata — rendering is unaffected).
    #[test]
    fn bidi_levels_partition_rows_into_directional_runs() {
        fn level_runs(galley: &Galley) -> Vec<(u8, String)> {
            assert_eq!(galley.rows.len(), 1, "expected a single-row galley");
            let mut runs: Vec<(u8, String)> = Vec::new();
            for glyph in &galley.rows[0].row.glyphs {
                match runs.last_mut() {
                    Some((level, text)) if *level == glyph.bidi_level => {
                        text.push(glyph.chr);
                    }
                    _ => runs.push((glyph.bidi_level, glyph.chr.to_string())),
                }
            }
            runs
        }

        // LTR base with an embedded RTL run: 0 / 1 / 0. (The run texts are the
        // glyph stream's *visual* order, so the RTL run reads reversed:
        // `level_runs` partitions glyphs, not logical text.)
        assert_eq!(
            level_runs(&layout_with_direction("Hello שלום world", None)),
            vec![
                (0, "Hello ".to_owned()),
                (1, "םולש".to_owned()),
                (0, " world".to_owned()),
            ]
        );
        // Two runs suffice when the RTL text ends the row:
        assert_eq!(
            level_runs(&layout_with_direction("Hello שלום", None)),
            vec![(0, "Hello ".to_owned()), (1, "םולש".to_owned())]
        );

        // RTL base with European numbers: numbers sit at level 2 (even), the
        // Arabic run at level 1 (the space between them resolves to the base
        // level, so it belongs to the RTL run). Visual run order: digits at
        // the left, the RTL run (visually reversed) at the right:
        assert_eq!(
            level_runs(&layout_with_direction("عربي 123", None)),
            vec![(2, "123".to_owned()), (1, " يبرع".to_owned())]
        );

        // Pure RTL: every glyph at the odd base level:
        for direction in [None, Some(TextDirection::RightToLeft)] {
            for glyph in &layout_with_direction("تقرير", direction).rows[0].row.glyphs {
                assert!(
                    glyph.is_rtl(),
                    "pure-RTL glyph {:?} must carry an odd level",
                    glyph.chr
                );
            }
        }

        // Pure LTR: every glyph at level 0 (also with an explicit LTR pin):
        for direction in [None, Some(TextDirection::LeftToRight)] {
            for glyph in &layout_with_direction("Hello World 123", direction).rows[0]
                .row
                .glyphs
            {
                assert_eq!(
                    glyph.bidi_level, 0,
                    "pure-LTR glyph {:?} must carry level 0",
                    glyph.chr
                );
            }
        }
    }

    /// Continuation glyphs (zero-width stand-ins that keep
    /// `glyphs.len() == char_count` for multi-char clusters) carry their
    /// cluster's bidi level: they belong to the same directional run as the
    /// glyphs before them. No bundled font shapes a multi-glyph cluster, so
    /// this uses the synthetic shaper vector (see
    /// `descending_run_first_emitted_cluster_keeps_its_continuation_glyphs`).
    #[test]
    fn continuation_glyphs_carry_their_cluster_bidi_level() {
        let run_text = "ab\u{E000}";
        let mut fonts = test_fonts();
        let codepoints = [('b', 1), ('\u{E000}', 1), ('a', 0)];
        let (glyph_buffer, face_key, face_metrics) = shape_synthetic_run(&mut fonts, &codepoints);

        let family = fonts.family_key(&FontFamily::Proportional);
        // An odd level stands in for any RTL run: the point is that every
        // glyph of the run — continuations included — carries the run's level.
        let mut ctx = ShapingContext {
            family,
            pixels_per_point: 1.0,
            font_size: 14.0,
            line_height: face_metrics.row_height,
            extra_letter_spacing: 0.0,
            section_index: 0,
            font_metrics: face_metrics.clone(),
            is_first_glyph_in_section: true,
            prev_cluster: None,
            track_cluster_bytes: true,
            segment_base: 0,
            bidi_level: 3,
        };
        let run = TextRun {
            font_key: face_key,
            byte_range: ByteRangeExt::full(run_text),
            raster: None,
        };
        let mut paragraph = Paragraph::from_section_index(0);
        layout_shaped_run(
            &mut fonts,
            &run,
            run_text,
            &glyph_buffer,
            &face_metrics,
            &mut ctx,
            &mut paragraph,
        );

        assert_eq!(paragraph.glyphs.len(), run_text.chars().count());
        for glyph in &paragraph.glyphs {
            assert_eq!(glyph.bidi_level, 3, "glyph {:?}", glyph.chr);
            assert!(glyph.is_rtl());
        }
        // The continuation glyph (the zero-advance one) is covered too:
        assert!(paragraph.glyphs.iter().any(|g| g.advance_width == 0.0));
    }

    /// C1 — directional runs are recorded on the paragraph during emission, in
    /// visual order with job-absolute byte ranges, for every segment that
    /// resolved to more than one run or to a right-to-left run — and only when
    /// the job can wrap (a finite wrap width): the spans are consumed by the
    /// wrapping path alone. Pure-LTR segments record nothing.
    #[test]
    fn bidi_run_spans_are_recorded_on_paragraphs_for_wrapping_jobs() {
        let finite = 10_000.0; // big enough to never overflow these shapes
        let mut fonts = tofu_fonts();

        // The rename-ar oracle shape (T1): four runs in visual order — the
        // latin extension, the dot, the Arabic-Indic digits, the Arabic word
        // plus its trailing space.
        let mut job = LayoutJob::simple(
            "تقرير ٢٠٢٦.pdf".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            finite,
        );
        job.direction = Some(TextDirection::RightToLeft);
        let paragraphs = shape_paragraphs(&mut fonts, 1.0, &job, false);
        assert_eq!(paragraphs.len(), 1);
        let runs = &paragraphs[0].bidi_runs;
        assert_eq!(runs.len(), 4, "runs: {runs:?}");

        // Byte map: "تقرير" = 0..10, " " = 10..11, "٢٠٢٦" = 11..19,
        // "." = 19..20, "pdf" = 20..23. Visual order (left to right) is
        // `pdf.٢٠٢٦ تقرير`, so the recorded byte ranges descend:
        assert_eq!(runs[0].bytes, 20..23, "runs: {runs:?}");
        assert!(!runs[0].rtl, "the latin extension sits at an even level");
        assert_eq!(runs[1].bytes, 19..20, "runs: {runs:?}");
        assert!(runs[1].rtl, "the dot joins the base level");
        assert_eq!(runs[2].bytes, 11..19, "runs: {runs:?}");
        assert!(!runs[2].rtl, "Arabic-Indic digits keep an LTR level");
        assert_eq!(runs[3].bytes, 0..11, "runs: {runs:?}");
        assert!(runs[3].rtl);

        // The glyph ranges partition the paragraph's glyphs in emission order:
        assert_eq!(runs[0].glyphs, 0..3, "runs: {runs:?}");
        assert_eq!(runs[1].glyphs, 3..4, "runs: {runs:?}");
        assert_eq!(runs[2].glyphs, 4..8, "runs: {runs:?}");
        assert_eq!(
            runs[3].glyphs.end,
            paragraphs[0].glyphs.len(),
            "runs: {runs:?}"
        );
        assert_eq!(
            paragraphs[0].glyphs.len(),
            "تقرير ٢٠٢٦.pdf".chars().count(),
            "one glyph slot per char"
        );

        // A mixed RTL-base segment records its two runs:
        let mut job = LayoutJob::simple(
            "عربي aaa bbb".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            finite,
        );
        job.direction = Some(TextDirection::RightToLeft);
        let paragraphs = shape_paragraphs(&mut fonts, 1.0, &job, false);
        let runs = &paragraphs[0].bidi_runs;
        assert_eq!(runs.len(), 2, "runs: {runs:?}");
        // Visual order: the level-2 island `aaa bbb` (bytes 9..16) is emitted
        // first, the level-1 `عربي ` (bytes 0..9) second:
        assert_eq!(runs[0].bytes, 9..16, "runs: {runs:?}");
        assert!(!runs[0].rtl);
        assert_eq!(runs[1].bytes, 0..9, "runs: {runs:?}");
        assert!(runs[1].rtl);
        assert_eq!(paragraphs[0].glyphs.len(), "عربي aaa bbb".chars().count());

        // Pure-LTR segments (the ASCII fast path and a non-ASCII single run
        // alike) record nothing:
        for text in ["Hello World 123", "Héllo wörld"] {
            let mut job = LayoutJob::simple(
                text.into(),
                FontId::proportional(14.0),
                Color32::WHITE,
                finite,
            );
            job.direction = Some(TextDirection::LeftToRight);
            let paragraphs = shape_paragraphs(&mut fonts, 1.0, &job, false);
            assert!(
                paragraphs[0].bidi_runs.is_empty(),
                "{text:?} must not record runs: {:?}",
                paragraphs[0].bidi_runs
            );
        }

        // An infinite wrap width never records: the spans would never be
        // consumed, because such a job can never overflow.
        let mut job = LayoutJob::simple(
            "تقرير ٢٠٢٦.pdf".into(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        job.direction = Some(TextDirection::RightToLeft);
        let paragraphs = shape_paragraphs(&mut fonts, 1.0, &job, false);
        assert!(
            paragraphs[0].bidi_runs.is_empty(),
            "infinite wrap must not record runs: {:?}",
            paragraphs[0].bidi_runs
        );
    }

    /// Shape `codepoints` — (char, cluster byte offset) pairs in output order —
    /// with hand-picked cluster values, so a cluster can be given fewer glyphs
    /// than characters. No bundled font produces a multi-char→fewer-glyphs
    /// cluster (no lam-alef ligature; flag RIs keep one glyph per char), so the
    /// continuation-glyph bookkeeping needs a synthetic glyph vector to
    /// exercise that shape. Non-monotone cluster levels keep the hand-picked
    /// (descending) values instead of rewriting them.
    fn shape_synthetic_run(
        fonts: &mut FontsImpl,
        codepoints: &[(char, u32)],
    ) -> (harfrust::GlyphBuffer, FontFaceKey, StyledMetrics) {
        let family = fonts.family_key(&FontFamily::Proportional);
        let face_key = fonts.resolve_face(family, 'a');
        let coords = VariationCoords::default();
        let face_metrics = fonts
            .face(face_key)
            .expect("the test font must resolve 'a'")
            .styled_metrics(1.0, 14.0, &coords);

        let mut buffer = harfrust::UnicodeBuffer::default();
        for (chr, cluster) in codepoints {
            buffer.add(*chr, *cluster);
        }
        buffer.set_cluster_level(harfrust::BufferClusterLevel::Characters);
        buffer.set_direction(harfrust::Direction::LeftToRight);
        buffer.guess_segment_properties();

        let face = fonts
            .face(face_key)
            .expect("the test font must resolve 'a'");
        let shaper = face.shaper_data().shaper(face.skrifa_font_ref()).build();
        let glyph_buffer = shaper.shape(buffer, harfrust::ShapeOptions::new());
        (glyph_buffer, face_key, face_metrics)
    }

    /// A multi-char cluster with fewer glyphs than chars at the logically-last
    /// position of a descending-order run keeps its continuation glyph.
    ///
    /// The synthetic vector emits two clusters in descending byte order:
    /// cluster 1 covers the two chars "b" + U+E000 with a single glyph (the
    /// second codepoint maps to no glyph, like a dropped mark), then cluster 0
    /// covers "a". The first descending transition's outgoing cluster extends
    /// to the end of the run text; the previous-cluster bookkeeping used to
    /// seed itself with the dummy start `0` there, emitting an inverted
    /// `1..0` range that silently dropped the continuation glyph (2 slots for
    /// 3 chars).
    #[test]
    fn descending_run_first_emitted_cluster_keeps_its_continuation_glyphs() {
        let run_text = "ab\u{E000}";
        let mut fonts = test_fonts();
        let codepoints = [('b', 1), ('\u{E000}', 1), ('a', 0)];
        let (glyph_buffer, face_key, face_metrics) = shape_synthetic_run(&mut fonts, &codepoints);

        // Precondition: the synthetic vector is what the bookkeeping expects —
        // three glyphs in descending clusters, the second one unmapped (so the
        // run emits one glyph for the two-char cluster 1):
        let infos = glyph_buffer.glyph_infos();
        assert_eq!(infos.len(), 3, "infos: {infos:?}");
        let clusters: Vec<u32> = infos.iter().map(|info| info.cluster).collect();
        assert_eq!(clusters, [1, 1, 0], "infos: {infos:?}");
        assert_eq!(
            infos[1].glyph_id, 0,
            "U+E000 must have no glyph in the test font for this vector to be synthetic"
        );

        let family = fonts.family_key(&FontFamily::Proportional);
        let mut ctx = ShapingContext {
            family,
            pixels_per_point: 1.0,
            font_size: 14.0,
            line_height: face_metrics.row_height,
            extra_letter_spacing: 0.0,
            section_index: 0,
            font_metrics: face_metrics.clone(),
            is_first_glyph_in_section: true,
            prev_cluster: None,
            track_cluster_bytes: true,
            segment_base: 0,
            bidi_level: 0,
        };
        let run = TextRun {
            font_key: face_key,
            byte_range: ByteRangeExt::full(run_text),
            raster: None,
        };
        let mut paragraph = Paragraph::from_section_index(0);
        layout_shaped_run(
            &mut fonts,
            &run,
            run_text,
            &glyph_buffer,
            &face_metrics,
            &mut ctx,
            &mut paragraph,
        );

        let chars = run_text.chars().count();
        assert_eq!(
            paragraph.glyphs.len(),
            chars,
            "the continuation glyph of the logically-last cluster must not be lost: \
             glyphs {:?}",
            paragraph.glyphs.iter().map(|g| g.chr).collect::<Vec<_>>()
        );

        // The continuation is grouped with its cluster: it carries the cluster's
        // byte start (1, like the 'b' glyph before it, not 0), zero advance (it
        // draws nothing), and the char it stands in for:
        assert_eq!(
            paragraph.cluster_byte_starts,
            [1, 1, 0],
            "the continuation glyph must belong to cluster 1, with 'a' in cluster 0"
        );
        let continuation = &paragraph.glyphs[1];
        assert_eq!(continuation.chr, '\u{E000}');
        assert_eq!(
            continuation.advance_width, 0.0,
            "a continuation glyph is zero-width"
        );
        assert!(
            paragraph.glyphs[2].pos.x >= continuation.pos.x,
            "the glyph vector stays in visual order: {:?}",
            paragraph.glyphs.iter().map(|g| g.pos.x).collect::<Vec<_>>()
        );
    }

    /// The caret boundaries of a pure-RTL row mirror the reading direction: the
    /// first column sits at the ink's right edge, the last at the ink's left
    /// edge, and the LTR twin keeps the historic left-to-right geometry.
    #[test]
    fn rtl_caret_positions_mirror_the_reading_direction() {
        let rtl = layout_with_direction("تقرير", None);
        assert_eq!(rtl.rows.len(), 1);
        let row = &rtl.rows[0];
        let n = row.row.char_count_excluding_newline().0;
        let ink_min = row
            .glyphs
            .iter()
            .map(|glyph| glyph.pos.x)
            .fold(f32::INFINITY, f32::min);
        let ink_max = row
            .glyphs
            .iter()
            .map(|glyph| glyph.max_x())
            .fold(f32::NEG_INFINITY, f32::max);

        let x = |column: usize| row.row.x_offset(CharIndex(column));
        assert!(
            (x(0) - ink_max).abs() < 1.0,
            "column 0 (reading start) at the ink right edge: {} vs {ink_max}",
            x(0)
        );
        assert!(
            (x(n) - ink_min).abs() < 1.0,
            "column {n} (reading end) at the ink left edge: {} vs {ink_min}",
            x(n)
        );

        // The LTR twin keeps the historic geometry: column 0 at the ink's left
        // edge, column n at the row width:
        let ltr = layout_with_direction("abcde", Some(TextDirection::LeftToRight));
        let ltr_row = &ltr.rows[0];
        let ltr_n = ltr_row.row.char_count_excluding_newline().0;
        let ltr_x = |column: usize| ltr_row.row.x_offset(CharIndex(column));
        assert!(
            (ltr_x(0) - ltr_row.glyphs[0].pos.x).abs() < 1e-3,
            "LTR column 0 keeps its glyph-left caret"
        );
        assert!(
            (ltr_x(ltr_n) - ltr_row.size.x).abs() < 1e-3,
            "LTR column {ltr_n} keeps the end-of-row caret"
        );
    }

    /// A mixed row under an RTL base direction keeps exact caret geometry at
    /// the reading edges (the base-direction mapping is exact for the RTL run
    /// and clamped at the row ends), and clicks inside the Arabic run land
    /// between the right graphemes. Inside embedded left-to-right runs the
    /// mapping is approximate (documented in `Row`); this pins the edges.
    #[test]
    fn mixed_rtl_base_caret_edges() {
        let galley = layout_with_direction("عربي abc", None);
        assert_eq!(galley.rows.len(), 1);
        let row = &galley.rows[0];
        let n = row.row.char_count_excluding_newline().0;
        assert_eq!(n, 8);
        let mid_y = (row.min_y() + row.max_y()) * 0.5;

        let pos = |column: usize| {
            galley
                .pos_from_layout_cursor(&LayoutCursor {
                    row: 0,
                    column: CharIndex(column),
                })
                .min
                .x
        };
        assert!(
            pos(0) > galley.rect.center().x,
            "reading start at the visual right"
        );
        assert!(
            pos(n) < galley.rect.center().x,
            "reading end at the visual left"
        );

        // Clicking the visual center of `ب` (visual slot 5) puts the caret
        // between `ب` and `ي`:
        let ba = &row.glyphs[5];
        assert_eq!(ba.chr, 'ب');
        let ba_center = ba.pos.x + ba.advance_width * 0.5 + row.pos.x;
        let clicked = galley.cursor_from_pos(vec2(ba_center, mid_y));
        assert_eq!(
            clicked.index.0, 3,
            "the click inside the Arabic run must land between the right graphemes"
        );
    }

    /// Arrow-key movement is visual: under a right-to-left base direction,
    /// moving toward the visual left steps to the logically NEXT character and
    /// vice versa. The LTR twin keeps the historic logical movement.
    #[test]
    fn rtl_arrow_keys_move_visually() {
        let rtl = layout_with_direction("تقرير", None);
        assert_eq!(rtl.rows.len(), 1);
        assert_eq!(rtl.end().index.0, 5);
        assert_eq!(
            rtl.cursor_left_one_character(&rtl.begin()).index.0,
            1,
            "visual left from the reading start enters the text"
        );
        assert_eq!(
            rtl.cursor_right_one_character(&rtl.begin()).index.0,
            0,
            "the reading start is the visual right edge"
        );
        assert_eq!(
            rtl.cursor_right_one_character(&rtl.end()).index.0,
            4,
            "visual right from the reading end steps back into the text"
        );

        let ltr = layout_with_direction("abcde", Some(TextDirection::LeftToRight));
        assert_eq!(ltr.cursor_left_one_character(&ltr.begin()).index.0, 0);
        assert_eq!(ltr.cursor_right_one_character(&ltr.end()).index.0, 5);
    }

    /// T1 — mixed-direction rows step the caret by each embedded run's *local*
    /// bidi level, not by the row's base direction.
    ///
    /// Columns stay logical (0 = the reading start, anchored by
    /// [`Row::base_direction`]); only the stepping changes: inside a maximal
    /// run of one embedding level the caret moves one *visual* position per
    /// keypress (direction = the run's own parity), and crossing into the
    /// visually-adjacent run lands on the column of the shared visual
    /// boundary. Row-edge and degenerate cases fall back to the historic
    /// base-mirrored step.
    ///
    /// Oracles, hand-derived from the UAX #9 levels the shaper stamps (the
    /// boundary→column tables are in the plan):
    ///
    /// * `"عربي abc"`, RTL base — visual slots `a(2) b(2) c(2) ␣(1) ي(1) ب(1) ر(1) ع(1)`,
    ///   logical columns `ع=0 ر=1 ب=2 ي=3 ␣=4 a=5 b=6 c=7`. Visual left from
    ///   the reading end steps *into* the embedded LTR run, visual right from
    ///   the reading end teleports across the run boundary onto the Arabic
    ///   run's reading-exit caret (the space), and the run-edge guards fall
    ///   back to the historic step.
    /// * `"abc عربي"`, LTR base — the exact vice-versa.
    /// * `"تقرير"` pinned to an explicit LTR base (an all-odd row in an even
    ///   base; only explicit-direction jobs produce this) — the mirrored
    ///   vice-versa of the `Field`-stamp defect shape: visual left from the
    ///   reading start enters the text instead of clamping.
    /// * `"Hello"`, RTL base (an all-even row in an odd base) — the
    ///   Latin-value-under-RTL-chrome shape every `TextEdit` field hits:
    ///   `ArrowLeft` retreats one character (`left(5) == 4`).
    /// * `"اب١٢"`, LTR base — nested levels (Arabic-Indic digits continue at
    ///   the higher even level inside an RTL run inside an even base): the
    ///   caret steps visually right from the shared ا|ب boundary onto ا's
    ///   right edge (`right(1) == 0`).
    ///
    /// Uniform rows (pure LTR, pure RTL, empty) never reach the local-level
    /// table — the byte-identity guards at the bottom pin the historic step.
    #[test]
    fn mixed_direction_rows_step_by_local_level() {
        let at = |column: usize| CCursor::new(column);

        // "عربي abc", RTL base (an embedded LTR run in an RTL row):
        let galley = layout_with_direction("عربي abc", Some(TextDirection::RightToLeft));
        assert!(galley.is_rtl(), "the row's base direction is RTL");
        assert_eq!(galley.end().index.0, 8, "one glyph slot per char");
        // Visual left from the reading end steps into the embedded LTR run:
        assert_eq!(
            galley.cursor_left_one_character(&at(8)).index.0,
            7,
            "visual left from the reading end enters the embedded LTR run"
        );
        assert_eq!(galley.cursor_left_one_character(&at(7)).index.0, 6);
        assert_eq!(galley.cursor_left_one_character(&at(6)).index.0, 5);
        // Visual left from the LTR run's left edge leaves the row's table
        // (the shared boundary is the row's visual left edge): falls back to
        // the historic step (logically next under an RTL base):
        assert_eq!(
            galley.cursor_left_one_character(&at(5)).index.0,
            6,
            "visual left from the LTR run's left edge falls back to the historic step"
        );
        // Visual right walks back through the LTR run, then teleports across
        // the run boundary onto the Arabic run's reading-exit caret:
        assert_eq!(
            galley.cursor_right_one_character(&at(5)).index.0,
            6,
            "visual right from the LTR run's left edge re-enters the run"
        );
        assert_eq!(galley.cursor_right_one_character(&at(6)).index.0, 7);
        assert_eq!(galley.cursor_right_one_character(&at(7)).index.0, 8);
        assert_eq!(
            galley.cursor_right_one_character(&at(8)).index.0,
            4,
            "visual right from the reading end teleports to the Arabic run's exit caret"
        );
        assert_eq!(
            galley.cursor_right_one_character(&at(4)).index.0,
            3,
            "visual right from the space caret steps into the Arabic run"
        );
        assert_eq!(
            galley.cursor_right_one_character(&at(0)),
            CCursor::default(),
            "visual right from the reading start clamps to the galley begin"
        );

        // First-strong resolution produces the same row (no explicit pin):
        let first_strong = layout_with_direction("عربي abc", None);
        assert!(first_strong.is_rtl(), "first-strong resolves RTL");
        assert_eq!(first_strong.cursor_left_one_character(&at(8)).index.0, 7);
        assert_eq!(first_strong.cursor_right_one_character(&at(8)).index.0, 4);

        // "abc عربي", LTR base (the exact vice-versa):
        let galley = layout_with_direction("abc عربي", Some(TextDirection::LeftToRight));
        assert!(!galley.is_rtl(), "the row's base direction is LTR");
        assert_eq!(galley.end().index.0, 8, "one glyph slot per char");
        assert_eq!(
            galley.cursor_right_one_character(&at(3)).index.0,
            8,
            "visual right from the space caret teleports to the Arabic run's entry caret"
        );
        assert_eq!(galley.cursor_right_one_character(&at(8)).index.0, 7);
        assert_eq!(galley.cursor_right_one_character(&at(7)).index.0, 6);
        assert_eq!(
            galley.cursor_left_one_character(&at(4)).index.0,
            5,
            "visual left from the Arabic run's entry caret re-enters the run rightward"
        );
        assert_eq!(galley.cursor_left_one_character(&at(5)).index.0, 6);
        assert_eq!(
            galley.cursor_left_one_character(&at(8)).index.0,
            3,
            "visual left from the Arabic run's reading end teleports back to the LTR run"
        );
        assert_eq!(
            galley.cursor_right_one_character(&at(4)).index.0,
            5,
            "visual right from the Arabic run's entry edge falls back to the historic step"
        );
        assert_eq!(
            galley.cursor_left_one_character(&at(0)),
            CCursor::default(),
            "visual left from the reading start clamps to the galley begin"
        );

        // "تقرير" pinned to an explicit LTR base (an all-odd row in an even
        // base — no egui call site produces this today; explicit-direction
        // jobs do). RED at base on the first two (today: 0 and 5, the clamps):
        let galley = layout_with_direction("تقرير", Some(TextDirection::LeftToRight));
        assert_eq!(galley.end().index.0, 5);
        assert_eq!(
            galley.cursor_left_one_character(&at(0)).index.0,
            1,
            "visual left from the reading start enters the all-odd row"
        );
        assert_eq!(
            galley.cursor_right_one_character(&at(5)).index.0,
            4,
            "visual right from the reading end steps back into the all-odd row"
        );
        // Row-edge guards stay on the historic step via the fallback:
        assert_eq!(galley.cursor_right_one_character(&at(0)).index.0, 1);
        assert_eq!(galley.cursor_left_one_character(&at(5)).index.0, 4);

        // "Hello", RTL base (an all-even row in an odd base — the task-467
        // shape: every Latin/digit value in an RTL UI):
        let galley = layout_with_direction("Hello", Some(TextDirection::RightToLeft));
        assert!(galley.is_rtl());
        assert_eq!(galley.end().index.0, 5);
        assert_eq!(
            galley.cursor_left_one_character(&at(5)).index.0,
            4,
            "ArrowLeft retreats one character from the end"
        );
        assert_eq!(galley.cursor_right_one_character(&at(4)).index.0, 5);
        assert_eq!(galley.cursor_left_one_character(&at(4)).index.0, 3);
        assert_eq!(galley.cursor_right_one_character(&at(3)).index.0, 4);

        // "اب١٢", LTR base (nested levels: the Arabic-Indic digits continue
        // at the higher even level inside the RTL run). RED at base on the
        // first assert (today: 2):
        let galley = layout_with_direction("اب١٢", Some(TextDirection::LeftToRight));
        assert_eq!(galley.end().index.0, 4);
        assert_eq!(
            galley.cursor_right_one_character(&at(1)).index.0,
            0,
            "visual right from the shared ا|ب boundary lands on ا's right edge"
        );
        assert_eq!(
            galley.cursor_right_one_character(&at(0)).index.0,
            1,
            "visual right from the row edge falls back to the historic step"
        );

        // Uniform byte-identity guards: the fast path keeps the historic step
        // for every base-parity-matching row (these all pass at base too):
        let rtl_uniform = layout_with_direction("تقرير", Some(TextDirection::RightToLeft));
        assert_eq!(rtl_uniform.cursor_left_one_character(&at(0)).index.0, 1);
        assert_eq!(rtl_uniform.cursor_right_one_character(&at(0)).index.0, 0);
        assert_eq!(rtl_uniform.cursor_right_one_character(&at(5)).index.0, 4);
        let ltr_uniform = layout_with_direction("abcde", Some(TextDirection::LeftToRight));
        assert_eq!(ltr_uniform.cursor_left_one_character(&at(0)).index.0, 0);
        assert_eq!(ltr_uniform.cursor_right_one_character(&at(5)).index.0, 5);
    }

    /// T6 — hostile input must not panic: unpaired-surrogate stand-ins, control
    /// and isolate characters, direction-mark soup, huge direction mixes, and
    /// wrapping/elision of the same, across all three direction settings.
    #[test]
    fn bidi_hostile_input_does_not_panic() {
        let hostile: Vec<String> = vec![
            String::new(),
            "\u{FFFD}".into(),
            "\u{FFFD}\u{FFFD}a\u{FFFD}".into(),
            "a\u{0301}".into(),
            "\u{0301}a".into(),
            "\u{061C}".into(),
            "\u{200F}\u{200E}".repeat(50),
            "aרbعc".repeat(40),
            "\u{2066}تقرير\u{2069} pdf \u{2067}שלום\u{2069}".into(),
            "\r\n\t\u{2029}\u{0085}".into(),
            "…".repeat(100),
            "تقرير ٢٠٢٦.pdf ".repeat(20),
            "أول ٢٠٢٦ ثالث".into(),
            "\u{2066}أول\u{2069} \u{2067}ثاني\u{2069} ثالث \u{200F}\u{200E}".repeat(6),
        ];
        let directions = [
            None,
            Some(TextDirection::LeftToRight),
            Some(TextDirection::RightToLeft),
        ];

        for text in &hostile {
            for direction in directions {
                // No wrap:
                let unwrapped = layout_with_direction(text, direction);
                let _ = unwrapped.end();

                // Wrapped + elided (exercises line_break and both elision mirrors):
                let mut fonts = test_fonts();
                let mut job = LayoutJob::simple(
                    text.clone(),
                    FontId::proportional(14.0),
                    Color32::WHITE,
                    100.0,
                );
                job.direction = direction;
                job.wrap.max_rows = 5;
                job.wrap.overflow_character = Some('…');
                let galley = layout(&mut fonts, 1.0, job.into());

                // One glyph slot per char, preserved by the logical wrap
                // path when it produced all the rows (no truncation).
                // Control characters other than tab are dropped by the
                // shaper (pre-existing), so they are excluded:
                let droppable = text.chars().any(|c| c.is_control() && c != '\t');
                if !galley.elided && !droppable {
                    let total: usize = galley
                        .rows
                        .iter()
                        .map(|placed| placed.row.glyphs.len())
                        .sum();
                    assert_eq!(
                        total,
                        text.chars().count(),
                        "glyph slots must equal the char count for {text:?} (direction {direction:?})"
                    );
                }

                // Smoke the cursor paths on both galleys:
                let _ = unwrapped.cursor_from_pos(vec2(0.0, 0.0));
                let _ = galley.end();
                let _ = galley.cursor_from_pos(vec2(0.0, 0.0));
                let _ = galley.pos_from_cursor(galley.end());

                // Stepping the caret one visual position must stay panic-free
                // and clamped to the galley end on hostile input: walk both
                // arrow directions from both reading edges, on the unwrapped
                // and the wrapped+elided layouts. Row-edge steps fall back to
                // the historic base-mirrored step, so every walk terminates
                // in a clamp; none may ever exceed the galley's end.
                let max_steps = text.chars().count().saturating_add(2).min(64);
                for walked in [&unwrapped, &galley] {
                    let end = walked.end().index;
                    let mut left_from_begin = walked.begin();
                    let mut left_from_end = walked.end();
                    let mut right_from_begin = walked.begin();
                    let mut right_from_end = walked.end();
                    for _ in 0..max_steps {
                        left_from_begin = walked.cursor_left_one_character(&left_from_begin);
                        left_from_end = walked.cursor_left_one_character(&left_from_end);
                        right_from_begin = walked.cursor_right_one_character(&right_from_begin);
                        right_from_end = walked.cursor_right_one_character(&right_from_end);
                        assert!(
                            left_from_begin.index <= end
                                && left_from_end.index <= end
                                && right_from_begin.index <= end
                                && right_from_end.index <= end,
                            "arrow stepping must stay within the galley for {text:?} (direction {direction:?})"
                        );
                    }
                }
            }
        }

        // Two-section wrapped RTL jobs (both section orders, mixed with a
        // pure-LTR section — the unrecorded-edge shape) through the same
        // wrap + elision gauntlet:
        let formats = [
            TextFormat::simple(FontId::proportional(14.0), Color32::RED),
            TextFormat::simple(FontId::proportional(14.0), Color32::WHITE),
        ];
        for section_texts in [
            ["تقرير ٢٠٢٦", " aaa bbb الأول ثاني"],
            ["abcd", " عربي aaa \u{2066}שלום\u{2069} pdf"],
        ] {
            let mut text = String::new();
            let mut sections = Vec::new();
            for (i, section_text) in section_texts.iter().enumerate() {
                let start = ByteIndex(text.len());
                text.push_str(section_text);
                sections.push(LayoutSection {
                    leading_space: 0.0,
                    byte_range: start..ByteIndex(text.len()),
                    format: formats[i].clone(),
                });
            }
            for direction in directions {
                let mut job = LayoutJob {
                    sections: sections.clone(),
                    text: text.clone(),
                    wrap: TextWrapping {
                        max_width: 100.0,
                        max_rows: 5,
                        break_anywhere: true,
                        ..Default::default()
                    },
                    break_on_newline: true,
                    ..Default::default()
                };
                job.direction = direction;
                job.wrap.overflow_character = Some('…');
                let mut fonts = test_fonts();
                let galley = layout(&mut fonts, 1.0, job.into());
                let _ = galley.end();
                let _ = galley.cursor_from_pos(vec2(0.0, 0.0));
                let _ = galley.pos_from_cursor(galley.end());
            }
        }
    }

    /// Shaped complex scripts emit glyph vectors that are NOT sorted by x:
    /// zero-advance continuation placeholders at both ends and real glyphs
    /// with negative x offsets in between. `halign_and_justify_row` used to
    /// take the ink span from only the first and last glyph, so a right- or
    /// center-aligned row (what an RTL label gets) computed a too-narrow
    /// width and left its ink positioned outside the allocated rect —
    /// button hover fills then failed to cover the ink.
    ///
    /// No Arabic font is bundled (fonts are never committed), so this test
    /// builds a row by hand with the geometry measured from shaping the
    /// Arabic word "ملف" with a real Arabic font (Noto Sans Arabic, 14 px):
    /// a placeholder at 0, the ligature glyph drawn 10 px left of the pen,
    /// the remaining glyphs, and trailing placeholders at the natural width.
    #[test]
    fn halign_uses_full_ink_span_of_non_monotonic_glyph_vector() {
        /// Only self-consistent metrics are needed: `logical_rect` spans
        /// `[pos.x, pos.x + advance_width]` around the baseline.
        fn glyph(chr: char, x: f32, advance: f32) -> Glyph {
            Glyph {
                chr,
                pos: pos2(x, 0.0),
                advance_width: advance,
                line_height: 16.0,
                font_ascent: 13.0,
                font_height: 16.0,
                font_face_ascent: 13.0,
                font_face_height: 16.0,
                uv_rect: Default::default(),
                is_color: false,
                bidi_level: 0,
                section_index: 0,
                first_vertex: 0,
            }
        }

        fn placed_row(
            glyphs: Vec<Glyph>,
            natural_width: f32,
            base_direction: TextDirection,
            leading_advance: f32,
        ) -> PlacedRow {
            PlacedRow {
                pos: pos2(0.0, 0.0),
                row: Arc::new(Row {
                    section_index_at_start: 0,
                    base_direction,
                    leading_advance,
                    glyphs,
                    size: vec2(natural_width, 16.0),
                    visuals: RowVisuals::default(),
                }),
                ends_with_newline: false,
            }
        }

        fn assert_close(got: f32, want: f32) {
            assert!((got - want).abs() < 1e-3, "got {got:.4}, want {want:.4}");
        }

        fn ink_span(row: &Row) -> (f32, f32) {
            (
                row.glyphs
                    .iter()
                    .map(|glyph| glyph.logical_rect().min.x)
                    .fold(f32::INFINITY, f32::min),
                row.glyphs
                    .iter()
                    .map(|glyph| glyph.logical_rect().max.x)
                    .fold(f32::NEG_INFINITY, f32::max),
            )
        }

        /// The measured row-relative geometry of an Arabic "ملف" label,
        /// in visual (left-to-right) order: a zero-advance placeholder,
        /// the ligature glyph drawn left of the pen (negative offset),
        /// the remaining glyphs, trailing zero-advance continuations.
        fn shaped_arabic() -> Vec<Glyph> {
            vec![
                glyph('ف', 0.0, 0.0),
                glyph('ف', -10.0, 15.37),
                glyph('ل', 5.0, 4.2),
                glyph('م', 9.0, 7.35),
                glyph('ل', 17.0, 0.0),
                glyph('ف', 17.0, 0.0),
            ]
        }

        let natural_width: f32 = shaped_arabic().iter().map(|g| g.advance_width).sum();
        // The first glyph starts at 0 and the last ends at 17, but the real
        // ink spans [-10, 17]: the vector is non-monotonic and the endpoints
        // do not bracket it. Sanity-check the premise:
        let (ink_min, ink_max) = ink_span(&Row {
            section_index_at_start: 0,
            base_direction: TextDirection::LeftToRight,
            leading_advance: 0.0,
            glyphs: shaped_arabic(),
            size: vec2(natural_width, 16.0),
            visuals: RowVisuals::default(),
        });
        assert_close(ink_min, -10.0);
        assert_close(ink_max, 17.0);

        let point_scale = PointScale::new(1.0);

        // `Align::Max` is what a right-to-left label is laid out with. The
        // row must be anchored as if 27 wide (the true ink span), not 17:
        let mut placed = placed_row(
            shaped_arabic(),
            natural_width,
            TextDirection::LeftToRight,
            0.0,
        );
        halign_and_justify_row(
            point_scale,
            &mut placed,
            Align::Max,
            f32::INFINITY,
            false,
            false,
        );
        assert_close(placed.pos.x, -27.0);
        // The ink is repositioned to end at the galley origin (right
        // alignment) and to start exactly at the row's left edge — before
        // the fix it extended 10 px to the left of the allocated rect:
        let (ink_min, ink_max) = ink_span(&placed.row);
        assert_close(ink_min + placed.pos.x, placed.pos.x);
        assert_close(ink_max + placed.pos.x, 0.0);
        // The allocation keeps the natural width (trailing-whitespace
        // semantics), so inter-widget spacing is unchanged:
        assert_close(placed.row.size.x, natural_width);

        // Centered rows must center the true ink on the galley origin:
        let mut placed = placed_row(
            shaped_arabic(),
            natural_width,
            TextDirection::LeftToRight,
            0.0,
        );
        halign_and_justify_row(
            point_scale,
            &mut placed,
            Align::Center,
            f32::INFINITY,
            false,
            false,
        );
        assert_close(placed.pos.x, -14.0); // round_to_pixel(-13.5)
        let (ink_min, ink_max) = ink_span(&placed.row);
        // Centered on the galley origin, up to pixel rounding:
        assert!(((ink_min + ink_max) * 0.5 + placed.pos.x).abs() <= 0.5);

        // A monotonic row (endpoints bracket the ink) is laid out exactly as
        // before the fix:
        let mut placed = placed_row(
            vec![glyph('a', 0.0, 17.0)],
            17.0,
            TextDirection::LeftToRight,
            0.0,
        );
        halign_and_justify_row(
            point_scale,
            &mut placed,
            Align::Max,
            f32::INFINITY,
            false,
            false,
        );
        assert_close(placed.pos.x, -17.0);
        assert_close(placed.row.size.x, 17.0);
        assert_close(placed.row.glyphs[0].pos.x, 0.0);

        // Composition with the trailing-whitespace semantics: a trailing
        // space is trimmed from the ink range (the row stays anchored on the
        // ink, at -27) while the allocation keeps counting it:
        let mut with_trailing_space = shaped_arabic();
        with_trailing_space.push(glyph(' ', 17.0, 3.0));
        let natural_with_space = natural_width + 3.0;

        let mut placed = placed_row(
            with_trailing_space.clone(),
            natural_with_space,
            TextDirection::LeftToRight,
            0.0,
        );
        halign_and_justify_row(
            point_scale,
            &mut placed,
            Align::Max,
            f32::INFINITY,
            false,
            false,
        );
        assert_close(placed.pos.x, -27.0);
        assert_close(placed.row.size.x, natural_with_space);

        // With `keep_trailing_whitespace` the space is part of the aligned
        // span, so the row is anchored on [-10, 20] and sized to it:
        let mut placed = placed_row(
            with_trailing_space,
            natural_with_space,
            TextDirection::LeftToRight,
            0.0,
        );
        halign_and_justify_row(
            point_scale,
            &mut placed,
            Align::Max,
            f32::INFINITY,
            false,
            true,
        );
        assert_close(placed.pos.x, -30.0);
        assert_close(placed.row.size.x, 30.0);
    }

    // -------------------------------------------------------------------------
    // Leading-advance threading (the row field, the anchoring inset and the
    // two wrap paths). Red-at-base evidence: see the commit messages — T1's
    // base overlay form bakes the advance into the glyph positions (the field
    // does not exist at base) and fails on the same ink-offset assertion.
    //
    // No fonts needed: `halign_and_justify_row` anchors from glyph-relative
    // ink, and `line_break`/`line_break_logically` are called directly on
    // hand-built paragraphs (they are in-module).

    fn advance_glyph(chr: char, x: f32, advance: f32, bidi_level: u8) -> Glyph {
        Glyph {
            chr,
            pos: pos2(x, 0.0),
            advance_width: advance,
            line_height: 16.0,
            font_ascent: 13.0,
            font_height: 16.0,
            font_face_ascent: 13.0,
            font_face_height: 16.0,
            uv_rect: Default::default(),
            is_color: false,
            bidi_level,
            section_index: 0,
            first_vertex: 0,
        }
    }

    fn advance_assert_close(got: f32, want: f32) {
        assert!((got - want).abs() < 1e-3, "got {got:.4}, want {want:.4}");
    }

    fn advance_ink_span(row: &Row) -> (f32, f32) {
        (
            row.glyphs
                .iter()
                .map(|glyph| glyph.logical_rect().min.x)
                .fold(f32::INFINITY, f32::min),
            row.glyphs
                .iter()
                .map(|glyph| glyph.logical_rect().max.x)
                .fold(f32::NEG_INFINITY, f32::max),
        )
    }

    /// The `shaped_arabic()` ink pattern ([-10, 17], non-monotonic) with the
    /// leading advance carried by the row: `natural_width = la + ink_span`.
    fn advance_row(base_direction: TextDirection, leading_advance: f32) -> PlacedRow {
        PlacedRow {
            pos: pos2(0.0, 0.0),
            row: Arc::new(Row {
                section_index_at_start: 0,
                base_direction,
                leading_advance,
                glyphs: vec![
                    advance_glyph('ف', 0.0, 0.0, 1),
                    advance_glyph('ف', -10.0, 15.37, 1),
                    advance_glyph('ل', 5.0, 4.2, 1),
                    advance_glyph('م', 9.0, 7.35, 1),
                    advance_glyph('ل', 17.0, 0.0, 1),
                    advance_glyph('ف', 17.0, 0.0, 1),
                ],
                size: vec2(leading_advance + 27.0, 16.0),
                visuals: RowVisuals::default(),
            }),
            ends_with_newline: false,
        }
    }

    /// A right-anchored right-to-left row keeps the leading advance in its
    /// INK at the reading start (its right edge): the anchored ink is inset
    /// by the advance instead of being absorbed into the row box. This is
    /// the unit pin of the mechanism the wrapped-RTL label tests ride; the
    /// falsified behavior (advance swallowed, ink right at 0.0) is what the
    /// base overlay observed.
    #[test]
    fn right_anchored_rtl_row_keeps_the_leading_advance_in_the_ink() {
        let mut placed = advance_row(TextDirection::RightToLeft, 40.0);
        halign_and_justify_row(
            PointScale::new(1.0),
            &mut placed,
            Align::RIGHT,
            f32::INFINITY,
            false,
            false,
        );

        // Galley-frame ink (row-local ink translated by `pos.x`): inset by
        // the advance at the reading start.
        let (ink_min, ink_max) = advance_ink_span(&placed.row);
        advance_assert_close(ink_max + placed.pos.x, -40.0);
        advance_assert_close(ink_min + placed.pos.x, -40.0 - 27.0);
        // The box right lands at the trailing wobble (zero here):
        advance_assert_close(placed.pos.x + placed.row.size.x, 0.0);
        // The size keeps the natural width:
        advance_assert_close(placed.row.size.x, 67.0);
    }

    /// The no-blast-radius condition: a zero leading advance leaves the
    /// anchoring byte-identical (ink right at the galley origin).
    #[test]
    fn zero_leading_advance_leaves_anchoring_unchanged() {
        let mut placed = advance_row(TextDirection::RightToLeft, 0.0);
        halign_and_justify_row(
            PointScale::new(1.0),
            &mut placed,
            Align::RIGHT,
            f32::INFINITY,
            false,
            false,
        );

        let (ink_min, ink_max) = advance_ink_span(&placed.row);
        advance_assert_close(ink_max + placed.pos.x, 0.0);
        advance_assert_close(ink_min + placed.pos.x, -27.0);
    }

    /// Left-to-right rows ignore the reading-start inset (the advance stays
    /// absorbed, exactly as before) — the epaint side of LTR byte-identity.
    #[test]
    fn ltr_row_ignores_the_reading_start_inset() {
        let mut placed = advance_row(TextDirection::LeftToRight, 40.0);
        halign_and_justify_row(
            PointScale::new(1.0),
            &mut placed,
            Align::RIGHT,
            f32::INFINITY,
            false,
            false,
        );

        let (ink_min, ink_max) = advance_ink_span(&placed.row);
        advance_assert_close(ink_max + placed.pos.x, 0.0);
        advance_assert_close(ink_min + placed.pos.x, -27.0);
    }

    /// Center-anchored rows still absorb the advance (the documented
    /// residual: which edge would a Center inset even use?).
    #[test]
    fn center_anchored_rows_still_absorb_the_advance() {
        let mut placed = advance_row(TextDirection::RightToLeft, 40.0);
        halign_and_justify_row(
            PointScale::new(1.0),
            &mut placed,
            Align::Center,
            f32::INFINITY,
            false,
            false,
        );

        // Centered on the galley origin, up to pixel rounding:
        let (ink_min, ink_max) = advance_ink_span(&placed.row);
        assert!(((ink_min + ink_max) * 0.5 + placed.pos.x).abs() <= 0.5);
    }

    /// The legacy wrap path threads the paragraph's leading advance onto the
    /// row that carries the paragraph's first glyph — the advance stays in
    /// that row's glyphs (production leaves it in the first glyph's position)
    /// and in its `leading_advance` — and every later row is re-based without
    /// it. The empty-first-row escape consumes the advance: no row keeps it.
    #[test]
    fn line_break_threads_the_leading_advance_onto_the_first_glyph_row() {
        // "ab cd ef" with a 40 px indent in the first glyph's position, as
        // production leaves it (the `leading_space` cursor advance precedes
        // shaping).
        let glyphs = vec![
            advance_glyph('a', 40.0, 10.0, 0),
            advance_glyph('b', 50.0, 10.0, 0),
            advance_glyph(' ', 60.0, 5.0, 0),
            advance_glyph('c', 65.0, 10.0, 0),
            advance_glyph('d', 75.0, 10.0, 0),
            advance_glyph(' ', 85.0, 5.0, 0),
            advance_glyph('e', 90.0, 10.0, 0),
            advance_glyph('f', 100.0, 10.0, 0),
        ];
        let mut paragraph = Paragraph::from_section_index(0);
        paragraph.leading_advance = 40.0;
        paragraph.glyphs = glyphs;
        paragraph.byte_span = 0..8;

        let job = LayoutJob {
            text: "ab cd ef".to_owned(),
            wrap: TextWrapping {
                max_width: 65.0,
                ..Default::default()
            },
            direction: Some(TextDirection::RightToLeft),
            round_output_to_gui: false,
            ..Default::default()
        };

        // At width 65 the first break lands on the first space (after the
        // words "ab" fit beside the indent): row 0 carries the advance.
        let mut rows = Vec::new();
        let mut elided = false;
        line_break(&paragraph, &job, &[], &mut rows, &mut elided);
        assert!(!elided);
        assert_eq!(
            rows.len(),
            2,
            "two rows: {:?}",
            rows.iter().map(|r| r.size.x).collect::<Vec<_>>()
        );
        advance_assert_close(rows[0].leading_advance, 40.0);
        advance_assert_close(rows[0].glyphs[0].pos.x, 40.0);
        advance_assert_close(rows[1].leading_advance, 0.0);
        advance_assert_close(rows[1].glyphs[0].pos.x, 0.0);

        // Below one word plus the indent, the empty-first-row escape fires
        // and consumes the advance: every row lands at 0.0 (mirrors the LTR
        // arm's pre-existing loss of the indent in this edge case).
        let narrow = LayoutJob {
            wrap: TextWrapping {
                max_width: 45.0,
                ..Default::default()
            },
            ..job.clone()
        };
        let mut rows = Vec::new();
        let mut elided = false;
        line_break(&paragraph, &narrow, &[], &mut rows, &mut elided);
        assert!(!elided);
        assert!(rows.len() >= 2);
        assert!(
            rows.iter().all(|placed| placed.leading_advance == 0.0),
            "the escape consumes the advance: {:?}",
            rows.iter().map(|r| r.leading_advance).collect::<Vec<_>>()
        );
    }

    /// The same threading rule on the logical wrap path (`place_row`): the
    /// reading-first row (block range starting at 0) receives the advance as
    /// its stacking-pen origin — the advance survives in its glyphs AND in
    /// its `size.x` — and rows after a break or after the empty-first-row
    /// escape carry 0.0. This is the unit pin behind the wrapped-Arabic
    /// label scenario (which rides this path via `wraps_logically`).
    #[test]
    fn line_break_logically_threads_the_leading_advance_onto_the_reading_first_row() {
        // A pure-RTL paragraph ("أول ثاني"), shaped in visual order: stream
        // glyph 0 is the visually-leftmost (logically-last) char. The
        // advance shifted every glyph right by 40, and the recorded run
        // covers the whole paragraph — `wraps_logically`'s route. The
        // glyphs and their cluster byte starts are emitted in tandem (as
        // `track_cluster_bytes` records them), so the paragraph is TRACKED
        // — the state `layout`'s gate guarantees for every text that
        // records runs, and what the records-runs-implies-tracked assert
        // in `line_break_logically` requires.
        let logical = "أول ثاني";
        let mut glyphs = Vec::new();
        let mut cluster_byte_starts = Vec::new();
        for (i, (byte_start, chr)) in logical.char_indices().rev().enumerate() {
            cluster_byte_starts.push(byte_start);
            glyphs.push(advance_glyph(chr, 40.0 + 10.0 * i as f32, 10.0, 1));
        }
        let mut paragraph = Paragraph::from_section_index(0);
        paragraph.leading_advance = 40.0;
        paragraph.glyphs = glyphs;
        paragraph.cluster_byte_starts = cluster_byte_starts;
        paragraph.byte_span = 0..15;
        paragraph.bidi_runs = vec![RecordedBidiRun {
            glyphs: 0..8,
            bytes: 0..15,
            rtl: true,
        }];

        let job = LayoutJob {
            text: "أول ثاني".to_owned(),
            wrap: TextWrapping {
                max_width: 115.0,
                ..Default::default()
            },
            direction: Some(TextDirection::RightToLeft),
            round_output_to_gui: false,
            ..Default::default()
        };

        // At width 115 the pen (starting at the indent) overflows on the
        // last word: the break lands on the space block, row 0 is the
        // reading-first row ("أول "), the final row is "ثاني".
        let mut rows = Vec::new();
        let mut elided = false;
        line_break_logically(&paragraph, &job, &[], &mut rows, &mut elided);
        assert!(!elided);
        assert_eq!(
            rows.len(),
            2,
            "two rows: {:?}",
            rows.iter().map(|r| r.size.x).collect::<Vec<_>>()
        );
        advance_assert_close(rows[0].leading_advance, 40.0);
        // The advance is preserved in the row's glyphs (the pen started at
        // it) and counted exactly once in its size:
        advance_assert_close(
            rows[0]
                .glyphs
                .iter()
                .map(|glyph| glyph.pos.x)
                .fold(f32::INFINITY, f32::min),
            40.0,
        );
        advance_assert_close(rows[0].size.x, 80.0);
        advance_assert_close(rows[1].leading_advance, 0.0);
        advance_assert_close(
            rows[1]
                .glyphs
                .iter()
                .map(|glyph| glyph.pos.x)
                .fold(f32::INFINITY, f32::min),
            0.0,
        );
        advance_assert_close(rows[1].size.x, 40.0);

        // Below one word plus the indent, the empty-first-row escape fires
        // and consumes the advance: every row lands at 0.0.
        let narrow = LayoutJob {
            wrap: TextWrapping {
                max_width: 45.0,
                ..Default::default()
            },
            ..job.clone()
        };
        let mut rows = Vec::new();
        let mut elided = false;
        line_break_logically(&paragraph, &narrow, &[], &mut rows, &mut elided);
        assert!(!elided);
        assert!(rows.len() >= 2);
        assert!(
            rows.iter().all(|placed| placed.leading_advance == 0.0),
            "the escape consumes the advance: {:?}",
            rows.iter().map(|r| r.leading_advance).collect::<Vec<_>>()
        );
    }

    // -------------------------------------------------------------------------
    // NG9 wrapped-row oracles (fail-before evidence: see the commit messages).
    //
    // All wrap-width measurements come from an LTR-pinned twin of the oracle
    // job (same text, `round_output_to_gui = false`, infinite width): the
    // twin's halign pass is skipped entirely (LTR jobs are not flipped), so
    // its glyph x values are exactly the emitted values the wrap compares,
    // and every measured quantity is a span or gap, never an absolute x.
    // The synthetic tofu fonts serve every glyph with an advance that is a
    // whole number of physical pixels, so the twin's spans are bit-identical
    // to the widths the oracle's wrap compares. Each intended row's width is
    // its content INCLUDING its trailing break space; `W` is the maximum of
    // those over all intended rows (the final row contributes its bare
    // content) plus 0.5 px, with per-boundary asserts.

    /// The visible glyphs of every row of a galley as `(chr, x, advance)`.
    fn visible_glyphs_per_row(galley: &Galley) -> Vec<Vec<(char, f32, f32)>> {
        galley
            .rows
            .iter()
            .map(|placed| {
                placed
                    .row
                    .glyphs
                    .iter()
                    .filter(|g| g.advance_width > 0.0)
                    .map(|g| (g.chr, g.pos.x, g.advance_width))
                    .collect()
            })
            .collect()
    }

    /// The visible non-whitespace glyphs of every row, in stored (visual)
    /// order.
    fn visible_chars_per_row(galley: &Galley) -> Vec<Vec<char>> {
        galley
            .rows
            .iter()
            .map(|placed| {
                placed
                    .row
                    .glyphs
                    .iter()
                    .filter(|g| g.advance_width > 0.0 && !g.chr.is_whitespace())
                    .map(|g| g.chr)
                    .collect()
            })
            .collect()
    }

    /// One row with no wrapping, from the synthetic tofu fonts at ppp 1.0
    /// (every advance a whole number of physical pixels).
    fn layout_unwrapped(text: &str, direction: Option<TextDirection>) -> Galley {
        let mut fonts = tofu_fonts();
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        job.direction = direction;
        job.round_output_to_gui = false;
        layout(&mut fonts, 1.0, job.into())
    }

    /// Lay out with a finite wrap width, from the synthetic tofu fonts.
    fn layout_wrapped(text: &str, direction: Option<TextDirection>, wrap_width: f32) -> Galley {
        let mut fonts = tofu_fonts();
        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            wrap_width,
        );
        job.direction = direction;
        job.round_output_to_gui = false;
        layout(&mut fonts, 1.0, job.into())
    }

    /// The whitespace-separated visible runs of a single-row stream: each
    /// run's span (last `max_x` − first x) and the gaps between consecutive
    /// runs (next run's first x − previous run's last `max_x`). A run boundary
    /// is a gap in the stream (the filtered-out space's advance).
    fn visible_runs(galley: &Galley) -> (Vec<f32>, Vec<f32>) {
        assert_eq!(galley.rows.len(), 1, "expected a single-row galley");
        let glyphs: Vec<(char, f32, f32)> = galley.rows[0]
            .row
            .glyphs
            .iter()
            .filter(|g| g.advance_width > 0.0 && !g.chr.is_whitespace())
            .map(|g| (g.chr, g.pos.x, g.advance_width))
            .collect();
        let mut spans = Vec::new();
        let mut gaps = Vec::new();
        let mut run_first = 0usize;
        for i in 1..=glyphs.len() {
            let boundary =
                i == glyphs.len() || glyphs[i].1 - (glyphs[i - 1].1 + glyphs[i - 1].2) > 0.5;
            if boundary {
                spans.push(glyphs[i - 1].1 + glyphs[i - 1].2 - glyphs[run_first].1);
                if i < glyphs.len() {
                    gaps.push(glyphs[i].1 - (glyphs[i - 1].1 + glyphs[i - 1].2));
                }
                run_first = i;
            }
        }
        (spans, gaps)
    }

    /// A — a wrapped pure-RTL paragraph reads top-down: one word per row, in
    /// logical order. Fail-before: the old path sliced the emitted visual
    /// stream from its visually-left (logically-last) end, producing the rows
    /// bottom-up (`ثالث` / `ثاني` / `أول`).
    #[test]
    fn wrapped_pure_rtl_reads_top_down() {
        let text = "أول ثاني ثالث";

        // Wrap width measured from the LTR twin (a single run emits the same
        // stream in both bases): `ثالث ␣ ثاني ␣ أول`, so run 0 = ثالث,
        // run 1 = ثاني, run 2 = أول.
        let twin = layout_unwrapped(text, Some(TextDirection::LeftToRight));
        let (spans, gaps) = visible_runs(&twin);
        assert_eq!(
            spans.len(),
            3,
            "the twin's stream is three words: {spans:?}"
        );
        assert_eq!(gaps.len(), 2, "spans {spans:?} gaps {gaps:?}");
        let (w_awwal, w_thani, w_thalith) = (spans[2], spans[1], spans[0]);
        let s = gaps[0];
        assert!(
            (gaps[1] - s).abs() < 1e-3,
            "the tofu space advance is uniform: {spans:?} {gaps:?}"
        );
        let wrap_width = (w_awwal + s).max(w_thani + s).max(w_thalith) + 0.5;
        // Per-boundary asserts: each intended row's content + break space
        // fits, the next word overflows, and the final row's bare content
        // fits:
        assert!(
            w_awwal + s <= wrap_width && w_awwal + s + w_thani > wrap_width,
            "spans {spans:?} gaps {gaps:?} W {wrap_width}"
        );
        assert!(
            w_thani + s <= wrap_width && w_thani + s + w_thalith > wrap_width,
            "spans {spans:?} gaps {gaps:?} W {wrap_width}"
        );
        assert!(w_thalith <= wrap_width, "W {wrap_width}");

        for direction in [Some(TextDirection::RightToLeft), None] {
            let galley = layout_wrapped(text, direction, wrap_width);
            assert_eq!(galley.rows.len(), 3, "one word per row");
            let chars = visible_chars_per_row(&galley);
            assert_eq!(chars[0], ['ل', 'و', 'أ'], "row 0 renders أول: {chars:?}");
            assert_eq!(
                chars[1],
                ['ي', 'ن', 'ا', 'ث'],
                "row 1 renders ثاني: {chars:?}"
            );
            assert_eq!(
                chars[2],
                ['ث', 'ل', 'ا', 'ث'],
                "row 2 renders ثالث: {chars:?}"
            );
            // Row invariant: x non-decreasing, and one glyph slot per char of
            // the row's logical slice (the word plus its trailing break
            // space):
            for glyphs in visible_glyphs_per_row(&galley) {
                assert_x_nondecreasing(&glyphs);
            }
            let counts: Vec<usize> = galley
                .rows
                .iter()
                .map(|placed| placed.row.glyphs.len())
                .collect();
            assert_eq!(counts, vec![4, 5, 4], "glyph slots per row: {counts:?}");
        }

        // Two words then one: W = w(أول) + s + w(ثاني) + s + 0.5.
        let wrap_width2 = w_awwal + s + w_thani + s + 0.5;
        assert!(w_awwal + s + w_thani + s <= wrap_width2);
        assert!(w_awwal + s + w_thani + s + w_thalith > wrap_width2);
        assert!(w_thalith <= wrap_width2);
        let galley = layout_wrapped(text, Some(TextDirection::RightToLeft), wrap_width2);
        assert_eq!(galley.rows.len(), 2);
        let chars = visible_chars_per_row(&galley);
        // Row 0 reads `أول ثاني` right-to-left: أول at the visual RIGHT, so
        // the stored visual order is ثاني's glyphs first:
        assert_eq!(chars[0], ['ي', 'ن', 'ا', 'ث', 'ل', 'و', 'أ'], "{chars:?}");
        assert_eq!(chars[1], ['ث', 'ل', 'ا', 'ث'], "{chars:?}");
        for glyphs in visible_glyphs_per_row(&galley) {
            assert_x_nondecreasing(&glyphs);
        }
    }

    /// Single-row byte-identity pins for the wrapped-oracle strings, captured
    /// from the pre-NG9 tree (tofu fonts, ppp 1.0, infinite width): the
    /// no-wrap early-out path must stay byte-identical across the wrap-path
    /// refactor.
    #[test]
    fn wrapped_oracle_strings_single_row_are_byte_identical() {
        let glyphs = visible_glyphs(&layout_unwrapped("أول ثاني ثالث", None));
        let expected: Vec<(char, f32, f32)> = [
            ('ث', 0.0, 9.0),
            ('ل', 9.0, 9.0),
            ('ا', 18.0, 9.0),
            ('ث', 27.0, 9.0),
            (' ', 36.0, 9.0),
            ('ي', 45.0, 9.0),
            ('ن', 54.0, 9.0),
            ('ا', 63.0, 9.0),
            ('ث', 72.0, 9.0),
            (' ', 81.0, 9.0),
            ('ل', 90.0, 9.0),
            ('و', 99.0, 9.0),
            ('أ', 108.0, 9.0),
        ]
        .into();
        assert_glyphs_close(&glyphs, &expected);

        let glyphs = visible_glyphs(&layout_unwrapped(
            "عربي aaa bbb",
            Some(TextDirection::RightToLeft),
        ));
        let expected: Vec<(char, f32, f32)> = [
            ('a', 0.0, 9.0),
            ('a', 9.0, 9.0),
            ('a', 18.0, 9.0),
            (' ', 27.0, 9.0),
            ('b', 36.0, 9.0),
            ('b', 45.0, 9.0),
            ('b', 54.0, 9.0),
            (' ', 63.0, 9.0),
            ('ي', 72.0, 9.0),
            ('ب', 81.0, 9.0),
            ('ر', 90.0, 9.0),
            ('ع', 99.0, 9.0),
        ]
        .into();
        assert_glyphs_close(&glyphs, &expected);
    }

    /// B — a wrapped mixed-direction RTL-base paragraph keeps UAX #9 L1/L2
    /// per rendered line: `عربي aaa bbb` (RTL-pinned) breaks at the logical
    /// space after `aaa`, and row 1 must render `aaa يبرع` (the latin island
    /// at the visual left, the Arabic word at its reading position), not the
    /// old path's geometric slicing of the whole-segment visual stream
    /// (`aaa bbb يبرع`), which stranded the Arabic word alone on the last
    /// row. Fail-before: rows came out `aaa bbb ` / `يبرع`.
    #[test]
    fn wrapped_mixed_rtl_base_keeps_uax9_per_row() {
        let text = "عربي aaa bbb";

        // Wrap width measured from the LTR twin: its stream is
        // `يبرع ␣ aaa ␣ bbb` (the RTL run reverses to the front), so
        // run 0 = يبرع, run 1 = aaa, run 2 = bbb. The intended row 1 is
        // `عربي aaa ␣` (content + trailing break space); the `aaa`→`bbb`
        // gap's space sits at an even level in both bases (level 0 in the
        // twin, level 2 inside the island in the oracle job), so its shaping
        // — and hence its advance — is identical in both.
        let twin = layout_unwrapped(text, Some(TextDirection::LeftToRight));
        let (spans, gaps) = visible_runs(&twin);
        assert_eq!(spans.len(), 3, "spans {spans:?}");
        assert_eq!(gaps.len(), 2, "spans {spans:?} gaps {gaps:?}");
        let (w_arabi, w_aaa, w_bbb) = (spans[0], spans[1], spans[2]);
        let s = gaps[1];
        let wrap_width = w_arabi + s + w_aaa + s + 0.5;
        // Per-boundary asserts: row 1's content + break space fits W, the
        // next word (bbb) overflows, and the final row's bare content fits:
        assert!(
            w_arabi + s + w_aaa + s <= wrap_width && w_arabi + s + w_aaa + s + w_bbb > wrap_width,
            "spans {spans:?} gaps {gaps:?} W {wrap_width}"
        );
        assert!(w_bbb <= wrap_width, "W {wrap_width}");

        let galley = layout_wrapped(text, Some(TextDirection::RightToLeft), wrap_width);
        assert_eq!(
            galley.rows.len(),
            2,
            "rows: {rows:?}",
            rows = visible_chars_per_row(&galley)
        );
        let chars = visible_chars_per_row(&galley);
        assert_eq!(chars[0], ['a', 'a', 'a', 'ي', 'ب', 'ر', 'ع'], "{chars:?}");
        assert_eq!(chars[1], ['b', 'b', 'b'], "{chars:?}");
        for glyphs in visible_glyphs_per_row(&galley) {
            assert_x_nondecreasing(&glyphs);
        }
        // One glyph slot per char of each row's logical slice (row 1 =
        // `عربي aaa ␣`, row 2 = `bbb`):
        let counts: Vec<usize> = galley
            .rows
            .iter()
            .map(|placed| placed.row.glyphs.len())
            .collect();
        assert_eq!(counts, vec![9, 3], "glyph slots per row: {counts:?}");

        // Trailing-space pin: the kept break-candidate space survives, is
        // L1-reset to the base level, and renders at the row's visual
        // reading end — the visually-LEFT slot of an RTL-base row (its
        // `pos.x` is at or before every non-space glyph's x). At an
        // under-sized wrap width this pin fails by design (the kept space
        // becomes the row-internal one).
        let row0 = &galley.rows[0];
        assert_eq!(
            row0.glyphs[0].chr, ' ',
            "the break space leads the row's vector"
        );
        let min_visible_x = row0
            .glyphs
            .iter()
            .filter(|glyph| glyph.advance_width > 0.0 && !glyph.chr.is_whitespace())
            .map(|glyph| glyph.pos.x)
            .fold(f32::INFINITY, f32::min);
        assert!(
            row0.glyphs[0].pos.x <= min_visible_x,
            "the L1-reset break space must sit at the visual reading end: {} vs {min_visible_x}",
            row0.glyphs[0].pos.x
        );
    }

    /// C — an LTR-base paragraph with embedded RTL wraps correctly: the
    /// Hebrew word renders visually reordered (`םולש`) inside whichever row
    /// it lands, the surrounding latin words keep their logical order, and
    /// every row keeps the x-nondecreasing invariant. The wrap width anchors
    /// on the job's own single-row galley — an LTR-pinned job skips the
    /// halign pass entirely, so its glyph x values are the exact emitted
    /// values (equivalently measurable from a twin).
    #[test]
    fn wrapped_ltr_base_embedded_rtl_stays_uax9() {
        let text = "Hello שלום world wrapped again";

        // Break after `שלום`: the first `w` glyph of the stream (of "world")
        // starts at 9 px per char over 11 chars:
        let single = layout_unwrapped(text, Some(TextDirection::LeftToRight));
        let first_world_x = single.rows[0]
            .glyphs
            .iter()
            .find(|glyph| glyph.chr == 'w')
            .map(|glyph| glyph.pos.x)
            .unwrap_or_default();
        let wrap_width = first_world_x + 0.5;
        // The intended row 1 (`Hello שלום `) fits, the next word overflows,
        // and the final row's content fits (uniform tofu advances):
        assert!(first_world_x <= wrap_width && first_world_x + 5.0 * 9.0 > wrap_width);

        let galley = layout_wrapped(text, Some(TextDirection::LeftToRight), wrap_width);
        assert!(galley.rows.len() >= 2);
        let chars = visible_chars_per_row(&galley);
        // Row 0 carries the pinned boundary: `Hello שלום ` — the Hebrew word
        // visually reordered:
        assert_eq!(
            chars[0],
            ['H', 'e', 'l', 'l', 'o', 'ם', 'ו', 'ל', 'ש'],
            "{chars:?}"
        );
        // The remaining latin words keep their logical order, in whichever
        // rows they land (the remainder is wider than W, so the partition is
        // deliberately loose beyond row 0):
        let words: [&[char]; 3] = [
            &['w', 'o', 'r', 'l', 'd'],
            &['w', 'r', 'a', 'p', 'p', 'e', 'd'],
            &['a', 'g', 'a', 'i', 'n'],
        ];
        for word in words {
            assert!(
                chars
                    .iter()
                    .any(|row| row.windows(word.len()).any(|w| w == word)),
                "{word:?} must appear in logical order in some row: {chars:?}"
            );
        }
        for glyphs in visible_glyphs_per_row(&galley) {
            assert_x_nondecreasing(&glyphs);
        }
    }

    /// Build a two-section job: `specs` are (section text, format) pairs;
    /// the byte ranges are derived from the concatenation.
    fn two_section_job(
        section_texts: &[&str],
        formats: &[TextFormat; 2],
        wrap_width: f32,
        direction: Option<TextDirection>,
    ) -> LayoutJob {
        let mut text = String::new();
        let mut sections = Vec::new();
        for (i, section_text) in section_texts.iter().enumerate() {
            let start = ByteIndex(text.len());
            text.push_str(section_text);
            sections.push(LayoutSection {
                leading_space: 0.0,
                byte_range: start..ByteIndex(text.len()),
                format: formats[i].clone(),
            });
        }
        let mut job = LayoutJob {
            sections,
            text,
            wrap: TextWrapping {
                max_width: wrap_width,
                ..Default::default()
            },
            break_on_newline: true,
            ..Default::default()
        };
        job.direction = direction;
        job.round_output_to_gui = false;
        job
    }

    /// D — a paragraph spanning several `LayoutSection`s wraps jointly: the
    /// recorded runs accumulate across the section boundary, the per-row
    /// reorder resolves UAX #9 over the paragraph's whole byte span, and an
    /// RTL word SPLIT by a color change (two sections, `عر` + `بي`) reorders
    /// as ONE run on its row. Fail-before: the old path sliced the visual
    /// stream geometrically, putting the words on the wrong rows.
    #[test]
    fn wrapped_multi_section_rtl_reorders_jointly() {
        let text = "عربي aaa bbb ccc ddd eee";
        let formats = [
            TextFormat::simple(FontId::proportional(14.0), Color32::RED),
            TextFormat::simple(FontId::proportional(14.0), Color32::WHITE),
        ];
        // `عر` in the red section, the rest in the white one — the word
        // عربي is split across the boundary:
        assert_eq!(&text[..4], "عر");
        let job = two_section_job(
            &[&text[..4], &text[4..]],
            &formats,
            (4 + 1 + 3 + 1) as f32 * 9.0 + 0.5,
            Some(TextDirection::RightToLeft),
        );
        let mut fonts = tofu_fonts();
        let galley = layout(&mut fonts, 1.0, job.into());

        // Rows partition logically (عربي aaa / bbb ccc / ddd eee):
        assert_eq!(galley.rows.len(), 3);
        let chars = visible_chars_per_row(&galley);
        // Row 0 renders `عربي aaa`: the split word as ONE run in visual
        // order (يبرع at the right, jointly across the section boundary),
        // the latin island at the visual left:
        assert_eq!(chars[0], ['a', 'a', 'a', 'ي', 'ب', 'ر', 'ع'], "{chars:?}");
        assert_eq!(chars[1], ['b', 'b', 'b', 'c', 'c', 'c'], "{chars:?}");
        assert_eq!(chars[2], ['d', 'd', 'd', 'e', 'e', 'e'], "{chars:?}");
        // Both sections' glyphs appear (no drops):
        assert_eq!(
            galley
                .rows
                .iter()
                .map(|placed| placed.row.glyphs.len())
                .sum::<usize>(),
            text.chars().count(),
            "one glyph slot per char across both sections"
        );
        for glyphs in visible_glyphs_per_row(&galley) {
            assert_x_nondecreasing(&glyphs);
        }
    }

    /// D (unrecorded edge) — a paragraph mixing a pure-LTR section (a single
    /// even-level run, never recorded) with a wrapped RTL section keeps every
    /// glyph: the block inventory interleaves the recorded runs with the
    /// complement intervals. LTR-first order (the LTR stretch is the
    /// paragraph's head).
    #[test]
    fn wrapped_unrecorded_edge_ltr_first_keeps_all_glyphs() {
        // "abcd" resolves to a single level-2 run at an RTL base and is not
        // recorded; the second section's leading space joins عربي's run.
        let formats = [
            TextFormat::simple(FontId::proportional(14.0), Color32::RED),
            TextFormat::simple(FontId::proportional(14.0), Color32::WHITE),
        ];
        let job = two_section_job(
            &["abcd", " عربي aaa bbb"],
            &formats,
            (4 + 1 + 4 + 1) as f32 * 9.0 + 0.5,
            Some(TextDirection::RightToLeft),
        );
        let mut fonts = tofu_fonts();
        let galley = layout(&mut fonts, 1.0, job.into());

        assert_eq!(galley.rows.len(), 2);
        let chars = visible_chars_per_row(&galley);
        // Row 0 reads `abcd عربي `: visually ␣ يبرع ␣ abcd — the LTR word
        // kept, the Arabic word reordered as one run:
        assert_eq!(
            chars[0],
            ['ي', 'ب', 'ر', 'ع', 'a', 'b', 'c', 'd'],
            "{chars:?}"
        );
        assert_eq!(chars[1], ['a', 'a', 'a', 'b', 'b', 'b'], "{chars:?}");
        // No dropped glyphs (the unrecorded stretch's chars are all present):
        assert_eq!(
            galley
                .rows
                .iter()
                .map(|placed| placed.row.glyphs.len())
                .sum::<usize>(),
            "abcd عربي aaa bbb".chars().count(),
        );
        for glyphs in visible_glyphs_per_row(&galley) {
            assert_x_nondecreasing(&glyphs);
        }
    }

    /// Wrapped RTL elision is directed at the READING end: the `…` lands at
    /// the visual left of the logically-LAST row (which is now the last
    /// produced row), replacing the row's leading glyphs — including the
    /// L1-reset trailing break space, which must not strand the `…` behind
    /// it. The LTR mirror keeps the historic trailing `…`.
    #[test]
    fn wrapped_rtl_elision_lands_at_the_reading_end() {
        let text = "أول ثاني ثالث";
        // The one-word-per-row width from test A:
        let twin = layout_unwrapped(text, Some(TextDirection::LeftToRight));
        let (spans, gaps) = visible_runs(&twin);
        let wrap_width = (spans[2] + gaps[0]).max(spans[1] + gaps[0]).max(spans[0]) + 0.5;

        for direction in [
            Some(TextDirection::RightToLeft),
            None,
            Some(TextDirection::LeftToRight),
        ] {
            let mut fonts = tofu_fonts();
            let mut job = LayoutJob::simple(
                text.to_owned(),
                FontId::proportional(14.0),
                Color32::WHITE,
                wrap_width,
            );
            job.direction = direction;
            job.wrap.max_rows = 2;
            job.wrap.overflow_character = Some('…');
            let galley = layout(&mut fonts, 1.0, job.into());
            assert!(galley.elided, "max_rows 2 must elide the third row");
            assert_eq!(galley.rows.len(), 2);
            let last = &galley.rows[1];
            let is_rtl = direction == Some(TextDirection::RightToLeft) || direction.is_none();

            if is_rtl {
                // The `…` sits at the visual LEFT edge (the reading end), and
                // nothing renders behind it: its x is at or before every other
                // glyph's x. The L1-reset break space was dropped with the
                // other replaced glyphs, not kept in front of the `…`.
                let first = &last.row.glyphs[0];
                assert_eq!(
                    first.chr,
                    '…',
                    "glyphs: {:?}",
                    last.row.glyphs.iter().map(|g| g.chr).collect::<Vec<_>>()
                );
                let min_other_x = last
                    .row
                    .glyphs
                    .iter()
                    .skip(1)
                    .map(|glyph| glyph.pos.x)
                    .fold(f32::INFINITY, f32::min);
                assert!(
                    first.pos.x <= min_other_x,
                    "the ellipsis must be at the visual reading end: {} vs {min_other_x}",
                    first.pos.x
                );
            } else {
                // The LTR mirror: the `…` trails the row's visual right edge.
                let last_glyph = last.row.glyphs.last().map(|glyph| glyph.chr);
                assert_eq!(last_glyph, Some('…'));
            }
        }
    }

    /// D (unrecorded edge, mirrored) — the unrecorded pure-LTR stretch is the
    /// paragraph's TAIL: the recorded stretches come first, the complement
    /// interval last, and the trailing section's glyphs still land in their
    /// logical row.
    #[test]
    fn wrapped_unrecorded_edge_ltr_last_keeps_all_glyphs() {
        let formats = [
            TextFormat::simple(FontId::proportional(14.0), Color32::RED),
            TextFormat::simple(FontId::proportional(14.0), Color32::WHITE),
        ];
        // Section 0 = "عربي aaa " (three recorded runs), section 1 = "abcd"
        // (single even run, unrecorded):
        let job = two_section_job(
            &["عربي aaa ", "abcd"],
            &formats,
            (4 + 1 + 3 + 1) as f32 * 9.0 + 0.5,
            Some(TextDirection::RightToLeft),
        );
        let mut fonts = tofu_fonts();
        let galley = layout(&mut fonts, 1.0, job.into());

        assert_eq!(galley.rows.len(), 2);
        let chars = visible_chars_per_row(&galley);
        // Row 0 reads `عربي aaa ` (visually ␣ aaa ␣ يبرع), row 1 the
        // unrecorded tail in logical order:
        assert_eq!(chars[0], ['a', 'a', 'a', 'ي', 'ب', 'ر', 'ع'], "{chars:?}");
        assert_eq!(chars[1], ['a', 'b', 'c', 'd'], "{chars:?}");
        assert_eq!(
            galley
                .rows
                .iter()
                .map(|placed| placed.row.glyphs.len())
                .sum::<usize>(),
            "عربي aaa abcd".chars().count(),
        );
        for glyphs in visible_glyphs_per_row(&galley) {
            assert_x_nondecreasing(&glyphs);
        }
    }

    // --- egui-bidi-oob-panic scope tests (splice marker for red-at-base overlays) ---

    /// The tracking predicate recognizes exactly the characters that can
    /// make a segment record a directional run: the right-to-left blocks
    /// and the RLM/isolate/embedding marks — and nothing in plain LTR text
    /// (including the LRM, which is bidi class L and can never force a run).
    #[test]
    fn text_may_record_bidi_runs_matches_run_recording_characters() {
        for text in [
            "مرحبا",
            "שלום",
            "\u{200F}",
            "\u{202E}reversed",
            "\u{2066}isolate\u{2069}",
            "a\u{FEF3}b",
            "\u{FB50}",
            "\u{202A}embed",
        ] {
            assert!(
                text_may_record_bidi_runs(text),
                "{text:?} must be recognized as run-recording"
            );
        }
        for text in ["", "Hello, world!", "123 …!", "\u{200E}\u{202F} \u{2061}"] {
            assert!(
                !text_may_record_bidi_runs(text),
                "{text:?} must not be recognized as run-recording"
            );
        }
    }

    /// The backend run-mode label that crashed both observed user sites:
    /// exactly 100 UTF-8 bytes, 55 chars, no combining marks.
    const BACKEND_LABEL: &str = "تشغيل شيفرة واجهة المستخدم فقط عند وجود حركات أو إدخال.";

    /// The letters a decomposing Arabic font (Noto Sans Arabic) emits as a
    /// zero-advance dot glyph plus a dotless base — both stamped the
    /// cluster's first char. The synthetic only needs the shape; the exact
    /// per-font decomposition set is irrelevant to the invariants.
    fn is_decomposed_letter(c: char) -> bool {
        matches!(
            c,
            'ب' | 'ت'
                | 'ث'
                | 'ج'
                | 'خ'
                | 'ز'
                | 'ش'
                | 'ض'
                | 'ط'
                | 'ظ'
                | 'غ'
                | 'ف'
                | 'ق'
                | 'ن'
                | 'ي'
        )
    }

    fn deco_glyph(chr: char, x: f32, advance: f32) -> Glyph {
        Glyph {
            chr,
            pos: pos2(x, 0.0),
            advance_width: advance,
            line_height: 16.0,
            font_ascent: 13.0,
            font_height: 16.0,
            font_face_ascent: 13.0,
            font_face_height: 16.0,
            uv_rect: Default::default(),
            is_color: false,
            bidi_level: 1,
            section_index: 0,
            first_vertex: 0,
        }
    }

    /// Hand-built `Paragraph` mimicking the measured emission of an RTL
    /// label under a decomposing font: glyphs in VISUAL order (the
    /// logically-last char first, x ascending by the char's advance), every
    /// decomposed letter as a zero-advance dot followed by its base — both
    /// stamped the cluster's first char and the cluster's byte start — and
    /// one recorded right-to-left run over the whole span.
    fn decomposed_rtl_paragraph(text: &str, advance_for: impl Fn(usize, char) -> f32) -> Paragraph {
        let mut paragraph = Paragraph::from_section_index(0);
        paragraph.byte_span = 0..text.len();
        let indexed_chars: Vec<(usize, usize, char)> = text
            .char_indices()
            .enumerate()
            .map(|(char_idx, (byte_idx, c))| (char_idx, byte_idx, c))
            .collect();
        let mut x = 0.0_f32;
        for &(char_idx, byte_idx, c) in indexed_chars.iter().rev() {
            let advance = advance_for(char_idx, c);
            if is_decomposed_letter(c) {
                // The decomposition dot leads the pair in the emitted
                // (visual) stream: zero-advance, same cluster, same chr.
                paragraph.cluster_byte_starts.push(byte_idx);
                paragraph.glyphs.push(deco_glyph(c, x, 0.0));
            }
            paragraph.cluster_byte_starts.push(byte_idx);
            paragraph.glyphs.push(deco_glyph(c, x, advance));
            x += advance;
        }
        paragraph.bidi_runs.push(RecordedBidiRun {
            glyphs: 0..paragraph.glyphs.len(),
            bytes: 0..text.len(),
            rtl: true,
        });
        paragraph
    }

    /// T1a — a wrapped row's byte ranges must come from the glyphs' shaping
    /// clusters, never from per-glyph first-char sums: a decomposed char
    /// (dot + base, one cluster) used to count its char once per glyph,
    /// drifting every later block past the paragraph span until the final
    /// row clamped to an end-equal range and the unicode-bidi call in
    /// `place_row` panicked (the +40 overcount the probe measured on this
    /// exact label).
    #[test]
    fn logical_blocks_byte_ranges_stay_inside_the_paragraph_span_for_decomposed_glyphs() {
        let text = BACKEND_LABEL;
        assert_eq!(
            text.len(),
            100,
            "the crash-era backend label is exactly 100 bytes"
        );
        let paragraph = decomposed_rtl_paragraph(text, |_, _| 8.0);
        let blocks = logical_blocks(&paragraph);
        assert!(!blocks.is_empty(), "every glyph lands in a block");

        let mut covered: Vec<Range<usize>> = Vec::new();
        let mut prev_start = 0usize;
        for block in &blocks {
            assert!(
                block.bytes.start >= paragraph.byte_span.start
                    && block.bytes.end <= paragraph.byte_span.end,
                "block bytes {:?} must stay inside the span {:?}",
                block.bytes,
                paragraph.byte_span
            );
            assert!(
                block.bytes.start >= prev_start,
                "block starts must not decrease: {prev_start} then {}",
                block.bytes.start
            );
            prev_start = block.bytes.start;
            assert!(
                !block.bytes.is_empty(),
                "a well-formed block's range is non-empty"
            );
            covered.push(block.bytes.clone());
        }

        // The union of the block ranges covers the whole paragraph span:
        covered.sort_by_key(|bytes| bytes.start);
        let mut cursor = paragraph.byte_span.start;
        for bytes in &covered {
            assert!(
                bytes.start <= cursor,
                "the block ranges must cover the span: gap at byte {cursor}"
            );
            cursor = cursor.max(bytes.end);
        }
        assert_eq!(
            cursor, paragraph.byte_span.end,
            "the union of the block ranges must cover the whole span"
        );

        // Scoped to this synthetic (every char is its own cluster and its
        // base leads a block): each char's cluster start is some block's
        // start.
        for (byte_idx, _) in text.char_indices() {
            assert!(
                blocks.iter().any(|block| block.bytes.start == byte_idx),
                "cluster start {byte_idx} must be some block's start"
            );
        }
    }

    /// T1a variant (i) — gapped cluster starts (`HarfBuzz`'s documented
    /// merged/deleted-code-point shape, e.g. `0,0,3,3`): a block's end is
    /// the next strictly greater distinct cluster start scanning forward,
    /// else the paragraph span's end — and the union still covers the span.
    #[test]
    fn logical_blocks_byte_ranges_bridge_gapped_cluster_starts() {
        // Three glyphs over a 10-byte span with cluster starts 0, 0, 5:
        // two glyphs share a cluster, the third starts at 5, and nothing
        // starts at 1-4 or 6-9.
        let mut paragraph = Paragraph::from_section_index(0);
        paragraph.byte_span = 0..10;
        let mut x = 0.0_f32;
        for (cluster_start, advance) in [(0usize, 3.0f32), (0, 3.0), (5, 3.0)] {
            paragraph.cluster_byte_starts.push(cluster_start);
            paragraph.glyphs.push(deco_glyph('خ', x, advance));
            x += advance;
        }
        paragraph.bidi_runs.push(RecordedBidiRun {
            glyphs: 0..3,
            bytes: 0..10,
            rtl: false,
        });

        let blocks = logical_blocks(&paragraph);
        let starts: Vec<usize> = blocks.iter().map(|block| block.bytes.start).collect();
        assert_eq!(starts, vec![0, 0, 5], "starts are the cluster starts");
        let ranges: Vec<Range<usize>> = blocks.iter().map(|block| block.bytes.clone()).collect();
        assert_eq!(
            ranges,
            vec![0..5, 0..5, 5..10],
            "ends are the next strictly greater distinct cluster start, or the span end"
        );
    }

    /// T1a variant (ii) — a right-to-left run emits its clusters in
    /// DESCENDING visual order; the logical-order reversal in
    /// `logical_blocks` must restore ascending block starts with
    /// cluster-exact ranges.
    #[test]
    fn logical_blocks_byte_ranges_ascend_after_the_rtl_order_reversal() {
        // Three decomposed chars (dot + base, one cluster each), emitted in
        // visual order: the logically-last char's glyphs first, so the
        // cluster starts descend in vector order before the reversal.
        let mut paragraph = Paragraph::from_section_index(0);
        paragraph.byte_span = 0..6;
        let mut x = 0.0_f32;
        for cluster_start in [4usize, 2, 0] {
            paragraph.cluster_byte_starts.push(cluster_start);
            paragraph.glyphs.push(deco_glyph('خ', x, 0.0)); // the dot
            paragraph.cluster_byte_starts.push(cluster_start);
            paragraph.glyphs.push(deco_glyph('خ', x, 2.0)); // the base
            x += 2.0;
        }
        paragraph.bidi_runs.push(RecordedBidiRun {
            glyphs: 0..6,
            bytes: 0..6,
            rtl: true,
        });

        let blocks = logical_blocks(&paragraph);
        let starts: Vec<usize> = blocks.iter().map(|block| block.bytes.start).collect();
        assert_eq!(
            starts,
            vec![0, 2, 4, 4],
            "the reversal restores ascending starts (the trailing dot block shares its base's)"
        );
        let ranges: Vec<Range<usize>> = blocks.iter().map(|block| block.bytes.clone()).collect();
        assert_eq!(
            ranges,
            vec![0..2, 2..4, 4..6, 4..6],
            "cluster-exact ranges, the span end closing the last"
        );
    }

    /// T1b — the exact crash: wrapping a decomposed RTL paragraph used to
    /// hand `place_row` a clamped end-equal byte range for a real row and
    /// panic inside unicode-bidi (`index out of bounds: the len is 100 but
    /// the index is 100`). With cluster-exact ranges every row is placed,
    /// nothing is dropped, and the final row keeps the paragraph's final
    /// word.
    #[test]
    fn wrapped_rtl_decomposed_glyphs_do_not_panic_and_keep_every_row() {
        let text = BACKEND_LABEL;
        let word = "إدخال.";
        assert!(text.ends_with(word), "the label ends with the final word");
        let char_count = text.chars().count();
        let word_chars = word.chars().count();
        let word_start = char_count - word_chars;

        // The final word's glyphs get tiny advances so the whole word is
        // guaranteed to fit on one row together; everything else wraps at
        // 8 px per char under a 20 px wrap width.
        let paragraph =
            decomposed_rtl_paragraph(
                text,
                |char_idx, _| {
                    if char_idx >= word_start { 0.5 } else { 8.0 }
                },
            );

        let mut job = LayoutJob::simple(
            text.to_owned(),
            FontId::proportional(14.0),
            Color32::WHITE,
            20.0,
        );
        job.direction = Some(TextDirection::RightToLeft);
        job.round_output_to_gui = false;

        let mut rows: Vec<PlacedRow> = Vec::new();
        let mut elided = false;
        line_break_logically(&paragraph, &job, &[], &mut rows, &mut elided);

        assert!(!elided, "the default max_rows must not elide");
        assert!(
            rows.len() >= 2,
            "a 20 px wrap wraps the label into many rows, got {}",
            rows.len()
        );
        for placed in &rows {
            assert!(!placed.row.glyphs.is_empty(), "every row keeps its glyphs");
        }

        // Nothing dropped or duplicated across the rows:
        let mut all_chrs: Vec<char> = rows
            .iter()
            .flat_map(|placed| placed.row.glyphs.iter().map(|glyph| glyph.chr))
            .collect();
        all_chrs.sort_unstable();
        let mut want_chrs: Vec<char> = paragraph.glyphs.iter().map(|glyph| glyph.chr).collect();
        want_chrs.sort_unstable();
        assert_eq!(all_chrs, want_chrs, "no glyph may be dropped or duplicated");

        // The final row contains the final word's glyphs (a decomposed
        // letter contributes its dot twice, both stamped the same chr):
        let last = rows.last().expect("rows are non-empty");
        let mut want_last: Vec<char> = Vec::new();
        for (char_idx, (_, c)) in text.char_indices().enumerate() {
            if char_idx >= word_start {
                want_last.push(c);
                if is_decomposed_letter(c) {
                    want_last.push(c);
                }
            }
        }
        for c in &want_last {
            let want = want_last.iter().filter(|want| want == &c).count();
            let got = last
                .row
                .glyphs
                .iter()
                .filter(|glyph| &glyph.chr == c)
                .count();
            assert!(
                got >= want,
                "the final row must keep the final word: {c:?} x{want} missing ({got} placed)"
            );
        }
    }

    /// T2 — the exact app stack with a real decomposing Arabic font: any
    /// wrapped RTL paragraph used to panic inside unicode-bidi at a final
    /// wrapped row (the probe measured 24/24 panicking width/font
    /// combinations on the backend label alone). Loads the font from
    /// `KITTEST_RTL_FONT` — the kittest fixture convention
    /// (`crates/egui_kittest/src/rtl_font.rs`): `~/` expansion, skipped
    /// silently when unset or unreadable (fonts are never committed).
    /// Sweeps wrap widths over the 100-byte backend label and a 400+ byte
    /// two-paragraph label through `FontsView::layout_job` — the exact
    /// crash stack — asserting no panic and that every input char keeps a
    /// visible glyph (the decomposition dots are zero-advance and excluded).
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn wrapped_rtl_real_font_labels_do_not_panic() {
        use std::path::PathBuf;

        let Some(raw) = std::env::var_os("KITTEST_RTL_FONT") else {
            return; // fixture not installed: nothing to assert
        };
        if raw.is_empty() {
            return;
        }
        // `~/` expansion, as `egui_kittest::rtl_font` does it:
        let path = match raw.to_str().and_then(|s| s.strip_prefix("~/")) {
            Some(rest) => match std::env::var_os("HOME") {
                Some(home) if !home.is_empty() => PathBuf::from(home).join(rest),
                _ => PathBuf::from(&raw),
            },
            None => PathBuf::from(&raw),
        };
        let Ok(font_data) = std::fs::read(&path) else {
            return; // unreadable fixture: skipped, as in kittest
        };

        let mut definitions = FontDefinitions::default();
        definitions.font_data.insert(
            "kittest-rtl-font".to_owned(),
            Arc::new(FontData::from_owned(font_data)),
        );
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            definitions
                .families
                .entry(family)
                .or_default()
                .push("kittest-rtl-font".to_owned());
        }
        let mut fonts = Fonts::new(TextOptions::default(), definitions);

        // A 400+ byte second paragraph, mirroring the crash's 400-byte ID
        // Test paragraph:
        let long_label = format!(
            "{BACKEND_LABEL}\n{BACKEND_LABEL} {BACKEND_LABEL} {BACKEND_LABEL} {BACKEND_LABEL}"
        );
        assert!(
            long_label
                .split('\n')
                .nth(1)
                .is_some_and(|paragraph| paragraph.len() >= 400),
            "the second paragraph must be 400+ bytes, like the observed one"
        );

        let labels = [
            ("backend label", BACKEND_LABEL),
            ("two-paragraph label", long_label.as_str()),
        ];
        for (name, text) in labels {
            // The newline splits paragraphs and emits no glyph.
            let expected_visible = text.chars().filter(|&c| c != '\n').count();
            for wrap_width in [20.0, 40.0, 60.0, 80.0, 100.0, 150.0] {
                let job = LayoutJob::simple(
                    text.to_owned(),
                    FontId::proportional(14.0),
                    Color32::WHITE,
                    wrap_width,
                );
                let galley = fonts.with_pixels_per_point(2.0).layout_job(job);
                let visible: usize = galley
                    .rows
                    .iter()
                    .map(|placed| {
                        placed
                            .row
                            .glyphs
                            .iter()
                            .filter(|glyph| glyph.advance_width > 0.0)
                            .count()
                    })
                    .sum();
                assert_eq!(
                    visible, expected_visible,
                    "{name} at width {wrap_width}: every char keeps a visible glyph"
                );
            }
        }
    }
}
