use crate::{AtomPaint, Image, SizedContainerAtom, SizedWidgetAtom};
use core::fmt::Debug;
use emath::Vec2;
use epaint::Galley;
use std::sync::Arc;

/// A sized [`crate::AtomKind`].
#[derive(Clone)]
pub enum SizedAtomKind<'a> {
    Empty { size: Option<Vec2> },
    Text(Arc<Galley>),
    Image { image: Image<'a>, size: Vec2 },
    Paint { paint: AtomPaint<'a>, size: Vec2 },
    Container(Box<SizedContainerAtom<'a>>),
    Widget(Box<SizedWidgetAtom<'a>>),
}

impl Debug for SizedAtomKind<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SizedAtomKind::Empty { size } => write!(f, "SizedAtomKind::Empty({size:?})"),
            SizedAtomKind::Text(galley) => write!(f, "SizedAtomKind::Text({galley:?})"),
            SizedAtomKind::Image { image, size } => {
                write!(f, "SizedAtomKind::Image({image:?}, {size:?})")
            }
            SizedAtomKind::Paint { size, .. } => {
                write!(f, "SizedAtomKind::Paint(<closure>, {size:?})")
            }
            SizedAtomKind::Widget(layout) => write!(f, "SizedAtomKind::Widget({layout:?})"),
            SizedAtomKind::Container(container) => {
                write!(f, "SizedAtomKind::Container({container:?})")
            }
        }
    }
}

impl Default for SizedAtomKind<'_> {
    fn default() -> Self {
        Self::Empty { size: None }
    }
}

impl SizedAtomKind<'_> {
    /// Get the calculated size.
    pub fn size(&self) -> Vec2 {
        match self {
            SizedAtomKind::Text(galley) => galley.size(),
            SizedAtomKind::Image { size, .. } | SizedAtomKind::Paint { size, .. } => *size,
            SizedAtomKind::Empty { size } => size.unwrap_or_default(),
            SizedAtomKind::Widget(layout) => layout.outer_size,
            SizedAtomKind::Container(container) => container.outer_size,
        }
    }
}
