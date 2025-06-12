use std::sync::Arc;

use ecolor::Color32;
use emath::{pos2, vec2, GuiRounding as _, NumExt as _, Pos2, Rect, Vec2};
use log::debug;
use parley::{AlignmentOptions, BreakReason, GlyphRun, InlineBox, PositionedLayoutItem};

use crate::{
    tessellator::Path,
    text::{LayoutAndOffset, Row, RowVisuals},
    Mesh, Stroke,
};

use super::{fonts::FontsLayoutView, Galley, LayoutJob};

fn render_decoration(
    pixels_per_point: f32,
    run: &GlyphRun<'_, Color32>,
    mesh: &mut Mesh,
    offset: (f32, f32),
    stroke: Stroke,
) {
    let mut y = run.baseline() + offset.1;
    stroke.round_center_to_pixel(pixels_per_point, &mut y);
    let x_start = run.offset() + offset.0;
    let x_end = x_start + run.advance();

    let mut path = Path::default();
    path.reserve(2);
    path.add_line_segment([pos2(x_start, y), pos2(x_end, y)]);
    path.stroke_open(1.0 / pixels_per_point, &stroke.into(), mesh);
}

pub(super) fn layout(fonts: &mut FontsLayoutView<'_>, job: LayoutJob) -> Galley {
    let Some(first_section) = job.sections.first() else {
        // Early-out: no text
        return Galley {
            job: Arc::new(job),
            rows: Default::default(),
            parley_layout: LayoutAndOffset::default(),
            overflow_char_layout: None,
            #[cfg(feature = "accesskit")]
            accessibility: Default::default(),
            selection_color: Color32::TRANSPARENT,
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            mesh_bounds: Rect::NOTHING,
            num_vertices: 0,
            num_indices: 0,
            pixels_per_point: fonts.pixels_per_point,
            elided: true,
        };
    };

    let justify = job.justify && job.wrap.max_width.is_finite();

    let mut default_style = first_section.format.as_parley();
    job.wrap.apply_to_parley_style(&mut default_style);
    let mut builder =
        fonts
            .layout_context
            .tree_builder(fonts.font_context, 1.0, false, &default_style);

    let first_row_height = job.first_row_min_height;

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
        let mut style = section.format.as_parley();
        job.wrap.apply_to_parley_style(&mut style);
        if i == 0 {
            // If the first section takes up more than one row, this will apply to the entire first section. There
            // doesn't seem to be any way to prevent this because we don't know ahead of time what the "first row" will
            // be due to line wrapping.

            //TODO(valadaptive): how to make this work with metrics-relative line height?
            //first_row_height = first_row_height.max(section.format.line_height());
            //style.line_height = parley::LineHeight::Absolute(first_row_height);
        }

        builder.push_style_span(style);
        builder.push_text(&job.text[section.byte_range.clone()]);
        builder.pop_style_span();
    });

    // TODO(valadaptive): we don't need to assemble this string
    // (but RangedBuilder requires one call per individual style attribute :( )
    let (mut layout, _text) = builder.build();

    let mut overflow_char_layout = None;

    let mut break_lines = layout.break_lines();
    for i in 0..job.wrap.max_rows {
        let wrap_width = job.effective_wrap_width();
        let wrap_width = if wrap_width.is_finite() {
            wrap_width
        } else {
            f32::MAX
        };
        let line = break_lines.break_next(wrap_width);

        // We're truncating the text with an overflow character.
        if let (Some(overflow_character), true, true) = (
            job.wrap.overflow_character,
            i == job.wrap.max_rows - 1,
            !break_lines.is_done(),
        ) {
            let mut builder =
                fonts
                    .layout_context
                    .tree_builder(fonts.font_context, 1.0, false, &default_style);

            builder.push_text(&overflow_character.to_string());
            let (mut layout, _text) = builder.build();
            layout.break_all_lines(None);

            break_lines.revert();
            break_lines.break_next(wrap_width - layout.full_width());
            overflow_char_layout = Some((layout, Vec2::ZERO));
        }

        if line.is_none() {
            break;
        }
    }
    let broke_all_lines: bool = break_lines.is_done();
    break_lines.finish();

    // Parley will left-align the line if there's not enough space. In this
    // case, that could occur due to floating-point error if we use
    // `layout.width()` as the alignment width, but it's okay as left-alignment
    // will look the same as the "correct" alignment.
    //
    // Note that we only use the "effective wrap width" for determining line
    // breaks. Everywhere else, we want to use the actual specified width.
    let alignment_width = if job.wrap.max_width.is_finite() {
        job.wrap.max_width
    } else {
        layout.width()
    };
    let horiz_offset = match (justify, job.halign) {
        (false, emath::Align::Center) => -alignment_width * 0.5,
        (false, emath::Align::RIGHT) => -alignment_width,
        _ => 0.0,
    };

    layout.align(
        Some(alignment_width),
        match (justify, job.halign) {
            (true, _) => parley::Alignment::Justified,
            (false, emath::Align::Min) => parley::Alignment::Left,
            (false, emath::Align::Center) => parley::Alignment::Middle,
            (false, emath::Align::Max) => parley::Alignment::Right,
        },
        AlignmentOptions::default(),
    );

    let mut rows = Vec::new();
    let mut acc_mesh_bounds = Rect::NOTHING;
    let mut acc_num_indices = 0;
    let mut acc_num_vertices = 0;
    let mut acc_logical_bounds = Rect::NOTHING;
    // Temporary mesh used to store each row's decorations before they're all appended after the glyphs. Reused to avoid
    // allocations.
    let mut decorations = Mesh::default();

    let mut vertical_offset = 0f32;

    let mut prev_break_reason = None;
    for (i, line) in layout.lines().enumerate() {
        let mut mesh = Mesh::default();
        let mut background_mesh = Mesh::default();
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

        let mut draw_run =
            |run: &GlyphRun<'_, Color32>,
             section_idx: usize,
             (horiz_offset, vertical_offset): (f32, f32)| {
                let section_format = &job.sections[section_idx].format;
                let background = section_format.background;

                let mut visual_vertical_offset = vertical_offset;
                visual_vertical_offset += match section_format.valign {
                    emath::Align::TOP => run.run().metrics().ascent - line.metrics().ascent,
                    emath::Align::Center => 0.0,
                    emath::Align::BOTTOM => run.run().metrics().descent - line.metrics().descent,
                };

                if background != Color32::TRANSPARENT {
                    let min_y = run.baseline() - run.run().metrics().ascent;
                    let max_y = run.baseline() + run.run().metrics().descent;
                    let min_x = run.offset();
                    let max_x = run.offset() + run.advance();

                    background_mesh.add_colored_rect(
                        Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y))
                            .translate(vec2(horiz_offset, visual_vertical_offset))
                            .expand(section_format.expand_bg),
                        background,
                    );
                }

                let run_metrics = run.run().metrics();

                for (_glyph, uv_rect, (x, y), color) in fonts.glyph_atlas.render_glyph_run(
                    fonts.texture_atlas,
                    run,
                    vec2(horiz_offset, visual_vertical_offset),
                    fonts.hinting_enabled,
                    fonts.pixels_per_point,
                    fonts.font_tweaks,
                ) {
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
                    mesh.add_rect_with_uv(rect, uv, color);
                }

                if let Some(underline) = &run.style().underline {
                    let offset = underline.offset.unwrap_or(run_metrics.underline_offset);
                    let size = underline.size.unwrap_or(run_metrics.underline_size);
                    render_decoration(
                        fonts.pixels_per_point,
                        run,
                        &mut decorations,
                        (horiz_offset, visual_vertical_offset - offset),
                        Stroke {
                            width: size,
                            color: underline.brush,
                        },
                    );
                }

                if let Some(strikethrough) = &run.style().strikethrough {
                    let offset = strikethrough
                        .offset
                        .unwrap_or(run_metrics.strikethrough_offset);
                    let size = strikethrough.size.unwrap_or(run_metrics.strikethrough_size);
                    render_decoration(
                        fonts.pixels_per_point,
                        run,
                        &mut decorations,
                        (horiz_offset, visual_vertical_offset - offset),
                        Stroke {
                            width: size,
                            color: strikethrough.brush,
                        },
                    );
                }
            };

        let mut is_inline_box_only = i == 0;
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
                        is_inline_box_only = false;
                    }

                    let run_range = run.run().text_range();

                    // Get the layout section corresponding to this run, for the background color. We have to do a
                    // binary search each time because in mixed-direction text, we may not traverse glyph runs in
                    // increasing byte order.
                    let section_idx = job
                        .sections
                        .binary_search_by(|section| section.byte_range.start.cmp(&run_range.start))
                        .unwrap_or_else(|i| i.saturating_sub(1));

                    draw_run(&run, section_idx, (horiz_offset, vertical_offset));
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

        // The text overflowed, and we have an overflow character to use when truncating the text. Add it to the mesh.
        if i == layout.len() - 1 && !broke_all_lines {
            if let Some((overflow_line, overflow_layout_offset)) = overflow_char_layout
                .as_mut()
                .and_then(|(layout, overflow_layout_offset)| {
                    Some((layout.lines().next()?, overflow_layout_offset))
                })
            {
                let overflow_metrics = overflow_line.metrics();
                let overflow_horiz_offset = (line_metrics.offset
                    + horiz_offset
                    + line_metrics.advance
                    + overflow_metrics.advance)
                    .min(job.wrap.max_width)
                    - overflow_metrics.advance;
                let overflow_vertical_offset =
                    vertical_offset + line_metrics.baseline - overflow_metrics.baseline;

                for item in overflow_line.items() {
                    let PositionedLayoutItem::GlyphRun(run) = item else {
                        continue;
                    };

                    draw_run(&run, 0, (overflow_horiz_offset, overflow_vertical_offset));
                }

                row_logical_bounds = row_logical_bounds.union(Rect::from_min_max(
                    pos2(
                        overflow_horiz_offset,
                        overflow_metrics.baseline
                            - overflow_metrics.ascent
                            - (overflow_metrics.leading * 0.5)
                            + overflow_vertical_offset,
                    ),
                    pos2(
                        overflow_horiz_offset + overflow_metrics.advance,
                        overflow_metrics.baseline
                            + overflow_metrics.descent
                            + (overflow_metrics.leading * 0.5)
                            + overflow_vertical_offset,
                    ),
                ));

                overflow_layout_offset.x = overflow_horiz_offset;
                overflow_layout_offset.y = overflow_vertical_offset;
            }
        }

        // Don't include the leading inline box when calculating text bounds.
        // TODO(valadaptive): the old layout code includes this box. Is that good?
        if is_inline_box_only {
            continue;
        }

        // Don't include the leading inline box in the row bounds
        let line_start = line_metrics.offset + horiz_offset + box_offset;

        // Be flexible with trailing whitespace.
        // - max_line_end is the widest the line can be if all trailing
        //   whitespace is included.
        // - min_line_end is the narrowest the line can be if all trailing
        //   whitespace is excluded.
        //
        // This lets us count trailing whitespace for inline labels while
        // ignoring it for the purpose of text wrapping.
        let max_line_end = line_metrics.offset + horiz_offset + line_metrics.advance;
        let min_line_end = max_line_end - line_metrics.trailing_whitespace;
        // If this line's trailing whitespace is what would push the line over
        // the max wrap width, clamp it. However, we must be at least as wide as
        // min_line_end.
        let line_end = max_line_end.min(job.wrap.max_width).max(min_line_end);

        row_logical_bounds = row_logical_bounds.union(Rect::from_min_max(
            pos2(
                line_start,
                line_metrics.baseline - line_metrics.ascent - (line_metrics.leading * 0.5)
                    + vertical_offset,
            ),
            pos2(
                line_end,
                line_metrics.baseline
                    + line_metrics.descent
                    + (line_metrics.leading * 0.5)
                    + vertical_offset,
            ),
        ));

        if job.round_output_to_gui {
            let did_exceed_wrap_width_by_a_lot =
                row_logical_bounds.max.x > job.wrap.max_width + 1.0;

            row_logical_bounds = row_logical_bounds.round_ui();

            if did_exceed_wrap_width_by_a_lot {
                // If the user picked a too aggressive wrap width (e.g. more narrow than any individual glyph),
                // we should let the user know by reporting that our width is wider than the wrap width.
            } else {
                // Make sure we don't report being wider than the wrap width the user picked:
                row_logical_bounds.max.x = row_logical_bounds.max.x.at_most(job.wrap.max_width);
            }
        }

        acc_logical_bounds = acc_logical_bounds.union(row_logical_bounds);

        if acc_logical_bounds.width() > job.wrap.max_width {
            debug!(
                "actual wrapped text width {} exceeds max_width {}",
                acc_logical_bounds.width(),
                job.wrap.max_width
            );
        }

        let num_glyph_vertices = mesh.vertices.len();

        // TODO(valadaptive): it would be really nice to avoid this temporary allocation, which only exists to ensure
        // that override_text_color doesn't change the decoration color by moving all decoration meshes past the end of
        // `glyph_vertex_range`.
        if !decorations.is_empty() {
            mesh.append_ref(&decorations);
            decorations.clear();
        }

        // Glyph vertices start after (above) the background vertices.
        let glyph_index_start = background_mesh.indices.len();
        let glyph_vertex_range = glyph_index_start..glyph_index_start + num_glyph_vertices;
        // Prepend the background to the text mesh. Actually, we append the *text* to the *background*, then set the
        // text mesh to the newly-appended-to background mesh.
        if !background_mesh.is_empty() {
            background_mesh.append(mesh);
            mesh = background_mesh;
        }

        let mesh_bounds = mesh.calc_bounds();

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
                selection_rects: None,
                glyph_index_start,
                glyph_vertex_range,
            },
        };

        rows.push(row);
    }

    // In case no glyphs got drawn (e.g. all whitespace)
    for bounds in [&mut acc_logical_bounds, &mut acc_mesh_bounds] {
        if !bounds.is_finite() {
            *bounds = Rect::from_min_size(pos2(horiz_offset, 0.0), Vec2::ZERO);
        }

        debug_assert!(
            !bounds.is_negative(),
            "Invalid bounds for galley mesh: {bounds:?}"
        );
    }

    if job.round_output_to_gui {
        acc_logical_bounds = acc_logical_bounds.round_ui();
    }

    Galley {
        job: Arc::new(job),
        rows,
        parley_layout: LayoutAndOffset::new(layout, vec2(horiz_offset, vertical_offset)),
        overflow_char_layout: overflow_char_layout
            .map(|(layout, offset)| Box::new(LayoutAndOffset::new(layout, offset))),
        #[cfg(feature = "accesskit")]
        accessibility: Default::default(),
        selection_color: Color32::TRANSPARENT,
        elided: !broke_all_lines,
        rect: acc_logical_bounds,
        mesh_bounds: acc_mesh_bounds,
        num_vertices: acc_num_vertices,
        num_indices: acc_num_indices,
        pixels_per_point: fonts.pixels_per_point,
    }
}
