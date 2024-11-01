use crate::emath::{Pos2, Rect, Vec2};
use crate::epaint::{CircleShape, Color32, PathShape, Shape, Stroke};
use crate::{Sense, Ui, Widget};
use std::ops::RangeInclusive;

// offset for stroke highlight
const OFFSET: f32 = 2.0;

/// Control two numbers with a double slider.
///
/// The slider range defines the values you get when pulling the slider to the far edges.
///
/// The range can include any numbers, and go from low-to-high or from high-to-low.
///
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_f32: f32 = 0.0;
/// # let mut my_other_f32: f32 = 0.0;
/// ui.add(egui::DoubleSlider::new(&mut my_f32,&mut my_other_f32, 0.0..=100.0));
/// # });
/// ```
///
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct DoubleSlider<'a> {
    left_slider: &'a mut f32,
    right_slider: &'a mut f32,
    separation_distance: f32,
    control_point_radius: f32,
    width: f32,
    color: Color32,
    cursor_fill: Color32,
    stroke: Stroke,
    range: RangeInclusive<f32>,
}

impl<'a> DoubleSlider<'a> {
    pub fn new(
        lower_value: &'a mut f32,
        upper_value: &'a mut f32,
        range: RangeInclusive<f32>,
    ) -> Self {
        DoubleSlider {
            left_slider: lower_value,
            right_slider: upper_value,
            separation_distance: 75.0,
            control_point_radius: 7.0,
            width: 100.0,
            cursor_fill: Color32::DARK_GRAY,
            color: Color32::DARK_GRAY,
            stroke: Stroke::new(7.0, Color32::RED.linear_multiply(0.5)),
            range,
        }
    }

    /// Set the primary width for the slider.
    #[inline]
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Set the separation distance for the two sliders.
    #[inline]
    pub fn separation_distance(mut self, separation_distance: f32) -> Self {
        self.separation_distance = separation_distance;
        self
    }

    /// Set the primary color for the slider.
    #[inline]
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Set the stroke for the main line.
    #[inline]
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = stroke;
        self
    }

    /// Set the color fill for the slider cursor.
    #[inline]
    pub fn cursor_fill(mut self, cursor_fill: Color32) -> Self {
        self.cursor_fill = cursor_fill;
        self
    }

    /// Set the auxiliary stroke.
    #[inline]
    pub fn aux_stroke(mut self, aux_stroke: Stroke) -> Self {
        self.stroke = aux_stroke;
        self
    }

    /// Set the control point radius
    #[inline]
    pub fn control_point_radius(mut self, control_point_radius: f32) -> Self {
        self.control_point_radius = control_point_radius;
        self
    }

    fn val_to_x(&self, val: f32) -> f32 {
        (self.width - 2.0 * self.control_point_radius - 2.0 * OFFSET)
            / (self.range.end() - self.range.start())
            * (val - self.range.start())
            + self.control_point_radius
            + OFFSET
    }

    fn x_to_val(&self, x: f32) -> f32 {
        (self.range.end() - self.range.start())
            / (self.width - 2.0 * self.control_point_radius - 2.0 * OFFSET)
            * x
    }
}

impl<'a> Widget for DoubleSlider<'a> {
    fn ui(self, ui: &mut Ui) -> crate::Response {
        // calculate height
        let height = 2.0 * self.control_point_radius + 2.0 * OFFSET;

        let (mut response, painter) =
            ui.allocate_painter(Vec2::new(self.width, height), Sense::hover());
        let mut left_edge = response.rect.left_center();
        left_edge.x += self.control_point_radius;
        let mut right_edge = response.rect.right_center();
        right_edge.x -= self.control_point_radius;

        // draw the line
        painter.add(PathShape::line(
            vec![left_edge, right_edge],
            Stroke::new(self.stroke.width, self.color),
        ));

        let to_screen = emath::RectTransform::from_to(
            Rect::from_min_size(Pos2::ZERO, response.rect.size()),
            response.rect,
        );

        // handle lower bound
        let lower_bound = {
            // get the control point
            let size = Vec2::splat(2.0 * self.control_point_radius);
            let point_in_screen = to_screen.transform_pos(Pos2 {
                x: self.val_to_x(*self.left_slider),
                y: self.control_point_radius + OFFSET,
            });
            let point_rect = Rect::from_center_size(point_in_screen, size);
            let point_id = response.id.with(0);
            let point_response = ui.interact(point_rect, point_id, Sense::drag());

            if point_response.dragged() {
                response.mark_changed();
            }

            // handle logic
            *self.left_slider += self.x_to_val(point_response.drag_delta().x);
            if *self.right_slider < *self.left_slider + self.separation_distance {
                *self.right_slider = *self.left_slider + self.separation_distance;
            }
            *self.right_slider = self
                .right_slider
                .clamp(*self.range.start(), *self.range.end());
            *self.left_slider = self
                .left_slider
                .clamp(*self.range.start(), *self.range.end());

            let stroke = ui.style().interact(&point_response).fg_stroke;

            CircleShape {
                center: point_in_screen,
                radius: self.control_point_radius,
                fill: self.cursor_fill,
                stroke,
            }
        };

        // handle upper bound
        let upper_bound = {
            // get the control point
            let size = Vec2::splat(2.0 * self.control_point_radius);
            let point_in_screen = to_screen.transform_pos(Pos2 {
                x: self.val_to_x(*self.right_slider),
                y: self.control_point_radius + OFFSET,
            });
            let point_rect = Rect::from_center_size(point_in_screen, size);
            let point_id = response.id.with(1);
            let point_response = ui.interact(point_rect, point_id, Sense::drag());

            if point_response.dragged() {
                response.mark_changed();
            }

            // handle logic
            *self.right_slider += self.x_to_val(point_response.drag_delta().x);
            if *self.left_slider > *self.right_slider - self.separation_distance {
                *self.left_slider = *self.right_slider - self.separation_distance;
            }
            *self.right_slider = self
                .right_slider
                .clamp(*self.range.start(), *self.range.end());
            *self.left_slider = self
                .left_slider
                .clamp(*self.range.start(), *self.range.end());

            let stroke = ui.style().interact(&point_response).fg_stroke;

            CircleShape {
                center: point_in_screen,
                radius: self.control_point_radius,
                fill: self.cursor_fill,
                stroke,
            }
        };

        let points_in_screen: Vec<Pos2> = [
            self.val_to_x(*self.left_slider),
            self.val_to_x(*self.right_slider),
        ]
        .iter()
        .map(|p| {
            to_screen
                * Pos2 {
                    x: *p,
                    y: self.control_point_radius + OFFSET,
                }
        })
        .collect();

        // draw line between points
        painter.add(PathShape::line(points_in_screen, self.stroke));
        // draw control points
        painter.extend([Shape::Circle(lower_bound), Shape::Circle(upper_bound)]);

        response
    }
}
