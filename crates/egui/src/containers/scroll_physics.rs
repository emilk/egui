//! How one axis of a [`crate::ScrollArea`] moves: kinetic coasting, and rubber-banding
//! past the edge of the content.
//!
//! This is pure math with no egui state, so that it is easy to test.
//!
//! Everything is in "offset space": a positive offset means the content is scrolled down/right,
//! and a positive velocity means the offset is increasing.
//! Distances are in ui points, times in seconds.

use emath::Rangef;

use crate::{MouseWheelSource, style::KineticScrollStyle};

/// How closely the content follows the finger when first dragged past the edge on a touch screen.
///
/// `1.0` would be exactly, `0.0` not at all. The resistance then grows the further out you go.
/// This is what iOS uses.
const TOUCH_RUBBER_BAND_SLOPE: f32 = 0.55;

/// How far past the edge a touch drag can stretch, as a fraction of the viewport.
///
/// This is what iOS uses.
const TOUCH_MAX_STRETCH: f32 = 1.0;

/// Like [`TOUCH_RUBBER_BAND_SLOPE`], but for trackpads.
///
/// Trackpad scroll deltas are accelerated by the OS, so the fingers move much less
/// than the deltas suggest. We also want to match the tight feel of native macOS,
/// so the content follows the fingers a lot less than on a touch screen.
const TRACKPAD_RUBBER_BAND_SLOPE: f32 = 0.3;

/// How far past the edge a trackpad scroll can stretch, as a fraction of the viewport.
const TRACKPAD_MAX_STRETCH: f32 = 1.0 / 3.0;

/// Time constant of the critically damped spring that pulls the content back to the edge,
/// in seconds. It settles in roughly five time constants.
const RUBBER_BAND_TIME: f32 = 0.1;

/// The valid range of scroll offsets along one axis.
///
/// `max_offset` is `content_size - viewport_size`, which is negative if the content fits.
pub fn scroll_bounds(max_offset: f32) -> Rangef {
    Rangef::new(0.0, max_offset.max(0.0))
}

/// A scroll-wheel (or trackpad) delta, for [`AxisPhysics::wheel`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WheelScroll {
    /// How far to move the offset.
    pub delta: f32,

    /// What is driving the scrolling? See [`crate::InputState::scroll_source`].
    pub source: Option<MouseWheelSource>,

    /// Is there an enclosing scroll area that could take the delta if we can't use it?
    pub can_chain_to_parent: bool,
}

/// How one axis of a scroll area moves when coasting, and when pulled past the edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisPhysics {
    /// The valid offsets. Anything outside is overscroll.
    pub bounds: Rangef,

    /// Size of the viewport along this axis.
    ///
    /// The rubber band can never stretch further than this.
    pub viewport_extent: f32,

    /// Set [`KineticScrollStyle::rubber_band`] to `false` to never move past the edge:
    /// no resistance, no spring.
    pub style: KineticScrollStyle,
}

impl AxisPhysics {
    pub fn clamp(&self, offset: f32) -> f32 {
        self.bounds.clamp(offset)
    }

    /// How far past the edge `offset` is. Negative means before the start.
    pub fn overscroll(&self, offset: f32) -> f32 {
        offset - self.clamp(offset)
    }

    pub fn is_overscrolled(&self, offset: f32) -> bool {
        self.overscroll(offset) != 0.0
    }

    /// Are we deliberately past the edge, i.e. should the offset be left unclamped this frame?
    pub fn is_rubber_banding(&self, offset: f32) -> bool {
        self.style.rubber_band && self.is_overscrolled(offset)
    }

    /// A finger on a touch screen moved the content by `delta`.
    ///
    /// Inside the bounds the content follows the finger exactly.
    /// Past the edge it follows with increasing resistance, if rubber-banding is enabled.
    /// If it isn't, the content still follows the finger exactly,
    /// and the caller is expected to clamp the offset at the end of the frame.
    pub fn drag(&self, offset: f32, delta: f32) -> f32 {
        if self.style.rubber_band {
            rubber_band_drag(
                offset,
                delta,
                self.bounds,
                self.viewport_extent * TOUCH_MAX_STRETCH,
                TOUCH_RUBBER_BAND_SLOPE,
            )
        } else {
            offset + delta
        }
    }

