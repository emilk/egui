use egui::Rangef;

use core::iter::zip;

/// Size hint for table column/strip cell.
#[derive(Clone, Debug, Copy)]
pub enum Size {
    /// Absolute size in points, with a given range of allowed sizes to resize within.
    Absolute { initial: f32, range: Rangef },

    /// Relative size relative to all available space.
    Relative { fraction: f32, range: Rangef },

    /// Multiple remainders each get the same space.
    Remainder { range: Rangef },
}

impl Size {
    /// Exactly this big, with no room for resize.
    pub fn exact(points: f32) -> Self {
        Self::Absolute {
            initial: points,
            range: Rangef::new(points, points),
        }
    }

    /// Initially this big, but can resize.
    pub fn initial(points: f32) -> Self {
        Self::Absolute {
            initial: points,
            range: Rangef::new(0.0, f32::INFINITY),
        }
    }

    /// Relative size relative to all available space. Values must be in range `0.0..=1.0`.
    pub fn relative(fraction: f32) -> Self {
        debug_assert!(
            0.0 <= fraction && fraction <= 1.0,
            "fraction should be in the range [0, 1], but was {fraction}"
        );
        Self::Relative {
            fraction,
            range: Rangef::new(0.0, f32::INFINITY),
        }
    }

    /// Multiple remainders each get the same space.
    pub fn remainder() -> Self {
        Self::Remainder {
            range: Rangef::new(0.0, f32::INFINITY),
        }
    }

    /// Won't shrink below this size (in points).
    #[inline]
    pub fn at_least(mut self, minimum: f32) -> Self {
        self.range_mut().min = minimum;
        self
    }

    /// Won't grow above this size (in points).
    #[inline]
    pub fn at_most(mut self, maximum: f32) -> Self {
        self.range_mut().max = maximum;
        self
    }

    #[inline]
    pub fn with_range(mut self, range: Rangef) -> Self {
        *self.range_mut() = range;
        self
    }

    /// Allowed range of movement (in points), if in a resizable [`Table`](crate::table::Table).
    pub fn range(self) -> Rangef {
        match self {
            Self::Absolute { range, .. }
            | Self::Relative { range, .. }
            | Self::Remainder { range, .. } => range,
        }
    }

    pub fn range_mut(&mut self) -> &mut Rangef {
        match self {
            Self::Absolute { range, .. }
            | Self::Relative { range, .. }
            | Self::Remainder { range, .. } => range,
        }
    }

    #[inline]
    pub fn is_absolute(&self) -> bool {
        matches!(self, Self::Absolute { .. })
    }

    #[inline]
    pub fn is_relative(&self) -> bool {
        matches!(self, Self::Relative { .. })
    }

    #[inline]
    pub fn is_remainder(&self) -> bool {
        matches!(self, Self::Remainder { .. })
    }
}

#[derive(Clone, Default)]
pub struct Sizing {
    pub(crate) sizes: Vec<Size>,
}

impl Sizing {
    pub fn add(&mut self, size: Size) {
        self.sizes.push(size);
    }

    pub fn to_lengths(&self, length: f32, spacing: f32) -> Vec<f32> {
        if self.sizes.is_empty() {
            return vec![];
        }

        let mut num_remainders = 0;
        let sum_non_remainder = self
            .sizes
            .iter()
            .map(|&size| match size {
                Size::Absolute { initial, .. } => initial,
                Size::Relative { fraction, range } => {
                    assert!(
                        0.0 <= fraction && fraction <= 1.0,
                        "fraction should be in the range [0, 1], but was {fraction}"
                    );
                    range.clamp(length * fraction)
                }
                Size::Remainder { .. } => {
                    num_remainders += 1;
                    0.0
                }
            })
            .sum::<f32>()
            + spacing * (self.sizes.len() - 1) as f32;

        let avg_remainder_length = if num_remainders == 0 {
            0.0
        } else {
            let mut remainder_length = length - sum_non_remainder;
            let avg_remainder_length = 0.0f32.max(remainder_length / num_remainders as f32).floor();
            for &size in &self.sizes {
                if let Size::Remainder { range } = size
                    && avg_remainder_length < range.min
                {
                    remainder_length -= range.min;
                    num_remainders -= 1;
                }
            }
            if num_remainders > 0 {
                0.0f32.max(remainder_length / num_remainders as f32)
            } else {
                0.0
            }
        };

        self.sizes
            .iter()
            .map(|&size| match size {
                Size::Absolute { initial, .. } => initial,
                Size::Relative { fraction, range } => range.clamp(length * fraction),
                Size::Remainder { range } => range.clamp(avg_remainder_length),
            })
            .collect()
    }
}

impl From<Vec<Size>> for Sizing {
    fn from(sizes: Vec<Size>) -> Self {
        Self { sizes }
    }
}

