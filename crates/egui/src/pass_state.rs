use std::collections::HashSet;

use ahash::HashMap;

use crate::{Align, Id, IdMap, LayerId, Rangef, Rect, Vec2, WidgetRects, id::IdSet, style};

#[cfg(debug_assertions)]
use crate::{Align2, Color32, FontId, NumExt as _, Painter, pos2};

/// Reset at the start of each frame.
#[derive(Clone, Debug, Default)]
pub struct TooltipPassState {
    /// If a tooltip has been shown this frame, where was it?
    /// This is used to prevent multiple tooltips to cover each other.
    pub widget_tooltips: IdMap<PerWidgetTooltipState>,
}

impl TooltipPassState {
    pub fn clear(&mut self) {
        let Self { widget_tooltips } = self;
        widget_tooltips.clear();
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PerWidgetTooltipState {
    /// Bounding rectangle for all widget and all previous tooltips.
    pub bounding_rect: Rect,

    /// How many tooltips have been shown for this widget this frame?
    pub tooltip_count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct PerLayerState {
    /// Is there any open popup (menus, combo-boxes, etc)?
    ///
    /// Does NOT include tooltips.
    pub open_popups: IdSet,

    /// Which widget is showing a tooltip (if any)?
    ///
    /// Only one widget per layer may show a tooltip.
    /// But if a tooltip contains a tooltip, you can show a tooltip on top of a tooltip.
    pub widget_with_tooltip: Option<Id>,
}

#[derive(Clone, Debug)]
pub struct ScrollTarget {
    // The range that the scroll area should scroll to.
    pub range: Rangef,

    /// How should we align the rect within the visible area?
    /// If `align` is [`Align::TOP`] it means "put the top of the rect at the top of the scroll area", etc.
    /// If `align` is `None`, it'll scroll enough to bring the UI into view.
    pub align: Option<Align>,

    /// How should the scroll be animated?
    pub animation: style::ScrollAnimation,
}

impl ScrollTarget {
    pub fn new(range: Rangef, align: Option<Align>, animation: style::ScrollAnimation) -> Self {
        Self {
            range,
            align,
            animation,
        }
    }
}

#[derive(Clone)]
pub struct AccessKitPassState {
    pub nodes: IdMap<accesskit::Node>,
    pub parent_map: IdMap<Id>,

    /// Nodes belonging to explicitly invisible UI (not merely clipped or discarded).
    pub excluded: IdSet,
}

impl AccessKitPassState {
    /// Remove explicitly invisible nodes and repair references to the remaining tree.
    pub fn prune_excluded_nodes(&mut self) {
        if self.excluded.is_empty() {
            return;
        }

        // Propagate exclusion to descendants registered before an ancestor was excluded.
        loop {
            let num_excluded_before = self.excluded.len();
            #[expect(clippy::iter_over_hash_type)] // iterating over hash map to populate a hashmap
            for (&id, parent) in &self.parent_map {
                if self.excluded.contains(parent) {
                    self.excluded.insert(id);
                }
            }
            // Repeat until no new exclusions are added, so the result covers every descendant regardless of hash-map iteration order.
            if self.excluded.len() == num_excluded_before {
                break;
            }
        }

        // Remove excluded nodes and all references to them.
        self.nodes.retain(|id, _| !self.excluded.contains(id));
        let node_ids = self.nodes.keys().map(|id| id.accesskit_id()).collect();
        #[expect(clippy::iter_over_hash_type)] // Every node is updated independently/
        for node in self.nodes.values_mut() {
            prune_accesskit_references(node, &node_ids);
        }
    }
}

/// Keep all relationships, including those supplied by custom widgets, within the tree.
fn prune_accesskit_references(
    node: &mut accesskit::Node,
    valid_node_ids: &HashSet<accesskit::NodeId>,
) {
    fn retain(
        ids: &[accesskit::NodeId],
        valid_node_ids: &HashSet<accesskit::NodeId>,
    ) -> Vec<accesskit::NodeId> {
        ids.iter()
            .copied()
            .filter(|id| valid_node_ids.contains(id))
            .collect()
    }

    node.set_children(retain(node.children(), valid_node_ids));
    node.set_controls(retain(node.controls(), valid_node_ids));
    node.set_details(retain(node.details(), valid_node_ids));
    node.set_described_by(retain(node.described_by(), valid_node_ids));
    node.set_flow_to(retain(node.flow_to(), valid_node_ids));
    node.set_labelled_by(retain(node.labelled_by(), valid_node_ids));
    node.set_owns(retain(node.owns(), valid_node_ids));
    node.set_radio_group(retain(node.radio_group(), valid_node_ids));

    let is_missing = |id: accesskit::NodeId| !valid_node_ids.contains(&id);

    if node.active_descendant().is_some_and(is_missing) {
        node.clear_active_descendant();
    }
    if node.error_message().is_some_and(is_missing) {
        node.clear_error_message();
    }
    if node.in_page_link_target().is_some_and(is_missing) {
        node.clear_in_page_link_target();
    }
    if node.member_of().is_some_and(is_missing) {
        node.clear_member_of();
    }
    if node.next_on_line().is_some_and(is_missing) {
        node.clear_next_on_line();
    }
    if node.previous_on_line().is_some_and(is_missing) {
        node.clear_previous_on_line();
    }
    if node.popup_for().is_some_and(is_missing) {
        node.clear_popup_for();
    }

    // A selection is only meaningful if both endpoints survive pruning.
    if node.text_selection().is_some_and(|selection| {
        is_missing(selection.anchor.node) || is_missing(selection.focus.node)
    }) {
        node.clear_text_selection();
    }
}

#[cfg(debug_assertions)]
#[derive(Clone)]
pub struct DebugRect {
    pub rect: Rect,
    pub callstack: String,
    pub is_clicking: bool,
}

#[cfg(debug_assertions)]
impl DebugRect {
    pub fn paint(self, painter: &Painter) {
        let Self {
            rect,
            callstack,
            is_clicking,
        } = self;

        let ctx = painter.ctx();

        // Paint rectangle around widget:
        {
            // Print width and height:
            let text_color = if ctx.global_style().visuals.dark_mode {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            painter.debug_text(
                rect.left_center() + 2.0 * Vec2::LEFT,
                Align2::RIGHT_CENTER,
                text_color,
                format!("H: {:.1}", rect.height()),
            );
            painter.debug_text(
                rect.center_top(),
                Align2::CENTER_BOTTOM,
                text_color,
                format!("W: {:.1}", rect.width()),
            );

            // Paint rect:
            let rect_fg_color = if is_clicking {
                Color32::WHITE
            } else {
                Color32::LIGHT_BLUE
            };
            let rect_bg_color = Color32::BLUE.gamma_multiply(0.5);
            painter.rect(
                rect,
                0.0,
                rect_bg_color,
                (1.0, rect_fg_color),
                crate::StrokeKind::Outside,
            );
        }

        if !callstack.is_empty() {
            let font_id = FontId::monospace(12.0);
            let text = format!("{callstack}\n\n(click to copy)");
            let text_color = Color32::WHITE;
            let galley = painter.layout_no_wrap(text, font_id, text_color);

            // Position the text either under or above:
            let content_rect = ctx.content_rect();
            let y = if galley.size().y <= rect.top() {
                // Above
                rect.top() - galley.size().y - 16.0
            } else {
                // Below
                rect.bottom()
            };

            let y = y
                .at_most(content_rect.bottom() - galley.size().y)
                .at_least(0.0);

            let x = rect
                .left()
                .at_most(content_rect.right() - galley.size().x)
                .at_least(0.0);
            let text_pos = pos2(x, y);

            let text_bg_color = Color32::from_black_alpha(180);
            let text_rect_stroke_color = if is_clicking {
                Color32::WHITE
            } else {
                text_bg_color
            };
            let text_rect = Rect::from_min_size(text_pos, galley.size());
            painter.rect(
                text_rect,
                0.0,
                text_bg_color,
                (1.0, text_rect_stroke_color),
                crate::StrokeKind::Middle,
            );
            painter.galley(text_pos, galley, text_color);

            if is_clicking {
                ctx.copy_text(callstack);
            }
        }
    }
}

/// State that is collected during a pass, then saved for the next pass,
/// and then cleared.
///
/// (NOTE: we usually run only one pass per frame).
///
/// One per viewport.
#[derive(Clone)]
pub struct PassState {
    /// All [`Id`]s that were used this pass.
    pub used_ids: IdMap<Rect>,

    /// All widgets produced this pass.
    pub widgets: WidgetRects,

    /// Per-layer state.
    ///
    /// Not all layers registers themselves there though.
    pub layers: HashMap<LayerId, PerLayerState>,

    pub tooltips: TooltipPassState,

    /// What the root UI had available at the end of the previous pass.
    ///
    /// Only set if [`crate::Context::run_ui`] or [`crate::Context::root_ui`] has been called.
    pub root_ui_available_rect: Option<Rect>,

    /// What the root UI had used at the end of the previous pass.
    ///
    /// Only set if [`crate::Context::run_ui`] or [`crate::Context::root_ui`] has been called.
    pub root_ui_min_rect: Option<Rect>,

    /// The current scroll area should scroll to this range (horizontal, vertical).
    pub scroll_target: [Option<ScrollTarget>; 2],

    /// The current scroll area should scroll by this much.
    ///
    /// The delta dictates how the _content_ should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    pub scroll_delta: (Vec2, style::ScrollAnimation),

    pub accesskit_state: Option<AccessKitPassState>,

    /// Highlight these widgets the next pass.
    pub highlight_next_pass: IdSet,

    #[cfg(debug_assertions)]
    pub debug_rect: Option<DebugRect>,
}

impl Default for PassState {
    fn default() -> Self {
        Self {
            used_ids: Default::default(),
            widgets: Default::default(),
            layers: Default::default(),
            tooltips: Default::default(),
            root_ui_available_rect: None,
            root_ui_min_rect: None,
            scroll_target: [None, None],
            scroll_delta: (Vec2::default(), style::ScrollAnimation::none()),
            accesskit_state: None,
            highlight_next_pass: Default::default(),

            #[cfg(debug_assertions)]
            debug_rect: None,
        }
    }
}

impl PassState {
    pub(crate) fn begin_pass(&mut self) {
        profiling::function_scope!();
        let Self {
            used_ids,
            widgets,
            tooltips,
            layers,
            root_ui_available_rect,
            root_ui_min_rect,
            scroll_target,
            scroll_delta,
            accesskit_state,
            highlight_next_pass,

            #[cfg(debug_assertions)]
            debug_rect,
        } = self;

        used_ids.clear();
        widgets.clear();
        tooltips.clear();
        layers.clear();
        *root_ui_available_rect = None;
        *root_ui_min_rect = None;
        *scroll_target = [None, None];
        *scroll_delta = Default::default();

        #[cfg(debug_assertions)]
        {
            *debug_rect = None;
        }

        *accesskit_state = None;

        highlight_next_pass.clear();
    }
}
