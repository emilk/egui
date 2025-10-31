use emath::{Rect, Vec2, vec2};

use crate::{InputOptions, Modifiers, MouseWheelUnit, TouchPhase};

/// The current state of scrolling.
///
/// There are two important types of scroll input deviced:
/// * Discreen scroll wheels on a mouse
/// * Smooth scroll input from a trackpad
///
/// Scroll wheels will usually fire one single scroll event,
/// so it is important that egui smooths it out over time.
///
/// On the contrary, trackpads usually provide smooth scroll input,
/// and with kinetic scrolling (which on Mac is implemented by the OS)
/// scroll events can arrive _after_ the user lets go of the trackpad.
///
/// In either case, we consider use to be scrolling until there is no more
/// scroll events expected.
///
/// This means there are a few different states we can be in:
/// * Not scrolling
/// * "Smooth scrolling" (low-pass filter of discreet scroll events)
/// * Trackpad-scrolling (we receive begin/end phases for these)
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not scrolling,
    Static,

    /// We're smoothing out previous scroll events
    Smoothing,

    // We're in-between [`TouchPhase::Start`] and [`TouchPhase::End`] of a trackpad scroll.
    InTouch,
}

#[derive(Clone, Debug)]
pub struct ScrollState {
    /// Are we currently in a scroll action?
    ///
    /// This may be true even if no scroll events came in this frame,
    /// but we are in a kinetic scroll or in a smoothed scroll.
    pub status: Status,

    /// The modifiers at the start of the scroll.
    pub modifiers: Modifiers,

    /// Time of the last scroll event.
    pub last_scroll_event: f64,

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

    /// Used for smoothing the scroll delta.
    pub unprocessed_scroll_delta: Vec2,

    /// Used for smoothing the scroll delta when zooming.
    pub unprocessed_scroll_delta_for_zoom: f32,

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

    pub smooth_scroll_delta_for_zoom: f32,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            status: Status::Static,
            modifiers: Default::default(),
            last_scroll_event: f64::NEG_INFINITY,
            raw_scroll_delta: Vec2::ZERO,
            unprocessed_scroll_delta: Vec2::ZERO,
            unprocessed_scroll_delta_for_zoom: 0.0,
            smooth_scroll_delta: Vec2::ZERO,
            smooth_scroll_delta_for_zoom: 0.0,
        }
    }
}

impl ScrollState {
    pub fn on_wheel_event(
        &mut self,
        viewport_rect: Rect,
        options: &InputOptions,
        time: f64,
        unit: MouseWheelUnit,
        delta: Vec2,
        phase: TouchPhase,
        latest_modifiers: Modifiers,
    ) {
        self.last_scroll_event = time;
        match phase {
            crate::TouchPhase::Start => {
                self.status = Status::InTouch;
                self.modifiers = latest_modifiers;
            }
            crate::TouchPhase::Move => {
                match self.status {
                    Status::Static => {
                        self.modifiers = latest_modifiers;
                        self.status = Status::Smoothing;
                    }
                    Status::Smoothing => {
                        self.modifiers = latest_modifiers;
                        self.status = Status::Smoothing;
                    }
                    Status::InTouch => {
                        // keep same modifiers
                        self.status = Status::InTouch;
                    }
                }

                let mut delta = match unit {
                    MouseWheelUnit::Point => delta,
                    MouseWheelUnit::Line => options.line_scroll_speed * delta,
                    MouseWheelUnit::Page => viewport_rect.height() * delta,
                };

                let is_horizontal = self
                    .modifiers
                    .matches_any(options.horizontal_scroll_modifier);
                let is_vertical = self.modifiers.matches_any(options.vertical_scroll_modifier);

                if is_horizontal && !is_vertical {
                    // Treat all scrolling as horizontal scrolling.
                    // Note: one Mac we already get horizontal scroll events when shift is down.
                    delta = vec2(delta.x + delta.y, 0.0);
                }
                if !is_horizontal && is_vertical {
                    // Treat all scrolling as vertical scrolling.
                    delta = vec2(0.0, delta.x + delta.y);
                }

                self.raw_scroll_delta += delta;

                // Mouse wheels often go very large steps.
                // A single notch on a logitech mouse wheel connected to a Macbook returns 14.0 raw_scroll_delta.
                // So we smooth it out over several frames for a nicer user experience when scrolling in egui.
                // BUT: if the user is using a nice smooth mac trackpad, we don't add smoothing,
                // because it adds latency.
                let is_smooth = self.status == Status::InTouch
                    || match unit {
                        MouseWheelUnit::Point => delta.length() < 8.0, // a bit arbitrary here
                        MouseWheelUnit::Line | MouseWheelUnit::Page => false,
                    };

                let is_zoom = self.modifiers.matches_any(options.zoom_modifier);

                #[expect(clippy::collapsible_else_if)]
                if is_zoom {
                    if is_smooth {
                        self.smooth_scroll_delta_for_zoom += delta.x + delta.y;
                    } else {
                        self.unprocessed_scroll_delta_for_zoom += delta.x + delta.y;
                    }
                } else {
                    if is_smooth {
                        self.smooth_scroll_delta += delta;
                    } else {
                        self.unprocessed_scroll_delta += delta;
                    }
                }
            }
            crate::TouchPhase::End | crate::TouchPhase::Cancel => {
                self.status = Status::Static;
                self.modifiers = Default::default();
                self.unprocessed_scroll_delta = Default::default();
                self.unprocessed_scroll_delta_for_zoom = Default::default();
                self.smooth_scroll_delta = Default::default();
                self.smooth_scroll_delta_for_zoom = Default::default();
            }
        }
    }

