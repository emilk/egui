use std::{borrow::Cow, sync::Arc};

use ecolor::Color32;
use emath::{pos2, vec2, Pos2, Rect, Vec2};
use parley::{
    AlignmentOptions, FontStyle, FontWeight, FontWidth, InlineBox, PositionedLayoutItem, TextStyle,
};

use crate::{
    text::{Row, RowVisuals},
    Mesh,
};

use super::{font::UvRect, FontFamily, FontsImpl, Galley, Glyph, LayoutJob, TextFormat};

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
        line_height: format
            .line_height
            .map_or(1.0, |line_height| line_height / format.font_id.size),
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
            rect: Rect::from_min_max(Pos2::ZERO, Pos2::ZERO),
            mesh_bounds: Rect::NOTHING,
            num_vertices: 0,
            num_indices: 0,
            pixels_per_point: fonts.pixels_per_point(),
            elided: true,
        };
    };

    let justify = job.justify && job.wrap.max_width.is_finite();

    let horiz_offset = match (job.wrap.max_width.is_finite() && !justify, job.halign) {
        (true, emath::Align::Center) => -job.wrap.max_width * 0.5,
        (true, emath::Align::RIGHT) => -job.wrap.max_width,
        _ => 0.0,
    };

    let default_style = text_format_to_style(&first_section.format);
    let mut builder =
        fonts
            .layout_context
            .tree_builder(&mut fonts.font_context, 1.0, &default_style);

    job.sections.iter().for_each(|section| {
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
        if job.first_row_min_height > 0.0 {
            // TODO(valadaptive): This is only supposed to apply to the first row, but there's no way ahead of time to
            // know which span of text the "first row" is. It's also supposed to be the *minimum* height, but for
            // alignment purposes, we really want it to just be the height itself.
            let min_height = job.first_row_min_height / section.format.font_id.size;
            style.line_height = min_height;
        }

        builder.push_style_span(style);
        builder.push_text(&job.text[section.byte_range.clone()]);
        builder.pop_style_span();
    });

    // TODO(valadaptive): we don't need to assemble this string
    // (but RangedBuilder requires one call per individual style attribute :( )
    let (mut layout, _text) = builder.build();

    let max_width = job.wrap.max_width.is_finite().then_some(job.wrap.max_width);
    layout.break_all_lines(max_width);

    layout.align(
        max_width,
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

    for (i, line) in layout.lines().enumerate() {
        let mut glyphs = Vec::new();
        let mut mesh = Mesh::default();
        let mut row_logical_bounds = Rect::NOTHING;

        for item in line.items() {
            match item {
                PositionedLayoutItem::GlyphRun(run) => {
                    // We saw something that isn't a box on the first line. (See below for why we need vertical_offset)
                    if i == 0 {
                        if vertical_offset != 0.0 {
                            println!(
                                "reset vertical offset because we saw {:?}",
                                &job.text[run.run().text_range()]
                            );
                        }
                        vertical_offset = 0.0;
                    }

                    // TODO(valadaptive): use this to implement faux italics (and faux bold?)
                    // run.run.synthesis()

                    for (mut glyph, uv_rect, (x, y)) in fonts.glyph_atlas.render_glyph_run(
                        &run,
                        (horiz_offset, vertical_offset),
                        fonts.pixels_per_point(),
                    ) {
                        glyph.x += horiz_offset;
                        glyph.y += vertical_offset;
                        let glyph_rect = Rect::from_min_size(
                            pos2(glyph.x, line.metrics().min_coord + vertical_offset),
                            vec2(
                                glyph.advance,
                                line.metrics().max_coord - line.metrics().min_coord,
                                //line.metrics().min_coord + line.metrics().baseline,
                            ),
                        );
                        acc_logical_bounds = acc_logical_bounds.union(glyph_rect);
                        row_logical_bounds = row_logical_bounds.union(glyph_rect);
                        // TODO(valadaptive): stop storing glyphs!
                        glyphs.push(Glyph {
                            chr: ' ',
                            pos: pos2(glyph.x, glyph.y),
                            advance_width: glyph.advance,
                            line_height: line.metrics().line_height,
                            uv_rect: uv_rect.unwrap_or_default(),
                        });

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

                        let color = run.style().brush;

                        //mesh.add_colored_rect(glyph_rect, Color32::DEBUG_COLOR.gamma_multiply(0.3));
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
                PositionedLayoutItem::InlineBox(_inline_box) => {
                    /*mesh.add_colored_rect(
                        Rect::from_min_size(
                            pos2(_inline_box.x, _inline_box.y),
                            vec2(_inline_box.width, _inline_box.height.max(1.0)),
                        ),
                        Color32::RED.gamma_multiply(0.3),
                    );*/

                    // As described above, the InlineBox can't have any height, but that means that if the first line
                    // completely wraps, it'll end up at the same place instead of one line down. To avoid this, add a
                    // vertical offset to all text in this layout if it wraps.
                    vertical_offset += job.first_row_min_height;
                }
            }
        }

        glyphs.push(Glyph {
            chr: '\n',
            pos: pos2(0.0, line.metrics().baseline),
            advance_width: 0.0,
            line_height: line.metrics().line_height,
            uv_rect: UvRect::default(),
        });

        let mesh_bounds = mesh.calc_bounds();
        let glyph_vertex_range = 0..mesh.vertices.len();

        acc_mesh_bounds = acc_mesh_bounds.union(mesh_bounds);
        acc_num_indices += mesh.indices.len();
        acc_num_vertices += mesh.vertices.len();

        if !row_logical_bounds.is_finite() {
            row_logical_bounds = Rect::ZERO;
        }
        //row_logical_bounds.max.y -= 8.0;

        let row = Row {
            section_index_at_start: 0,
            glyphs,
            rect: row_logical_bounds,
            visuals: RowVisuals {
                mesh,
                mesh_bounds,
                glyph_index_start: 0,
                glyph_vertex_range,
            },
            ends_with_newline: false,
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
        elided: false,
        rect: acc_logical_bounds,
        mesh_bounds: acc_mesh_bounds,
        num_vertices: acc_num_vertices,
        num_indices: acc_num_indices,
        pixels_per_point: fonts.pixels_per_point(),
    }
}
