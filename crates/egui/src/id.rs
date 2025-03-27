// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use ahash::HashMap;
use epaint::mutex::{Mutex, RwLock};
use std::any::TypeId;
use std::hash::Hasher;
use std::num::NonZeroU64;
use std::sync::LazyLock;

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

enum IdSource {
    Id(Id),
    Other(String),
}

static ID_MAP: LazyLock<RwLock<HashMap<Id, (IdSource, Option<Id>)>>> = LazyLock::new(|| {
    let mut map = HashMap::default();
    map.insert(Id::NULL, (IdSource::Other("Id::NULL".to_owned()), None));
    RwLock::new(map)
});

impl nohash_hasher::IsEnabled for Id {}

pub trait IdTrait: std::hash::Hash + std::fmt::Debug {}
impl<T: std::hash::Hash + std::fmt::Debug> IdTrait for T {}

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

    /// Checks if [`T`] is a [`Id`].
    ///
    /// If it is, it returns `IdSource::Id`, otherwise it returns `IdSource::Other`.
    fn get_source<T: IdTrait>(t: T) -> IdSource {
        /// Ugly hack to try to determine if T is an Id or not.
        struct FakeHasher {
            val: Option<u64>,
            first: bool,
        }

        impl Hasher for FakeHasher {
            fn finish(&self) -> u64 {
                unreachable!()
            }

            fn write(&mut self, bytes: &[u8]) {
                self.first = false;
            }

            fn write_u64(&mut self, i: u64) {
                if self.first {
                    self.val = Some(i);
                    self.first = false;
                } else {
                    self.val = None;
                }
            }
        }

        let mut hasher = FakeHasher {
            val: None,
            first: true,
        };

        t.hash(&mut hasher);

        let maybe_source_id = hasher.val.map(Id::from_hash);

        // Ideally we would just implement IdTriat for Id with specialization, but that's not
        // a thing yet :( So we check if the hash is already in the map, if so, the source must be
        // an Id.
        if let Some(maybe_source_id) = maybe_source_id {
            if ID_MAP.read().contains_key(&maybe_source_id) {
                IdSource::Id(maybe_source_id)
            } else {
                IdSource::Other(format!("{:?}", t))
            }
        } else {
            IdSource::Other(format!("{:?}", t))
        }
    }

    /// Generate a new [`Id`] by hashing some source (e.g. a string or integer).
    pub fn new<T: IdTrait>(source: T) -> Self {
        let id = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        if !ID_MAP.read().contains_key(&id) {
            let source = Self::get_source(source);
            ID_MAP.write().insert(id, (source, None));
        }

        id
    }

    /// Generate a new [`Id`] by hashing the parent [`Id`] and the given argument.
    pub fn with(self, child: impl std::hash::Hash + std::fmt::Debug) -> Self {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.0.get());
        (&child).hash(&mut hasher);
        let id = Self::from_hash(hasher.finish());

        if !ID_MAP.read().contains_key(&id) {
            let source = Self::get_source(child);
            ID_MAP.write().insert(id, (source, Some(self)));
        }

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

    #[cfg(feature = "accesskit")]
    pub(crate) fn accesskit_id(&self) -> accesskit::NodeId {
        self.value().into()
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.value() as u16)?;
        let lock = ID_MAP.read();
        if let Some((source, parent)) = lock.get(self) {
            match source {
                IdSource::Id(source_id) => {
                    write!(f, "({:?})", source_id)?;
                }
                IdSource::Other(label) => {
                    write!(f, " ({})", label)?;
                }
            }
            if let Some(parent) = parent {
                // Let's hope there are no cycles!
                write!(f, " <- {:?}", parent)?;
            }
        }

        Ok(())
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

// ----------------------------------------------------------------------------

/// `IdSet` is a `HashSet<Id>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet = nohash_hasher::IntSet<Id>;

/// `IdMap<V>` is a `HashMap<Id, V>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<V> = nohash_hasher::IntMap<Id, V>;
