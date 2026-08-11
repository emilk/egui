use core::time::Duration;
use std::time::Instant;

use ahash::HashMap;
use winit::window::WindowId;

/// Detects redraws that were requested but never delivered.
///
/// On Wayland a compositor may withhold `wl_surface.frame` for a hidden
/// surface. winit then suppresses `RedrawRequested` until the callback
/// arrives, and the gate does not re-arm on its own, so `App::update` stops
/// forever. See <https://github.com/emilk/egui/issues/5136>.
pub struct FrameWatchdog {
    deadline: Duration,
    /// When each window's outstanding redraw was requested.
    pending: HashMap<WindowId, Instant>,
}

impl FrameWatchdog {
    pub fn new(deadline: Duration) -> Self {
        Self {
            deadline,
            pending: HashMap::default(),
        }
    }

    pub fn note_redraw_requested(&mut self, window_id: WindowId, now: Instant) {
        self.pending.entry(window_id).or_insert(now);
    }

    pub fn note_redraw_delivered(&mut self, window_id: WindowId) {
        self.forget(window_id);
    }

    pub fn forget(&mut self, window_id: WindowId) {
        self.pending.remove(&window_id);
    }

    /// Returns the windows whose redraw is overdue, re-arming each one so a
    /// lasting stall is reported again after every deadline.
    pub fn overdue(&mut self, now: Instant) -> Vec<WindowId> {
        let overdue: Vec<WindowId> = self
            .pending
            .iter()
            .filter(|(_, requested)| now.duration_since(**requested) >= self.deadline)
            .map(|(window_id, _)| *window_id)
            .collect();
        for window_id in &overdue {
            self.pending.insert(*window_id, now);
        }
        overdue
    }

    pub fn next_check(&self) -> Option<Instant> {
        self.pending
            .values()
            .min()
            .map(|requested| *requested + self.deadline)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        watchdog: FrameWatchdog,
        start: Instant,
    }

    impl Fixture {
        const DEADLINE: Duration = Duration::from_millis(500);

        fn new() -> Self {
            Self {
                watchdog: FrameWatchdog::new(Self::DEADLINE),
                start: Instant::now(),
            }
        }

        fn at(&self, millis: u64) -> Instant {
            self.start + Duration::from_millis(millis)
        }

        fn window() -> WindowId {
            WindowId::dummy()
        }

        fn overdue_at(&mut self, millis: u64) -> Vec<WindowId> {
            let now = self.at(millis);
            self.watchdog.overdue(now)
        }
    }

    #[test]
    fn reports_nothing_when_no_redraw_was_requested() {
        let mut fixture = Fixture::new();
        assert!(fixture.overdue_at(10_000).is_empty());
        assert_eq!(fixture.watchdog.next_check(), None);
    }

    #[test]
    fn reports_nothing_before_the_deadline() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(0));
        assert!(fixture.overdue_at(499).is_empty());
    }

    #[test]
    fn reports_the_window_once_the_deadline_passes() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(0));
        assert_eq!(fixture.overdue_at(500), vec![Fixture::window()]);
    }

    #[test]
    fn a_delivered_redraw_clears_the_pending_request() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(0));
        fixture.watchdog.note_redraw_delivered(Fixture::window());
        assert!(fixture.overdue_at(10_000).is_empty());
    }

    #[test]
    fn keeps_the_original_request_time_while_still_pending() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(0));
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(400));
        assert_eq!(fixture.overdue_at(500), vec![Fixture::window()]);
    }

    #[test]
    fn rearms_after_reporting_so_a_stall_repeats_at_the_deadline() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(0));

        assert_eq!(fixture.overdue_at(500), vec![Fixture::window()]);
        assert!(fixture.overdue_at(600).is_empty());
        assert_eq!(fixture.overdue_at(1000), vec![Fixture::window()]);
    }

    #[test]
    fn forget_drops_the_window() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(0));
        fixture.watchdog.forget(Fixture::window());
        assert!(fixture.overdue_at(10_000).is_empty());
    }

    #[test]
    fn next_check_is_the_earliest_deadline() {
        let mut fixture = Fixture::new();
        fixture
            .watchdog
            .note_redraw_requested(Fixture::window(), fixture.at(200));
        assert_eq!(fixture.watchdog.next_check(), Some(fixture.at(700)));
    }
}
