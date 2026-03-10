use crate::Image;
use emath::Vec2;
use epaint::Galley;
use std::sync::Arc;

/// A sized [`crate::AtomKind`].
#[derive(Clone, Debug)]
pub enum SizedAtomKind<'a> {
    Empty { size: Option<Vec2> },
    Text(Arc<Galley>),
    Image { image: Image<'a>, size: Vec2 },
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
        }
    }
}
