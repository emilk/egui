use crate::{AtomPaint, Image, SizedAtomLayout};
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
    Paint(AtomPaint<'a>),
    Layout(Box<SizedAtomLayout<'a>>),
}

impl Debug for SizedAtomKind<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SizedAtomKind::Empty { size } => write!(f, "SizedAtomKind::Empty({size:?})"),
            SizedAtomKind::Text(galley) => write!(f, "SizedAtomKind::Text({galley:?})"),
            SizedAtomKind::Image { image, size } => {
                write!(f, "SizedAtomKind::Image({image:?}, {size:?})")
            }
            SizedAtomKind::Paint(_) => write!(f, "SizedAtomKind::Paint(<closure>)"),
            SizedAtomKind::Layout(layout) => write!(f, "SizedAtomKind::Layout({layout:?})"),
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
            SizedAtomKind::Image { image: _, size } => *size,
            SizedAtomKind::Empty { size } => size.unwrap_or_default(),
            SizedAtomKind::Paint(_) => Vec2::ZERO,
            SizedAtomKind::Layout(layout) => layout.outer_size,
        }
    }
}
