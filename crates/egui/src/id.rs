// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use core::num::NonZeroU64;

use crate::{AsIdSalt, IdSalt};

/// Deprecated
#[deprecated = "Use AsIdSalt or Id instead"]
pub trait AsId: core::hash::Hash + core::fmt::Debug {}

#[expect(deprecated)]
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
    ///
    /// The source is anything that implements [`Hash`](core::hash::Hash),
    /// including strings, integers, and tuples. Prefer a tuple over formatting a string:
    ///
    /// ```
    /// # use egui::Id;
    /// # let (row, column) = (0, 0);
    /// let good = Id::unique(("my_table_cell", row, column)); // No allocation
    /// let bad = Id::unique(format!("my_table_cell {row} {column}")); // Allocates
    /// # let _ = (good, bad);
    /// ```
    pub fn unique(source: impl core::hash::Hash + core::fmt::Debug) -> Self {
        let id = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        #[cfg(debug_assertions)]
        id_source::insert_root(id, &source);

        id
    }

    /// Generate a new, globally unique, root [`Id`] by hashing some source (e.g. a string or integer).
    #[deprecated = "Use `Id::unique` (for a globally unique id) or `IdSalt::new` (for a locally unique salt) instead"]
    pub fn new(source: impl core::hash::Hash + core::fmt::Debug) -> Self {
        Self::unique(source)
    }

    /// Generate a child [`Id`] by salting the parent [`Id`] with the given argument.
    ///
    /// `id.with(salt)` is the same as `id.with_salt(IdSalt::new(salt))`.
    ///
    /// The salt is anything that implements [`Hash`](core::hash::Hash),
    /// including strings, integers, and tuples.
    /// Prefer a single tuple over a chain of [`Self::with`]:
    ///
    /// ```
    /// # use egui::Id;
    /// # let (parent, row, column) = (Id::NULL, 0, 0);
    /// let good = parent.with((row, column)); // Hashes once
    /// let bad = parent.with(row).with(column); // Hashes twice
    /// # let _ = (good, bad);
    /// ```
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
///
/// Entries that have not been seen recently are purged by [`id_source::end_pass`],
/// so that the map does not grow without bound.
#[cfg(debug_assertions)]
pub(crate) mod id_source {
    use super::{AsIdSalt, Id};
    use core::sync::atomic::{AtomicU64, Ordering::Relaxed};
    use epaint::mutex::RwLock;
    use nohash_hasher::IntMap;
    use std::sync::LazyLock;

    /// Stored sources are truncated to at most this many bytes.
    ///
    /// Without a limit, an [`Id`] salted with (something containing) its own ancestor,
    /// e.g. `ui.indent(header_id, …)` in a nested `CollapsingHeader`,
    /// doubles the length of the stored source at every level of nesting.
    const MAX_SOURCE_LEN: usize = 1024;

    /// Only purge stale entries once a map grows larger than this.
    ///
    /// This keeps the names of long-lived ids (e.g. created once at startup)
    /// in small apps, while still bounding the memory use of huge ones.
    const PURGE_THRESHOLD: usize = 10_000;

    /// Incremented at the end of each pass (of any [`crate::Context`]).
    static GENERATION: AtomicU64 = AtomicU64::new(0);

    static SOURCE_MAP: LazyLock<SourceMap<Id>> = LazyLock::new(SourceMap::default);

    struct Entry {
        source: String,

        /// The last [`GENERATION`] in which this entry was created or recreated.
        last_seen: AtomicU64,
    }

    /// Maps an [`Id`] or [`crate::IdSalt`] to the `Debug`-formatted source that produced it.
    pub(crate) struct SourceMap<K>(RwLock<IntMap<K, Entry>>);

    impl<K> Default for SourceMap<K> {
        fn default() -> Self {
            Self(RwLock::default())
        }
    }