    pub fn end_frame(&mut self, time: f64, dt: f32) {
        let t = crate::emath::exponential_smooth_factor(0.90, 0.1, dt); // reach _% in _ seconds. TODO(emilk): parameterize

        if self.unprocessed_scroll_delta != Vec2::ZERO {
            for d in 0..2 {
                if self.unprocessed_scroll_delta[d].abs() < 1.0 {
                    self.smooth_scroll_delta[d] += self.unprocessed_scroll_delta[d];
                    self.unprocessed_scroll_delta[d] = 0.0;
                } else {
                    let applied = t * self.unprocessed_scroll_delta[d];
                    self.smooth_scroll_delta[d] += applied;
                    self.unprocessed_scroll_delta[d] -= applied;
                }
            }
        }

        {
            // Smooth scroll-to-zoom:
            if self.unprocessed_scroll_delta_for_zoom.abs() < 1.0 {
                self.smooth_scroll_delta_for_zoom += self.unprocessed_scroll_delta_for_zoom;
                self.unprocessed_scroll_delta_for_zoom = 0.0;
            } else {
                let applied = t * self.unprocessed_scroll_delta_for_zoom;
                self.smooth_scroll_delta_for_zoom += applied;
                self.unprocessed_scroll_delta_for_zoom -= applied;
            }
        }

        let time_since_last_scroll = time - self.last_scroll_event;

        if self.status == Status::Smoothing
            && self.smooth_scroll_delta == Vec2::ZERO
            && 0.150 < time_since_last_scroll
        {
            // On certain platforms, like web, we don't get the start & stop scrolling events, so
            // we rely on a timer there.
            //
            // Tested on a mac touchpad 2025, where the largest observed gap between scroll events
            // was 68 ms. But we add some margin to be safe
            self.status = Status::Static;
            self.modifiers = Default::default();
        }
    }

    /// True if there is an active scroll action that might scroll more when using [`Self::smooth_scroll_delta`].
    pub fn is_scrolling(&self) -> bool {
        self.status != Status::Static
    }

    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            status,
            modifiers,
            last_scroll_event,
            raw_scroll_delta,
            unprocessed_scroll_delta,
            unprocessed_scroll_delta_for_zoom,
            smooth_scroll_delta,
            smooth_scroll_delta_for_zoom,
        } = self;

        let time = ui.input(|i| i.time);

        crate::Grid::new("ScrollState")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("status");
                ui.monospace(format!("{status:?}"));
                ui.end_row();

                ui.label("modifiers");
                ui.monospace(format!("{modifiers:?}"));
                ui.end_row();

                ui.label("last_scroll_event");
                ui.monospace(format!("{:.1}s ago", time - *last_scroll_event));
                ui.end_row();

                ui.label("raw_scroll_delta");
                ui.monospace(raw_scroll_delta.to_string());
                ui.end_row();

                ui.label("unprocessed_scroll_delta");
                ui.monospace(unprocessed_scroll_delta.to_string());
                ui.end_row();

                ui.label("unprocessed_scroll_delta_for_zoom");
                ui.monospace(unprocessed_scroll_delta_for_zoom.to_string());
                ui.end_row();

                ui.label("smooth_scroll_delta");
                ui.monospace(smooth_scroll_delta.to_string());
                ui.end_row();

                ui.label("smooth_scroll_delta_for_zoom");
                ui.monospace(smooth_scroll_delta_for_zoom.to_string());
                ui.end_row();
            });
    }
}
