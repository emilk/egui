use std::{borrow::Cow, sync::Arc};

use ecolor::Color32;
use emath::{pos2, vec2, Pos2, Rect, Vec2};
use log::debug;
use parley::{
    AlignmentOptions, BreakReason, FontStyle, FontWeight, FontWidth, InlineBox,
    PositionedLayoutItem, TextStyle,
};

use crate::{
    mutex::Mutex,
    text::{Row, RowVisuals},
    Mesh,
};

use super::{FontFamily, FontsImpl, Galley, LayoutJob, TextFormat};

fn text_format_to_line_height(format: &TextFormat) -> f32 {
    format.line_height.unwrap_or(format.font_id.size)
}

fn text_format_to_style<'b: 'c, 'c>(format: &'b TextFormat) -> TextStyle<'c, Color32> {
    TextStyle {
        font_stack: match &format.font_id.family {
            FontFamily::Proportional => parley::FontStack::Single(parley::FontFamily::Generic(
                parley::GenericFamily::SansSerif,
            )),
            FontFamily::Monospace => parley::FontStack::Single(parley::FontFamily::Generic(
                parley::GenericFamily::Monospace,
            )),
            FontFamily::Name(name) => {
                parley::FontStack::Single(parley::FontFamily::Named(Cow::Borrowed(name)))
            }
        },
        font_size: format.font_id.size,
        font_width: FontWidth::NORMAL,
        font_style: if format.italics {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        font_weight: FontWeight::NORMAL,
        font_variations: parley::FontSettings::List(Cow::Borrowed(&[])),
        font_features: parley::FontSettings::List(Cow::Borrowed(&[])),
        locale: None,
        brush: format.color,
        has_underline: !format.underline.is_empty(),
        underline_offset: None,
        underline_size: (!format.underline.is_empty()).then_some(format.underline.width),
        underline_brush: (!format.underline.is_empty()).then_some(format.underline.color),
        has_strikethrough: !format.strikethrough.is_empty(),
        strikethrough_offset: None,
        strikethrough_size: (!format.strikethrough.is_empty())
            .then_some(format.strikethrough.width),
        strikethrough_brush: (!format.strikethrough.is_empty())
            .then_some(format.strikethrough.color),
        line_height: text_format_to_line_height(format) / format.font_id.size,
        word_spacing: 0.0,
        letter_spacing: format.extra_letter_spacing,
    }
}