#[test]
fn test_sizing() {
    let sizing: Sizing = vec![].into();
    assert_eq!(sizing.to_lengths(50.0, 0.0), Vec::<f32>::new());

    let sizing: Sizing = vec![Size::remainder().at_least(20.0), Size::remainder()].into();
    assert_eq!(sizing.to_lengths(50.0, 0.0), vec![25.0, 25.0]);
    assert_eq!(sizing.to_lengths(30.0, 0.0), vec![20.0, 10.0]);
    assert_eq!(sizing.to_lengths(20.0, 0.0), vec![20.0, 0.0]);
    assert_eq!(sizing.to_lengths(10.0, 0.0), vec![20.0, 0.0]);
    assert_eq!(sizing.to_lengths(20.0, 10.0), vec![20.0, 0.0]);
    assert_eq!(sizing.to_lengths(30.0, 10.0), vec![20.0, 0.0]);
    assert_eq!(sizing.to_lengths(40.0, 10.0), vec![20.0, 10.0]);
    assert_eq!(sizing.to_lengths(110.0, 10.0), vec![50.0, 50.0]);

    let sizing: Sizing = vec![Size::relative(0.5).at_least(10.0), Size::exact(10.0)].into();
    assert_eq!(sizing.to_lengths(50.0, 0.0), vec![25.0, 10.0]);
    assert_eq!(sizing.to_lengths(30.0, 0.0), vec![15.0, 10.0]);
    assert_eq!(sizing.to_lengths(20.0, 0.0), vec![10.0, 10.0]);
    assert_eq!(sizing.to_lengths(10.0, 0.0), vec![10.0, 10.0]);
}

/// Shrink the given natural (unwrapped) widths so that they fit in `available`.
///
/// If everything fits, each column keeps its natural width.
/// Otherwise the wide columns shrink, sharing the available width in proportion to
/// their natural widths, while the narrow columns keep their natural width.
/// No column is shrunk below its minimum, so the result can be wider than `available`.
///
/// `natural` and `minimums` must be the same length.
pub(crate) fn shrink_to_fit(natural: &[f32], minimums: &[f32], available: f32) -> Vec<f32> {
    debug_assert_eq!(
        natural.len(),
        minimums.len(),
        "There must be one minimum per column"
    );

    let available = available.max(0.0);
    let natural: Vec<f32> = natural.iter().map(|w| w.max(0.0)).collect();
    let mut widths = natural.clone();
    let mut is_fixed = vec![false; widths.len()];

    loop {
        let fixed_sum: f32 = zip(&widths, &is_fixed)
            .filter(|(_, fixed)| **fixed)
            .map(|(width, _)| width)
            .sum();
        let remaining = (available - fixed_sum).max(0.0);

        let flexible: Vec<usize> = (0..widths.len()).filter(|&i| !is_fixed[i]).collect();
        let flexible_natural: f32 = flexible.iter().map(|&i| natural[i]).sum();

        if flexible_natural <= remaining || flexible_natural <= 0.0 {
            for i in flexible {
                widths[i] = natural[i];
            }
            return widths;
        }

        // Columns narrower than their fair share keep their natural width:
        let fair = remaining / flexible.len() as f32;
        let mut changed = false;
        for &i in &flexible {
            if natural[i] <= fair {
                widths[i] = natural[i];
                is_fixed[i] = true;
                changed = true;
            }
        }
        if changed {
            continue;
        }

        // The rest share the remaining width in proportion to their natural width:
        for i in flexible {
            let target = remaining * natural[i] / flexible_natural;
            let minimum = minimums[i].max(0.0);
            if target < minimum {
                widths[i] = minimum.min(natural[i]);
                is_fixed[i] = true;
                changed = true;
            } else {
                widths[i] = target;
            }
        }

        if !changed {
            return widths;
        }
    }
}

#[cfg(test)]
mod tests_shrink_to_fit {
    use super::shrink_to_fit;

    fn shrink(natural: &[f32], available: f32) -> Vec<f32> {
        shrink_to_fit(natural, &vec![0.0; natural.len()], available)
    }

    #[test]
    fn empty() {
        assert_eq!(shrink(&[], 100.0), Vec::<f32>::new());
    }

    #[test]
    fn everything_fits() {
        assert_eq!(shrink(&[10.0, 20.0], 100.0), vec![10.0, 20.0]);
        // Exactly fitting is still fitting:
        assert_eq!(shrink(&[10.0, 20.0], 30.0), vec![10.0, 20.0]);
    }

    #[test]
    fn narrow_columns_keep_their_width() {
        // 10 is below the fair share of 50, so only the long column shrinks:
        assert_eq!(shrink(&[10.0, 200.0], 100.0), vec![10.0, 90.0]);
    }

    #[test]
    fn wide_columns_shrink_in_proportion() {
        assert_eq!(shrink(&[100.0, 300.0], 100.0), vec![25.0, 75.0]);
    }

    #[test]
    fn nothing_is_shrunk_below_its_minimum() {
        // The first column would get 25 of 100, but may not go below 40,
        // so it is pinned there and the other column takes what is left:
        assert_eq!(
            shrink_to_fit(&[100.0, 300.0], &[40.0, 0.0], 100.0),
            vec![40.0, 60.0]
        );

        // A minimum wider than the natural width does not widen a column:
        assert_eq!(
            shrink_to_fit(&[10.0, 300.0], &[50.0, 0.0], 20.0),
            vec![10.0, 10.0]
        );
    }

    #[test]
    fn the_result_may_be_wider_than_available() {
        // Both minimums together already exceed the 10 px available:
        assert_eq!(
            shrink_to_fit(&[100.0, 100.0], &[30.0, 30.0], 10.0),
            vec![30.0, 30.0]
        );
    }

    #[test]
    fn zero_and_negative_available_width() {
        assert_eq!(shrink(&[100.0, 100.0], 0.0), vec![0.0, 0.0]);
        assert_eq!(shrink(&[100.0, 100.0], -50.0), vec![0.0, 0.0]);
    }

    #[test]
    fn zero_width_columns() {
        assert_eq!(shrink(&[0.0, 0.0], 100.0), vec![0.0, 0.0]);
        assert_eq!(shrink(&[0.0, 200.0], 100.0), vec![0.0, 100.0]);
    }
}
