use crate::{Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use epaint::Rgba;
use hsluv::{lch_to_rgb, rgb_to_lch};
use std::any::Any;

pub trait Lerp: PartialEq + Clone + Any {
    fn lerp(&self, to: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        ((to - self) * t) + self
    }
}

impl Lerp for f64 {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        ((to - self) * t as f64) + self
    }
}

impl Lerp for Pos2 {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Pos2 {
            x: self.x.lerp(&to.x, t),
            y: self.y.lerp(&to.y, t),
        }
    }
}

impl Lerp for Vec2 {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Vec2 {
            x: self.x.lerp(&to.x, t),
            y: self.y.lerp(&to.y, t),
        }
    }
}

impl Lerp for Rounding {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Rounding {
            nw: self.nw.lerp(&to.nw, t),
            ne: self.ne.lerp(&to.ne, t),
            sw: self.sw.lerp(&to.sw, t),
            se: self.se.lerp(&to.se, t),
        }
    }
}

impl Lerp for Rect {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Rect {
            min: self.min.lerp(&to.min, t),
            max: self.max.lerp(&to.max, t),
        }
    }
}

impl Lerp for Color32 {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Color32::from(Rgba::from(*self).lerp(&Rgba::from(*to), t))
    }
}

impl Lerp for Rgba {
    // Lerps in lch for better blending
    fn lerp(&self, to: &Self, t: f32) -> Self {
        let v0 = self.to_rgba_unmultiplied();
        let v1 = to.to_rgba_unmultiplied();
        let lch = rgb_to_lch((v0[0] as f64, v0[1] as f64, v0[2] as f64));
        let to_lch = rgb_to_lch((v1[0] as f64, v1[1] as f64, v1[2] as f64));

        let out = lch_to_rgb((
            lch.0.lerp(&to_lch.0, t),
            lch.1.lerp(&to_lch.1, t),
            lch.2.lerp(&to_lch.2, t),
        ));

        Rgba::from_rgba_unmultiplied(
            out.0 as f32,
            out.1 as f32,
            out.2 as f32,
            self.a().lerp(&to.a(), t),
        )
    }
}

impl Lerp for Stroke {
    fn lerp(&self, to: &Self, t: f32) -> Self {
        Stroke {
            width: self.width.lerp(&to.width, t),
            color: self.color.lerp(&to.color, t),
        }
    }
}
