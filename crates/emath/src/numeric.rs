/// Implemented for all builtin numeric types
pub trait Numeric: Clone + Copy + PartialEq + PartialOrd + 'static {
    /// Is this an integer type?
    const INTEGRAL: bool;

    /// Smallest finite value
    const MIN: Self;

    /// Largest finite value
    const MAX: Self;

    fn to_f64(self) -> f64;

    fn from_f64(num: f64) -> Self;
}

macro_rules! impl_numeric_float {
    ($t: ident) => {
        impl Numeric for $t {
            const INTEGRAL: bool = false;
            const MIN: Self = $t::MIN;
            const MAX: Self = $t::MAX;

            #[inline(always)]
            fn to_f64(self) -> f64 {
                #[allow(clippy::allow_attributes, trivial_numeric_casts)]
                {
                    self as f64
                }
            }

            #[inline(always)]
            fn from_f64(num: f64) -> Self {
                #[allow(clippy::allow_attributes, trivial_numeric_casts)]
                {
                    num as Self
                }
            }
        }
    };
}

macro_rules! impl_numeric_integer {
    ($t: ident) => {
        impl Numeric for $t {
            const INTEGRAL: bool = true;
            const MIN: Self = $t::MIN;
            const MAX: Self = $t::MAX;

            #[inline(always)]
            fn to_f64(self) -> f64 {
                self as f64
            }

            #[inline(always)]
            fn from_f64(num: f64) -> Self {
                num as Self
            }
        }
    };
}

macro_rules! impl_numeric_non_zero_unsigned {
    ($t: path) => {
        impl Numeric for $t {
            const INTEGRAL: bool = true;
            const MIN: Self = Self::MIN;
            const MAX: Self = Self::MAX;

            #[inline(always)]
            fn to_f64(self) -> f64 {
                self.get() as f64
            }

            #[inline(always)]
            fn from_f64(num: f64) -> Self {
                Self::new(num.round().max(1.0) as _).unwrap_or(Self::MIN)
            }
        }
    };
}

impl_numeric_float!(f32);
impl_numeric_float!(f64);
impl_numeric_integer!(i8);
impl_numeric_integer!(u8);
impl_numeric_integer!(i16);
impl_numeric_integer!(u16);
impl_numeric_integer!(i32);
impl_numeric_integer!(u32);
impl_numeric_integer!(i64);
impl_numeric_integer!(u64);
impl_numeric_integer!(isize);
impl_numeric_integer!(usize);
impl_numeric_non_zero_unsigned!(std::num::NonZeroU8);
impl_numeric_non_zero_unsigned!(std::num::NonZeroU16);
impl_numeric_non_zero_unsigned!(std::num::NonZeroU32);
impl_numeric_non_zero_unsigned!(std::num::NonZeroU64);
impl_numeric_non_zero_unsigned!(std::num::NonZeroU128);
impl_numeric_non_zero_unsigned!(std::num::NonZeroUsize);
