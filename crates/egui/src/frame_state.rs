use std::ops::RangeInclusive;

use crate::{id::IdSet, *};

#[derive(Clone, Copy, Debug)]
pub(crate) struct TooltipFrameState {
    pub common_id: Id,
    pub rect: Rect,
    pub count: usize,
}

#[cfg(feature = "accesskit")]
#[derive(Clone)]
pub(crate) struct AccessKitFrameState {
    pub(crate) node_builders: IdMap<accesskit::NodeBuilder>,
    pub(crate) parent_stack: Vec<Id>,
}

/// State that is collected during a frame and then cleared.
/// Short-term (single frame) memory.
#[derive(Clone)]
pub(crate) struct FrameState {
    /// All [`Id`]s that were used this frame.
    /// Used to debug [`Id`] clashes of widgets.
    pub(crate) used_ids: IdMap<Rect>,

    /// Starts off as the screen_rect, shrinks as panels are added.
    /// The [`CentralPanel`] does not change this.
    /// This is the area available to Window's.
    pub(crate) available_rect: Rect,

    /// Starts off as the screen_rect, shrinks as panels are added.
    /// The [`CentralPanel`] retracts from this.
    pub(crate) unused_rect: Rect,

    /// How much space is used by panels.
    pub(crate) used_by_panels: Rect,

    /// If a tooltip has been shown this frame, where was it?
    /// This is used to prevent multiple tooltips to cover each other.
    /// Initialized to `None` at the start of each frame.
    pub(crate) tooltip_state: Option<TooltipFrameState>,

    /// Set to [`InputState::scroll_delta`] on the start of each frame.
    ///
    /// Cleared by the first [`ScrollArea`] that makes use of it.
    pub(crate) scroll_delta: Vec2, // TODO(emilk): move to `InputState` ?

    /// horizontal, vertical
    pub(crate) scroll_target: [Option<(RangeInclusive<f32>, Option<Align>)>; 2],

    #[cfg(feature = "accesskit")]
    pub(crate) accesskit_state: Option<AccessKitFrameState>,

    /// Highlight these widgets this next frame. Read from this.
    pub(crate) highlight_this_frame: IdSet,

    /// Highlight these widgets the next frame. Write to this.
    pub(crate) highlight_next_frame: IdSet,
}

impl Default for FrameState {
    fn default() -> Self {
        Self {
            used_ids: Default::default(),
            available_rect: Rect::NAN,
            unused_rect: Rect::NAN,
            used_by_panels: Rect::NAN,
            tooltip_state: None,
            scroll_delta: Vec2::ZERO,
            scroll_target: [None, None],
            #[cfg(feature = "accesskit")]
            accesskit_state: None,
            highlight_this_frame: Default::default(),
            highlight_next_frame: Default::default(),
        }
    }
}

impl FrameState {
    pub(crate) fn begin_frame(&mut self, input: &InputState) {
        let Self {
            used_ids,
            available_rect,
            unused_rect,
            used_by_panels,
            tooltip_state,
            scroll_delta,
            scroll_target,
            #[cfg(feature = "accesskit")]
            accesskit_state,
            highlight_this_frame,
            highlight_next_frame,
        } = self;

        used_ids.clear();
        *available_rect = input.screen_rect();
        *unused_rect = input.screen_rect();
        *used_by_panels = Rect::NOTHING;
        *tooltip_state = None;
        *scroll_delta = input.scroll_delta;
        *scroll_target = [None, None];

        #[cfg(feature = "accesskit")]
        {
            *accesskit_state = None;
        }

        *highlight_this_frame = std::mem::take(highlight_next_frame);
    }

    /// How much space is still available after panels has been added.
    /// This is the "background" area, what egui doesn't cover with panels (but may cover with windows).
    /// This is also the area to which windows are constrained.
    pub(crate) fn available_rect(&self) -> Rect {
        crate::egui_assert!(
            self.available_rect.is_finite(),
            "Called `available_rect()` before `Context::run()`"
        );
        self.available_rect
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_left_panel(&mut self, panel_rect: Rect) {
        crate::egui_assert!(
            panel_rect.min.distance(self.available_rect.min) < 0.1,
            "Mismatching left panel. You must not create a panel from within another panel."
        );
        self.available_rect.min.x = panel_rect.max.x;
        self.unused_rect.min.x = panel_rect.max.x;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_right_panel(&mut self, panel_rect: Rect) {
        crate::egui_assert!(
            panel_rect.max.distance(self.available_rect.max) < 0.1,
            "Mismatching right panel. You must not create a panel from within another panel."
        );
        self.available_rect.max.x = panel_rect.min.x;
        self.unused_rect.max.x = panel_rect.min.x;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_top_panel(&mut self, panel_rect: Rect) {
        crate::egui_assert!(
            panel_rect.min.distance(self.available_rect.min) < 0.1,
            "Mismatching top panel. You must not create a panel from within another panel."
        );
        self.available_rect.min.y = panel_rect.max.y;
        self.unused_rect.min.y = panel_rect.max.y;
        self.used_by_panels = self.used_by_panels.union(panel_rect);
    }

    /// Shrink `available_rect`.
    pub(crate) fn allocate_bottom_panel(&mut self, panel_rect: Rect) {
        crate::egui_assert!(
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
