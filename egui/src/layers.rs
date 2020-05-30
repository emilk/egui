use ahash::AHashMap;

use crate::{math::Rect, paint::PaintCmd, Id};

/// Different layer categories
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Order {
    /// Painted behind all floating windows
    Background,
    /// Normal moveable windows that you reorder by click
    Middle,
    /// Popups, menus etc that should always be painted on top of windows
    Foreground,
    /// Debug layer, always painted last / on top
    Debug,
}

/// An ideintifer for a paint layer.
/// Also acts as an identifier for `Area`:s.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Layer {
    pub order: Order,
    pub id: Id,
}

impl Layer {
    pub fn debug() -> Self {
        Self {
            order: Order::Debug,
            id: Id::new("debug"),
        }
    }
}

/// Each `PaintCmd` is paired with a clip rectangle.
type PaintList = Vec<(Rect, PaintCmd)>;

/// TODO: improve this
#[derive(Clone, Default)]
pub struct GraphicLayers(AHashMap<Layer, PaintList>);

impl GraphicLayers {
    pub fn layer(&mut self, layer: Layer) -> &mut PaintList {
        self.0.entry(layer).or_default()
    }

    pub fn drain(
        &mut self,
        area_order: &[Layer],
    ) -> impl ExactSizeIterator<Item = (Rect, PaintCmd)> {
        let mut all_commands: Vec<_> = Default::default();

        for layer in area_order {
            if let Some(commands) = self.0.get_mut(layer) {
                all_commands.extend(commands.drain(..));
            }
        }

        if let Some(commands) = self.0.get_mut(&Layer::debug()) {
            all_commands.extend(commands.drain(..));
        }

        all_commands.into_iter()
    }
}
