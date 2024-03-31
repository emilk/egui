use ahash::HashMap;

use crate::*;

/// Used to store each widget's [Id], [Rect] and [Sense] each frame.
///
/// Used to check which widget gets input when a user clicks somewhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetRect {
    /// The globally unique widget id.
    ///
    /// For interactive widgets, this better be globally unique.
    /// If not there will be weird bugs,
    /// and also big red warning test on the screen in debug builds
    /// (see [`Options::warn_on_id_clash`]).
    ///
    /// You can ensure globally unique ids using [`Ui::push_id`].
    pub id: Id,

    /// What layer the widget is on.
    pub layer_id: LayerId,

    /// The full widget rectangle.
    pub rect: Rect,

    /// Where the widget is.
    ///
    /// This is after clipping with the parent ui clip rect.
    pub interact_rect: Rect,

    /// How the widget responds to interaction.
    ///
    /// Note: if [`Self::enabled`] is `false`, then
    /// the widget _effectively_ doesn't sense anything,
    /// but can still have the same `Sense`.
    /// This is because the sense informs the styling of the widget,
    /// but we don't want to change the style when a widget is disabled
    /// (that is handled by the `Painter` directly).
    pub sense: Sense,

    /// Is the widget enabled?
    pub enabled: bool,
}

/// Stores the [`WidgetRect`]s of all widgets generated during a single egui update/frame.
///
/// All [`Ui`]s have a [`WidgetRects`], but whether or not their rects are correct
/// depends on if [`Ui::interact_bg`] was ever called.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct WidgetRects {
    /// All widgets, in painting order.
    by_layer: HashMap<LayerId, Vec<WidgetRect>>,

    /// All widgets, by id, and their order in their respective layer
    by_id: IdMap<(usize, WidgetRect)>,
}

impl WidgetRects {
    /// All known layers with widgets.
    pub fn layer_ids(&self) -> impl ExactSizeIterator<Item = LayerId> + '_ {
        self.by_layer.keys().copied()
    }

    pub fn layers(&self) -> impl Iterator<Item = (&LayerId, &[WidgetRect])> + '_ {
        self.by_layer
            .iter()
            .map(|(layer_id, rects)| (layer_id, &rects[..]))
    }

    #[inline]
    pub fn get(&self, id: Id) -> Option<&WidgetRect> {
        self.by_id.get(&id).map(|(_, w)| w)
    }

    /// In which layer, and in which order in that layer?
    pub fn order(&self, id: Id) -> Option<(LayerId, usize)> {
        self.by_id.get(&id).map(|(idx, w)| (w.layer_id, *idx))
    }

    #[inline]
    pub fn contains(&self, id: Id) -> bool {
        self.by_id.contains_key(&id)
    }

    /// All widgets in this layer, sorted back-to-front.
    #[inline]
    pub fn get_layer(&self, layer_id: LayerId) -> impl Iterator<Item = &WidgetRect> + '_ {
        self.by_layer.get(&layer_id).into_iter().flatten()
    }

    /// Clear the contents while retaining allocated memory.
    pub fn clear(&mut self) {
        let Self { by_layer, by_id } = self;

        for rects in by_layer.values_mut() {
            rects.clear();
        }

        by_id.clear();
    }

    /// Insert the given widget rect in the given layer.
    pub fn insert(&mut self, layer_id: LayerId, widget_rect: WidgetRect) {
        let Self { by_layer, by_id } = self;

        let layer_widgets = by_layer.entry(layer_id).or_default();

        match by_id.entry(widget_rect.id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                // A new widget
                let idx_in_layer = layer_widgets.len();
                entry.insert((idx_in_layer, widget_rect));
                layer_widgets.push(widget_rect);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                // This is a known widget, but we might need to update it!
                // e.g. calling `response.interact(…)` to add more interaction.
                let (idx_in_layer, existing) = entry.get_mut();

                egui_assert!(
                    existing.layer_id == widget_rect.layer_id,
                    "Widget changed layer_id during the frame"
                );

                // Update it:
                existing.rect = widget_rect.rect; // last wins
                existing.interact_rect = widget_rect.interact_rect; // last wins
                existing.sense |= widget_rect.sense;
                existing.enabled |= widget_rect.enabled;

                if existing.layer_id == widget_rect.layer_id {
                    layer_widgets[*idx_in_layer] = *existing;
                }
            }
        }
    }
}
