//! Which part of a back buffer to repaint, from what changed in each frame and how old the buffer is.

use std::collections::VecDeque;

use egui::{
    Rect,
    epaint::{ClippedShape, Damage, DamageTracker, textures::TexturesDelta},
};

/// A back buffer older than this many frames is repainted whole. Triple buffering gives an age of 3.
const MAX_BUFFER_AGE: usize = 4;

/// What to repaint in the back buffer and what to tell the compositor, for one frame.
#[derive(Debug, PartialEq, Eq)]
pub struct Plan {
    /// Where the back buffer differs from this frame.
    pub repaint: Damage,

    /// Where this frame differs from the previous one.
    pub presented: Damage,
}

#[derive(Default)]
pub struct PartialRepaint {
    tracker: DamageTracker,

    /// The damage of the last frames, newest first.
    history: VecDeque<Damage>,

    screen_size_px: [u32; 2],
}

impl PartialRepaint {
    /// The window surface was recreated or its pixels lost: the next frame is painted whole.
    pub fn reset(&mut self) {
        self.tracker.reset();
        self.history.clear();
    }

    /// `buffer_age` is how many frames ago the back buffer was presented, 0 when unknown.
    pub fn plan(
        &mut self,
        shapes: &[ClippedShape],
        pixels_per_point: f32,
        textures_delta: &TexturesDelta,
        screen_size_px: [u32; 2],
        buffer_age: u32,
    ) -> Plan {
        if screen_size_px != self.screen_size_px {
            self.screen_size_px = screen_size_px;
            self.reset();
        }
        let presented = self
            .tracker
            .update(shapes, pixels_per_point, textures_delta);
        self.history.push_front(presented.clone());
        self.history.truncate(MAX_BUFFER_AGE);

        let age = buffer_age as usize;
        let repaint = if (1..=self.history.len()).contains(&age) {
            self.history
                .iter()
                .take(age)
                .cloned()
                .fold(Damage::None, Damage::union)
        } else {
            Damage::Full
        };
        Plan { repaint, presented }
    }
}

/// `damage` in whole pixels from the bottom left, as EGL wants it; `None` for the whole window.
///
/// An unchanged frame gives one pixel, since no rectangle at all would mean the whole window.
pub fn damage_in_pixels(
    damage: &Damage,
    pixels_per_point: f32,
    [width, height]: [u32; 2],
) -> Option<Vec<[i32; 4]>> {
    let rects = match damage {
        Damage::Full => return None,
        Damage::None => return Some(vec![[0, 0, 1, 1]]),
        Damage::Partial(region) => region.rects(),
    };
    let screen = Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(width as f32, height as f32));
    let pixels = rects
        .iter()
        .map(|rect| {
            let rect = (*rect * pixels_per_point).expand(1.0).intersect(screen);
            let (left, right) = (rect.min.x.floor() as i32, rect.max.x.ceil() as i32);
            let (top, bottom) = (rect.min.y.floor() as i32, rect.max.y.ceil() as i32);
            [left, height as i32 - bottom, right - left, bottom - top]
        })
        .filter(|[_, _, width, height]| *width > 0 && *height > 0)
        .collect::<Vec<_>>();
    Some(if pixels.is_empty() {
        vec![[0, 0, 1, 1]]
    } else {
        pixels
    })
}

#[cfg(test)]
mod tests {
    use egui::{Color32, epaint::RectShape, pos2, vec2};

    use super::*;

    const SCREEN: [u32; 2] = [200, 100];

    fn frame(x: f32) -> Vec<ClippedShape> {
        let everything = Rect::from_min_size(pos2(0.0, 0.0), vec2(200.0, 100.0));
        vec![
            ClippedShape::new(
                everything,
                RectShape::filled(everything, 0.0, Color32::GRAY),
            ),
            ClippedShape::new(
                everything,
                RectShape::filled(
                    Rect::from_min_size(pos2(x, 10.0), vec2(10.0, 10.0)),
                    0.0,
                    Color32::RED,
                ),
            ),
        ]
    }

    fn plan(partial: &mut PartialRepaint, x: f32, buffer_age: u32) -> Plan {
        partial.plan(
            &frame(x),
            1.0,
            &TexturesDelta::default(),
            SCREEN,
            buffer_age,
        )
    }

    fn bounding_rect(damage: &Damage) -> Rect {
        match damage {
            Damage::Partial(region) => region.bounding_rect(),
            other => panic!("expected a partial damage, got {other:?}"),
        }
    }

