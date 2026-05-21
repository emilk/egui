use std::num::NonZeroU64;

/// Types that can be converted to an [`IdSalt`].
///
/// This is all types implementing `Hash` and `Debug`,
/// which includes things like string, integers, tuples of those, etc.
pub trait AsIdSalt: std::hash::Hash + std::fmt::Debug {}

impl<T: std::hash::Hash + std::fmt::Debug> AsIdSalt for T {}

/// Uniquely identifies a child widget within a parent widget.
///
/// An [`IdSalt`] is only unique within a parent [`crate::Id`].
/// An [`IdSalt`] is NOT globally unique.
///
/// You combine a parent [`crate::Id`] with an [`IdSalt`] to get a child [`crate::Id`],
/// using [`crate::Id::with`].
///
/// An [`IdSalt`] is usually a string, an integer, or similar.
///
/// An [`IdSalt`] should NOT be produced from an [`crate::Id`].
///
/// This is niche-optimized to that `Option<IdSalt>` is the same size as `IdSalt`.
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct IdSalt(NonZeroU64);

impl nohash_hasher::IsEnabled for IdSalt {}

impl IdSalt {
    /// Create a new [`IdSalt`] by hashing some source (e.g. a string or integer).
    pub fn new(source: impl AsIdSalt) -> Self {
        Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(source))
    }

    /// Create a new root [`IdSalt`] from a high-entropy hash.
    #[inline]
    const fn from_hash(hash: u64) -> Self {
        if let Some(nonzero) = NonZeroU64::new(hash) {
            Self(nonzero)
        } else {
            Self(NonZeroU64::MIN) // The hash was exactly zero
        }
    }

    /// The inner value of the [`IdSalt`].
    ///
    /// This is a high-entropy hash.
    #[inline(always)]
    pub fn value(&self) -> u64 {
        self.0.get()
    }
}

impl std::fmt::Debug for IdSalt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "salt_{:04X}", self.value() as u16)
    }
}
