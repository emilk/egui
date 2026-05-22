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
        let id_salt = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        #[cfg(debug_assertions)]
        id_salt_source::maybe_insert(id_salt, &source);

        id_salt
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
        #[cfg(debug_assertions)]
        if let Some(source) = id_salt_source::get(*self) {
            return write!(f, "IdSalt::new({source})");
        }
        write!(f, "salt_{:04X}", self.value() as u16)
    }
}

/// In debug builds, remember the `Debug`-formatted source that produced each [`IdSalt`].
///
/// Used by [`IdSalt`]'s `Debug` impl so that `IdSalt::new("foo")` prints as
/// `IdSalt::new("foo")`, and `IdSalt::new(IdSalt::new("foo"))` prints as
/// `IdSalt::new(IdSalt::new("foo"))`, etc.
#[cfg(debug_assertions)]
mod id_salt_source {
    use super::{AsIdSalt, IdSalt};
    use epaint::mutex::RwLock;
    use nohash_hasher::IntMap;
    use std::sync::LazyLock;

    static SOURCE_MAP: LazyLock<RwLock<IntMap<IdSalt, String>>> = LazyLock::new(RwLock::default);

    pub(super) fn maybe_insert(id_salt: IdSalt, source: &impl AsIdSalt) {
        if !SOURCE_MAP.read().contains_key(&id_salt) {
            let formatted = format!("{source:?}");
            SOURCE_MAP.write().insert(id_salt, formatted);
        }
    }

    pub(super) fn get(id_salt: IdSalt) -> Option<String> {
        SOURCE_MAP.read().get(&id_salt).cloned()
    }
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod tests {
    use super::IdSalt;

    #[test]
    fn debug_format_string_source() {
        let salt = IdSalt::new("foo");
        assert_eq!(format!("{salt:?}"), r#"IdSalt::new("foo")"#);
    }

    #[test]
    fn debug_format_integer_source() {
        let salt = IdSalt::new(42_i32);
        assert_eq!(format!("{salt:?}"), "IdSalt::new(42)");
    }

    #[test]
    fn debug_format_nested_salt() {
        let inner = IdSalt::new("foo");
        let outer = IdSalt::new(inner);
        assert_eq!(format!("{outer:?}"), r#"IdSalt::new(IdSalt::new("foo"))"#);
    }

    #[test]
    fn debug_format_triple_nested_salt() {
        let a = IdSalt::new("foo");
        let b = IdSalt::new(a);
        let c = IdSalt::new(b);
        assert_eq!(
            format!("{c:?}"),
            r#"IdSalt::new(IdSalt::new(IdSalt::new("foo")))"#
        );
    }

    #[test]
    fn debug_format_tuple_source() {
        let salt = IdSalt::new(("foo", 7_i32));
        assert_eq!(format!("{salt:?}"), r#"IdSalt::new(("foo", 7))"#);
    }
}
