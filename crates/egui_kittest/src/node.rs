use crate::Harness;
use egui::accesskit::ActionRequest;
use egui::mutex::Mutex;
use egui::{accesskit, Modifiers, PointerButton, Pos2};
use kittest::{AccessKitNode, Key, NodeT};
use std::fmt::{Debug, Formatter};

pub type EventQueue = Mutex<Vec<egui::Event>>;

#[derive(Clone, Copy)]
pub struct Node<'tree> {
    pub(crate) accesskit_node: AccessKitNode<'tree>,
    pub(crate) queue: &'tree EventQueue,
}

impl Debug for Node<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<'tree> NodeT<'tree> for Node<'tree> {
    fn accesskit_node(&self) -> AccessKitNode<'tree> {
        self.accesskit_node
    }

    fn new_child(&self, child_node: AccessKitNode<'tree>) -> Self {
        Self {
            queue: self.queue,
            accesskit_node: child_node,
        }
    }
}

impl<'tree> Node<'tree> {
    fn event(&self, event: egui::Event) {
        self.queue.lock().push(event);
    }

    pub fn hover(&self) {
        self.event(egui::Event::PointerMoved(self.rect().center()))
    }

    pub fn click(&self) {
        self.click_button_modifiers(PointerButton::Primary, Modifiers::default());
    }

    #[deprecated = "Use `click()` instead."]
    pub fn simulate_click(&self) {
        self.click();
    }

    pub fn click_secondary(&self) {
        self.click_button_modifiers(PointerButton::Secondary, Modifiers::default());
    }

    pub fn click_button(&self, button: PointerButton) {
        self.click_button_modifiers(button, Modifiers::default());
    }

    pub fn click_modifiers(&self, modifiers: Modifiers) {
        self.click_button_modifiers(PointerButton::Primary, modifiers);
    }

    pub fn click_button_modifiers(&self, button: PointerButton, modifiers: Modifiers) {
        self.hover();
        for pressed in [true, false] {
            self.event(egui::Event::PointerButton {
                pos: self.rect().center(),
                button,
                pressed,
                modifiers,
            })
        }
    }

    pub fn click_accesskit(&self) {
        self.event(egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                target: self.accesskit_node.id(),
                action: accesskit::Action::Click,
                data: None,
            },
        ))
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
            modifiers: Modifiers::default(), // TODO: Handle modifiers
            repeat: false,
            physical_key: None,
        });
    }

    #[deprecated = "Use `Harness::key_up` instead."]
    pub fn key_up(&self, key: egui::Key) {
        self.event(egui::Event::Key {
            key,
            pressed: false,
            modifiers: Modifiers::default(), // TODO: Handle modifiers
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
}
