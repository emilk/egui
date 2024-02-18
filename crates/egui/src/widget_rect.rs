use ahash::HashMap;

use crate::*;

/// Used to store each widget's [Id], [Rect] and [Sense] each frame.
/// Used to check for overlaps between widgets when handling events.
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
    pub sense: Sense,

    /// Is the widget enabled?
    pub enabled: bool,
}

/// Stores the positions of all widgets generated during a single egui update/frame.
///
/// Actually, only those that are on screen.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct WidgetRects {
    /// All widgets, in painting order.
    pub by_layer: HashMap<LayerId, Vec<WidgetRect>>,

    /// All widgets
    pub by_id: IdMap<WidgetRect>,
}

impl WidgetRects {
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
        if !widget_rect.interact_rect.is_positive() {
            return;
        }

        let Self { by_layer, by_id } = self;

        let layer_widgets = by_layer.entry(layer_id).or_default();

        match by_id.entry(widget_rect.id) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                // A new widget
                entry.insert(widget_rect);
                layer_widgets.push(widget_rect);
            }
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                // e.g. calling `response.interact(…)` to add more interaction.
                let existing = entry.get_mut();
                existing.rect = existing.rect.union(widget_rect.rect);
                existing.interact_rect = existing.interact_rect.union(widget_rect.interact_rect);
                existing.sense |= widget_rect.sense;
                existing.enabled |= widget_rect.enabled;

                // Find the existing widget in this layer and update it:
                for previous in layer_widgets.iter_mut().rev() {
                    if previous.id == widget_rect.id {
                        *previous = *existing;
                        break;
                    }
                }
            }
        }
    }
}
