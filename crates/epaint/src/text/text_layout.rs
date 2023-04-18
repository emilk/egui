use std::ops::RangeInclusive;
use std::sync::Arc;

use super::{FontsImpl, Galley, Glyph, LayoutJob, LayoutSection, Row, RowVisuals};
use crate::{Color32, Mesh, Stroke, Vertex};
use emath::*;

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
#[derive(Default, Clone)]
struct Paragraph {
    /// Start of the next glyph to be added.
    pub cursor_x: f32,

    pub glyphs: Vec<Glyph>,

    /// In case of an empty paragraph ("\n"), use this as height.
    pub empty_paragraph_height: f32,
}

/// Layout text into a [`Galley`].
///
/// In most cases you should use [`crate::Fonts::layout_job`] instead
/// since that memoizes the input, making subsequent layouting of the same text much faster.
pub fn layout(fonts: &mut FontsImpl, job: Arc<LayoutJob>) -> Galley {
    let mut paragraphs = vec![Paragraph::default()];
    for (section_index, section) in job.sections.iter().enumerate() {
        layout_section(fonts, &job, section_index as u32, section, &mut paragraphs);
    }

    let point_scale = PointScale::new(fonts.pixels_per_point());

    let mut rows = rows_from_paragraphs(fonts, paragraphs, &job);

    let justify = job.justify && job.wrap.max_width.is_finite();

    if justify || job.halign != Align::LEFT {
        let num_rows = rows.len();
        for (i, row) in rows.iter_mut().enumerate() {
            let is_last_row = i + 1 == num_rows;
            let justify_row = justify && !row.ends_with_newline && !is_last_row;
            halign_and_justify_row(
                point_scale,
                row,
                job.halign,
                job.wrap.max_width,
                justify_row,
            );
        }
    }

    galley_from_rows(point_scale, job, rows)
}

fn layout_section(
    fonts: &mut FontsImpl,
    job: &LayoutJob,
    section_index: u32,
    section: &LayoutSection,
    out_paragraphs: &mut Vec<Paragraph>,
) {
    let LayoutSection {
        leading_space,
        byte_range,
        format,
    } = section;
    let font = fonts.font(&format.font_id);
    let font_height = font.row_height();

    let mut paragraph = out_paragraphs.last_mut().unwrap();
    if paragraph.glyphs.is_empty() {
        paragraph.empty_paragraph_height = font_height; // TODO(emilk): replace this hack with actually including `\n` in the glyphs?
    }

    paragraph.cursor_x += leading_space;

    let mut last_glyph_id = None;

    for chr in job.text[byte_range.clone()].chars() {
        if job.break_on_newline && chr == '\n' {
            out_paragraphs.push(Paragraph::default());
            paragraph = out_paragraphs.last_mut().unwrap();
            paragraph.empty_paragraph_height = font_height; // TODO(emilk): replace this hack with actually including `\n` in the glyphs?
        } else {
            let (font_impl, glyph_info) = font.glyph_info_and_font_impl(chr);
            if let Some(font_impl) = font_impl {
                if let Some(last_glyph_id) = last_glyph_id {
                    paragraph.cursor_x += font_impl.pair_kerning(last_glyph_id, glyph_info.id);
                }
            }

            paragraph.glyphs.push(Glyph {
                chr,
                pos: pos2(paragraph.cursor_x, f32::NAN),
                size: vec2(glyph_info.advance_width, glyph_info.row_height),
                ascent: glyph_info.ascent,
                uv_rect: glyph_info.uv_rect,
                section_index,
            });

            paragraph.cursor_x += glyph_info.advance_width;
            paragraph.cursor_x = font.round_to_pixel(paragraph.cursor_x);
            last_glyph_id = Some(glyph_info.id);
        }
    }
}

/// We ignore y at this stage
fn rect_from_x_range(x_range: RangeInclusive<f32>) -> Rect {
    Rect::from_x_y_ranges(x_range, 0.0..=0.0)
}

