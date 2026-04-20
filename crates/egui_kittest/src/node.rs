use egui::accesskit::ActionRequest;
use egui::mutex::Mutex;
use egui::{Modifiers, PointerButton, Pos2, accesskit};
use kittest::{AccessKitNode, NodeT, debug_fmt_node};
use std::fmt::{Debug, Formatter};

/// Source-location info stashed alongside queued events. We store a runtime backtrace so the
/// inspector can walk *past* non-`#[track_caller]` helper functions and find the common
/// test-source file that all events came from. Zero-cost when the `inspector` feature is off.
#[cfg(feature = "inspector")]
pub(crate) type EventSite = Option<Box<backtrace::Backtrace>>;
#[cfg(not(feature = "inspector"))]
pub(crate) type EventSite = ();

/// Capture a backtrace at the call site. Unresolved so capture is cheap (~microseconds);
/// resolution happens lazily when we actually ship the frame to the inspector.
#[cfg(feature = "inspector")]
#[expect(clippy::unnecessary_wraps)] // Option<_> is the shape of EventSite by design.
pub(crate) fn capture_site() -> EventSite {
    Some(Box::new(backtrace::Backtrace::new_unresolved()))
}
#[cfg(not(feature = "inspector"))]
pub(crate) fn capture_site() -> EventSite {}

/// The "empty" value for an [`EventSite`] — used as a default when no location has been
/// captured yet (e.g. `Harness` construction). Zero-cost when the feature is off.
#[cfg(feature = "inspector")]
pub(crate) fn empty_site() -> EventSite {
    None
}
#[cfg(not(feature = "inspector"))]
pub(crate) fn empty_site() -> EventSite {}

pub(crate) enum EventType {
    Event(egui::Event, EventSite),
    Modifiers(Modifiers, EventSite),
}

pub(crate) type EventQueue = Mutex<Vec<EventType>>;

#[derive(Clone, Copy)]
pub struct Node<'tree> {
    pub(crate) accesskit_node: AccessKitNode<'tree>,
    pub(crate) queue: &'tree EventQueue,
}

impl Debug for Node<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        debug_fmt_node(self, f)
    }
}

impl<'tree> NodeT<'tree> for Node<'tree> {
    fn accesskit_node(&self) -> AccessKitNode<'tree> {
        self.accesskit_node
    }

    fn new_related(&self, child_node: AccessKitNode<'tree>) -> Self {
        Self {
            queue: self.queue,
            accesskit_node: child_node,
        }
    }
}

impl Node<'_> {
    fn event(&self, event: egui::Event) {
        self.queue
            .lock()
            .push(EventType::Event(event, capture_site()));
    }

    fn modifiers(&self, modifiers: Modifiers) {
        self.queue
            .lock()
            .push(EventType::Modifiers(modifiers, capture_site()));
    }

    #[track_caller]
    pub fn hover(&self) {
        self.event(egui::Event::PointerMoved(self.rect().center()));
    }

    /// Click at the node center with the primary button.
    #[track_caller]
    pub fn click(&self) {
        self.click_button(PointerButton::Primary);
    }

    #[track_caller]
    pub fn click_secondary(&self) {
        self.click_button(PointerButton::Secondary);
    }

    #[track_caller]
    pub fn click_button(&self, button: PointerButton) {
        self.hover();
        for pressed in [true, false] {
            self.event(egui::Event::PointerButton {
                pos: self.rect().center(),
                button,
                pressed,
                modifiers: Modifiers::default(),
            });
        }
    }

    #[track_caller]
    pub fn click_modifiers(&self, modifiers: Modifiers) {
        self.click_button_modifiers(PointerButton::Primary, modifiers);
    }

    #[track_caller]
    pub fn click_button_modifiers(&self, button: PointerButton, modifiers: Modifiers) {
        self.hover();
        self.modifiers(modifiers);
        for pressed in [true, false] {
            self.event(egui::Event::PointerButton {
                pos: self.rect().center(),
                button,
                pressed,
                modifiers,
            });
        }
        self.modifiers(Modifiers::default());
    }

    /// Click the node via accesskit.
    ///
    /// This will trigger a [`accesskit::Action::Click`] action.
    /// In contrast to `click()`, this can also click widgets that are not currently visible.
    #[track_caller]
    pub fn click_accesskit(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                target_node,
                target_tree,
                action: accesskit::Action::Click,
                data: None,
            },
        ));
    }

    pub fn rect(&self) -> egui::Rect {
        let rect = self
            .accesskit_node
            .bounding_box()
            .expect("Every egui node should have a rect");
        egui::Rect {
            min: Pos2::new(rect.x0 as f32, rect.y0 as f32),
            max: Pos2::new(rect.x1 as f32, rect.y1 as f32),
        }
    }

    #[track_caller]
    pub fn focus(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::Focus,
            target_node,
            target_tree,
            data: None,
        }));
    }

    #[track_caller]
    pub fn type_text(&self, text: &str) {
        self.event(egui::Event::Text(text.to_owned()));
    }

    pub fn value(&self) -> Option<String> {
        self.accesskit_node.value()
    }

    pub fn is_focused(&self) -> bool {
        self.accesskit_node.is_focused()
    }

    /// Scroll the node into view.
    #[track_caller]
    pub fn scroll_to_me(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollIntoView,
            target_node,
            target_tree,
            data: None,
        }));
    }

    /// Scroll the [`egui::ScrollArea`] containing this node down (100px).
    #[track_caller]
    pub fn scroll_down(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollDown,
            target_node,
            target_tree,
            data: None,
        }));
    }

    /// Scroll the [`egui::ScrollArea`] containing this node up (100px).
    #[track_caller]
    pub fn scroll_up(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollUp,
            target_node,
            target_tree,
            data: None,
        }));
    }

    /// Scroll the [`egui::ScrollArea`] containing this node left (100px).
    #[track_caller]
    pub fn scroll_left(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollLeft,
            target_node,
            target_tree,
            data: None,
        }));
    }

    /// Scroll the [`egui::ScrollArea`] containing this node right (100px).
    #[track_caller]
    pub fn scroll_right(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollRight,
            target_node,
            target_tree,
            data: None,
        }));
    }
}
