use std::iter::FusedIterator;
use std::sync::Arc;

use crate::{Direction, Frame, Id, Rect};

/// What kind is this [`crate::Ui`]?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiKind {
    /// A [`crate::Window`].
    Window,

    /// A [`crate::CentralPanel`].
    CentralPanel,

    /// A left [`crate::SidePanel`].
    LeftPanel,

    /// A right [`crate::SidePanel`].
    RightPanel,

    /// A top [`crate::TopBottomPanel`].
    TopPanel,

    /// A bottom [`crate::TopBottomPanel`].
    BottomPanel,

    /// A [`crate::Frame`].
    Frame,

    /// A [`crate::ScrollArea`].
    ScrollArea,

    /// A [`crate::Resize`].
    Resize,

    /// The content of a regular menu.
    Menu,

    /// The content of a popup menu.
    Popup,

    /// A tooltip, as shown by e.g. [`crate::Response::on_hover_ui`].
    Tooltip,

    /// A picker, such as color picker.
    Picker,

    /// A table cell (from the `egui_extras` crate).
    TableCell,

    /// An [`crate::Area`] that is not of any other kind.
    GenericArea,
}

impl UiKind {
    /// Is this any kind of panel?
    pub fn is_panel(&self) -> bool {
        matches!(
            self,
            Self::CentralPanel
                | Self::LeftPanel
                | Self::RightPanel
                | Self::TopPanel
                | Self::BottomPanel
        )
    }
}

// ----------------------------------------------------------------------------

/// Information about a [`crate::Ui`] to be included in the corresponding [`UiStack`].
#[derive(Default, Copy, Clone, Debug)]
pub struct UiStackInfo {
    pub kind: Option<UiKind>,
    pub frame: Frame,
}

impl UiStackInfo {
    /// Create a new [`UiStackInfo`] with the given kind and an empty frame.
    pub fn new(kind: UiKind) -> Self {
        Self {
            kind: Some(kind),
            frame: Default::default(),
        }
    }
}

// ----------------------------------------------------------------------------

/// Information about a [`crate::Ui`] and its parents.
///
/// [`UiStack`] serves to keep track of the current hierarchy of [`crate::Ui`]s, such
/// that nested widgets or user code may adapt to the surrounding context or obtain layout information
/// from a [`crate::Ui`] that might be several steps higher in the hierarchy.
///
/// Note: since [`UiStack`] contains a reference to its parent, it is both a stack, and a node within
/// that stack. Most of its methods are about the specific node, but some methods walk up the
/// hierarchy to provide information about the entire stack.
#[derive(Clone, Debug)]
pub struct UiStack {
    // stuff that `Ui::child_ui` can deal with directly
    pub id: Id,
    pub kind: Option<UiKind>,
    pub frame: Frame,
    pub layout_direction: Direction,
    pub min_rect: Rect,
    pub max_rect: Rect,
    pub parent: Option<Arc<UiStack>>,
}

// these methods act on this specific node
impl UiStack {
    /// Is this [`crate::Ui`] a panel?
    #[inline]
    pub fn is_panel_ui(&self) -> bool {
        self.kind.map_or(false, |kind| kind.is_panel())
    }

    /// Is this a root [`crate::Ui`], i.e. created with [`crate::Ui::new()`]?
    #[inline]
    pub fn is_root_ui(&self) -> bool {
        self.parent.is_none()
    }

    /// This this [`crate::Ui`] a [`crate::Frame`] with a visible stroke?
    #[inline]
    pub fn has_visible_frame(&self) -> bool {
        !self.frame.stroke.is_empty()
    }
}

// these methods act on the entire stack
impl UiStack {
    /// Return an iterator that walks the stack from this node to the root.
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> UiStackIterator<'_> {
        UiStackIterator { next: Some(self) }
    }

    /// Check if this node is or is contained in a [`crate::Ui`] of a specific kind.
    pub fn contained_in(&self, kind: UiKind) -> bool {
        self.iter().any(|frame| frame.kind == Some(kind))
    }
}

// ----------------------------------------------------------------------------

/// Iterator that walks up a stack of `StackFrame`s.
///
/// See [`UiStack::iter`].
pub struct UiStackIterator<'a> {
    next: Option<&'a UiStack>,
}

impl<'a> Iterator for UiStackIterator<'a> {
    type Item = &'a UiStack;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next;
        self.next = current.and_then(|frame| frame.parent.as_deref());
        current
    }
}

impl<'a> FusedIterator for UiStackIterator<'a> {}
