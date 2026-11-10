use crate::{Pos2, Rangef, Rect};

/// A region of space made up of [`Rect`]s: every point that is inside at least one of them.
///
/// The rects may overlap. A rect that isn't positive ([`Rect::is_positive`]) adds nothing to the
/// region.
///
/// Two sets are equal if they have the same rects in the same order, so two sets that cover
/// the same region can still differ.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct RectSet(pub Vec<Rect>);

impl RectSet {
    /// A set without rects, which covers nothing.
    #[inline]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Does the set cover nothing?
    ///
    /// That is the case if it has no positive rect ([`Rect::is_positive`]).
    #[inline]
    pub fn is_empty(&self) -> bool {
        !self.0.iter().any(Rect::is_positive)
    }

    /// Iterates over the rects of the set.
    #[inline]
    pub fn iter(&self) -> core::slice::Iter<'_, Rect> {
        self.0.iter()
    }

    /// Is `pos` inside one of the positive rects?
    ///
    /// Like with [`Rect::contains`], the edges of a rect are inside it.
    #[inline]
    pub fn contains(&self, pos: Pos2) -> bool {
        (self.0.iter()).any(|rect| rect.is_positive() && rect.contains(pos))
    }

    /// The smallest [`Rect`] around all the positive rects of the set.
    ///
    /// [`Rect::NOTHING`] if the set is empty.
    pub fn bounding_rect(&self) -> Rect {
        self.0
            .iter()
            .copied()
            .filter(Rect::is_positive)
            .fold(Rect::NOTHING, Rect::union)
    }

    /// The region covered by this set or by `other`: the rects of both.
    #[must_use]
    #[inline]
    pub fn union(mut self, other: &Self) -> Self {
        self.0.extend_from_slice(&other.0);
        self
    }

    /// The region covered by this set but not by `other`, as rects that don't overlap.
    ///
    /// It cuts the region into horizontal bands at the top and bottom edges of all the rects,
    /// and joins the parts of adjacent bands that span the same x-range, so the result is a few
    /// tall rects rather than many small ones.
    #[must_use]
    pub fn subtract(&self, other: &Self) -> Self {
        let rects: Vec<Rect> = (self.0.iter().copied()).filter(Rect::is_positive).collect();
        let bounds = self.bounding_rect();
        let cuts: Vec<Rect> = (other.0.iter())
            .map(|cut| cut.intersect(bounds))
            .filter(Rect::is_positive)
            .collect();

        let mut ys: Vec<f32> = core::iter::chain(&rects, &cuts)
            .flat_map(|rect| [rect.min.y, rect.max.y])
            .collect();
        ys.sort_by(f32::total_cmp);
        ys.dedup();

        let mut parts = Vec::new();
        // The parts of the band above, which a part of this band with the same x-range extends.
        let mut open: Vec<Rect> = Vec::new();
        for band in ys.windows(2) {
            let (top, bottom) = (band[0], band[1]);
            // Every rect that reaches into the band spans its whole height.
            let x_ranges_in_band = |rects: &[Rect]| {
                let x_ranges = (rects.iter())
                    .filter(|rect| rect.min.y < bottom && top < rect.max.y)
                    .map(|rect| rect.x_range())
                    .collect();
                merged(x_ranges)
            };
            let covered = x_ranges_in_band(&rects);
            let cuts = x_ranges_in_band(&cuts);

            let mut still_open = Vec::new();
            for x_range in without(&covered, &cuts) {
                let above = open.iter().position(|part| part.x_range() == x_range);
                let part = if let Some(above) = above {
                    let mut part = open.swap_remove(above);
                    part.max.y = bottom;
                    part
                } else {
                    Rect::from_x_y_ranges(x_range, top..=bottom)
                };
                still_open.push(part);
            }
            parts.append(&mut open);
            open = still_open;
        }
        parts.append(&mut open);
        Self(parts)
    }
}

/// Joins the ranges that overlap or touch. The result is sorted, with a gap between any two.
fn merged(mut ranges: Vec<Rangef>) -> Vec<Rangef> {
    ranges.sort_by(|a, b| a.min.total_cmp(&b.min));
    let mut merged: Vec<Rangef> = Vec::with_capacity(ranges.len());
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.min <= last.max => last.max = last.max.max(range.max),
            _ => merged.push(range),
        }
    }
    merged
}

