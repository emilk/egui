/// Size hint for table column/strip cell.
#[derive(Clone, Debug, Copy)]
pub enum Size {
    /// Absolute size in points, with a given range of allowed sizes to resize within.
    Absolute { initial: f32, range: (f32, f32) },

    /// Relative size relative to all available space.
    Relative { fraction: f32, range: (f32, f32) },

    /// Multiple remainders each get the same space.
    Remainder { range: (f32, f32) },
}

impl Size {
    /// Exactly this big, with no room for resize.
    pub fn exact(points: f32) -> Self {
        Self::Absolute {
            initial: points,
            range: (points, points),
        }
    }

    /// Initially this big, but can resize.
    pub fn initial(points: f32) -> Self {
        Self::Absolute {
            initial: points,
            range: (0.0, f32::INFINITY),
        }
    }

    /// Relative size relative to all available space. Values must be in range `0.0..=1.0`.
    pub fn relative(fraction: f32) -> Self {
        egui::egui_assert!(0.0 <= fraction && fraction <= 1.0);
        Self::Relative {
            fraction,
            range: (0.0, f32::INFINITY),
        }
    }

    /// Multiple remainders each get the same space.
    pub fn remainder() -> Self {
        Self::Remainder {
            range: (0.0, f32::INFINITY),
        }
    }

    /// Won't shrink below this size (in points).
    pub fn at_least(mut self, minimum: f32) -> Self {
        match &mut self {
            Self::Absolute { range, .. }
            | Self::Relative { range, .. }
            | Self::Remainder { range, .. } => {
                range.0 = minimum;
            }
        }
        self
    }

    /// Won't grow above this size (in points).
    pub fn at_most(mut self, maximum: f32) -> Self {
        match &mut self {
            Self::Absolute { range, .. }
            | Self::Relative { range, .. }
            | Self::Remainder { range, .. } => {
                range.1 = maximum;
            }
        }
        self
    }

    /// Allowed range of movement (in points), if in a resizable [`Table`](crate::table::Table).
    pub fn range(self) -> (f32, f32) {
        match self {
            Self::Absolute { range, .. }
            | Self::Relative { range, .. }
            | Self::Remainder { range, .. } => range,
        }
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

        let mut remainders = 0;
        let sum_non_remainder = self
            .sizes
            .iter()
            .map(|&size| match size {
                Size::Absolute { initial, .. } => initial,
                Size::Relative {
                    fraction,
                    range: (min, max),
                } => {
                    assert!(0.0 <= fraction && fraction <= 1.0);
                    (length * fraction).clamp(min, max)
                }
                Size::Remainder { .. } => {
                    remainders += 1;
                    0.0
                }
            })
            .sum::<f32>()
            + spacing * (self.sizes.len() - 1) as f32;

        let avg_remainder_length = if remainders == 0 {
            0.0
        } else {
            let mut remainder_length = length - sum_non_remainder;
            let avg_remainder_length = 0.0f32.max(remainder_length / remainders as f32).floor();
            self.sizes.iter().for_each(|&size| {
                if let Size::Remainder { range: (min, _max) } = size {
                    if avg_remainder_length < min {
                        remainder_length -= min;
                        remainders -= 1;
                    }
                }
            });
            if remainders > 0 {
                0.0f32.max(remainder_length / remainders as f32)
            } else {
                0.0
            }
        };

        self.sizes
            .iter()
            .map(|&size| match size {
                Size::Absolute { initial, .. } => initial,
                Size::Relative {
                    fraction,
                    range: (min, max),
                } => (length * fraction).clamp(min, max),
                Size::Remainder { range: (min, max) } => avg_remainder_length.clamp(min, max),
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
    assert_eq!(sizing.to_lengths(50.0, 0.0), vec![]);

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
