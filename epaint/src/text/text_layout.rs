use std::ops::RangeInclusive;
use std::sync::Arc;

use super::{Fonts, Galley, Glyph, LayoutJob, LayoutSection, Row, RowVisuals};
use crate::{Color32, Mesh, Stroke, Vertex};
use emath::*;

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
/// In most cases you should use [`Fonts::layout_job`] instead
/// since that memoizes the input, making subsequent layouting of the same text much faster.
pub fn layout(fonts: &Fonts, job: Arc<LayoutJob>) -> Galley {
    let mut paragraphs = vec![Paragraph::default()];
    for (section_index, section) in job.sections.iter().enumerate() {
        layout_section(fonts, &job, section_index as u32, section, &mut paragraphs);
    }

    let rows = rows_from_paragraphs(paragraphs, job.wrap_width);

    galley_from_rows(fonts, job, rows)
}

fn layout_section(
    fonts: &Fonts,
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
    let font = &fonts[format.style];
    let font_height = font.row_height();

    let mut paragraph = out_paragraphs.last_mut().unwrap();
    if paragraph.glyphs.is_empty() {
        paragraph.empty_paragraph_height = font_height; // TODO: replace this hack with actually including `\n` in the glyphs?
    }

    paragraph.cursor_x += leading_space;

    let mut last_glyph_id = None;

    for chr in job.text[byte_range.clone()].chars() {
        if job.break_on_newline && chr == '\n' {
            out_paragraphs.push(Paragraph::default());
            paragraph = out_paragraphs.last_mut().unwrap();
            paragraph.empty_paragraph_height = font_height; // TODO: replace this hack with actually including `\n` in the glyphs?
        } else {
            let (font_impl, glyph_info) = font.glyph_info_and_font_impl(chr);
            if let Some(last_glyph_id) = last_glyph_id {
                paragraph.cursor_x += font_impl.pair_kerning(last_glyph_id, glyph_info.id)
            }

            paragraph.glyphs.push(Glyph {
                chr,
                pos: pos2(paragraph.cursor_x, f32::NAN),
                size: vec2(glyph_info.advance_width, font_height),
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

fn rows_from_paragraphs(paragraphs: Vec<Paragraph>, wrap_width: f32) -> Vec<Row> {
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
            if paragraph_max_x <= wrap_width {
                // early-out optimization
                let paragraph_min_x = paragraph.glyphs[0].pos.x;
                rows.push(Row {
                    glyphs: paragraph.glyphs,
                    visuals: Default::default(),
                    rect: rect_from_x_range(paragraph_min_x..=paragraph_max_x),
                    ends_with_newline: !is_last_paragraph,
                });
            } else {
                line_break(&paragraph, wrap_width, &mut rows);
                rows.last_mut().unwrap().ends_with_newline = !is_last_paragraph;
            }
        }
    }

    rows
}

fn line_break(paragraph: &Paragraph, wrap_width: f32, out_rows: &mut Vec<Row>) {
    // Keeps track of good places to insert row break if we exceed `wrap_width`.
    let mut row_break_candidates = RowBreakCandidates::default();

    let mut first_row_indentation = paragraph.glyphs[0].pos.x;
    let mut row_start_x = 0.0;
    let mut row_start_idx = 0;

    for (i, glyph) in paragraph.glyphs.iter().enumerate() {
        let potential_row_width = glyph.pos.x - row_start_x;

        if potential_row_width > wrap_width {
            if first_row_indentation > 0.0 && !row_break_candidates.has_word_boundary() {
                // Allow the first row to be completely empty, because we know there will be more space on the next row:
                // TODO: this records the height of this first row as zero, though that is probably fine since first_row_indentation usually comes with a first_row_min_height.
                out_rows.push(Row {
                    glyphs: vec![],
                    visuals: Default::default(),
                    rect: rect_from_x_range(first_row_indentation..=first_row_indentation),
                    ends_with_newline: false,
                });
                row_start_x += first_row_indentation;
                first_row_indentation = 0.0;
            } else if let Some(last_kept_index) = row_break_candidates.get() {
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
            } else {
                // Found no place to break, so we have to overrun wrap_width.
            }
        }

        row_break_candidates.add(i, glyph.chr);
    }

    if row_start_idx < paragraph.glyphs.len() {
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

/// Calculate the Y positions and tessellate the text.
fn galley_from_rows(fonts: &Fonts, job: Arc<LayoutJob>, mut rows: Vec<Row>) -> Galley {
    let mut first_row_min_height = job.first_row_min_height;
    let mut cursor_y = 0.0;
    let mut max_x: f32 = 0.0;
    for row in &mut rows {
        let mut row_height = first_row_min_height.max(row.rect.height());
        first_row_min_height = 0.0;
        for glyph in &row.glyphs {
            row_height = row_height.max(glyph.size.y);
        }
        row_height = fonts.round_to_pixel(row_height);

        // Now positions each glyph:
        for glyph in &mut row.glyphs {
            let format = &job.sections[glyph.section_index as usize].format;
            glyph.pos.y = cursor_y + format.valign.to_factor() * (row_height - glyph.size.y);
            glyph.pos.y = fonts.round_to_pixel(glyph.pos.y);
        }

        row.rect.min.y = cursor_y;
        row.rect.max.y = cursor_y + row_height;

        max_x = max_x.max(row.rect.right());
        cursor_y += row_height;
        cursor_y = fonts.round_to_pixel(cursor_y);
    }

    let format_summary = format_summary(&job);

    let mut num_vertices = 0;
    let mut num_indices = 0;

    for row in &mut rows {
        row.visuals = tessellate_row(fonts, &job, &format_summary, row);
        num_vertices += row.visuals.mesh.vertices.len();
        num_indices += row.visuals.mesh.indices.len();
    }

    let size = vec2(max_x, cursor_y);

    Galley {
        job,
        rows,
        size,
        num_vertices,
        num_indices,
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
        format_summary.any_underline |= section.format.underline != Stroke::none();
        format_summary.any_strikethrough |= section.format.strikethrough != Stroke::none();
    }
    format_summary
}

fn tessellate_row(
    fonts: &Fonts,
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
    tessellate_glyphs(fonts, job, row, &mut mesh);
    let glyph_vertex_end = mesh.vertices.len();

    if format_summary.any_underline {
        add_row_hline(fonts, row, &mut mesh, |glyph| {
            let format = &job.sections[glyph.section_index as usize].format;
            let stroke = format.underline;
            let y = glyph.logical_rect().bottom();
            (stroke, y)
        });
    }

    if format_summary.any_strikethrough {
        add_row_hline(fonts, row, &mut mesh, |glyph| {
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

fn tessellate_glyphs(fonts: &Fonts, job: &LayoutJob, row: &Row, mesh: &mut Mesh) {
    for glyph in &row.glyphs {
        let uv_rect = glyph.uv_rect;
        if !uv_rect.is_nothing() {
            let mut left_top = glyph.pos + uv_rect.offset;
            left_top.x = fonts.round_to_pixel(left_top.x);
            left_top.y = fonts.round_to_pixel(left_top.y);

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
    fonts: &Fonts,
    row: &Row,
    mesh: &mut Mesh,
    stroke_and_y: impl Fn(&Glyph) -> (Stroke, f32),
) {
    let mut end_line = |start: Option<(Stroke, Pos2)>, stop_x: f32| {
        if let Some((stroke, start)) = start {
            add_hline(fonts, [start, pos2(stop_x, start.y)], stroke, mesh);
        }
    };

    let mut line_start = None;
    let mut last_right_x = f32::NAN;

    for glyph in &row.glyphs {
        let (stroke, y) = stroke_and_y(glyph);

        if stroke == Stroke::none() {
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

fn add_hline(fonts: &Fonts, [start, stop]: [Pos2; 2], stroke: Stroke, mesh: &mut Mesh) {
    let antialiased = true;

    if antialiased {
        let mut path = crate::tessellator::Path::default(); // TODO: reuse this to avoid re-allocations.
        path.add_line_segment([start, stop]);
        let options = crate::tessellator::TessellationOptions::from_pixels_per_point(
            fonts.pixels_per_point(),
        );
        path.stroke_open(stroke, options, mesh);
    } else {
        // Thin lines often lost, so this is a bad idea

        assert_eq!(start.y, stop.y);

        let min_y = fonts.round_to_pixel(start.y - 0.5 * stroke.width);
        let max_y = fonts.round_to_pixel(min_y + stroke.width);

        let rect = Rect::from_min_max(
            pos2(fonts.round_to_pixel(start.x), min_y),
            pos2(fonts.round_to_pixel(stop.x), max_y),
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
    /// Logograms (single character representing a whole word) are good candidates for line break.
    logogram: Option<usize>,
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
    fn add(&mut self, index: usize, chr: char) {
        const NON_BREAKING_SPACE: char = '\u{A0}';
        if chr.is_whitespace() && chr != NON_BREAKING_SPACE {
            self.space = Some(index);
        } else if is_chinese(chr) {
            self.logogram = Some(index);
        } else if chr == '-' {
            self.dash = Some(index);
        } else if chr.is_ascii_punctuation() {
            self.punctuation = Some(index);
        } else {
            self.any = Some(index);
        }
    }

    fn has_word_boundary(&self) -> bool {
        self.space.is_some() || self.logogram.is_some()
    }

    fn get(&self) -> Option<usize> {
        self.space
            .or(self.logogram)
            .or(self.dash)
            .or(self.punctuation)
            .or(self.any)
    }
}

#[inline]
fn is_chinese(c: char) -> bool {
    ('\u{4E00}' <= c && c <= '\u{9FFF}')
        || ('\u{3400}' <= c && c <= '\u{4DBF}')
        || ('\u{2B740}' <= c && c <= '\u{2B81F}')
}