/// The parts of `ranges` outside `cuts`. Both are as [`merged`] returns them.
fn without(ranges: &[Rangef], cuts: &[Rangef]) -> Vec<Rangef> {
    let mut parts = Vec::new();
    for range in ranges {
        let mut x = range.min;
        for cut in cuts {
            if cut.min < range.max && range.min < cut.max {
                if x < cut.min {
                    parts.push(Rangef::new(x, cut.min));
                }
                x = cut.max;
            }
        }
        if x < range.max {
            parts.push(Rangef::new(x, range.max));
        }
    }
    parts
}

impl From<Rect> for RectSet {
    #[inline]
    fn from(rect: Rect) -> Self {
        Self(vec![rect])
    }
}

impl From<Vec<Rect>> for RectSet {
    #[inline]
    fn from(rects: Vec<Rect>) -> Self {
        Self(rects)
    }
}

impl FromIterator<Rect> for RectSet {
    #[inline]
    fn from_iter<I: IntoIterator<Item = Rect>>(rects: I) -> Self {
        Self(rects.into_iter().collect())
    }
}

impl Extend<Rect> for RectSet {
    #[inline]
    fn extend<I: IntoIterator<Item = Rect>>(&mut self, rects: I) {
        self.0.extend(rects);
    }
}

impl IntoIterator for RectSet {
    type Item = Rect;
    type IntoIter = alloc::vec::IntoIter<Rect>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a RectSet {
    type Item = &'a Rect;
    type IntoIter = core::slice::Iter<'a, Rect>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::RectSet;
    use crate::{Rect, pos2, vec2};

    fn assert_disjoint(parts: &RectSet) {
        for (i, a) in parts.iter().enumerate() {
            for b in &parts.0[i + 1..] {
                assert!(!a.intersect(*b).is_positive(), "{a:?} overlaps {b:?}");
            }
        }
    }

    fn area(set: &RectSet) -> f32 {
        set.iter().map(Rect::area).sum()
    }

    #[test]
    fn subtracting_a_rect_in_the_middle_leaves_a_frame_around_it() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0));
        let other = Rect::from_min_size(pos2(10.0, 20.0), vec2(50.0, 30.0));
        let parts = rect.subtract(&other.into());
        assert_eq!(parts.0.len(), 4, "{parts:?}");
        assert_disjoint(&parts);
        assert_eq!(area(&parts), rect.area() - other.area(), "{parts:?}");
        for part in &parts {
            assert!(!part.intersect(other).is_positive(), "{part:?} covers it");
        }
    }

    #[test]
    fn subtracting_overlapping_rects_removes_the_overlap_once() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0));
        let others = RectSet(vec![
            Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 60.0)),
            Rect::from_min_size(pos2(40.0, 40.0), vec2(60.0, 60.0)),
        ]);
        let parts = rect.subtract(&others);
        assert_disjoint(&parts);
        let covered = 2.0 * 60.0 * 60.0 - 20.0 * 20.0;
        assert_eq!(area(&parts), rect.area() - covered, "{parts:?}");
    }

    #[test]
    fn subtracting_a_ring_of_rects_leaves_few_rects() {
        // A page with margins around it, and a window over it whose own page is cut out too:
        // what is left is the margins and the window's frame.
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(1000.0, 700.0));
        let page = Rect::from_min_max(pos2(8.0, 33.0), pos2(992.0, 640.0));
        let window = Rect::from_min_max(pos2(300.0, 200.0), pos2(780.0, 560.0));
        let window_page = Rect::from_min_max(pos2(306.0, 250.0), pos2(774.0, 554.0));
        // The page around the window, as four rects.
        let others = RectSet(vec![
            Rect::from_min_max(page.min, pos2(page.max.x, window.min.y)),
            Rect::from_min_max(pos2(page.min.x, window.max.y), page.max),
            Rect::from_min_max(
                pos2(page.min.x, window.min.y),
                pos2(window.min.x, window.max.y),
            ),
            Rect::from_min_max(
                pos2(window.max.x, window.min.y),
                pos2(page.max.x, window.max.y),
            ),
            window_page,
        ]);
        let parts = canvas.subtract(&others);
        assert_disjoint(&parts);
        let frame = window.area() - window_page.area();
        assert_eq!(
            area(&parts),
            canvas.area() - page.area() + frame,
            "{parts:?}"
        );
        assert_eq!(parts.0.len(), 8, "four margins and the window's four sides");
    }

    #[test]
    fn subtracting_nothing_everything_or_a_rect_outside() {
        let rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 80.0));
        assert_eq!(rect.subtract(&RectSet::new()), RectSet::from(rect));
        assert!(rect.subtract(&rect.expand(5.0).into()).is_empty());
        let outside = Rect::from_min_size(pos2(200.0, 0.0), vec2(50.0, 50.0));
        assert_eq!(rect.subtract(&outside.into()), RectSet::from(rect));
    }

    #[test]
    fn subtracting_from_overlapping_rects_counts_the_overlap_once() {
        let set = RectSet(vec![
            Rect::from_min_size(pos2(0.0, 0.0), vec2(60.0, 60.0)),
            Rect::from_min_size(pos2(40.0, 40.0), vec2(60.0, 60.0)),
        ]);
        let covered = 2.0 * 60.0 * 60.0 - 20.0 * 20.0;

        let all = set.subtract(&RectSet::new());
        assert_disjoint(&all);
        assert_eq!(area(&all), covered, "{all:?}");

        // The cut takes the overlap and nothing else.
        let overlap = Rect::from_min_max(pos2(40.0, 40.0), pos2(60.0, 60.0));
        let parts = set.subtract(&overlap.into());
        assert_disjoint(&parts);
        assert_eq!(area(&parts), covered - overlap.area(), "{parts:?}");
        assert!(!parts.contains(overlap.center()), "{parts:?}");
    }

    #[test]
    fn subtracting_joins_rects_that_touch() {
        let top = Rect::from_min_max(pos2(0.0, 0.0), pos2(50.0, 20.0));
        let bottom = Rect::from_min_max(pos2(0.0, 20.0), pos2(50.0, 40.0));
        let left = Rect::from_min_max(pos2(-30.0, 0.0), pos2(0.0, 40.0));
        let set = RectSet(vec![top, bottom, left]);
        let joined = set.subtract(&RectSet::new());
        assert_eq!(joined, RectSet::from(set.bounding_rect()));
    }

    #[test]
    fn a_set_contains_the_points_of_its_rects() {
        let set = RectSet(vec![
            Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)),
            Rect::from_min_size(pos2(20.0, 0.0), vec2(10.0, 10.0)),
        ]);
        assert!(set.contains(pos2(5.0, 5.0)));
        assert!(set.contains(pos2(25.0, 5.0)));
        assert!(!set.contains(pos2(15.0, 5.0)), "between the two rects");
        assert_eq!(
            set.bounding_rect(),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(30.0, 10.0))
        );

        assert!(!set.is_empty());
        assert!(RectSet::new().is_empty());
        assert!(RectSet::from(Rect::NOTHING).is_empty());
        assert_eq!(RectSet::from(Rect::NOTHING).bounding_rect(), Rect::NOTHING);
    }

    #[test]
    fn a_rect_without_area_adds_nothing_to_a_set() {
        let line = Rect::from_min_max(pos2(0.0, 0.0), pos2(10.0, 0.0));
        let set = RectSet::from(line);
        assert!(set.is_empty());
        assert!(
            !set.contains(pos2(5.0, 0.0)),
            "an empty set contains nothing"
        );
        assert_eq!(set.bounding_rect(), Rect::NOTHING);
        assert_eq!(set.subtract(&RectSet::new()), RectSet::new());

        let rect = Rect::from_min_size(pos2(0.0, 10.0), vec2(10.0, 10.0));
        let set = RectSet(vec![line, Rect::NOTHING, rect]);
        assert_eq!(set.bounding_rect(), rect);
        assert_eq!(set.subtract(&RectSet::new()), RectSet::from(rect));
        // A cut without area removes nothing.
        assert_eq!(
            rect.subtract(&line.translate(vec2(0.0, 15.0)).into()),
            set.subtract(&RectSet::new())
        );
    }
}