fn rows_from_paragraphs(
    fonts: &mut FontsImpl,
    paragraphs: Vec<Paragraph>,
    job: &LayoutJob,
) -> Vec<Row> {
    let num_paragraphs = paragraphs.len();

    let mut rows = vec![];

    for (i, paragraph) in paragraphs.into_iter().enumerate() {
        let is_last_paragraph = (i + 1) == num_paragraphs;

        if paragraph.glyphs.is_empty() {
            rows.push(Row {
                glyphs: vec![],
                visuals: Default::default(),
                rect: Rect::from_min_size(
                    pos2(paragraph.cursor_x, 0.0),
                    vec2(0.0, paragraph.empty_paragraph_height),
                ),
                ends_with_newline: !is_last_paragraph,
            });
        } else {
            let paragraph_max_x = paragraph.glyphs.last().unwrap().max_x();
            if paragraph_max_x <= job.wrap.max_width {
                // early-out optimization
                let paragraph_min_x = paragraph.glyphs[0].pos.x;
                rows.push(Row {
                    glyphs: paragraph.glyphs,
                    visuals: Default::default(),
                    rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
                    ends_with_newline: !is_last_paragraph,
                });
            } else {
                line_break(fonts, &paragraph, job, &mut rows);
                rows.last_mut().unwrap().ends_with_newline = !is_last_paragraph;
            }
        }
    }

    rows
}

