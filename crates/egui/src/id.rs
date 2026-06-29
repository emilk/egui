// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use std::num::NonZeroU64;

use crate::{AsIdSalt, IdSalt};

/// Types that can be converted to an [`Id`].
///
/// This is all types implementing `Hash` and `Debug`,
/// which includes things like string, integers, tuples of those, etc.
pub trait AsId: std::hash::Hash + std::fmt::Debug {}

impl<T: std::hash::Hash + std::fmt::Debug> AsId for T {}

/// egui tracks widgets frame-to-frame using [`Id`]s.
///
/// For instance, if you start dragging a slider one frame, egui stores
/// the sliders [`Id`] as the current active id so that next frame when
/// you move the mouse the same slider changes, even if the mouse has
/// moved outside the slider.
///
/// For some widgets [`Id`]s are also used to persist some state about the
/// widgets, such as Window position or whether not a collapsing header region is open.
///
/// This implies that the [`Id`]s must be unique.
///
/// For simple things like sliders and buttons that don't have any memory and
/// doesn't move we can use the location of the widget as a source of identity.
/// For instance, a slider only needs a unique and persistent ID while you are
/// dragging the slider. As long as it is still while moving, that is fine.
///
/// For things that need to persist state even after moving (windows, collapsing headers)
/// the location of the widgets is obviously not good enough. For instance,
/// a collapsing region needs to remember whether or not it is open even
/// if the layout next frame is different and the collapsing is not lower down
/// on the screen.
///
/// Then there are widgets that need no identifiers at all, like labels,
/// because they have no state nor are interacted with.
///
/// This is niche-optimized to that `Option<Id>` is the same size as `Id`.
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Id(NonZeroU64);

impl nohash_hasher::IsEnabled for Id {}

impl Id {
    /// A special [`Id`], in particular as a key to [`crate::Memory::data`]
    /// for when there is no particular widget to attach the data.
    ///
    /// The null [`Id`] is still a valid id to use in all circumstances,
    /// though obviously it will lead to a lot of collisions if you do use it!
    pub const NULL: Self = Self(NonZeroU64::MAX);

    /// Create a new root [`Id`] from a high-entropy hash.
    #[inline]
    const fn from_hash(hash: u64) -> Self {
        if let Some(nonzero) = NonZeroU64::new(hash) {
            Self(nonzero)
        } else {
            Self(NonZeroU64::MIN) // The hash was exactly zero (very bad luck)
        }
    }

    /// Generate a new root [`Id`] by hashing some source (e.g. a string or integer).
    pub fn new(source: impl AsId) -> Self {
        let id = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        #[cfg(debug_assertions)]
        id_source::insert_root(id, &source);

        id
    }

    /// Generate a child [`Id`] by salting the parent [`Id`] with the given argument.
    pub fn with(self, salt: impl AsIdSalt) -> Self {
        use std::hash::{BuildHasher as _, Hasher as _};
        let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.value());
        hasher.write_u64(IdSalt::new(&salt).value());
        let id = Self::from_hash(hasher.finish());

        #[cfg(debug_assertions)]
        id_source::insert_child(id, self, &salt);

        id
    }

    /// Short and readable summary
    pub fn short_debug_format(&self) -> String {
        format!("{:04X}", self.value() as u16)
    }

    /// The inner value of the [`Id`].
    ///
    /// This is a high-entropy hash, or [`Self::NULL`].
    #[inline(always)]
    pub fn value(&self) -> u64 {
        self.0.get()
    }

    pub fn accesskit_id(&self) -> accesskit::NodeId {
        self.value().into()
    }

    /// Create a new [`Id`] from a high-entropy value. No hashing is done.
    ///
    /// This can be useful if you have an [`Id`] that was converted to some other type
    /// (e.g. accesskit::NodeId) and you want to convert it back to an [`Id`].
    ///
    /// # Safety
    /// You need to ensure that the value is high-entropy since it might be used in
    /// a [`IdSet`] or [`IdMap`], which rely on the assumption that [`Id`]s have good entropy.
    ///
    /// The method is not unsafe in terms of memory safety.
    ///
    /// # Panics
    /// If the value is zero, this will panic.
    #[doc(hidden)]
    #[expect(unsafe_code)]
    pub unsafe fn from_high_entropy_bits(value: u64) -> Self {
        Self(NonZeroU64::new(value).expect("Id must be non-zero."))
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if *self == Self::NULL {
            return write!(f, "Id::NULL");
        }
        #[cfg(debug_assertions)]
        if let Some(source) = id_source::get(*self) {
            return f.write_str(&source);
        }
        write!(f, "id_{:04X}", self.value() as u16)
    }
}

