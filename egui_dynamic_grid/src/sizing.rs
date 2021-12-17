#[derive(Clone, Debug)]
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

pub struct Sizing {
    length: f32,
    inner_padding: f32,
    sizes: Vec<Size>,
}

impl Sizing {
    pub fn new(length: f32, inner_padding: f32) -> Self {
        Self {
            length,
            inner_padding,
            sizes: vec![],
        }
    }

    pub fn add_size(&mut self, size: Size) {
        self.sizes.push(size);
    }

    pub fn into_lengths(self) -> Vec<f32> {
        let mut remainders = 0;
        let length = self.length;
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
            + self.inner_padding * (self.sizes.len() + 1) as f32;

        let avg_remainder_length = if remainders == 0 {
            0.0
        } else {
            let mut remainder_length = length - sum_non_remainder;
            let avg_remainder_length = 0.0f32.max(remainder_length / remainders as f32).floor();
            self.sizes.iter().for_each(|size| {
                if let Size::RemainderMinimum(minimum) = size {
                    if *minimum > avg_remainder_length {
                        remainder_length -= minimum - avg_remainder_length;
                    }
                }
            });
            0.0f32.max(remainder_length / remainders as f32)
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