pub fn layout(fonts: &mut FontsImpl, job: LayoutJob) -> Galley {
    let Some(first_section) = job.sections.first() else {
        // Early-out: no text
        return Galley {
            job: Arc::new(job),
            rows: Default::default(),
            parley_layout: Mutex::new(parley::Layout::new()),
            layout_offset: Vec2::ZERO,
            #[cfg(feature = "accesskit")]
            accessibility: Default::default(),
            selection_mesh: None,
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            mesh_bounds: Rect::NOTHING,
            num_vertices: 0,
            num_indices: 0,
            pixels_per_point: fonts.pixels_per_point(),
            elided: true,
        };
    };

    let justify = job.justify && job.wrap.max_width.is_finite();

    let default_style = text_format_to_style(&first_section.format);
    let mut builder =
        fonts
            .layout_context
            .tree_builder(&mut fonts.font_context, 1.0, &default_style);

    let mut first_row_height = job.first_row_min_height;

    job.sections.iter().enumerate().for_each(|(i, section)| {
        // TODO(valadaptive): this only works for the first section
        if section.leading_space > 0.0 {
            // Emulate the leading space with an inline box.
            builder.push_inline_box(InlineBox {
                id: 0,
                index: section.byte_range.start,
                width: section.leading_space,
                // If we set the height to first_row_min_height or similar, it will progressively push text downwards
                // because inline boxes are aligned to the baseline, not the descent, but first_row_min_height is set
                // from the previous text's ascent + descent.
                height: 0.0,
            });
        }
        let mut style = text_format_to_style(&section.format);
        if i == 0 {
            // If the first section takes up more than one row, this will apply to the entire first section. There
            // doesn't seem to be any way to prevent this because we don't know ahead of time what the "first row" will
            // be due to line wrapping.
            first_row_height = first_row_height.max(text_format_to_line_height(&section.format));
            style.line_height = first_row_height / section.format.font_id.size;
        }

        builder.push_style_span(style);
        builder.push_text(&job.text[section.byte_range.clone()]);
        builder.pop_style_span();
    });

    // TODO(valadaptive): we don't need to assemble this string
    // (but RangedBuilder requires one call per individual style attribute :( )
    let (mut layout, _text) = builder.build();

    let max_width = job.wrap.max_width.is_finite().then_some(job.wrap.max_width);
    let mut break_lines = layout.break_lines();
    for _ in 0..job.wrap.max_rows {
        if break_lines
            .break_next(max_width.unwrap_or(f32::MAX))
            .is_none()
        {
            break;
        }
    }
    break_lines.finish();

    // Parley will left-align the line if there's not enough space. In this
    // case, that could occur due to floating-point error if we use
    // `layout.width()` as the alignment width, but it's okay as left-alignment
    // will look the same as the "correct" alignment.
    let alignment_width = max_width.unwrap_or_else(|| layout.width());
    let horiz_offset = match (justify, job.halign) {
        (false, emath::Align::Center) => -alignment_width * 0.5,
        (false, emath::Align::RIGHT) => -alignment_width,
        _ => 0.0,
    };

    layout.align(
        Some(alignment_width),
        match (justify, job.halign) {
            (true, _) => parley::Alignment::Justified,
            (false, emath::Align::Min) => parley::Alignment::Start,
            (false, emath::Align::Center) => parley::Alignment::Middle,
            (false, emath::Align::Max) => parley::Alignment::End,
        },
        AlignmentOptions::default(),
    );

    let mut rows = Vec::new();
    let mut acc_mesh_bounds = Rect::NOTHING;
    let mut acc_num_indices = 0;
    let mut acc_num_vertices = 0;
    let mut acc_logical_bounds = Rect::NOTHING;

    let mut vertical_offset = 0f32;

    let mut prev_break_reason = None;
    for (i, line) in layout.lines().enumerate() {
        let mut mesh = Mesh::default();
        let mut row_logical_bounds = Rect::NOTHING;
        let mut box_offset = 0.0;

        // Parley will wrap the last whitespace character(s) onto a whole new
        // line. We don't want that to count for layout purposes.
        let is_trailing_wrapped_whitespace = i == layout.len() - 1
            && matches!(
                prev_break_reason,
                Some(BreakReason::Regular | BreakReason::Emergency)
            )
            && line
                .runs()
                .all(|run| run.clusters().all(|cluster| cluster.is_space_or_nbsp()));

        prev_break_reason = Some(line.break_reason());

        // Nothing on this line except whitespace that should be collapsed.
        if is_trailing_wrapped_whitespace {
            continue;
        }

        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(run) => {
                    // We saw something that isn't a box on the first line. (See below for why we need vertical_offset)
                    if i == 0 {
                        /*if vertical_offset != 0.0 {
                            println!(
                                "reset vertical offset because we saw {:?}",
                                &job.text[run.run().text_range()]
                            );
                        }*/
                        vertical_offset = 0.0;
                    }

                    // TODO(valadaptive): use this to implement faux italics (and faux bold?)
                    // run.run.synthesis()

                    for (mut glyph, uv_rect, (x, y), color) in fonts.glyph_atlas.render_glyph_run(
                        &run,
                        (horiz_offset, vertical_offset),
                        fonts.pixels_per_point(),
                    ) {
                        glyph.x += horiz_offset;
                        glyph.y += vertical_offset;

                        let Some(uv_rect) = uv_rect else {
                            continue;
                        };

                        let left_top =
                            (pos2(x as f32, y as f32) / fonts.pixels_per_point) + uv_rect.offset;

                        let rect = Rect::from_min_max(left_top, left_top + uv_rect.size);
                        let uv = Rect::from_min_max(
                            pos2(uv_rect.min[0] as f32, uv_rect.min[1] as f32),
                            pos2(uv_rect.max[0] as f32, uv_rect.max[1] as f32),
                        );

                        //mesh.add_colored_rect(rect, Color32::DEBUG_COLOR.gamma_multiply(0.3));
                        mesh.add_rect_with_uv(
                            rect,
                            uv,
                            Color32::from_rgba_premultiplied(
                                color.r(),
                                color.g(),
                                color.b(),
                                color.a(),
                            ),
                        );
                    }
                }
                PositionedLayoutItem::InlineBox(inline_box) => {
                    /*mesh.add_colored_rect(
                        Rect::from_min_size(
                            pos2(inline_box.x, inline_box.y),
                            vec2(inline_box.width, inline_box.height.max(1.0)),
                        ),
                        Color32::RED.gamma_multiply(0.3),
                    );*/

                    // As described above, the InlineBox can't have any height, but that means that if the first line
                    // completely wraps, it'll end up at the same place instead of one line down. To avoid this, add a
                    // vertical offset to all text in this layout if it wraps.
                    vertical_offset += first_row_height;
                    box_offset += inline_box.width;
                }
            }
        }

        let line_metrics = line.metrics();
        // Don't include the leading inline box in the row bounds
        let line_start = line_metrics.offset + horiz_offset + box_offset;

        // Be flexible with trailing whitespace.
        // - max_line_end is the widest the line can be if all trailing
        //   whitespace is included.
        // - min_line_end is the narrowest the line can be if all trailing
        //   whitespace is excluded.
        // - max_desired_line_end is the job's max wrap width.
        //
        // This lets us count trailing whitespace for inline labels while
        // ignoring it for the purpose of text wrapping.
        let max_line_end = line_metrics.offset + horiz_offset + line_metrics.advance;
        let min_line_end = max_line_end - line_metrics.trailing_whitespace;
        let max_desired_line_end = line_start + job.wrap.max_width;
        // If this line's trailing whitespace is what would push the line over
        // the max wrap width, clamp it. However, we must be at least as wide as
        // min_line_end.
        let line_end = max_line_end.min(max_desired_line_end).max(min_line_end);

        row_logical_bounds = row_logical_bounds.union(Rect::from_min_max(
            pos2(line_start, line_metrics.min_coord + vertical_offset),
            pos2(line_end, line_metrics.max_coord + vertical_offset),
        ));
        acc_logical_bounds = acc_logical_bounds.union(row_logical_bounds);

        if acc_logical_bounds.width() > job.wrap.max_width {
            debug!(
                "actual wrapped text width {} exceeds max_width {}",
                acc_logical_bounds.width(),
                job.wrap.max_width
            );
        }

        let mesh_bounds = mesh.calc_bounds();
        let glyph_vertex_range = 0..mesh.vertices.len();

        acc_mesh_bounds = acc_mesh_bounds.union(mesh_bounds);
        acc_num_indices += mesh.indices.len();
        acc_num_vertices += mesh.vertices.len();

        if !row_logical_bounds.is_finite() {
            row_logical_bounds = Rect::ZERO;
        }

        let row = Row {
            rect: row_logical_bounds,
            visuals: RowVisuals {
                mesh,
                mesh_bounds,
                glyph_index_start: 0,
                glyph_vertex_range,
            },
        };

        rows.push(row);
    }

    for bounds in [&mut acc_logical_bounds, &mut acc_mesh_bounds] {
        if !bounds.is_finite() {
            *bounds = Rect::from_min_size(pos2(horiz_offset, 0.0), Vec2::ZERO);
        }
    }

    debug_assert!(!acc_logical_bounds.is_negative());
    debug_assert!(!acc_mesh_bounds.is_negative());

    Galley {
        job: Arc::new(job),
        rows,
        parley_layout: Mutex::new(layout),
        layout_offset: vec2(horiz_offset, vertical_offset),
        #[cfg(feature = "accesskit")]
        accessibility: Default::default(),
        selection_mesh: None,
        elided: false,
        rect: acc_logical_bounds,
        mesh_bounds: acc_mesh_bounds,
        num_vertices: acc_num_vertices,
        num_indices: acc_num_indices,
        pixels_per_point: fonts.pixels_per_point(),
    }
}
