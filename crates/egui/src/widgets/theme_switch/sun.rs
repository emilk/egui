use super::rotated_rect::draw_rotated_rect;
use crate::Painter;
use emath::{vec2, Pos2, Rect, Rot2, Vec2};
use epaint::{Color32, Stroke};
use std::f32::consts::TAU;

pub(crate) fn sun(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let clipped = painter.with_clip_rect(Rect::from_center_size(center, Vec2::splat(radius * 2.)));
    let sun_radius = radius * 0.5;

    clipped.circle(center, sun_radius, color, Stroke::NONE);

    let rays = 8;
    let ray_radius = radius / 4.;
    let ray_spacing = radius / 7.5;
    let ray_length = radius - sun_radius - ray_spacing;

    for n in 0..rays {
        let ray_center = center - vec2(0., sun_radius + ray_spacing + ray_length / 2.);
        let ray_size = vec2(ray_radius, ray_length);
        let rect = Rect::from_center_size(ray_center, ray_size);
        let rotation = Rot2::from_angle(TAU / rays as f32 * n as f32);
        draw_rotated_rect(painter, rect, ray_radius / 2.0, color, rotation, center);
    }
}