    /// Nothing is touching the content; let `dt` seconds pass.
    ///
    /// If we are past the edge (and rubber-banding is enabled) we spring back to it,
    /// unless `hold` is set (the fingers are resting on the trackpad, like on macOS).
    /// Otherwise we coast with decaying velocity.
    ///
    /// Returns `true` if we are still moving and need another frame.
    pub fn step(&self, offset: &mut f32, vel: &mut f32, dt: f32, hold: bool) -> bool {
        let KineticScrollStyle {
            decay_time,
            stop_distance,
            rubber_band,
        } = self.style;

        let edge = self.clamp(*offset);
        let overscroll = *offset - edge;

        if rubber_band && overscroll != 0.0 {
            if hold {
                return false;
            }

            let (new_overscroll, new_vel) =
                critically_damped_spring_step(overscroll, *vel, dt, RUBBER_BAND_TIME);

            let settled = new_overscroll.abs() < stop_distance
                && (new_vel * RUBBER_BAND_TIME).abs() < stop_distance;
            if settled {
                *offset = edge;
                *vel = 0.0;
                false
            } else {
                *offset = edge + new_overscroll;
                *vel = new_vel;
                true
            }
        } else {
            // Kinetic scrolling, modeled after `UIScrollView` on iOS/macOS:
            // the velocity decays exponentially, `v(t) = v₀ · exp(-t / decay_time)`,
            // so the total coast distance is `v₀ · decay_time`.
            let remaining_distance = vel.abs() * decay_time;
            if remaining_distance < stop_distance || !remaining_distance.is_finite() {
                *vel = 0.0;
                false
            } else {
                let new_vel = *vel * (-dt / decay_time).exp();
                // The exact integral of the velocity over this frame is
                // `decay_time * (old_vel - new_vel)`, which makes the
                // coast distance independent of frame rate.
                *offset += decay_time * (*vel - new_vel);
                *vel = new_vel;
                true
            }
        }
    }

    /// A scroll-wheel or trackpad delta arrived while the pointer is over us.
    ///
    /// * If there is room, we scroll.
    /// * While fingers are on a trackpad we rubber-band past the edge (if enabled), unless we
    ///   were already at the edge when the gesture started and there is an enclosing scroll
    ///   area that can take the delta instead. Mouse wheels, and sources we can't tell,
    ///   stop hard at the edge, as they do on macOS.
    /// * During the OS-driven momentum phase we bounce off the edge once and then ignore the
    ///   rest of it, so that the decaying deltas don't keep pushing us out.
    ///   `bounced_this_momentum` remembers that between calls.
    ///
    /// Returns `true` if the delta was consumed, i.e. no enclosing scroll area should get it.
    pub fn wheel(
        &self,
        offset: &mut f32,
        vel: &mut f32,
        scroll: WheelScroll,
        bounced_this_momentum: &mut bool,
    ) -> bool {
        let WheelScroll {
            delta,
            source,
            can_chain_to_parent,
        } = scroll;

        let is_momentum = source == Some(MouseWheelSource::Momentum);
        let is_trackpad = matches!(
            source,
            Some(MouseWheelSource::Trackpad | MouseWheelSource::Momentum)
        );
        if !is_momentum {
            *bounced_this_momentum = false;
        }
        if delta == 0.0 {
            return false;
        }

        let can_scroll = (delta < 0.0 && self.bounds.min < *offset)
            || (0.0 < delta && *offset < self.bounds.max);

        let rubber_band = self.style.rubber_band
            && is_trackpad
            && (can_scroll
                || self.is_overscrolled(*offset)
                || !can_chain_to_parent
                || *bounced_this_momentum);

        if !(rubber_band || can_scroll) {
            return false;
        }

        if rubber_band && is_momentum {
            if *bounced_this_momentum {
                // Ignore the rest of the momentum phase.
            } else if self.is_overscrolled(*offset) {
                // We're already past the edge, pulled there by the fingers.
                // Let the spring bring us back, without adding the
                // OS momentum on top (that would overshoot a lot).
                *bounced_this_momentum = true;
            } else {
                *bounced_this_momentum = self.kick(offset, vel, delta);
            }
        } else if rubber_band {
            *offset = rubber_band_drag(
                *offset,
                delta,
                self.bounds,
                self.viewport_extent * TRACKPAD_MAX_STRETCH,
                TRACKPAD_RUBBER_BAND_SLOPE,
            );
            *vel = 0.0;
        } else {
            *offset = self.clamp(*offset + delta);
        }

        true
    }

    /// Scroll normally up to the edge. The part of `delta` that would have taken us past
    /// the edge becomes velocity instead, so that [`Self::step`] bounces us off the edge,
    /// about as far as that part of the delta would have scrolled us.
    ///
    /// Returns `true` if we hit the edge.
    fn kick(&self, offset: &mut f32, vel: &mut f32, delta: f32) -> bool {
        let unclamped = *offset + delta;
        *offset = self.clamp(unclamped);
        let past_edge = unclamped - *offset;
        if past_edge == 0.0 {
            false
        } else {
            // A critically damped spring kicked with velocity `v` from rest peaks at `v · τ / e`:
            *vel = past_edge * core::f32::consts::E / RUBBER_BAND_TIME;
            true
        }
    }
}

