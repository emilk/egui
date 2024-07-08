use super::rotated_rect::draw_rotated_rect;
use crate::Painter;
use emath::{vec2, Pos2, Rect, Rot2};
use epaint::{Color32, Rounding};
use std::f32::consts::TAU;

pub(crate) fn cogwheel(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let inner_radius = 0.5 * radius;
    let outer_radius = 0.8 * radius;
    let thickness = 0.3 * radius;

    painter.circle(
        center,
        inner_radius + thickness / 2.,
        Color32::TRANSPARENT,
        (thickness, color),
    );

    let cogs = 8;
    let cog_width = radius / 3.;
    let cog_rounding = radius / 16.;
    let cog_length = radius - outer_radius + thickness / 2.;

    for n in 0..cogs {
        let cog_center = center - vec2(0., outer_radius + cog_length / 2. - thickness / 2.);
        let cog_size = vec2(cog_width, cog_length);
        let rotation = Rot2::from_angle(TAU / cogs as f32 * n as f32);
        let rect = Rect::from_center_size(cog_center, cog_size);
        let rounding = Rounding {
            nw: cog_rounding,
            ne: cog_rounding,
            ..Default::default()
        };
        draw_rotated_rect(painter, rect, rounding, color, rotation, center);
    }
}