    impl<K: Copy + Eq + core::hash::Hash + nohash_hasher::IsEnabled> SourceMap<K> {
        /// Mark `key` as seen, inserting the output of `format_source` if it is new.
        ///
        /// `format_source` is called without holding the lock,
        /// since formatting may itself recurse into the map.
        pub(crate) fn touch_or_insert_with(&self, key: K, format_source: impl FnOnce() -> String) {
            let generation = GENERATION.load(Relaxed);
            if let Some(entry) = self.0.read().get(&key) {
                entry.last_seen.store(generation, Relaxed);
                return;
            }
            let source = truncate(format_source());
            self.0
                .write()
                .entry(key)
                .and_modify(|entry| entry.last_seen.store(generation, Relaxed))
                .or_insert_with(|| Entry {
                    source,
                    last_seen: AtomicU64::new(generation),
                });
        }

        pub(crate) fn get(&self, key: K) -> Option<String> {
            self.0.read().get(&key).map(|entry| entry.source.clone())
        }

        /// Remove entries not seen in this or the previous generation.
        pub(crate) fn purge(&self, generation: u64) {
            let mut map = self.0.write();
            if PURGE_THRESHOLD < map.len() {
                map.retain(|_, entry| generation <= entry.last_seen.load(Relaxed) + 1);
            }
        }
    }

    /// Keep the end, since that is the most specific part.
    fn truncate(mut source: String) -> String {
        if MAX_SOURCE_LEN < source.len() {
            let start = source.ceil_char_boundary(source.len() - MAX_SOURCE_LEN);
            source.replace_range(..start, "…");
        }
        source
    }

    /// Purge stale entries and start a new generation.
    ///
    /// Called at the end of each pass.
    pub(crate) fn end_pass() {
        let generation = GENERATION.fetch_add(1, Relaxed);
        SOURCE_MAP.purge(generation);
        crate::id_salt::id_salt_source::purge(generation);
    }

    pub(super) fn insert_root(id: Id, source: &impl core::fmt::Debug) {
        SOURCE_MAP.touch_or_insert_with(id, || format!("Id::unique({source:?})"));
    }

    pub(super) fn insert_child(id: Id, parent: Id, salt: &impl AsIdSalt) {
        SOURCE_MAP.touch_or_insert_with(id, || {
            let parent_repr = SOURCE_MAP
                .get(parent)
                .unwrap_or_else(|| format!("{parent:?}"));
            format!("{parent_repr}.with({salt:?})")
        });
    }

    pub(super) fn get(id: Id) -> Option<String> {
        SOURCE_MAP.get(id)
    }

    #[cfg(test)]
    mod tests {
        use super::{GENERATION, Id, PURGE_THRESHOLD, Relaxed, SourceMap};

        fn filled_map(len: usize) -> SourceMap<Id> {
            let map = SourceMap::default();
            for i in 1..=len as u64 {
                map.touch_or_insert_with(Id::from_hash(i), || i.to_string());
            }
            map
        }

        #[test]
        fn small_map_is_not_purged() {
            let map = filled_map(PURGE_THRESHOLD);
            map.purge(u64::MAX);
            assert_eq!(map.get(Id::from_hash(1)).as_deref(), Some("1"));
        }

        #[test]
        fn large_map_is_purged() {
            // Other tests may bump the generation concurrently.
            let generation_before = GENERATION.load(Relaxed);
            let map = filled_map(PURGE_THRESHOLD + 1);
            let generation_after = GENERATION.load(Relaxed);
            map.purge(generation_before + 1);
            assert_eq!(
                map.0.read().len(),
                PURGE_THRESHOLD + 1,
                "Seen last generation"
            );
            map.purge(generation_after + 2);
            assert!(map.0.read().is_empty());
        }
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

    // The debug source map memoizes the first spelling it sees for an `Id`, so two tests that
    // build the same `Id` by different routes race: whichever runs first decides what the other
    // one's `{:?}` prints. Every test here therefore uses sources no other test uses.
    #[test]
    fn with_salt_matches_with() {
        let parent = Id::unique("salted_parent");
        assert_eq!(
            parent.with_salt(IdSalt::new("salted_child")),
            parent.with("salted_child")
        );
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

    /// Salting an [`Id`] with itself used to double the stored source at every step.
    #[test]
    fn salting_with_self_is_truncated() {
        let mut id = Id::unique("self_salted");
        for _ in 0..64 {
            id = id.with(id);
        }
        let formatted = format!("{id:?}");
        assert!(formatted.starts_with('…'), "{formatted}");
        assert!(formatted.len() < 2000, "{} bytes", formatted.len());
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
