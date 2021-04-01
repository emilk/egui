// TODO: have separate types `PositionId` and `UniqueId`. ?

use std::hash::Hash;

/// egui tracks widgets frame-to-frame using `Id`s.
///
/// For instance, if you start dragging a slider one frame, egui stores
/// the sliders `Id` as the current active id so that next frame when
/// you move the mouse the same slider changes, even if the mouse has
/// moved outside the slider.
///
/// For some widgets `Id`s are also used to persist some state about the
/// widgets, such as Window position or wether not a collapsing header region is open.
///
/// This implies that the `Id`s must be unique.
///
/// For simple things like sliders and buttons that don't have any memory and
/// doesn't move we can use the location of the widget as a source of identity.
/// For instance, a slider only needs a unique and persistent ID while you are
/// dragging the slider. As long as it is still while moving, that is fine.
///
/// For things that need to persist state even after moving (windows, collapsing headers)
/// the location of the widgets is obviously not good enough. For instance,
/// a collapsing region needs to remember wether or not it is open even
/// if the layout next frame is different and the collapsing is not lower down
/// on the screen.
///
/// Then there are widgets that need no identifiers at all, like labels,
/// because they have no state nor are interacted with.
#[derive(Clone, Copy, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Id(u64);

impl Id {
    pub(crate) fn background() -> Self {
        Self(0)
    }

    /// Generate a new `Id` by hashing some source (e.g. a string or integer).
    pub fn new(source: impl Hash) -> Id {
        // NOTE: AHasher is NOT suitable for this!
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::default();
        source.hash(&mut hasher);
        Id(hasher.finish())
    }

    /// Generate a new `Id` by hashing the parent `Id` and the given argument.
    pub fn with(self, child: impl Hash) -> Id {
        // NOTE: AHasher is NOT suitable for this!
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        let mut hasher = DefaultHasher::default();
        hasher.write_u64(self.0);
        child.hash(&mut hasher);
        Id(hasher.finish())
    }

    pub(crate) fn short_debug_format(&self) -> String {
        format!("{:04X}", self.0 as u16)
    }

    #[inline(always)]
    pub(crate) fn value(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Debug for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:X}", self.0)
    }
}
