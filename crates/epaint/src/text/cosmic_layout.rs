use std::sync::Arc;

use cosmic_text::{
    Align, Attrs, Buffer, CacheKeyFlags, CacheMetrics, Color, Family, Metrics, Shaping, Stretch,
    Style, Weight,
};
use ecolor::Color32;
use emath::{pos2, vec2, Pos2, Rect};

use crate::{
    text::{Row, RowVisuals},
    Mesh,
};

use super::{font::UvRect, FontFamily, FontsImpl, Galley, Glyph, LayoutJob, TextFormat};

fn text_format_to_attrs<'b: 'c, 'c>(format: &'b TextFormat, default_line_height: f32) -> Attrs<'c> {
    let color = format.color;
    Attrs {
        color_opt: Some(Color::rgba(color.r(), color.g(), color.b(), color.a())),
        family: match &format.font_id.family {
            FontFamily::Proportional => Family::SansSerif,
            FontFamily::Monospace => Family::Monospace,
            FontFamily::Name(name) => Family::Name(name),
        },
        stretch: Stretch::Normal,
        style: Style::Normal,
        weight: Weight::LIGHT,
        metadata: 0,
        // TODO: *real* italics
        cache_key_flags: if format.italics {
            CacheKeyFlags::FAKE_ITALIC
        } else {
            CacheKeyFlags::empty()
        },
        metrics_opt: Some(CacheMetrics::from(Metrics::new(
            format.font_id.size,
            format.line_height.unwrap_or(default_line_height),
        ))),
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

    // TODO(valadaptive): this is seemingly always used for blank lines (e.g. in the EasyMark editor, whether the first
    // line is a header determines paragraph spacing).
    let default_line_height = fonts.font(&first_section.format.font_id).row_height();
    let metrics = Metrics::new(first_section.format.font_id.size, default_line_height);

    let mut buffer = Buffer::new(&mut fonts.font_system, metrics);

    buffer.set_size(
        &mut fonts.font_system,
        job.wrap.max_width.is_finite().then_some(job.wrap.max_width),
        None,
    );

    // TODO(valadaptive): we need to collect the spans because the fonts.font() call mutably borrows fonts.
    // Once ab_glyph is removed there's probably a better solution
    let spans = job
        .sections
        .iter()
        .map(|section| {
            let text_span = &job.text[section.byte_range.clone()];
            let attrs = text_format_to_attrs(
                &section.format,
                fonts.font(&section.format.font_id).row_height(),
            );

            (text_span, attrs)
        })
        .collect::<Vec<_>>();

    buffer.set_rich_text(
        &mut fonts.font_system,
        spans,
        text_format_to_attrs(&first_section.format, default_line_height),
        Shaping::Advanced,
        Some(match (justify, job.halign) {
            (true, _) => Align::Justified,
            (false, emath::Align::Min) => Align::Left,
            (false, emath::Align::Center) => Align::Center,
            (false, emath::Align::Max) => Align::Right,
        }),
    );

    buffer.set_leading_space(&mut fonts.font_system, first_section.leading_space);

    buffer.shape_until_scroll(&mut fonts.font_system, false);

    buffer.set_wrap(&mut fonts.font_system, cosmic_text::Wrap::WordOrGlyph);

    let mut rows = Vec::new();
    let mut acc_mesh_bounds = Rect::NOTHING;
    let mut acc_num_indices = 0;
    let mut acc_num_vertices = 0;
    let mut acc_logical_bounds = Rect::NOTHING;

    for run in buffer.layout_runs() {
        let mut mesh = Mesh::default();
        let line_y = (run.line_y * fonts.pixels_per_point).round() as i32;
        let mut row_logical_bounds = Rect::NOTHING;

        let mut glyphs = Vec::with_capacity(run.glyphs.len());

        for glyph in run.glyphs {
            let line_height = glyph.line_height_opt.unwrap_or(run.line_height);
            let glyph_rect = Rect::from_min_size(
                pos2(glyph.x + horiz_offset, glyph.y + run.line_top),
                vec2(glyph.w, line_height),
            );
            acc_logical_bounds = acc_logical_bounds.union(glyph_rect);
            row_logical_bounds = row_logical_bounds.union(glyph_rect);

            let physical = glyph.physical(
                (horiz_offset * fonts.pixels_per_point, 0.0),
                fonts.pixels_per_point,
            );
            let uv_rect = fonts.glyph_atlas.render_glyph(
                &mut fonts.font_system,
                &physical,
                fonts.pixels_per_point,
            );

            // TODO(valadaptive): stop storing glyphs!
            glyphs.push(Glyph {
                chr: ' ',
                pos: glyph_rect.min,
                advance_width: glyph.w,
                line_height,
                uv_rect: uv_rect.unwrap_or_default(),
            });

            let Some(uv_rect) = uv_rect else {
                continue;
            };

            let left_top = (pos2(physical.x as f32, (physical.y + line_y) as f32)
                / fonts.pixels_per_point)
                + uv_rect.offset;

            let rect = Rect::from_min_max(left_top, left_top + uv_rect.size);
            let uv = Rect::from_min_max(
                pos2(uv_rect.min[0] as f32, uv_rect.min[1] as f32),
                pos2(uv_rect.max[0] as f32, uv_rect.max[1] as f32),
            );

            let color = glyph.color_opt.unwrap_or(Color::rgb(255, 255, 255));

            //mesh.add_colored_rect(rect, Color32::DEBUG_COLOR.gamma_multiply(0.3));
            mesh.add_rect_with_uv(
                rect,
                uv,
                Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), color.a()),
            );
        }

        glyphs.push(Glyph {
            chr: '\n',
            pos: row_logical_bounds.max,
            advance_width: 0.0,
            line_height: 0.0,
            uv_rect: UvRect::default(),
        });

        let mesh_bounds = mesh.calc_bounds();
        let glyph_vertex_range = 0..mesh.vertices.len();

        acc_mesh_bounds = acc_mesh_bounds.union(mesh_bounds);
        acc_num_indices += mesh.indices.len();
        acc_num_vertices += mesh.vertices.len();

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