    #[test]
    fn the_first_frame_and_an_unknown_buffer_age_repaint_everything() {
        let mut partial = PartialRepaint::default();
        assert_eq!(plan(&mut partial, 0.0, 1).repaint, Damage::Full);
        let plan = plan(&mut partial, 20.0, 0);
        assert_eq!(plan.repaint, Damage::Full);
        assert!(
            matches!(plan.presented, Damage::Partial(_)),
            "the compositor still gets the change"
        );
    }

    #[test]
    fn a_buffer_from_the_previous_frame_repaints_this_change_only() {
        let mut partial = PartialRepaint::default();
        plan(&mut partial, 0.0, 0);
        let plan = plan(&mut partial, 100.0, 1);
        assert_eq!(plan.repaint, plan.presented);
        let changed = bounding_rect(&plan.repaint);
        assert!(changed.min.x < 10.0 && changed.max.x > 110.0, "{changed:?}");
    }

    #[test]
    fn an_older_buffer_repaints_the_changes_it_missed() {
        let mut partial = PartialRepaint::default();
        plan(&mut partial, 0.0, 0);
        plan(&mut partial, 50.0, 1);
        plan(&mut partial, 50.0, 1);
        let plan = plan(&mut partial, 150.0, 3);
        let repaint = match &plan.repaint {
            Damage::Partial(region) => region.rects().to_vec(),
            other => panic!("{other:?}"),
        };
        for missed in [0.0, 50.0, 150.0] {
            let square = Rect::from_min_size(pos2(missed, 10.0), vec2(10.0, 10.0));
            assert!(
                repaint.iter().any(|rect| rect.contains_rect(square)),
                "{square:?} not in {repaint:?}"
            );
        }
        assert_ne!(
            plan.repaint, plan.presented,
            "the compositor needs the last change only"
        );
    }

    #[test]
    fn a_buffer_older_than_the_history_repaints_everything() {
        let mut partial = PartialRepaint::default();
        for x in 0..=MAX_BUFFER_AGE {
            plan(&mut partial, x as f32, 1);
        }
        assert_eq!(
            plan(&mut partial, 0.0, MAX_BUFFER_AGE as u32 + 1).repaint,
            Damage::Full
        );
    }

    #[test]
    fn a_resized_window_repaints_everything() {
        let mut partial = PartialRepaint::default();
        plan(&mut partial, 0.0, 0);
        let plan = partial.plan(&frame(0.0), 1.0, &TexturesDelta::default(), [300, 100], 1);
        assert_eq!(plan.repaint, Damage::Full);
    }

    #[test]
    fn an_unchanged_frame_repaints_nothing_and_presents_a_pixel() {
        let mut partial = PartialRepaint::default();
        plan(&mut partial, 0.0, 0);
        let plan = plan(&mut partial, 0.0, 1);
        assert_eq!(plan.repaint, Damage::None);
        assert_eq!(
            damage_in_pixels(&plan.presented, 1.0, SCREEN),
            Some(vec![[0, 0, 1, 1]])
        );
    }

    #[test]
    fn damage_in_pixels_is_flipped_scaled_and_grown() {
        let mut partial = PartialRepaint::default();
        plan(&mut partial, 0.0, 0);
        let presented = plan(&mut partial, 0.0, 1).presented;
        assert_eq!(
            damage_in_pixels(&presented, 2.0, SCREEN),
            Some(vec![[0, 0, 1, 1]])
        );

        let damage = partial
            .plan(&frame(40.0), 2.0, &TexturesDelta::default(), SCREEN, 1)
            .repaint;
        assert_eq!(damage, Damage::Full, "another pixels_per_point");
        let presented = partial
            .plan(&frame(60.0), 2.0, &TexturesDelta::default(), SCREEN, 1)
            .presented;
        let changed = bounding_rect(&presented);
        let [[left, bottom, width, height]] =
            damage_in_pixels(&presented, 2.0, SCREEN).unwrap()[..]
        else {
            panic!("one rect expected");
        };
        assert!(left as f32 <= changed.min.x * 2.0 - 1.0, "{left}");
        assert!(
            (left + width) as f32 >= changed.max.x * 2.0 + 1.0,
            "{width}"
        );
        assert_eq!(
            bottom + height,
            SCREEN[1] as i32 - (changed.min.y * 2.0 - 1.0).floor() as i32
        );
        assert!(height > 0);
    }
}