/// Apply a drag `delta` to `offset`, with rubber-band resistance outside of `bounds`.
///
/// Modeled after iOS: the content follows the finger with the given `slope` at the edge,
/// and the resistance grows as we get further out,
/// asymptotically approaching an overscroll of `viewport_extent`.
fn rubber_band_drag(
    mut offset: f32,
    mut delta: f32,
    bounds: Rangef,
    viewport_extent: f32,
    slope: f32,
) -> f32 {
    // Move freely while inside the bounds:
    if bounds.contains(offset) {
        let unclamped = offset + delta;
        offset = bounds.clamp(unclamped);
        delta = unclamped - offset;
    }
    if delta == 0.0 {
        return offset;
    }

    let edge = bounds.clamp(offset);
    let overscroll = offset - edge;

    // iOS: `overscroll(raw) = d · (1 − 1 / (slope · raw / d + 1))`, with `d = viewport_extent`.
    // Differentiating gives `d overscroll / d raw = slope · (1 − overscroll / d)²`,
    // which we integrate incrementally so we don't need to track the raw finger distance.
    let fraction = if 0.0 < viewport_extent {
        (overscroll.abs() / viewport_extent).min(1.0)
    } else {
        1.0
    };
    let factor = slope * (1.0 - fraction).powi(2);

    let new_overscroll = overscroll + delta * factor;

    if overscroll != 0.0 && new_overscroll.signum() != overscroll.signum() {
        // We dragged back past the edge into the valid range.
        // Spend the rest of the delta moving freely inside the bounds:
        let delta_to_reach_edge = -overscroll / factor;
        let remaining = delta - delta_to_reach_edge;
        bounds.clamp(edge + remaining)
    } else {
        edge + new_overscroll
    }
}

