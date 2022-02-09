/// Size hint for table column/grid cell
#[derive(Clone, Debug, Copy)]
pub enum Size {
    /// Absolute size in points
    Absolute(f32),
    /// Relative size relative to all available space. Values must be in range `0.0..=1.0`
    Relative(f32),
    /// [`Size::Relative`] with a minimum size in points
    RelativeMinimum {
        /// Relative size relative to all available space. Values must be in range `0.0..=1.0`
        relative: f32,
        /// Absolute minimum size in points
        minimum: f32,
    },
    /// Multiple remainders each get the same space
    Remainder,
    ///  [`Size::Remainder`] with a minimum size in points
    RemainderMinimum(f32),
}

#[derive(Clone)]
pub struct Sizing {
    sizes: Vec<Size>,
}

impl Sizing {
    pub fn new() -> Self {
        Self { sizes: vec![] }
    }

    pub fn add(&mut self, size: Size) {
        self.sizes.push(size);
    }

    pub fn into_lengths(self, length: f32, spacing: f32) -> Vec<f32> {
        let mut remainders = 0;
        let sum_non_remainder = self
            .sizes
            .iter()
            .map(|size| match size {
                Size::Absolute(absolute) => *absolute,
                Size::Relative(relative) => {
                    assert!(*relative > 0.0, "Below 0.0 is not allowed.");
                    assert!(*relative <= 1.0, "Above 1.0 is not allowed.");
                    length * relative
                }
                Size::RelativeMinimum { relative, minimum } => {
                    assert!(*relative > 0.0, "Below 0.0 is not allowed.");
                    assert!(*relative <= 1.0, "Above 1.0 is not allowed.");
                    minimum.max(length * relative)
                }
                Size::Remainder | Size::RemainderMinimum(..) => {
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
            self.sizes.iter().for_each(|size| {
                if let Size::RemainderMinimum(minimum) = size {
                    if *minimum > avg_remainder_length {
                        remainder_length -= minimum;
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
            .into_iter()
            .map(|size| match size {
                Size::Absolute(absolute) => absolute,
                Size::Relative(relative) => length * relative,
                Size::RelativeMinimum { relative, minimum } => minimum.max(length * relative),
                Size::Remainder => avg_remainder_length,
                Size::RemainderMinimum(minimum) => minimum.max(avg_remainder_length),
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
    let sizing: Sizing = vec![Size::RemainderMinimum(20.0), Size::Remainder].into();
    assert_eq!(sizing.clone().into_lengths(50.0, 0.0), vec![25.0, 25.0]);
    assert_eq!(sizing.clone().into_lengths(30.0, 0.0), vec![20.0, 10.0]);
    assert_eq!(sizing.clone().into_lengths(20.0, 0.0), vec![20.0, 0.0]);
    assert_eq!(sizing.clone().into_lengths(10.0, 0.0), vec![20.0, 0.0]);
    assert_eq!(sizing.into_lengths(20.0, 10.0), vec![20.0, 0.0]);

    let sizing: Sizing = vec![
        Size::RelativeMinimum {
            relative: 0.5,
            minimum: 10.0,
        },
        Size::Absolute(10.0),
    ]
    .into();
    assert_eq!(sizing.clone().into_lengths(50.0, 0.0), vec![25.0, 10.0]);
    assert_eq!(sizing.clone().into_lengths(30.0, 0.0), vec![15.0, 10.0]);
    assert_eq!(sizing.clone().into_lengths(20.0, 0.0), vec![10.0, 10.0]);
    assert_eq!(sizing.into_lengths(10.0, 0.0), vec![10.0, 10.0]);
}
