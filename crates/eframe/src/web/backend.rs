use std::collections::BTreeMap;

use egui::mutex::Mutex;

use super::percent_decode;
use crate::epi;

// ----------------------------------------------------------------------------

/// Data gathered between frames.
#[derive(Default)]
pub(crate) struct WebInput {
    /// Required because we don't get a position on touchend
    pub primary_touch: Option<egui::TouchId>,

    /// Helps to track the delta scale from gesture events
    pub accumulated_scale: f32,

    /// Helps to track the delta rotation from gesture events
    pub accumulated_rotation: f32,

    /// The raw input to `egui`.
    pub raw: egui::RawInput,
}

impl WebInput {
    pub fn new_frame(&mut self, canvas_size: egui::Vec2) -> egui::RawInput {
        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Default::default(), canvas_size)),
            time: Some(super::now_sec()),
            ..self.raw.take()
        };
        raw_input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(super::native_pixels_per_point());
        raw_input
    }

    /// On alt-tab, or user clicking another HTML element.
    pub fn set_focus(&mut self, focused: bool) {
        if self.raw.focused == focused {
            return;
        }

        // log::debug!("on_web_page_focus_change: {focused}");
        self.raw.modifiers = egui::Modifiers::default(); // Avoid sticky modifier keys on alt-tab:
        self.raw.focused = focused;
        self.raw.events.push(egui::Event::WindowFocused(focused));
        self.primary_touch = None;
    }
}

// ----------------------------------------------------------------------------

/// Stores when to do the next repaint.
pub(crate) struct NeedRepaint {
    /// Time in seconds when the next repaint should happen.
    next_repaint: Mutex<f64>,

    /// Rate limit for repaint. 0 means "unlimited". The rate may still be limited by vsync.
    max_fps: u32,
}

impl NeedRepaint {
    pub fn new(max_fps: Option<u32>) -> Self {
        Self {
            next_repaint: Mutex::new(f64::NEG_INFINITY), // start with a repaint
            max_fps: max_fps.unwrap_or(0),
        }
    }
}

impl NeedRepaint {
    /// Returns the time (in [`now_sec`] scale) when
    /// we should next repaint.
    pub fn when_to_repaint(&self) -> f64 {
        *self.next_repaint.lock()
    }

    /// Unschedule repainting.
    pub fn clear(&self) {
        *self.next_repaint.lock() = f64::INFINITY;
    }

    pub fn repaint_after(&self, num_seconds: f64) {
        let mut time = super::now_sec() + num_seconds;
        time = self.round_repaint_time_to_rate(time);
        let mut repaint_time = self.next_repaint.lock();
        *repaint_time = repaint_time.min(time);
    }

    /// Request a repaint. Depending on the presence of rate limiting, this may not be instant.
    pub fn repaint(&self) {
        let time = self.round_repaint_time_to_rate(super::now_sec());
        let mut repaint_time = self.next_repaint.lock();
        *repaint_time = repaint_time.min(time);
    }

    pub fn repaint_asap(&self) {
        *self.next_repaint.lock() = f64::NEG_INFINITY;
    }

    pub fn needs_repaint(&self) -> bool {
        self.when_to_repaint() <= super::now_sec()
    }

    fn round_repaint_time_to_rate(&self, time: f64) -> f64 {
        if self.max_fps == 0 {
            time
        } else {
            let interval = 1.0 / self.max_fps as f64;
            (time / interval).ceil() * interval
        }
    }
}

// ----------------------------------------------------------------------------

/// The User-Agent of the user's browser.
pub fn user_agent() -> Option<String> {
    web_sys::window()?.navigator().user_agent().ok()
}

/// Get the [`epi::Location`] from the browser.
pub fn web_location() -> epi::Location {
    let location = web_sys::window().unwrap().location();

    let hash = percent_decode(&location.hash().unwrap_or_default());

    let query = location
        .search()
        .unwrap_or_default()
        .strip_prefix('?')
        .unwrap_or_default()
        .to_owned();

    epi::Location {
        // TODO(emilk): should we really percent-decode the url? 🤷‍♂️
        url: percent_decode(&location.href().unwrap_or_default()),
        protocol: percent_decode(&location.protocol().unwrap_or_default()),
        host: percent_decode(&location.host().unwrap_or_default()),
        hostname: percent_decode(&location.hostname().unwrap_or_default()),
        port: percent_decode(&location.port().unwrap_or_default()),
        hash,
        query_map: parse_query_map(&query),
        query,
        origin: percent_decode(&location.origin().unwrap_or_default()),
    }
}

/// query is percent-encoded
fn parse_query_map(query: &str) -> BTreeMap<String, Vec<String>> {
    let mut map: BTreeMap<String, Vec<String>> = Default::default();

    for pair in query.split('&') {
        if !pair.is_empty() {
            if let Some((key, value)) = pair.split_once('=') {
                map.entry(percent_decode(key))
                    .or_default()
                    .push(percent_decode(value));
            } else {
                map.entry(percent_decode(pair))
                    .or_default()
                    .push(String::new());
            }
        }
    }

    map
}

// TODO(emilk): this test is never acgtually run, because this whole module is wasm32 only 🤦‍♂️
#[test]
fn test_parse_query() {
    assert_eq!(parse_query_map(""), BTreeMap::default());
    assert_eq!(parse_query_map("foo"), BTreeMap::from_iter([("foo", "")]));
    assert_eq!(
        parse_query_map("foo=bar"),
        BTreeMap::from_iter([("foo", "bar")])
    );
    assert_eq!(
        parse_query_map("foo=bar&baz=42"),
        BTreeMap::from_iter([("foo", "bar"), ("baz", "42")])
    );
    assert_eq!(
        parse_query_map("foo&baz=42"),
        BTreeMap::from_iter([("foo", ""), ("baz", "42")])
    );
    assert_eq!(
        parse_query_map("foo&baz&&"),
        BTreeMap::from_iter([("foo", ""), ("baz", "")])
    );
    assert_eq!(
        parse_query_map("badger=data.rrd%3Fparam1%3Dfoo%26param2%3Dbar&mushroom=snake"),
        BTreeMap::from_iter([
            ("badger", "data.rrd?param1=foo&param2=bar"),
            ("mushroom", "snake")
        ])
    );
}
