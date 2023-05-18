use std::collections::BTreeMap;

use egui::mutex::Mutex;

use crate::epi;

use super::percent_decode;

// ----------------------------------------------------------------------------

/// Data gathered between frames.
#[derive(Default)]
pub struct WebInput {
    /// Required because we don't get a position on touched
    pub latest_touch_pos: Option<egui::Pos2>,

    /// Required to maintain a stable touch position for multi-touch gestures.
    pub latest_touch_pos_id: Option<egui::TouchId>,

    pub raw: egui::RawInput,
}

impl WebInput {
    pub fn new_frame(&mut self, canvas_size: egui::Vec2) -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(Default::default(), canvas_size)),
            pixels_per_point: Some(super::native_pixels_per_point()), // We ALWAYS use the native pixels-per-point
            time: Some(super::now_sec()),
            ..self.raw.take()
        }
    }

    pub fn on_web_page_focus_change(&mut self, focused: bool) {
        self.raw.modifiers = egui::Modifiers::default();
        self.raw.focused = focused;
        self.raw.events.push(egui::Event::WindowFocused(focused));
        self.latest_touch_pos = None;
        self.latest_touch_pos_id = None;
    }
}

// ----------------------------------------------------------------------------

use std::sync::atomic::Ordering::SeqCst;

/// Stores when to do the next repaint.
pub struct NeedRepaint(Mutex<f64>);

impl Default for NeedRepaint {
    fn default() -> Self {
        Self(Mutex::new(f64::NEG_INFINITY)) // start with a repaint
    }
}

impl NeedRepaint {
    /// Returns the time (in [`now_sec`] scale) when
    /// we should next repaint.
    pub fn when_to_repaint(&self) -> f64 {
        *self.0.lock()
    }

    /// Unschedule repainting.
    pub fn clear(&self) {
        *self.0.lock() = f64::INFINITY;
    }

    pub fn repaint_after(&self, num_seconds: f64) {
        let mut repaint_time = self.0.lock();
        *repaint_time = repaint_time.min(super::now_sec() + num_seconds);
    }

    pub fn repaint_asap(&self) {
        *self.0.lock() = f64::NEG_INFINITY;
    }
}

pub struct IsDestroyed(std::sync::atomic::AtomicBool);

impl Default for IsDestroyed {
    fn default() -> Self {
        Self(false.into())
    }
}

impl IsDestroyed {
    pub fn fetch(&self) -> bool {
        self.0.load(SeqCst)
    }

    pub fn set_true(&self) {
        self.0.store(true, SeqCst);
    }
}

// ----------------------------------------------------------------------------

pub fn user_agent() -> Option<String> {
    web_sys::window()?.navigator().user_agent().ok()
}

pub fn web_location() -> epi::Location {
    let location = web_sys::window().unwrap().location();

    let hash = percent_decode(&location.hash().unwrap_or_default());

    let query = location
        .search()
        .unwrap_or_default()
        .strip_prefix('?')
        .map(percent_decode)
        .unwrap_or_default();

    let query_map = parse_query_map(&query)
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();

    epi::Location {
        url: percent_decode(&location.href().unwrap_or_default()),
        protocol: percent_decode(&location.protocol().unwrap_or_default()),
        host: percent_decode(&location.host().unwrap_or_default()),
        hostname: percent_decode(&location.hostname().unwrap_or_default()),
        port: percent_decode(&location.port().unwrap_or_default()),
        hash,
        query,
        query_map,
        origin: percent_decode(&location.origin().unwrap_or_default()),
    }
}

fn parse_query_map(query: &str) -> BTreeMap<&str, &str> {
    query
        .split('&')
        .filter_map(|pair| {
            if pair.is_empty() {
                None
            } else {
                Some(if let Some((key, value)) = pair.split_once('=') {
                    (key, value)
                } else {
                    (pair, "")
                })
            }
        })
        .collect()
}

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
}