fn line_break(
    fonts: &mut FontsImpl,
    paragraph: &Paragraph,
    job: &LayoutJob,
    out_rows: &mut Vec<Row>,
) {
    // Keeps track of good places to insert row break if we exceed `wrap_width`.
    let mut row_break_candidates = RowBreakCandidates::default();

    let mut first_row_indentation = paragraph.glyphs[0].pos.x;
    let mut row_start_x = 0.0;
    let mut row_start_idx = 0;
    let mut non_empty_rows = 0;

    for i in 0..paragraph.glyphs.len() {
        let potential_row_width = paragraph.glyphs[i].max_x() - row_start_x;

        if job.wrap.max_rows > 0 && non_empty_rows >= job.wrap.max_rows {
            break;
        }

        if potential_row_width > job.wrap.max_width {
            if first_row_indentation > 0.0
                && !row_break_candidates.has_good_candidate(job.wrap.break_anywhere)
            {
                // Allow the first row to be completely empty, because we know there will be more space on the next row:
                // TODO(emilk): this records the height of this first row as zero, though that is probably fine since first_row_indentation usually comes with a first_row_min_height.
                out_rows.push(Row {
                    glyphs: vec![],
                    visuals: Default::default(),
                    rect: rect_from_x_range(first_row_indentation..=first_row_indentation),
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

                let paragraph_min_x = glyphs[0].pos.x;
                let paragraph_max_x = glyphs.last().unwrap().max_x();

                out_rows.push(Row {
                    glyphs,
                    visuals: Default::default(),
                    rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
                    ends_with_newline: false,
                });

                row_start_idx = last_kept_index + 1;
                row_start_x = paragraph.glyphs[row_start_idx].pos.x;
                row_break_candidates = Default::default();
                non_empty_rows += 1;
            } else {
                // Found no place to break, so we have to overrun wrap_width.
            }
        }

        row_break_candidates.add(i, &paragraph.glyphs[i..]);
    }

    if row_start_idx < paragraph.glyphs.len() {
        if job.wrap.max_rows > 0 && non_empty_rows == job.wrap.max_rows {
            if let Some(last_row) = out_rows.last_mut() {
                replace_last_glyph_with_overflow_character(fonts, job, last_row);
            }
        } else {
            let glyphs: Vec<Glyph> = paragraph.glyphs[row_start_idx..]
                .iter()
                .copied()
                .map(|mut glyph| {
                    glyph.pos.x -= row_start_x;
                    glyph
                })
                .collect();

            let paragraph_min_x = glyphs[0].pos.x;
            let paragraph_max_x = glyphs.last().unwrap().max_x();

            out_rows.push(Row {
                glyphs,
                visuals: Default::default(),
                rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
                ends_with_newline: false,
            });
        }
    }
}

fn replace_last_glyph_with_overflow_character(
    fonts: &mut FontsImpl,
    job: &LayoutJob,
    row: &mut Row,
) {
    let overflow_character = match job.wrap.overflow_character {
        Some(c) => c,
        None => return,
    };

    loop {
        let (prev_glyph, last_glyph) = match row.glyphs.as_mut_slice() {
            [.., prev, last] => (Some(prev), last),
            [.., last] => (None, last),
            _ => break,
        };

        let section = &job.sections[last_glyph.section_index as usize];
        let font = fonts.font(&section.format.font_id);
        let font_height = font.row_height();

        let prev_glyph_id = prev_glyph.map(|prev_glyph| {
            let (_, prev_glyph_info) = font.glyph_info_and_font_impl(prev_glyph.chr);
            prev_glyph_info.id
        });

        // undo kerning with previous glyph
        let (font_impl, glyph_info) = font.glyph_info_and_font_impl(last_glyph.chr);
        last_glyph.pos.x -= font_impl
            .zip(prev_glyph_id)
            .map(|(font_impl, prev_glyph_id)| font_impl.pair_kerning(prev_glyph_id, glyph_info.id))
            .unwrap_or_default();

        // replace the glyph
        last_glyph.chr = overflow_character;
        let (font_impl, glyph_info) = font.glyph_info_and_font_impl(last_glyph.chr);
        last_glyph.size = vec2(glyph_info.advance_width, font_height);
        last_glyph.uv_rect = glyph_info.uv_rect;

        // reapply kerning
        last_glyph.pos.x += font_impl
            .zip(prev_glyph_id)
            .map(|(font_impl, prev_glyph_id)| font_impl.pair_kerning(prev_glyph_id, glyph_info.id))
            .unwrap_or_default();

        // check if we're still within width budget
        let row_end_x = last_glyph.max_x();
        let row_start_x = row.glyphs.first().unwrap().pos.x; // if `last_mut()` returned `Some`, then so will `first()`
        let row_width = row_end_x - row_start_x;
        if row_width <= job.wrap.max_width {
            break;
        }

        row.glyphs.pop();
    }
}

fn halign_and_justify_row(
    point_scale: PointScale,
    row: &mut Row,
    halign: Align,
    wrap_width: f32,
    justify: bool,
) {
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
    assert!(num_glyphs_in_range > 0);

    let original_min_x = row.glyphs[glyph_range.0].logical_rect().min.x;
    let original_max_x = row.glyphs[glyph_range.1 - 1].logical_rect().max.x;
    let original_width = original_max_x - original_min_x;

    let target_width = if justify && num_glyphs_in_range > 1 {
        wrap_width
    } else {
        original_width
    };

    let (target_min_x, target_max_x) = match halign {
        Align::LEFT => (0.0, target_width),
        Align::Center => (-target_width / 2.0, target_width / 2.0),
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

    let mut translate_x = target_min_x - original_min_x - extra_x_per_glyph * glyph_range.0 as f32;

    for glyph in &mut row.glyphs {
        glyph.pos.x += translate_x;
        glyph.pos.x = point_scale.round_to_pixel(glyph.pos.x);
        translate_x += extra_x_per_glyph;
        if glyph.chr.is_whitespace() {
            translate_x += extra_x_per_space;
        }
    }

    // Note we ignore the leading/trailing whitespace here!
    row.rect.min.x = target_min_x;
    row.rect.max.x = target_max_x;
}

/// Calculate the Y positions and tessellate the text.
fn galley_from_rows(point_scale: PointScale, job: Arc<LayoutJob>, mut rows: Vec<Row>) -> Galley {
    let mut first_row_min_height = job.first_row_min_height;
    let mut cursor_y = 0.0;
    let mut min_x: f32 = 0.0;
    let mut max_x: f32 = 0.0;
    for row in &mut rows {
        let mut row_height = first_row_min_height.max(row.rect.height());
        let mut row_ascent = 0.0f32;
        first_row_min_height = 0.0;

        // take metrics from the highest font in this row
        if let Some(glyph) = row
            .glyphs
            .iter()
            .max_by(|a, b| a.size.y.partial_cmp(&b.size.y).unwrap())
        {
            row_height = glyph.size.y;
            row_ascent = glyph.ascent;
        }
        row_height = point_scale.round_to_pixel(row_height);

        // Now positions each glyph:
        for glyph in &mut row.glyphs {
            let format = &job.sections[glyph.section_index as usize].format;

            let align_offset = match format.valign {
                Align::Center | Align::Max => row_ascent,

                // raised text.
                Align::Min => glyph.ascent,
            };
            glyph.pos.y = cursor_y + align_offset;
        }

        row.rect.min.y = cursor_y;
        row.rect.max.y = cursor_y + row_height;

        min_x = min_x.min(row.rect.min.x);
        max_x = max_x.max(row.rect.max.x);
        cursor_y += row_height;
        cursor_y = point_scale.round_to_pixel(cursor_y);
    }

    let format_summary = format_summary(&job);

    let mut mesh_bounds = Rect::NOTHING;
    let mut num_vertices = 0;
    let mut num_indices = 0;

    for row in &mut rows {
        row.visuals = tessellate_row(point_scale, &job, &format_summary, row);
        mesh_bounds = mesh_bounds.union(row.visuals.mesh_bounds);
        num_vertices += row.visuals.mesh.vertices.len();
        num_indices += row.visuals.mesh.indices.len();
    }

    let rect = Rect::from_min_max(pos2(min_x, 0.0), pos2(max_x, cursor_y));

    Galley {
        job,
        rows,
        rect,
        mesh_bounds,
        num_vertices,
        num_indices,
        pixels_per_point: point_scale.pixels_per_point,
    }
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
        add_row_backgrounds(job, row, &mut mesh);
    }

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
        glyph_vertex_range: glyph_vertex_start..glyph_vertex_end,
    }
}

/// Create background for glyphs that have them.
/// Creates as few rectangular regions as possible.
fn add_row_backgrounds(job: &LayoutJob, row: &Row, mesh: &mut Mesh) {
    if row.glyphs.is_empty() {
        return;
    }

    let mut end_run = |start: Option<(Color32, Rect)>, stop_x: f32| {
        if let Some((color, start_rect)) = start {
            let rect = Rect::from_min_max(start_rect.left_top(), pos2(stop_x, start_rect.bottom()));
            let rect = rect.expand(1.0); // looks better
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
        } else if let Some((existing_color, start)) = run_start {
            if existing_color == color
                && start.top() == rect.top()
                && start.bottom() == rect.bottom()
            {
                // continue the same background rectangle
            } else {
                end_run(run_start.take(), last_rect.right());
                run_start = Some((color, rect));
            }
        } else {
            run_start = Some((color, rect));
        }

        last_rect = rect;
    }

    end_run(run_start.take(), last_rect.right());
}

fn tessellate_glyphs(point_scale: PointScale, job: &LayoutJob, row: &Row, mesh: &mut Mesh) {
    for glyph in &row.glyphs {
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
    let mut end_line = |start: Option<(Stroke, Pos2)>, stop_x: f32| {
        if let Some((stroke, start)) = start {
            add_hline(point_scale, [start, pos2(stop_x, start.y)], stroke, mesh);
        }
    };

    let mut line_start = None;
    let mut last_right_x = f32::NAN;

    for glyph in &row.glyphs {
        let (stroke, y) = stroke_and_y(glyph);

        if stroke == Stroke::NONE {
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

fn add_hline(point_scale: PointScale, [start, stop]: [Pos2; 2], stroke: Stroke, mesh: &mut Mesh) {
    let antialiased = true;

    if antialiased {
        let mut path = crate::tessellator::Path::default(); // TODO(emilk): reuse this to avoid re-allocations.
        path.add_line_segment([start, stop]);
        let feathering = 1.0 / point_scale.pixels_per_point();
        path.stroke_open(feathering, stroke, mesh);
    } else {
        // Thin lines often lost, so this is a bad idea

        assert_eq!(start.y, stop.y);

        let min_y = point_scale.round_to_pixel(start.y - 0.5 * stroke.width);
        let max_y = point_scale.round_to_pixel(min_y + stroke.width);

        let rect = Rect::from_min_max(
            pos2(point_scale.round_to_pixel(start.x), min_y),
            pos2(point_scale.round_to_pixel(stop.x), max_y),
        );

        mesh.add_colored_rect(rect, stroke.color);
    }
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
}

#[inline]
fn is_cjk_ideograph(c: char) -> bool {
    ('\u{4E00}' <= c && c <= '\u{9FFF}')
        || ('\u{3400}' <= c && c <= '\u{4DBF}')
        || ('\u{2B740}' <= c && c <= '\u{2B81F}')
}

#[inline]
fn is_kana(c: char) -> bool {
    ('\u{3040}' <= c && c <= '\u{309F}') // Hiragana block
        || ('\u{30A0}' <= c && c <= '\u{30FF}') // Katakana block
}

#[inline]
fn is_cjk(c: char) -> bool {
    // TODO: Add support for Korean Hangul.
    is_cjk_ideograph(c) || is_kana(c)
}

#[inline]
fn is_cjk_break_allowed(c: char) -> bool {
    // See: https://en.wikipedia.org/wiki/Line_breaking_rules_in_East_Asian_languages#Characters_not_permitted_on_the_start_of_a_line.
    !")]｝〕〉》」』】〙〗〟'\"｠»ヽヾーァィゥェォッャュョヮヵヶぁぃぅぇぉっゃゅょゎゕゖㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ々〻‐゠–〜?!‼⁇⁈⁉・、:;,。.".contains(c)
}

// ----------------------------------------------------------------------------

#[test]
fn test_zero_max_width() {
    let mut fonts = FontsImpl::new(1.0, 1024, super::FontDefinitions::default());
    let mut layout_job = LayoutJob::single_section("W".into(), super::TextFormat::default());
    layout_job.wrap.max_width = 0.0;
    let galley = super::layout(&mut fonts, layout_job.into());
    assert_eq!(galley.rows.len(), 1);
}

#[test]
fn test_cjk() {
    let mut fonts = FontsImpl::new(1.0, 1024, super::FontDefinitions::default());
    let mut layout_job = LayoutJob::single_section(
        "日本語とEnglishの混在した文章".into(),
        super::TextFormat::default(),
    );
    layout_job.wrap.max_width = 90.0;
    let galley = super::layout(&mut fonts, layout_job.into());
    assert_eq!(
        galley
            .rows
            .iter()
            .map(|row| row.glyphs.iter().map(|g| g.chr).collect::<String>())
            .collect::<Vec<_>>(),
        vec!["日本語と", "Englishの混在", "した文章"]
    );
}

#[test]
fn test_pre_cjk() {
    let mut fonts = FontsImpl::new(1.0, 1024, super::FontDefinitions::default());
    let mut layout_job = LayoutJob::single_section(
        "日本語とEnglishの混在した文章".into(),
        super::TextFormat::default(),
    );
    layout_job.wrap.max_width = 110.0;
    let galley = super::layout(&mut fonts, layout_job.into());
    assert_eq!(
        galley
            .rows
            .iter()
            .map(|row| row.glyphs.iter().map(|g| g.chr).collect::<String>())
            .collect::<Vec<_>>(),
        vec!["日本語とEnglish", "の混在した文章"]
    );
}
