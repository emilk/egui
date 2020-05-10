use std::collections::HashMap;

use crate::{math::Rect, Id, PaintCmd};

// TODO: support multiple windows
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub enum Layer {
    Background,
    Window(Id),
    /// Tooltips etc
    Popup,
    /// Debug text
    Debug,
}

/// Each `PaintCmd` is paired with a clip rectangle.
type PaintList = Vec<(Rect, PaintCmd)>;

/// TODO: improve this
#[derive(Clone, Default)]
pub struct GraphicLayers {
    bg: PaintList,
    windows: HashMap<Id, PaintList>,
    popup: PaintList,
    debug: PaintList,
}

impl GraphicLayers {
    pub fn layer(&mut self, layer: Layer) -> &mut PaintList {
        match layer {
            Layer::Background => &mut self.bg,
            Layer::Window(id) => self.windows.entry(id).or_default(),
            Layer::Popup => &mut self.popup,
            Layer::Debug => &mut self.debug,
        }
    }

    pub fn drain(
        &mut self,
        window_order: &[Id],
    ) -> impl ExactSizeIterator<Item = (Rect, PaintCmd)> {
        let mut all_commands: Vec<_> = self.bg.drain(..).collect();

        for id in window_order {
            if let Some(window) = self.windows.get_mut(id) {
                all_commands.extend(window.drain(..));
            }
        }

        all_commands.extend(self.popup.drain(..));
        all_commands.extend(self.debug.drain(..));
        all_commands.into_iter()
    }
}
