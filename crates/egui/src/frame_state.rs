use ahash::{HashMap, HashSet};

use crate::{id::IdSet, *};

/// Reset at the start of each frame.
#[derive(Clone, Debug, Default)]
pub struct TooltipFrameState {
    /// If a tooltip has been shown this frame, where was it?
    /// This is used to prevent multiple tooltips to cover each other.
    pub widget_tooltips: IdMap<PerWidgetTooltipState>,
}

impl TooltipFrameState {
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
    pub open_popups: HashSet<Id>,

    /// Which widget is showing a tooltip (if any)?
    ///
    /// Only one widget per layer may show a tooltip.
    /// But if a tooltip contains a tooltip, you can show a tooltip on top of a tooltip.
    pub widget_with_tooltip: Option<Id>,
}

#[cfg(feature = "accesskit")]
#[derive(Clone)]
pub struct AccessKitFrameState {
    pub node_builders: IdMap<accesskit::NodeBuilder>,
    pub parent_stack: Vec<Id>,
}

/// State that is collected during a frame, then saved for the next frame,
/// and then cleared.
///
/// One per viewport.
#[derive(Clone)]
pub struct FrameState {
    /// All [`Id`]s that were used this frame.
    pub used_ids: IdMap<Rect>,

    /// All widgets produced this frame.
    pub widgets: WidgetRects,

    /// Per-layer state.
    ///
    /// Not all layers registers themselves there though.
    pub layers: HashMap<LayerId, PerLayerState>,

    pub tooltips: TooltipFrameState,

    /// Starts off as the `screen_rect`, shrinks as panels are added.
    /// The [`CentralPanel`] does not change this.
    /// This is the area available to Window's.
    pub available_rect: Rect,

    /// Starts off as the `screen_rect`, shrinks as panels are added.
    /// The [`CentralPanel`] retracts from this.
    pub unused_rect: Rect,

    /// How much space is used by panels.
    pub used_by_panels: Rect,

    /// The current scroll area should scroll to this range (horizontal, vertical).
    pub scroll_target: [Option<(Rangef, Option<Align>)>; 2],

    /// The current scroll area should scroll by this much.
    ///
    /// The delta dictates how the _content_ should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    pub scroll_delta: Vec2,

    #[cfg(feature = "accesskit")]
    pub accesskit_state: Option<AccessKitFrameState>,

    /// Highlight these widgets the next frame.
    pub highlight_next_frame: IdSet,

    #[cfg(debug_assertions)]
    pub has_debug_viewed_this_frame: bool,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            used_ids: Default::default(),
            widgets: Default::default(),
            layers: Default::default(),
            tooltips: Default::default(),
            available_rect: Rect::NAN,
            unused_rect: Rect::NAN,
            used_by_panels: Rect::NAN,
            scroll_target: [None, None],
            scroll_delta: Vec2::default(),
            #[cfg(feature = "accesskit")]
            accesskit_state: None,
            highlight_next_frame: Default::default(),

            #[cfg(debug_assertions)]
            has_debug_viewed_this_frame: false,
        }
    }
}

impl FrameState {
    pub(crate) fn begin_frame(&mut self, screen_rect: Rect) {
        crate::profile_function!();
        let Self {
            used_ids,
            widgets,
            tooltips,
            layers,
            available_rect,
            unused_rect,
            used_by_panels,
            scroll_target,
            scroll_delta,
            #[cfg(feature = "accesskit")]
            accesskit_state,
            highlight_next_frame,

            #[cfg(debug_assertions)]
            has_debug_viewed_this_frame,
        } = self;

        used_ids.clear();
        widgets.clear();
        tooltips.clear();
        layers.clear();
        *available_rect = screen_rect;
        *unused_rect = screen_rect;
        *used_by_panels = Rect::NOTHING;
        *scroll_target = [None, None];
        *scroll_delta = Vec2::default();

        #[cfg(debug_assertions)]
        {
            *has_debug_viewed_this_frame = false;
        }

        #[cfg(feature = "accesskit")]
        {
            *accesskit_state = None;
        }

        highlight_next_frame.clear();
    }

    /// How much space is still available after panels has been added.
    /// This is the "background" area, what egui doesn't cover with panels (but may cover with windows).
    /// This is also the area to which windows are constrained.
    pub(crate) fn available_rect(&self) -> Rect {
        debug_assert!(
            self.available_rect.is_finite(),
            "Called `available_rect()` before `Context::run()`"
        );
        self.available_rect
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_left_panel(&mut self, panel_rect: Rect) {
        debug_assert!(
            panel_rect.min.distance(self.available_rect.min) < 0.1,
            "Mismatching left panel. You must not create a panel from within another panel."
        );
        self.available_rect.min.x = panel_rect.max.x;
        self.unused_rect.min.x = panel_rect.max.x;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_right_panel(&mut self, panel_rect: Rect) {
        debug_assert!(
            panel_rect.max.distance(self.available_rect.max) < 0.1,
            "Mismatching right panel. You must not create a panel from within another panel."
        );
        self.available_rect.max.x = panel_rect.min.x;
        self.unused_rect.max.x = panel_rect.min.x;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_top_panel(&mut self, panel_rect: Rect) {
        debug_assert!(
            panel_rect.min.distance(self.available_rect.min) < 0.1,
            "Mismatching top panel. You must not create a panel from within another panel."
        );
        self.available_rect.min.y = panel_rect.max.y;
        self.unused_rect.min.y = panel_rect.max.y;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_bottom_panel(&mut self, panel_rect: Rect) {
        debug_assert!(
            panel_rect.max.distance(self.available_rect.max) < 0.1,
            "Mismatching bottom panel. You must not create a panel from within another panel."
        );
        self.available_rect.max.y = panel_rect.min.y;
        self.unused_rect.max.y = panel_rect.min.y;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    pub(crate) fn allocate_central_panel(&mut self, panel_rect: Rect) {
        // Note: we do not shrink `available_rect`, because
        // we allow windows to cover the CentralPanel.
        self.unused_rect = Rect::NOTHING; // Nothing left unused after this
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }
}
