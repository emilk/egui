use emath::TSTransform;

use crate::{text::Selection, Context, Galley, Id};

/// Update accesskit with the current text state.
pub fn update_accesskit_for_text_widget(
    ctx: &Context,
    widget_id: Id,
    selection: Option<Selection>,
    role: accesskit::Role,
    global_from_galley: TSTransform,
    galley: &Galley,
) {
    let map_id = |parent_id: Id, node_id: accesskit::NodeId| parent_id.with(node_id.0);
    let parent_id = ctx.accesskit_node_builder(widget_id, |builder| {
        let parent_id = widget_id;

        if let Some(mut selection) = selection
            .as_ref()
            .and_then(|selection| galley.selection(|s| s.to_accesskit_selection(selection)))
        {
            selection.anchor.node = map_id(parent_id, selection.anchor.node).accesskit_id();
            selection.focus.node = map_id(parent_id, selection.focus.node).accesskit_id();
            builder.set_text_selection(selection);
        }

        builder.set_role(role);

        parent_id
    });

    let Some(parent_id) = parent_id else {
        return;
    };

    // TODO(valadaptive): mostly untested
    ctx.with_accessibility_parent(parent_id, || {
        for (node_id, node) in &galley.accessibility().nodes {
            let row_id = map_id(parent_id, *node_id);
            ctx.accesskit_node_builder(row_id, |dest_node| {
                *dest_node = node.clone();
                // Transform the node bounds
                if let Some(bounds) = node.bounds() {
                    let new_bounds = global_from_galley
                        * emath::Rect {
                            min: emath::Pos2::new(bounds.x0 as f32, bounds.y0 as f32),
                            max: emath::Pos2::new(bounds.x1 as f32, bounds.y1 as f32),
                        };
                    dest_node.set_bounds(accesskit::Rect {
                        x0: new_bounds.min.x.into(),
                        y0: new_bounds.min.y.into(),
                        x1: new_bounds.max.x.into(),
                        y1: new_bounds.max.y.into(),
                    });
                }
            });
        }
    });
}
