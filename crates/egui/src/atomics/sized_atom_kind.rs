use crate::{Id, Image};
use emath::Vec2;
use epaint::Galley;
use std::sync::Arc;

/// A sized [`crate::AtomKind`].
#[derive(Clone, Default, Debug)]
pub enum SizedAtomKind<'a> {
    #[default]
    Empty,
    Text(Arc<Galley>),
    Image(Image<'a>, Vec2),
    Sized(Vec2),
}

impl SizedAtomKind<'_> {
    /// Get the calculated size.
    pub fn size(&self) -> Vec2 {
        match self {
            SizedAtomKind::Text(galley) => galley.size(),
            SizedAtomKind::Image(_, size) => *size,
            SizedAtomKind::Sized(size) => *size,
            SizedAtomKind::Empty => Vec2::ZERO,
        }
    }
}
