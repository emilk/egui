// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use core::num::NonZeroU64;

use crate::{AsIdSalt, IdSalt};

/// Types that can be converted to an [`Id`].
///
/// This is all types implementing `Hash` and `Debug`,
/// which includes things like string, integers, tuples of those, etc.
pub trait AsId: core::hash::Hash + core::fmt::Debug {}

impl<T: core::hash::Hash + core::fmt::Debug> AsId for T {}

/// A (hopefully) unique identity within this application.
///
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
/// This implies that the [`Id`]s must be "globally" unique (unique within the running app).
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
#[cfg_attr(
    feature = "serde",
    expect(
        clippy::unsafe_derive_deserialize,
        reason = "`from_high_entropy_bits` is only `unsafe` about entropy, not memory safety"
    )
)]
pub struct Id(NonZeroU64);

impl nohash_hasher::IsEnabled for Id {}

impl Id {
    /// A special [`Id`], in particular as a key to [`crate::Memory::data`]
    /// for when there is no particular widget to attach the data.
    ///
    /// The null [`Id`] is still a valid id to use in all circumstances,
    /// though obviously it will lead to a lot of collisions if you do use it!
    pub const NULL: Self = Self(NonZeroU64::MAX);

    /// Create a new, globally unique, root [`Id`] from a high-entropy hash.
    #[inline]
    const fn from_hash(hash: u64) -> Self {
        if let Some(nonzero) = NonZeroU64::new(hash) {
            Self(nonzero)
        } else {
            Self(NonZeroU64::MIN) // The hash was exactly zero (very bad luck)
        }
    }

    /// Creates a new root [`Id`] from a globally unique source (e.g. a string or integer) by hashing it.
    ///
    /// The source must be unique within the whole application,
    /// or else you risk [`Id`] clashes with other widgets.
    ///
    /// If you only need something unique within a parent widget, use [`IdSalt`] instead.
    pub fn unique(source: impl AsId) -> Self {
        let id = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        #[cfg(debug_assertions)]
        id_source::insert_root(id, &source);

        id
    }

    /// Generate a new, globally unique, root [`Id`] by hashing some source (e.g. a string or integer).
    #[deprecated = "Use `Id::unique` (for a globally unique id) or `IdSalt::new` (for a locally unique salt) instead"]
    pub fn new(source: impl AsId) -> Self {
        Self::unique(source)
    }

    /// Generate a child [`Id`] by salting the parent [`Id`] with the given argument.
    ///
    /// `id.with(salt)` is the same as `id.with_salt(IdSalt::new(salt))`.
    pub fn with(self, salt: impl AsIdSalt) -> Self {
        let id = self.hash_with_salt(IdSalt::new(&salt));

        #[cfg(debug_assertions)]
        id_source::insert_child(id, self, &salt);

        id
    }

    /// Generate a child [`Id`] by salting the parent [`Id`] with the given [`IdSalt`].
    ///
    /// `id.with_salt(IdSalt::new(salt))` is the same as `id.with(salt)`.
    pub fn with_salt(self, salt: IdSalt) -> Self {
        let id = self.hash_with_salt(salt);

        #[cfg(debug_assertions)]
        id_source::insert_child(id, self, &salt);

        id
    }

    fn hash_with_salt(self, salt: IdSalt) -> Self {
        use core::hash::{BuildHasher as _, Hasher as _};
        let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.value());
        hasher.write_u64(salt.value());
        Self::from_hash(hasher.finish())
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

impl core::fmt::Debug for Id {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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

/// `IdSet` is a `HashSet<Id>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet = nohash_hasher::IntSet<Id>;

/// `IdMap<V>` is a `HashMap<Id, V>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<V> = nohash_hasher::IntMap<Id, V>;

// ----------------------------------------------------------------------------

/// In debug builds, remember the `Debug`-formatted call chain that produced each [`Id`].
///
/// Used by [`Id`]'s `Debug` impl so that `Id::unique("foo")` prints as `Id::unique("foo")`,
/// and `Id::unique("foo").with("bar")` prints as `Id::unique("foo").with("bar")`, etc.
#[cfg(debug_assertions)]
mod id_source {
    use super::{AsId, AsIdSalt, Id, IdMap};
    use epaint::mutex::RwLock;
    use std::sync::LazyLock;

    /// Max length of one salt, in bytes.
    const MAX_SALT_LEN: usize = 64;

    /// Max length of a whole chain, in bytes.
    const MAX_CHAIN_LEN: usize = 512;

    const ELLIPSIS: &str = "\u{2026}";

    /// One [`Id::unique`] or [`Id::with`] call.
    struct Link {
        /// The `Debug` of the source or salt.
        salt: String,

        /// `None` if this came from [`Id::unique`].
        parent: Option<Id>,
    }

