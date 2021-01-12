use epaint::ahash::AHashMap;

use crate::{math::Rect, paint::Shape, Id, *};

/// Different layer categories
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub enum Order {
    /// Painted behind all floating windows
    Background,
    /// Normal moveable windows that you reorder by click
    Middle,
    /// Popups, menus etc that should always be painted on top of windows
    /// Foreground objects can also have tooltips
    Foreground,
    /// Things floating on top of everything else, like tooltips.
    /// You cannot interact with these.
    Tooltip,
    /// Debug layer, always painted last / on top
    Debug,
}
impl Order {
    const COUNT: usize = 5;
    const ALL: [Order; Self::COUNT] = [
        Self::Background,
        Self::Middle,
        Self::Foreground,
        Self::Tooltip,
        Self::Debug,
    ];

    pub fn allow_interaction(&self) -> bool {
        match self {
            Self::Background | Self::Middle | Self::Foreground | Self::Debug => true,
            Self::Tooltip => false,
        }
    }
}

/// An identifier for a paint layer.
/// Also acts as an identifier for [`Area`]:s.
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct LayerId {
    pub order: Order,
    pub id: Id,
}

impl LayerId {
    pub fn new(order: Order, id: Id) -> Self {
        Self { order, id }
    }

    pub fn debug() -> Self {
        Self {
            order: Order::Debug,
            id: Id::new("debug"),
        }
    }

    pub fn background() -> Self {
        Self {
            order: Order::Background,
            id: Id::background(),
        }
    }

    pub fn allow_interaction(&self) -> bool {
        self.order.allow_interaction()
    }
}

/// A unique identifier of a specific [`Shape`] in a [`PaintList`].
#[derive(Clone, Copy, PartialEq)]
pub struct ShapeIdx(usize);

/// A list of [`Shape`]s paired with a clip rectangle.
#[derive(Clone, Default)]
pub struct PaintList(Vec<(Rect, Shape)>);

impl PaintList {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the index of the new [`Shape`] that can be used with `PaintList::set`.
    pub fn add(&mut self, clip_rect: Rect, shape: Shape) -> ShapeIdx {
        let idx = ShapeIdx(self.0.len());
        self.0.push((clip_rect, shape));
        idx
    }

    pub fn extend(&mut self, clip_rect: Rect, mut shapes: Vec<Shape>) {
        self.0
            .extend(shapes.drain(..).map(|shape| (clip_rect, shape)))
    }

    /// Modify an existing [`Shape`].
    ///
    /// Sometimes you want to paint a frame behind some contents, but don't know how large the frame needs to be
    /// until the contents have been added, and therefor also painted to the `PaintList`.
    ///
    /// The solution is to allocate a `Shape` using `let idx = paint_list.add(cr, Shape::Noop);`
    /// and then later setting it using `paint_list.set(idx, cr, frame);`.
    pub fn set(&mut self, idx: ShapeIdx, clip_rect: Rect, shape: Shape) {
        assert!(idx.0 < self.0.len());
        self.0[idx.0] = (clip_rect, shape);
    }

    /// Translate each [`Shape`] and clip rectangle by this much, in-place
    pub fn translate(&mut self, delta: Vec2) {
        for (clip_rect, shape) in &mut self.0 {
            *clip_rect = clip_rect.translate(delta);
            shape.translate(delta);
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct GraphicLayers([AHashMap<Id, PaintList>; Order::COUNT]);

impl GraphicLayers {
    pub fn list(&mut self, layer_id: LayerId) -> &mut PaintList {
        self.0[layer_id.order as usize]
            .entry(layer_id.id)
            .or_default()
    }

    pub fn drain(
        &mut self,
        area_order: &[LayerId],
    ) -> impl ExactSizeIterator<Item = (Rect, Shape)> {
        let mut all_shapes: Vec<_> = Default::default();

        for &order in &Order::ALL {
            let order_map = &mut self.0[order as usize];

            // If a layer is empty at the start of the frame
            // the nobody has added to it, and it is old and defunct.
            // Free it to save memory:
            order_map.retain(|_, list| !list.is_empty());

            // First do the layers part of area_order:
            for layer_id in area_order {
                if layer_id.order == order {
                    if let Some(shapes) = order_map.get_mut(&layer_id.id) {
                        all_shapes.extend(shapes.0.drain(..));
                    }
                }
            }

            // Also draw areas that are missing in `area_order`:
            for shapes in order_map.values_mut() {
                all_shapes.extend(shapes.0.drain(..));
            }
        }

        all_shapes.into_iter()
    }
}
