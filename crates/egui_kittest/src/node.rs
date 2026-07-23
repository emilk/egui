use egui::accesskit::ActionRequest;
use egui::mutex::Mutex;
use egui::{Modifiers, PointerButton, Pos2, accesskit};
use kittest::{AccessKitNode, NodeT, debug_fmt_node};
use std::fmt::{Debug, Formatter};

pub type EventQueue = Mutex<Vec<egui::Event>>;

#[derive(Clone, Copy)]
pub struct Node<'tree> {
    pub(crate) accesskit_node: AccessKitNode<'tree>,
    pub(crate) queue: &'tree EventQueue,
    pub(crate) pixels_per_point: f32,
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
        Self::new(child_node, self.queue, self.pixels_per_point)
    }
}

impl<'tree> Node<'tree> {
    /// Construct a new accesskit node
    pub fn new(
        accesskit_node: AccessKitNode<'tree>,
        queue: &'tree EventQueue,
        pixels_per_point: f32,
    ) -> Self {
        Self {
            accesskit_node,
            queue,
            pixels_per_point,
        }
    }

    fn event(&self, event: egui::Event) {
        self.queue.lock().push(event);
    }

    fn modifiers(&self, modifiers: Modifiers) {
        self.queue
            .lock()
            .push(egui::Event::ModifiersChanged(modifiers));
    }

    pub fn hover(&self) {
        self.event(egui::Event::PointerMoved(self.rect().center()));
    }

    /// Click at the node center with the primary button.
    pub fn click(&self) {
        self.click_button(PointerButton::Primary);
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

    /// This returns the rect in logical ui coordinates while the underlying [`accesskit::Node`] has it
    /// in physical screen coordinates.
    pub fn rect(&self) -> egui::Rect {
        let rect = self
            .accesskit_node
            .bounding_box()
            .expect("Every egui node should have a rect");
        let ppp = self.pixels_per_point;
        egui::Rect {
            min: Pos2::new(rect.x0 as f32 / ppp, rect.y0 as f32 / ppp),
            max: Pos2::new(rect.x1 as f32 / ppp, rect.y1 as f32 / ppp),
        }
    }

    pub fn focus(&self) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::Focus,
            target_node,
            target_tree,
            data: None,
        }));
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
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(ActionRequest {
            action: accesskit::Action::ScrollIntoView,
            target_node,
            target_tree,
            data: None,
        }));
    }

    /// Scroll the [`egui::ScrollArea`] containing this node down (100px).
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