    /// Only the last link is stored per [`Id`], so memory is linear in the number of ids.
    static SOURCE_MAP: LazyLock<RwLock<IdMap<Link>>> = LazyLock::new(RwLock::default);

    /// Keep the start of `s`, so the result is at most `max_len` bytes.
    ///
    /// A salt can itself be a whole chain, and cutting the middle out of one of those
    /// would leave unbalanced parentheses, so keep the readable head instead.
    fn truncate_head(mut s: String, max_len: usize) -> String {
        if s.len() <= max_len {
            return s;
        }
        s.truncate(s.floor_char_boundary(max_len - ELLIPSIS.len()));
        s.push_str(ELLIPSIS);
        s
    }

    pub(super) fn insert_root(id: Id, source: &impl AsId) {
        if SOURCE_MAP.read().contains_key(&id) {
            return;
        }
        // Format outside the lock since `{source:?}` may itself recurse into [`Id`]'s `Debug` impl.
        let salt = truncate_head(format!("{source:?}"), MAX_SALT_LEN);
        SOURCE_MAP.write().insert(id, Link { salt, parent: None });
    }

    pub(super) fn insert_child(id: Id, parent: Id, salt: &impl AsIdSalt) {
        if SOURCE_MAP.read().contains_key(&id) {
            return;
        }
        // Format outside the lock since `{salt:?}` may itself recurse into [`Id`]'s `Debug` impl.
        let salt = truncate_head(format!("{salt:?}"), MAX_SALT_LEN);
        SOURCE_MAP.write().insert(
            id,
            Link {
                salt,
                parent: Some(parent),
            },
        );
    }

    pub(super) fn get(id: Id) -> Option<String> {
        // Walk towards the root, keeping the salts closest to `id` (the most specific ones)
        // until we run out of budget. This never builds a string longer than the budget.
        // Format after dropping the lock, since `{unknown:?}` recurses into `Debug`.
        let mut salts = Vec::new(); // Closest to `id` first.
        let mut len = 0;
        let mut skipped = false;
        let mut root = None;
        let mut unknown = None;
        {
            let map = SOURCE_MAP.read();
            let mut current = id;
            loop {
                let Some(link) = map.get(&current) else {
                    unknown = Some(current);
                    break;
                };
                let Some(parent) = link.parent else {
                    root = Some(link.salt.clone());
                    break;
                };
                if !skipped && len + link.salt.len() <= MAX_CHAIN_LEN {
                    len += link.salt.len() + ".with()".len();
                    salts.push(link.salt.clone());
                } else {
                    skipped = true; // Keep going to find the root, but keep no more salts.
                }
                current = parent;
            }
        }

        if root.is_none() && salts.is_empty() {
            return None; // We know nothing about this `Id`.
        }

        let mut chain = match root {
            Some(root_salt) => format!("Id::unique({root_salt})"),
            None => format!("{:?}", unknown?),
        };
        if skipped {
            chain.push_str(ELLIPSIS);
        }
        for salt in salts.iter().rev() {
            chain.push_str(".with(");
            chain.push_str(salt);
            chain.push(')');
        }
        Some(chain)
    }
}

#[test]
fn id_size() {
    assert_eq!(core::mem::size_of::<Id>(), 8);
    assert_eq!(core::mem::size_of::<Option<Id>>(), 8);
}

#[cfg(test)]
#[cfg(debug_assertions)]
mod debug_format_tests {
    use crate::IdSalt;

    use super::Id;

    #[test]
    fn root_string() {
        let id = Id::unique("foo");
        assert_eq!(format!("{id:?}"), r#"Id::unique("foo")"#);
    }

    #[test]
    fn root_integer() {
        let id = Id::unique(42_i32);
        assert_eq!(format!("{id:?}"), "Id::unique(42)");
    }

    #[test]
    fn root_id_salt() {
        let id = Id::unique(IdSalt::new("foo"));
        assert_eq!(format!("{id:?}"), r#"Id::unique(IdSalt::new("foo"))"#);
    }

    #[test]
    fn with_salt_matches_with() {
        let parent = Id::unique("parent");
        assert_eq!(parent.with_salt(IdSalt::new("child")), parent.with("child"));
    }

    #[test]
    fn with_one_child() {
        let id = Id::unique("parent").with("child");
        assert_eq!(format!("{id:?}"), r#"Id::unique("parent").with("child")"#);
    }

    #[test]
    fn with_chain() {
        let id = Id::unique("a").with("b").with("c").with(7_i32);
        assert_eq!(
            format!("{id:?}"),
            r#"Id::unique("a").with("b").with("c").with(7)"#
        );
    }

    #[test]
    fn nested_id_as_source() {
        let inner = Id::unique("foo");
        let outer = Id::unique(inner);
        assert_eq!(format!("{outer:?}"), r#"Id::unique(Id::unique("foo"))"#);
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
