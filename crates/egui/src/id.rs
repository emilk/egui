// TODO(emilk): have separate types `PositionId` and `UniqueId`. ?

use crate::id::id_source::IdSource;
use crate::CollapsingHeader;
use epaint::Color32;
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

pub trait AsId: std::hash::Hash + std::fmt::Debug {}
impl<T: std::hash::Hash + std::fmt::Debug> AsId for T {}

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
    pub fn new<T: AsId>(source: T) -> Self {
        let id = Self::from_hash(ahash::RandomState::with_seeds(1, 2, 3, 4).hash_one(&source));

        #[cfg(debug_assertions)]
        id_source::maybe_insert(id, source, None);

        id
    }

    /// Generate a new [`Id`] by hashing the parent [`Id`] and the given argument.
    pub fn with(self, child: impl AsId) -> Self {
        use std::hash::{BuildHasher as _, Hasher as _};
        let mut hasher = ahash::RandomState::with_seeds(1, 2, 3, 4).build_hasher();
        hasher.write_u64(self.0.get());
        (&child).hash(&mut hasher);
        let id = Self::from_hash(hasher.finish());

        #[cfg(debug_assertions)]
        id_source::maybe_insert(id, &child, Some(self));

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

    pub(crate) fn accesskit_id(&self) -> accesskit::NodeId {
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

    fn source_ui(ui: &mut crate::Ui, source: IdSource) {
        match source {
            IdSource::Id(id) => {
                Self::parent_ui(ui, id);
            }
            IdSource::Other(other) => {
                ui.code(other);
            }
        }
    }

    fn parent_ui(ui: &mut crate::Ui, id: Id) {
        let data = id.info();
        if let Some(data) = data {
            if let Some(parent) = data.parent {
                Self::parent_ui(ui, parent);
                ui.horizontal(|ui| {
                    ui.code(".with(");
                    Self::source_ui(ui, data.source);
                    ui.code(format!("  /* {} */", id.short_debug_format()));
                    ui.code(")");
                });
            } else {
                ui.horizontal(|ui| {
                    ui.code("Id::new(");
                    Self::source_ui(ui, data.source);
                    ui.code(format!("  /* {} */", id.short_debug_format()));
                    ui.code(")");
                });
            }
        } else {
            ui.code(format!("Id::from_hash({})", id.short_debug_format()));
        }
    }

    fn group_ui(ui: &mut crate::Ui, id: Id) {
        ui.group(|ui| {
            let info = id.info();
            if let Some(info) = info {
                ui.horizontal(|ui| {
                    ui.label("Id(");
                    ui.code(format!("{:04X}", id.value() as u16));
                    ui.label(")");

                    ui.label("Source:");
                    match info.source {
                        IdSource::Id(id) => {
                            Self::group_ui(ui, id);
                        }
                        IdSource::Other(other) => {
                            ui.code(other);
                        }
                    }
                });
                if let Some(parent) = info.parent {
                    ui.label("^ with");
                    Self::group_ui(ui, parent);
                }
            } else {
            }
        });
    }

    fn tree_ui(ui: &mut crate::Ui, id: Id, prefix: &str, depth: usize) {
        let info = id.info();
        if let Some(info) = info {
            let response =
                CollapsingHeader::new(format!("{}Id({})", prefix, id.short_debug_format()))
                    .default_open(depth < 4)
                    .show(ui, |ui| {
                        match info.source {
                            IdSource::Id(id_source) => {
                                Self::tree_ui(ui, id_source, "Source: ", depth + 1);
                            }
                            IdSource::Other(other) => {
                                ui.horizontal(|ui| {
                                    ui.add_space(ui.spacing().indent);
                                    ui.label("Source:");
                                    ui.code(other);
                                });
                            }
                        }

                        if let Some(parent) = info.parent {
                            Self::tree_ui(ui, parent, "Parent: ", depth + 1);
                        }
                    });

            if response.header_response.hovered() {
                id.try_highlight(ui.ctx());
            }
        }
    }

    pub fn try_highlight(self, ctx: &crate::Context) {
        let response = ctx.read_response(self);
        if let Some(response) = response {
            ctx.debug_painter().debug_rect(
                response.rect,
                Color32::GREEN,
                self.short_debug_format(),
            );
        }

        if let Some(area_rect) = ctx.memory(|mem| mem.area_rect(self)) {
            ctx.debug_painter()
                .debug_rect(area_rect, Color32::RED, self.short_debug_format());
        }
    }

    pub fn ui(self, ui: &mut crate::Ui) -> crate::Response {
        let data = self.info();
        let label = if let Some(data) = &data {
            format!("{} ({})", self.short_debug_format(), data.source)
        } else {
            self.short_debug_format()
        };
        let response = ui.code(label).on_hover_ui(|ui| {
            Self::tree_ui(ui, self, "", 0);
        });

        if response.hovered() {
            self.try_highlight(ui.ctx());
        }

        response
    }
}

#[cfg(debug_assertions)]
mod id_source {
    use crate::{AsId, Id};
    use ahash::HashMap;
    use epaint::mutex::RwLock;
    use std::fmt::{Display, Formatter};
    use std::hash::Hasher;
    use std::sync::LazyLock;

    #[derive(Clone)]
    pub struct IdInfo {
        /// What was this Id generated from?
        pub source: IdSource,
        /// If the Id was crated via [`Id::with`], what was the parent Id?
        pub parent: Option<Id>,
    }

    #[derive(Clone)]
    pub enum IdSource {
        Id(Id),
        Other(String),
    }

    impl Display for IdSource {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            match self {
                IdSource::Id(id) => {
                    write!(f, "{}", id.short_debug_format())
                }
                IdSource::Other(other) => {
                    write!(f, "{}", other)
                }
            }
        }
    }

    static ID_MAP: LazyLock<RwLock<HashMap<Id, IdInfo>>> = LazyLock::new(|| {
        let mut map = HashMap::default();
        map.insert(
            Id::NULL,
            IdInfo {
                source: IdSource::Other("Id::NULL".to_owned()),
                parent: None,
            },
        );
        RwLock::new(map)
    });

    /// Ugly hack to try to determine if T is an Id or not.
    #[derive(Default)]
    struct ExtractIdHasher {
        val: Option<u64>,
        not_id: bool,
    }

    impl ExtractIdHasher {
        fn id(&self) -> Option<Id> {
            self.val.map(Id::from_hash)
        }
    }

    impl Hasher for ExtractIdHasher {
        fn finish(&self) -> u64 {
            unreachable!()
        }

        fn write(&mut self, _bytes: &[u8]) {
            self.not_id = true;
            self.val = None;
        }

        fn write_u64(&mut self, i: u64) {
            if !self.not_id && !self.val.is_some() {
                self.val = Some(i);
            } else {
                self.not_id = true;
                self.val = None;
            }
        }
    }

    /// Checks if [`T`] is a [`Id`].
    ///
    /// If it is, it returns `IdSource::Id`, otherwise it returns `IdSource::Other`.
    fn get_source<T: AsId>(t: T) -> IdSource {
        let mut hasher = ExtractIdHasher::default();

        t.hash(&mut hasher);

        let maybe_source_id = hasher.id();

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

    pub(super) fn maybe_insert(id: Id, source: impl AsId, parent: Option<Id>) {
        if !ID_MAP.read().contains_key(&id) {
            let source1 = get_source(source);
            ID_MAP.write().insert(
                id,
                IdInfo {
                    source: source1,
                    parent,
                },
            );
        }
    }

    impl Id {
        pub fn info(&self) -> Option<IdInfo> {
            ID_MAP.read().get(self).cloned()
        }
    }

    #[test]
    fn test_fake_hasher() {
        use std::hash::Hash;
        let mut hasher = ExtractIdHasher::default();

        let id = Id::new("test");
        id.hash(&mut hasher);

        assert_eq!(hasher.id(), Some(id));
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.value() as u16)?;

        #[cfg(debug_assertions)]
        if let Some(info) = self.info() {
            match info.source {
                id_source::IdSource::Id(source_id) => {
                    write!(f, "({:?})", source_id)?;
                }
                id_source::IdSource::Other(label) => {
                    write!(f, " ({})", label)?;
                }
            }
            if let Some(parent) = info.parent {
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
