use emath::{Rect, Vec2, vec2};

use crate::{InputOptions, Modifiers, MouseWheelUnit, TouchPhase};

/// If there has been no scroll event for this many seconds, the scroll action is over.
///
/// Tested on a mac touchpad 2025, where the largest observed gap between scroll events
/// was 68 ms. But we add some margin to be safe.
const SCROLL_ACTION_TIMEOUT: f64 = 0.150;

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

    /// We're in-between [`TouchPhase::Start`] and [`TouchPhase::End`] of a trackpad scroll.
    ///
    /// The [`TouchPhase::End`] is not guaranteed to arrive (e.g. winit on Wayland sends
    /// [`TouchPhase::Start`] for some mouse wheels, but never [`TouchPhase::End`]),
    /// so this also ends if there are no scroll events for a while.
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
        if self.is_scroll_action_over(time) {
            // `after_events` only sees the timeout if a pass runs after it,
            // which is not the case if the app has been idle since the last scroll event.
            // Without this, an `InTouch` that never got its `TouchPhase::End`
            // would latch the modifiers of that old scroll action onto this new one.
            self.end_scroll_action();
        }

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
                        // If the user lets go of a modifier - ignore it.
                        // More kinematic scrolling may arrive.
                        // But if the users presses down new modifiers - heed it!
                        self.modifiers |= latest_modifiers;
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

        if self.smooth_wheel_delta == Vec2::ZERO && self.is_scroll_action_over(time) {
            self.end_scroll_action();
        }
    }

    /// Has it been so long since the last scroll event that the scroll action must be over?
    ///
    /// On certain platforms, like web, we don't get the start & stop scrolling events,
    /// and on others (e.g. some mouse wheels on Wayland) we get a start but no stop event,
    /// so we rely on a timer.
    fn is_scroll_action_over(&self, time: f64) -> bool {
        self.status != Status::Static && SCROLL_ACTION_TIMEOUT < time - self.last_wheel_event
    }

    fn end_scroll_action(&mut self) {
        self.status = Status::Static;
        self.modifiers = Default::default();
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

#[cfg(test)]
mod tests {
    use emath::{Vec2, vec2};

    use crate::{Context, Event, Modifiers, MouseWheelUnit, RawInput, TouchPhase};

    fn wheel(phase: TouchPhase, modifiers: Modifiers) -> Event {
        Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, -5.0),
            phase,
            modifiers,
        }
    }

    /// Runs one frame, and returns `(smooth_scroll_delta, zoom_delta, is_scrolling)`.
    fn run_frame(ctx: &Context, time: f64, events: Vec<Event>) -> (Vec2, f32, bool) {
        let input = RawInput {
            time: Some(time),
            events,
            ..Default::default()
        };
        let mut result = None;
        let output = ctx.run_ui(input, |ui| {
            // Only the first pass sees the events:
            if result.is_none() {
                result =
                    Some(ui.input(|i| (i.smooth_scroll_delta(), i.zoom_delta(), i.is_scrolling())));
            }
        });
        output.drop_without_applying_deltas();
        result.unwrap_or_default()
    }

    /// winit on Wayland sends `TouchPhase::Start` for some mouse wheels, but never `TouchPhase::End`.
    ///
    /// See <https://github.com/emilk/egui/issues/8325>.
    #[test]
    fn modifiers_should_not_stick_when_touch_phase_end_never_arrives() {
        let ctx = Context::default();

        let (scroll, zoom, _) = run_frame(
            &ctx,
            0.0,
            vec![
                wheel(TouchPhase::Start, Modifiers::COMMAND),
                wheel(TouchPhase::Move, Modifiers::COMMAND),
            ],
        );
        assert_eq!(scroll, Vec2::ZERO, "ctrl+scroll should not scroll");
        assert!(zoom != 1.0, "ctrl+scroll should zoom");

        // Let go of ctrl, and scroll again a while later.
        // No frame runs in-between, like in a reactive app that has been idle:
        let (scroll, zoom, _) =
            run_frame(&ctx, 1.0, vec![wheel(TouchPhase::Move, Modifiers::NONE)]);
        assert_eq!(zoom, 1.0, "scrolling without ctrl should not zoom");
        assert_eq!(
            scroll,
            vec2(0.0, -5.0),
            "scrolling without ctrl should scroll"
        );
    }

    #[test]
    fn scroll_action_should_end_when_touch_phase_end_never_arrives() {
        let ctx = Context::default();

        let (_, _, is_scrolling) = run_frame(
            &ctx,
            0.0,
            vec![
                wheel(TouchPhase::Start, Modifiers::NONE),
                wheel(TouchPhase::Move, Modifiers::NONE),
            ],
        );
        assert!(is_scrolling, "we just scrolled");

        // `is_scrolling` hides tooltips, so it must not stay `true` forever:
        let (_, _, is_scrolling) = run_frame(&ctx, 1.0, vec![]);
        assert!(
            !is_scrolling,
            "no scroll events for a second: the scroll action is over"
        );
    }

    /// Letting go of a modifier during a (momentum) scroll should not change what the scroll does.
    ///
    /// See <https://github.com/emilk/egui/pull/7678>.
    #[test]
    fn modifiers_should_stick_until_touch_phase_end() {
        let ctx = Context::default();

        let (scroll, _, _) = run_frame(
            &ctx,
            0.0,
            vec![
                wheel(TouchPhase::Start, Modifiers::SHIFT),
                wheel(TouchPhase::Move, Modifiers::SHIFT),
            ],
        );
        assert_eq!(
            scroll,
            vec2(-5.0, 0.0),
            "shift+scroll should scroll horizontally"
        );

        // Let go of shift while the scroll events keep coming:
        let (scroll, _, is_scrolling) =
            run_frame(&ctx, 0.05, vec![wheel(TouchPhase::Move, Modifiers::NONE)]);
        assert_eq!(scroll, vec2(-5.0, 0.0), "should still scroll horizontally");
        assert!(is_scrolling, "the scroll action is still going");

        let (_, _, is_scrolling) =
            run_frame(&ctx, 0.1, vec![wheel(TouchPhase::End, Modifiers::NONE)]);
        assert!(!is_scrolling, "the scroll action ended");

        let (scroll, _, _) = run_frame(
            &ctx,
            0.2,
            vec![
                wheel(TouchPhase::Start, Modifiers::NONE),
                wheel(TouchPhase::Move, Modifiers::NONE),
            ],
        );
        assert_eq!(
            scroll,
            vec2(0.0, -5.0),
            "a new scroll action without shift is vertical"
        );
    }
}
