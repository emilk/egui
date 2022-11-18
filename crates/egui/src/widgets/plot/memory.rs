use epaint::Pos2;

use crate::{Id, Context};

use super::{AxisBools, transform::ScreenTransform};

/// Information about the plot that has to persist between frames.
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone)]
pub(super) struct PlotMemory {
    /// Indicates if the user has modified the bounds, for example by moving or zooming,
    /// or if the bounds should be calculated based by included point or auto bounds.
    pub(super) bounds_modified: AxisBools,
    pub(super) hovered_entry: Option<String>,
    pub(super) hidden_items: ahash::HashSet<String>,
    pub(super) last_screen_transform: ScreenTransform,
    /// Allows to remember the first click position when performing a boxed zoom
    pub(super) last_click_pos_for_zoom: Option<Pos2>,
}

impl PlotMemory {
    pub fn load(ctx: &Context, id: Id) -> Option<Self> {
        ctx.data().get_persisted(id)
    }

    pub fn store(self, ctx: &Context, id: Id) {
        ctx.data().insert_persisted(id, self);
    }
}
