use std::iter::FusedIterator;
use std::sync::Arc;

use crate::{Color32, Direction, Frame, Id, Rect};

/// What kind is this [`Ui`]?
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiKind {
    /// A [`egui::Window`].
    Window,

    /// A [`egui::CentralPanel`].
    CentralPanel,

    /// A left [`egui::SidePanel`].
    LeftPanel,

    /// A right [`egui::SidePanel`].
    RightPanel,

    /// A top [`egui::TopBottomPanel`].
    TopPanel,

    /// A bottom [`egui::TopBottomPanel`].
    BottomPanel,

    /// A [`egui::Frame`].
    Frame,

    /// A [`egui::ScrollArea`].
    ScrollArea,

    /// A [`egui::Resize`].
    Resize,

    /// The content of a regular menu.
    Menu,

    /// The content of a popup menu.
    Popup,

    /// A tooltip, as shown by e.g. [`egui::Response::on_hover_ui`].
    Tooltip,

    /// A picker, such as color picker.
    Picker,

    /// A table cell (from the `egui_extras` crate).
    TableCell,

    /// An [`egui::Area`] that is not of any other kind.
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

/// Information about a [`egui::Ui`] to be included in the corresponding [`UiStack`].
#[derive(Default, Copy, Clone, Debug)]
pub struct UiStackInfo {
    pub kind: Option<UiKind>,
    pub frame: Frame,
}

// ----------------------------------------------------------------------------

/// Information about a [`egui::Ui`] and its parents.
///
/// [`UiStack`] serves to keep track of the current hierarchy of [`egui::Ui`]s, such
/// that nested widgets or user code may adapt to the surrounding context or obtain layout information
/// from a [`egui::Ui`] that might be several steps higher in the hierarchy.
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
    /// Is this [`egui::Ui`] a panel?
    #[inline]
    pub fn is_panel_ui(&self) -> bool {
        self.kind.map_or(false, |kind| kind.is_panel())
    }

    /// Is this a root [`egui::Ui`], i.e. created with [`Ui::new()`]?
    #[inline]
    pub fn is_root_ui(&self) -> bool {
        self.parent.is_none()
    }

    /// This this [`egui::Ui`] a [`egui::Frame`] with a visible stroke?
    #[inline]
    pub fn has_visible_frame(&self) -> bool {
        self.frame.stroke.width > 0.0 && self.frame.stroke.color != Color32::TRANSPARENT
    }
}

// these methods act on the entire stack
impl UiStack {
    /// Return an iterator that walks the stack from this node to the root.
    #[allow(clippy::iter_without_into_iter)]
    pub fn iter(&self) -> UiStackIterator {
        UiStackIterator {
            next: Some(Arc::new(self.clone())),
        }
    }

    /// Check if this node is or is contained in a [`egui::Ui`] of a specific kind.
    pub fn contained_id(&self, kind: UiKind) -> bool {
        self.iter().any(|frame| frame.kind == Some(kind))
    }
}

// ----------------------------------------------------------------------------

/// Iterator that walks up a stack of `StackFrame`s.
///
/// See [`UiStack::iter`].
pub struct UiStackIterator {
    next: Option<Arc<UiStack>>,
}

impl Iterator for UiStackIterator {
    type Item = Arc<UiStack>;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next.clone();
        self.next = current.as_ref().and_then(|frame| frame.parent.clone());
        current
    }
}

impl FusedIterator for UiStackIterator {}
