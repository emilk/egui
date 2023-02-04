// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

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
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Id(u64);

impl Id {
    /// A special [`Id`], in particular as a key to [`crate::Memory::data`]
    /// for when there is no particular widget to attach the data.
    ///
    /// The null [`Id`] is still a valid id to use in all circumstances,
    /// though obviously it will lead to a lot of collisions if you do use it!
    pub fn null() -> Self {
        Self(0)
    }

    pub(crate) fn background() -> Self {
        Self(1)
    }

    /// Generate a new [`Id`] by hashing some source (e.g. a string or integer).
    pub fn new(source: impl std::hash::Hash) -> Id {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = epaint::ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        source.hash(&mut hasher);
        Id(hasher.finish())
    }

    /// Generate a new [`Id`] by hashing the parent [`Id`] and the given argument.
    pub fn with(self, child: impl std::hash::Hash) -> Id {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = epaint::ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.0);
        child.hash(&mut hasher);
        Id(hasher.finish())
    }

    /// Short and readable summary
    pub fn short_debug_format(&self) -> String {
        format!("{:04X}", self.0 as u16)
    }

    #[inline(always)]
    pub(crate) fn value(&self) -> u64 {
        self.0
    }

    #[cfg(feature = "accesskit")]
    pub(crate) fn accesskit_id(&self) -> accesskit::NodeId {
        std::num::NonZeroU64::new(self.0).unwrap().into()
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:016X}", self.0)
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

// ----------------------------------------------------------------------------

// Idea taken from the `nohash_hasher` crate.
#[derive(Default)]
pub struct IdHasher(u64);

impl std::hash::Hasher for IdHasher {
    fn write(&mut self, _: &[u8]) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u8(&mut self, _n: u8) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u16(&mut self, _n: u16) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_u32(&mut self, _n: u32) {
        unreachable!("Invalid use of IdHasher");
    }

    #[inline(always)]
    fn write_u64(&mut self, n: u64) {
        self.0 = n;
    }

    fn write_usize(&mut self, _n: usize) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i8(&mut self, _n: i8) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i16(&mut self, _n: i16) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i32(&mut self, _n: i32) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_i64(&mut self, _n: i64) {
        unreachable!("Invalid use of IdHasher");
    }

    fn write_isize(&mut self, _n: isize) {
        unreachable!("Invalid use of IdHasher");
    }

    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct BuilIdHasher {}

impl std::hash::BuildHasher for BuilIdHasher {
    type Hasher = IdHasher;

    #[inline(always)]
    fn build_hasher(&self) -> IdHasher {
        IdHasher::default()
    }
}

/// `IdSet` is a `HashSet<Id>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdSet = std::collections::HashSet<Id, BuilIdHasher>;

/// `IdMap<V>` is a `HashMap<Id, V>` optimized by knowing that [`Id`] has good entropy, and doesn't need more hashing.
pub type IdMap<V> = std::collections::HashMap<Id, V, BuilIdHasher>;