// ----------------------------------------------------------------------------

/// Convenience
impl From<&'static str> for Id {
    #[inline]
    fn from(string: &'static str) -> Self {
        Self::new(string)
    }
}

impl From<String> for Id {
    #[inline]
    fn from(string: String) -> Self {
        Self::new(string)
    }
}

// ----------------------------------------------------------------------------

/// `IdSet` is a `HashSet<Id>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet = nohash_hasher::IntSet<Id>;

/// `IdMap<V>` is a `HashMap<Id, V>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<V> = nohash_hasher::IntMap<Id, V>;

// ----------------------------------------------------------------------------

/// In debug builds, remember the `Debug`-formatted call chain that produced each [`Id`].
///
/// Used by [`Id`]'s `Debug` impl so that `Id::new("foo")` prints as `Id::new("foo")`,
/// and `Id::new("foo").with("bar")` prints as `Id::new("foo").with("bar")`, etc.
#[cfg(debug_assertions)]
mod id_source {
    use super::{AsId, AsIdSalt, Id, IdMap};
    use epaint::mutex::RwLock;
    use std::sync::LazyLock;

    static SOURCE_MAP: LazyLock<RwLock<IdMap<String>>> = LazyLock::new(RwLock::default);

    pub(super) fn insert_root(id: Id, source: &impl AsId) {
        if SOURCE_MAP.read().contains_key(&id) {
            return;
        }
        // Format outside the lock since `{source:?}` may itself recurse into [`Id`]'s `Debug` impl.
        let formatted = format!("Id::new({source:?})");
        SOURCE_MAP.write().insert(id, formatted);
    }

    pub(super) fn insert_child(id: Id, parent: Id, salt: &impl AsIdSalt) {
        if SOURCE_MAP.read().contains_key(&id) {
            return;
        }
        // Look up parent's repr and drop the read guard before formatting,
        // since `{parent:?}` and `{salt:?}` may themselves recurse into [`Id`]'s `Debug` impl.
        let cached_parent_repr = SOURCE_MAP.read().get(&parent).cloned();
        let parent_repr = cached_parent_repr.unwrap_or_else(|| format!("{parent:?}"));
        let formatted = format!("{parent_repr}.with({salt:?})");
        SOURCE_MAP.write().insert(id, formatted);
    }

    pub(super) fn get(id: Id) -> Option<String> {
        SOURCE_MAP.read().get(&id).cloned()
    }
}

#[test]
fn id_size() {
    assert_eq!(std::mem::size_of::<Id>(), 8);
    assert_eq!(std::mem::size_of::<Option<Id>>(), 8);
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod debug_format_tests {
    use crate::IdSalt;

    use super::Id;

    #[test]
    fn root_string() {
        let id = Id::new("foo");
        assert_eq!(format!("{id:?}"), r#"Id::new("foo")"#);
    }

    #[test]
    fn root_integer() {
        let id = Id::new(42_i32);
        assert_eq!(format!("{id:?}"), "Id::new(42)");
    }

    #[test]
    fn root_id_salt() {
        let id = Id::new(IdSalt::new("foo"));
        assert_eq!(format!("{id:?}"), r#"Id::new(IdSalt::new("foo"))"#);
    }

    #[test]
    fn with_one_child() {
        let id = Id::new("parent").with("child");
        assert_eq!(format!("{id:?}"), r#"Id::new("parent").with("child")"#);
    }

    #[test]
    fn with_chain() {
        let id = Id::new("a").with("b").with("c").with(7_i32);
        assert_eq!(
            format!("{id:?}"),
            r#"Id::new("a").with("b").with("c").with(7)"#
        );
    }

    #[test]
    fn nested_id_as_source() {
        let inner = Id::new("foo");
        let outer = Id::new(inner);
        assert_eq!(format!("{outer:?}"), r#"Id::new(Id::new("foo"))"#);
    }

    #[test]
    fn null_prints_as_null() {
        assert_eq!(format!("{:?}", Id::NULL), "Id::NULL");
    }

    #[test]
    fn null_as_parent() {
        let id = Id::NULL.with("foo");
        assert_eq!(format!("{id:?}"), r#"Id::NULL.with("foo")"#);
    }
}
