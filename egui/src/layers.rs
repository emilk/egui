use ahash::AHashMap;

use crate::{math::Rect, paint::PaintCmd, Id};

/// Different layer categories
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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

/// An identifier for a paint layer.
/// Also acts as an identifier for `Area`:s.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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

/// A unique identifier of a specific `PaintCmd` in a `PaintList`.
#[derive(Clone, Copy, PartialEq)]
pub struct PaintCmdIdx(usize);

/// Each `PaintCmd` is paired with a clip rectangle.
#[derive(Clone, Default)]
pub struct PaintList(Vec<(Rect, PaintCmd)>);

impl PaintList {
    /// Returns the index of the new command that can be used with `PaintList::set`.
    pub fn add(&mut self, clip_rect: Rect, cmd: PaintCmd) -> PaintCmdIdx {
        let idx = PaintCmdIdx(self.0.len());
        self.0.push((clip_rect, cmd));
        idx
    }

    pub fn extend(&mut self, clip_rect: Rect, mut cmds: Vec<PaintCmd>) {
        self.0.extend(cmds.drain(..).map(|cmd| (clip_rect, cmd)))
    }

    /// Modify an existing command.
    ///
    /// Sometimes you want to paint a frame behind some contents, but don't know how large the frame needs to be
    /// until the contents have been added, and therefor also painted to the `PaintList`.
    ///
    /// The solution is to allocate a `PaintCmd` using `let idx = paint_list.add(cr, PaintCmd::Noop);`
    /// and then later setting it using `paint_list.set(idx, cr, frame);`.
    pub fn set(&mut self, idx: PaintCmdIdx, clip_rect: Rect, cmd: PaintCmd) {
        assert!(idx.0 < self.0.len());
        self.0[idx.0] = (clip_rect, cmd);
    }
}

// TODO: improve this
#[derive(Clone, Default)]
pub struct GraphicLayers(AHashMap<Layer, PaintList>);

impl GraphicLayers {
    pub fn list(&mut self, layer: Layer) -> &mut PaintList {
        self.0.entry(layer).or_default()
    }

    pub fn drain(
        &mut self,
        area_order: &[Layer],
    ) -> impl ExactSizeIterator<Item = (Rect, PaintCmd)> {
        let mut all_commands: Vec<_> = Default::default();

        for layer in area_order {
            if let Some(commands) = self.0.get_mut(layer) {
                all_commands.extend(commands.0.drain(..));
            }
        }

        if let Some(commands) = self.0.get_mut(&Layer::debug()) {
            all_commands.extend(commands.0.drain(..));
        }

        all_commands.into_iter()
    }
}
