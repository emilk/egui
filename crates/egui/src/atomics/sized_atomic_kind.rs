use crate::{Id, Image};
use emath::Vec2;
use epaint::Galley;
use std::sync::Arc;

/// A sized [`crate::AtomicKind`].
#[derive(Clone, Default, Debug)]
pub enum SizedAtomicKind<'a> {
    #[default]
    Empty,
    Text(Arc<Galley>),
    Image(Image<'a>, Vec2),
    Custom(Id),
}

impl SizedAtomicKind<'_> {
    /// Get the calculated size.
    pub fn size(&self) -> Vec2 {
        match self {
            SizedAtomicKind::Text(galley) => galley.size(),
            SizedAtomicKind::Image(_, size) => *size,
            SizedAtomicKind::Empty | SizedAtomicKind::Custom(_) => Vec2::ZERO,
        }
    }
}
