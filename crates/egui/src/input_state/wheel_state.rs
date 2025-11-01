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

/// Keeps track of wheel (scroll) input.
#[derive(Clone, Debug)]
pub struct WheelState {
    /// Are we currently in a scroll action?
    ///
    /// This may be true even if no scroll events came in this frame,
    /// but we are in a kinetic scroll or in a smoothed scroll.
    pub status: Status,

    /// The modifiers at the start of the scroll.
    pub modifiers: Modifiers,

    /// Time of the last scroll event.
    pub last_wheel_event: f64,

    /// Used for smoothing the scroll delta.
    pub unprocessed_wheel_delta: Vec2,

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
    pub smooth_wheel_delta: Vec2,
}

impl Default for WheelState {
    fn default() -> Self {
        Self {
            status: Status::Static,
            modifiers: Default::default(),
            last_wheel_event: f64::NEG_INFINITY,
            unprocessed_wheel_delta: Vec2::ZERO,
            smooth_wheel_delta: Vec2::ZERO,
        }
    }
}

impl WheelState {
    #[expect(clippy::too_many_arguments)]
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
        self.last_wheel_event = time;
        match phase {
            crate::TouchPhase::Start => {
                self.status = Status::InTouch;
                self.modifiers = latest_modifiers;
            }
            crate::TouchPhase::Move => {
                match self.status {
                    Status::Static | Status::Smoothing => {
                        self.modifiers = latest_modifiers;
                        self.status = Status::Smoothing;
                    }
                    Status::InTouch => {
                        // keep same modifiers and status
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

                // Mouse wheels often go very large steps.
                // A single notch on a logitech mouse wheel connected to a Macbook returns 14.0 raw scroll delta.
                // So we smooth it out over several frames for a nicer user experience when scrolling in egui.
                // BUT: if the user is using a nice smooth mac trackpad, we don't add smoothing,
                // because it adds latency.
                let is_smooth = self.status == Status::InTouch
                    || match unit {
                        MouseWheelUnit::Point => delta.length() < 8.0, // a bit arbitrary here
                        MouseWheelUnit::Line | MouseWheelUnit::Page => false,
                    };

                if is_smooth {
                    self.smooth_wheel_delta += delta;
                } else {
                    self.unprocessed_wheel_delta += delta;
                }
            }
            crate::TouchPhase::End | crate::TouchPhase::Cancel => {
                self.status = Status::Static;
                self.modifiers = Default::default();
                self.unprocessed_wheel_delta = Default::default();
                self.smooth_wheel_delta = Default::default();
            }
        }
    }

    pub fn after_events(&mut self, time: f64, dt: f32) {
        let t = crate::emath::exponential_smooth_factor(0.90, 0.1, dt); // reach _% in _ seconds. TODO(emilk): parameterize

        if self.unprocessed_wheel_delta != Vec2::ZERO {
            for d in 0..2 {
                if self.unprocessed_wheel_delta[d].abs() < 1.0 {
                    self.smooth_wheel_delta[d] += self.unprocessed_wheel_delta[d];
                    self.unprocessed_wheel_delta[d] = 0.0;
                } else {
                    let applied = t * self.unprocessed_wheel_delta[d];
                    self.smooth_wheel_delta[d] += applied;
                    self.unprocessed_wheel_delta[d] -= applied;
                }
            }
        }

        let time_since_last_scroll = time - self.last_wheel_event;

        if self.status == Status::Smoothing
            && self.smooth_wheel_delta == Vec2::ZERO
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

    /// True if there is an active scroll action that might scroll more when using [`Self::smooth_wheel_delta`].
    pub fn is_scrolling(&self) -> bool {
        self.status != Status::Static
    }

    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            status,
            modifiers,
            last_wheel_event,
            unprocessed_wheel_delta,
            smooth_wheel_delta,
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

                ui.label("last_wheel_event");
                ui.monospace(format!("{:.1}s ago", time - *last_wheel_event));
                ui.end_row();

                ui.label("unprocessed_wheel_delta");
                ui.monospace(unprocessed_wheel_delta.to_string());
                ui.end_row();

                ui.label("smooth_wheel_delta");
                ui.monospace(smooth_wheel_delta.to_string());
                ui.end_row();
            });
    }
}
