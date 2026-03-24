// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use std::num::NonZeroU64;

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

    #[inline]
    const fn from_hash(hash: u64) -> Self {
        if let Some(nonzero) = NonZeroU64::new(hash) {
            Self(nonzero)
        } else {
            Self(NonZeroU64::MIN) // The hash was exactly zero (very bad luck)
        }
    }

    /// Generate a new [`Id`] by hashing some source (e.g. a string or integer).
    pub fn new(source: impl std::hash::Hash) -> Self {
        debug_assert!(
            std::any::type_name_of_val(&source) != std::any::type_name::<Self>(),
            "Don't pass an `Id` to `Id::new()`: use `.with()` to create child `Id`s"
        );
        Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(source))
    }

    /// Like [`Self::new`], but for use as an id salt.
    ///
    /// Unlike [`Self::new`], this does not reject [`Id`] input,
    /// because using an [`Id`] as a salt (to be mixed into a parent via [`.with()`](Self::with))
    /// is a valid use case.
    pub(crate) fn new_salt(source: impl std::hash::Hash) -> Self {
        Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(source))
    }

    /// Generate a new [`Id`] by hashing the parent [`Id`] and the given argument.
    pub fn with(self, child: impl std::hash::Hash) -> Self {
        use std::hash::{BuildHasher as _, Hasher as _};
        let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.0.get());
        child.hash(&mut hasher);
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

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.value() as u16)
    }
}

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

#[test]
fn id_size() {
    assert_eq!(std::mem::size_of::<Id>(), 8);
    assert_eq!(std::mem::size_of::<Option<Id>>(), 8);
}

#[test]
#[should_panic(expected = "Don't pass an `Id` to `Id::new()`")]
fn test_id_new_rejects_id() {
    let _ = Id::new(Id::NULL);
}

// ----------------------------------------------------------------------------

/// A value to be used as an [`Id`] salt.
///
/// This is used by builder methods like [`crate::UiBuilder::id_salt`], [`crate::Grid::new`], etc.
/// It can be created from common hashable types (`&str`, `String`, integers)
/// as well as from an existing [`Id`].
///
/// When created from an [`Id`], the value is stored directly without re-hashing.
/// When created from other types, the value is hashed into an [`Id`].
///
/// ## Example
/// ```
/// use egui::{Id, IdSalt};
///
/// // From a string:
/// let salt: IdSalt = "my_widget".into();
///
/// // From an existing Id (no re-hash):
/// let id = Id::new("parent");
/// let salt: IdSalt = id.into();
/// ```
#[derive(Clone, Copy, Debug, Hash)]
pub struct IdSalt(Id);

impl IdSalt {
    /// Create an [`IdSalt`] by hashing some source.
    ///
    /// Use this for types that don't have a `From` impl
    /// (e.g. tuples, custom types).
    #[inline]
    pub fn new(source: impl std::hash::Hash) -> Self {
        Self(Id::new_salt(source))
    }

    /// Get the inner [`Id`].
    #[inline]
    pub fn id(self) -> Id {
        self.0
    }
}

impl From<Id> for IdSalt {
    /// Store an [`Id`] directly as a salt, without re-hashing.
    #[inline]
    fn from(id: Id) -> Self {
        Self(id)
    }
}

impl From<&str> for IdSalt {
    #[inline]
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for IdSalt {
    #[inline]
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

macro_rules! impl_id_salt_from_int {
    ($($t:ty),*) => {
        $(
            impl From<$t> for IdSalt {
                #[inline]
                fn from(v: $t) -> Self {
                    Self::new(v)
                }
            }
        )*
    };
}

impl_id_salt_from_int!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, bool);

// ----------------------------------------------------------------------------

/// `IdSet` is a `HashSet<Id>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet = nohash_hasher::IntSet<Id>;

/// `IdMap<V>` is a `HashMap<Id, V>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<V> = nohash_hasher::IntMap<Id, V>;