/// Advance a critically damped spring by `dt`, returning the new `(position, velocity)`.
///
/// The spring pulls towards zero with time constant `time_constant`.
/// This is the exact solution, so it is independent of frame rate.
fn critically_damped_spring_step(x0: f32, v0: f32, dt: f32, time_constant: f32) -> (f32, f32) {
    if time_constant <= 0.0 {
        return (0.0, 0.0);
    }
    let w = 1.0 / time_constant;
    let e = (-w * dt).exp();
    let a = v0 + w * x0;
    ((x0 + a * dt) * e, (v0 - a * w * dt) * e)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    fn physics(rubber_band: bool) -> AxisPhysics {
        AxisPhysics {
            bounds: Rangef::new(0.0, 100.0),
            viewport_extent: 200.0,
            style: KineticScrollStyle {
                rubber_band,
                ..Default::default()
            },
        }
    }

    fn wheel(
        delta: f32,
        source: Option<MouseWheelSource>,
        can_chain_to_parent: bool,
    ) -> WheelScroll {
        WheelScroll {
            delta,
            source,
            can_chain_to_parent,
        }
    }

    /// Run `step` until it stops, returning the lowest offset seen.
    fn settle(p: &AxisPhysics, offset: &mut f32, vel: &mut f32) -> f32 {
        let mut min_offset = *offset;
        for _ in 0..1000 {
            if !p.step(offset, vel, DT, false) {
                return min_offset;
            }
            min_offset = min_offset.min(*offset);
        }
        panic!("Should settle eventually");
    }

    #[test]
    fn drag_resists_past_the_edge() {
        let p = physics(true);

        // At the edge, the content follows the finger with the given slope:
        let offset = p.drag(0.0, -10.0);
        assert!(
            (offset - -10.0 * TOUCH_RUBBER_BAND_SLOPE).abs() < 1e-4,
            "{offset}"
        );
        assert!(p.is_rubber_banding(offset));

        // Resistance grows the further out we go, and we never exceed the viewport:
        let mut offset = 0.0;
        let mut prev_step = f32::INFINITY;
        for _ in 0..1000 {
            let new_offset = p.drag(offset, -10.0);
            let step = offset - new_offset;
            assert!(0.0 <= step && step <= prev_step, "{step} > {prev_step}");
            prev_step = step;
            offset = new_offset;
        }
        assert!(
            -p.viewport_extent < offset && offset < -p.viewport_extent * 0.9,
            "{offset}"
        );

        // A delta that is partially inside is split: free up to the edge, resisted after.
        let offset = p.drag(5.0, -15.0);
        assert!(
            (offset - -10.0 * TOUCH_RUBBER_BAND_SLOPE).abs() < 1e-4,
            "{offset}"
        );
    }

    #[test]
    fn drag_back_into_bounds() {
        let p = physics(true);
        // From -5.5 overscroll, dragging +20 raw gets us back to the edge after ≈10.5
        // (the resistance factor is ≈0.52 there), then moves freely for the remaining ≈9.5:
        let offset = p.drag(-5.5, 20.0);
        assert!((offset - 9.43).abs() < 0.05, "{offset}");
    }

    #[test]
    fn step_coasts_and_stops() {
        let p = physics(true);
        let (mut offset, mut vel) = (10.0, 100.0);
        settle(&p, &mut offset, &mut vel);
        // Coast distance is `v₀ · decay_time`:
        let expected = 10.0 + 100.0 * p.style.decay_time;
        assert!((offset - expected).abs() < 1.0, "{offset}");
        assert_eq!(vel, 0.0);
    }

    #[test]
    fn step_springs_back_without_overshoot() {
        let p = physics(true);
        let (mut offset, mut vel) = (-50.0, 0.0);
        let mut prev = offset;
        while p.step(&mut offset, &mut vel, DT, false) {
            assert!(prev <= offset && offset <= 0.0, "{prev} -> {offset}");
            prev = offset;
        }
        assert_eq!((offset, vel), (0.0, 0.0));
    }

    #[test]
    fn spring_is_frame_rate_independent() {
        let tau = 0.1;
        let (x0, v0) = (50.0, -200.0);

        let (mut x, mut v) = (x0, v0);
        for _ in 0..60 {
            (x, v) = critically_damped_spring_step(x, v, DT, tau);
        }
        let (x_coarse, v_coarse) = critically_damped_spring_step(x0, v0, 1.0, tau);

        assert!((x - x_coarse).abs() < 1e-3, "{x} vs {x_coarse}");
        assert!((v - v_coarse).abs() < 1e-2, "{v} vs {v_coarse}");
    }

    #[test]
    fn trackpad_chains_to_parent_when_at_the_edge_at_gesture_start() {
        let p = physics(true);
        let (mut offset, mut vel) = (0.0, 0.0);
        let mut bounced = false;
        let fingers = Some(MouseWheelSource::Trackpad);

        // At the edge with a parent: let the parent have it.
        assert!(!p.wheel(
            &mut offset,
            &mut vel,
            wheel(-10.0, fingers, true),
            &mut bounced
        ));
        assert_eq!(offset, 0.0);

        // But once we have scrolled, we own the gesture, and rubber-band past the edge:
        offset = 5.0;
        assert!(p.wheel(
            &mut offset,
            &mut vel,
            wheel(-10.0, fingers, true),
            &mut bounced
        ));
        assert!(offset < 0.0, "{offset}");
        assert!(p.wheel(
            &mut offset,
            &mut vel,
            wheel(-10.0, fingers, true),
            &mut bounced
        ));
    }

    #[test]
    fn momentum_bounces_once_then_is_ignored() {
        let p = physics(true);
        let (mut offset, mut vel) = (30.0, 0.0);
        let mut bounced = false;
        let momentum = Some(MouseWheelSource::Momentum);

        // Scrolls normally until the edge, where the excess becomes velocity:
        for _ in 0..3 {
            assert!(p.wheel(
                &mut offset,
                &mut vel,
                wheel(-10.0, momentum, false),
                &mut bounced
            ));
        }
        assert_eq!((offset, vel, bounced), (0.0, 0.0, false));
        assert!(p.wheel(
            &mut offset,
            &mut vel,
            wheel(-10.0, momentum, false),
            &mut bounced
        ));
        assert!(bounced);
        assert!(vel < 0.0, "{vel}");

        // Further momentum is swallowed without effect:
        let before = (offset, vel);
        assert!(p.wheel(
            &mut offset,
            &mut vel,
            wheel(-10.0, momentum, false),
            &mut bounced
        ));
        assert_eq!((offset, vel), before);

        // The spring bounces us out and back:
        let min_offset = settle(&p, &mut offset, &mut vel);
        assert!(min_offset < -5.0, "{min_offset}");
        assert_eq!((offset, vel), (0.0, 0.0));

        // A new finger gesture resets the bounce:
        assert!(p.wheel(
            &mut offset,
            &mut vel,
            wheel(-1.0, Some(MouseWheelSource::Trackpad), false),
            &mut bounced
        ));
        assert!(!bounced);
    }

    #[test]
    fn momentum_after_rubber_banding_does_not_push_further() {
        let p = physics(true);
        let mut bounced = false;

        // The fingers pulled us past the edge:
        let (mut offset, mut vel) = (-40.0, 0.0);

        // Momentum arrives; it must not push us further out, nor kick us:
        for _ in 0..5 {
            assert!(p.wheel(
                &mut offset,
                &mut vel,
                wheel(-30.0, Some(MouseWheelSource::Momentum), false),
                &mut bounced
            ));
            assert_eq!((offset, vel), (-40.0, 0.0));
        }
        assert!(bounced);
    }
}
