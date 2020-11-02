// TODO: have separate types `PositionId` and `UniqueId`. ?

use std::hash::Hash;

/// Egui tracks widgets frame-to-frame using `Id`s.
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
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Id(u64);

impl Id {
    pub fn background() -> Self {
        Self(0)
    }

    pub fn tooltip() -> Self {
        Self(1)
    }

    pub fn new(source: impl Hash) -> Self {
        use std::hash::Hasher;
        let mut hasher = ahash::AHasher::default();
        source.hash(&mut hasher);
        Id(hasher.finish())
    }

    pub fn with(self, child: impl Hash) -> Self {
        use std::hash::Hasher;
        let mut hasher = ahash::AHasher::default();
        hasher.write_u64(self.0);
        child.hash(&mut hasher);
        Id(hasher.finish())
    }

    pub(crate) fn short_debug_format(&self) -> String {
        format!("{:04X}", self.0 as u16)
    }
}

// ----------------------------------------------------------------------------

/// This is an identifier that must be unique over long time.
/// Used for storing state, like window position, scroll amount, etc.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct StrongId(u64);

impl StrongId {
    pub fn background() -> Self {
        Self(0)
    }

    pub fn tooltip() -> Self {
        Self(1)
    }

    pub fn new(source: impl Hash) -> Self {
        use std::hash::Hasher;
        let mut hasher = ahash::AHasher::default();
        source.hash(&mut hasher);
        Id(hasher.finish())
    }

    pub fn with(self, child: impl Hash) -> Self {
        use std::hash::Hasher;
        let mut hasher = ahash::AHasher::default();
        hasher.write_u64(self.0);
        child.hash(&mut hasher);
        Id(hasher.finish())
    }

    pub(crate) fn short_debug_format(&self) -> String {
        format!("{:04X}", self.0 as u16)
    }
}

// ----------------------------------------------------------------------------

/// Ok to weaken a `StrongId`
impl From<StrongId> for Id {
    fn from(strong: StrongId) -> Self {
        Id(strong.0)
    }
}
