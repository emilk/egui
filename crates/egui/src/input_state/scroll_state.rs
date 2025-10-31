use emath::Vec2;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct ScrollState {
    /// Time of the last scroll event.
    pub last_scroll_time: f64,

    /// If we are currently in a scroll action.
    ///
    /// This is not the same as checking if [`Self::smooth_scroll_delta`], or
    /// [`Self::raw_scroll_delta`] are zero. This instead relies on the
    /// current touch phase received from the mouse wheel event.
    ///
    /// This value is only `Some` if we have ever received a [`crate::TouchPhase::Start`] event and then
    /// know that the current platform supports it.
    pub is_in_scroll_action: Option<bool>,

    /// Used for smoothing the scroll delta.
    pub unprocessed_scroll_delta: Vec2,

    /// Used for smoothing the scroll delta when zooming.
    pub unprocessed_scroll_delta_for_zoom: f32,

    /// You probably want to use [`Self::smooth_scroll_delta`] instead.
    ///
    /// The raw input of how many points the user scrolled.
    ///
    /// The delta dictates how the _content_ should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    ///
    /// When using a notched scroll-wheel this will spike very large for one frame,
    /// then drop to zero. For a smoother experience, use [`Self::smooth_scroll_delta`].
    pub raw_scroll_delta: Vec2,

    /// How many points the user scrolled, smoothed over a few frames.
    ///
    /// The delta dictates how the _content_ should move.
    ///
    /// A positive X-value indicates the content is being moved right,
    /// as when swiping right on a touch-screen or track-pad with natural scrolling.
    ///
    /// A positive Y-value indicates the content is being moved down,
    /// as when swiping down on a touch-screen or track-pad with natural scrolling.
    ///
    /// [`crate::ScrollArea`] will both read and write to this field, so that
    /// at the end of the frame this will be zero if a scroll-area consumed the delta.
    pub smooth_scroll_delta: Vec2,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            is_in_scroll_action: None,
            last_scroll_time: f64::NEG_INFINITY,
            unprocessed_scroll_delta: Vec2::ZERO,
            unprocessed_scroll_delta_for_zoom: 0.0,
            raw_scroll_delta: Vec2::ZERO,
            smooth_scroll_delta: Vec2::ZERO,
        }
    }
}
