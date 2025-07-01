use egui::accesskit::ActionRequest;
use egui::mutex::Mutex;
use egui::{Modifiers, PointerButton, Pos2, accesskit};
use kittest::{AccessKitNode, NodeT, debug_fmt_node};
use std::fmt::{Debug, Formatter};

pub(crate) enum EventType {
    Event(egui::Event),
    Modifiers(Modifiers),
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
        self.queue.lock().push(EventType::Event(event));
    }

    fn modifiers(&self, modifiers: Modifiers) {
        self.queue.lock().push(EventType::Modifiers(modifiers));
    }

    pub fn hover(&self) {
        self.event(egui::Event::PointerMoved(self.rect().center()));
    }

    /// Click at the node center with the primary button.
    pub fn click(&self) {
        self.click_button(PointerButton::Primary);
    }

    #[deprecated = "Use `click()` instead."]
    pub fn simulate_click(&self) {
        self.click();
    }

    pub fn click_secondary(&self) {
        self.click_button(PointerButton::Secondary);
    }

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

    pub fn click_modifiers(&self, modifiers: Modifiers) {
        self.click_button_modifiers(PointerButton::Primary, modifiers);
    }

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
    pub fn click_accesskit(&self) {
        self.event(egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                target: self.accesskit_node.id(),
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

    pub fn focus(&self) {
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::Focus,
            target: self.accesskit_node.id(),
            data: None,
        }));
    }

    #[deprecated = "Use `Harness::key_down` instead."]
    pub fn key_down(&self, key: egui::Key) {
        self.event(egui::Event::Key {
            key,
            pressed: true,
            modifiers: Modifiers::default(),
            repeat: false,
            physical_key: None,
        });
    }

    #[deprecated = "Use `Harness::key_up` instead."]
    pub fn key_up(&self, key: egui::Key) {
        self.event(egui::Event::Key {
            key,
            pressed: false,
            modifiers: Modifiers::default(),
            repeat: false,
            physical_key: None,
        });
    }

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
    pub fn scroll_to_me(&self) {
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollIntoView,
            target: self.accesskit_node.id(),
            data: None,
        }));
    }

    /// Scroll the ScrollArea containing this node down.
    pub fn scroll_down(&self) {
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollDown,
            target: self.accesskit_node.id(),
            data: None,
        }));
    }

    /// Scroll the ScrollArea containing this node up.
    pub fn scroll_up(&self) {
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollUp,
            target: self.accesskit_node.id(),
            data: None,
        }));
    }

    /// Scroll the ScrollArea containing this node left.
    pub fn scroll_left(&self) {
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollLeft,
            target: self.accesskit_node.id(),
            data: None,
        }));
    }

    /// Scroll the ScrollArea containing this node right.
    pub fn scroll_right(&self) {
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollRight,
            target: self.accesskit_node.id(),
            data: None,
        }));
    }
}
