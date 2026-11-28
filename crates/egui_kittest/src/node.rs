use core::fmt::{Debug, Formatter};
use egui::accesskit::ActionRequest;
use egui::mutex::Mutex;
use egui::{Modifiers, PointerButton, Pos2, accesskit};
use kittest::{AccessKitNode, NodeT, debug_fmt_node};

/// An event queued on the [`Harness`](crate::Harness), to be processed in a frame of its own.
pub struct QueuedEvent {
    /// The event to process.
    pub event: egui::Event,

    /// If set, the frame processing this event advances time by this much instead of
    /// [`HarnessBuilder::with_step_dt`](crate::HarnessBuilder::with_step_dt).
    ///
    /// [`Node::double_click`] uses this to keep its clicks within egui's double-click window
    /// ([`egui::InputOptions::max_double_click_delay`], 0.3 s by default).
    pub dt: Option<f32>,
}

impl QueuedEvent {
    /// Queue a plain event, to be processed with the harness
    /// [`step_dt`](crate::HarnessBuilder::with_step_dt).
    pub fn new(event: egui::Event) -> Self {
        Self { event, dt: None }
    }
}

pub type EventQueue = Mutex<Vec<QueuedEvent>>;

#[derive(Clone, Copy)]
pub struct Node<'tree> {
    pub(crate) accesskit_node: AccessKitNode<'tree>,
    pub(crate) queue: &'tree EventQueue,
    pub(crate) pixels_per_point: f32,
}

impl Debug for Node<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
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
        self.queue.lock().push(QueuedEvent::new(event));
    }

    fn modifiers(&self, modifiers: Modifiers) {
        self.queue
            .lock()
            .push(QueuedEvent::new(egui::Event::ModifiersChanged(modifiers)));
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

    /// Double-click at the node center with the primary button.
    ///
    /// The clicks are queued close together in time, so that they register as a double click
    /// ([`egui::Response::double_clicked`]) even with the default
    /// [`step_dt`](crate::HarnessBuilder::with_step_dt): each queued event is processed in a
    /// frame of its own, so two separate [`Node::click`]s would land 0.75 s apart, which is
    /// further apart than egui's double-click window.
    pub fn double_click(&self) {
        self.click_repeatedly(PointerButton::Primary, 2);
    }

    /// Triple-click at the node center with the primary button.
    ///
    /// See [`Node::double_click`].
    pub fn triple_click(&self) {
        self.click_repeatedly(PointerButton::Primary, 3);
    }

    /// Click the node `clicks` times in quick succession with the given button.
    ///
    /// The clicks are queued with a short time step in between, so that they land within
    /// egui's double-click window ([`egui::InputOptions::max_double_click_delay`]).
    fn click_repeatedly(&self, button: PointerButton, clicks: usize) {
        /// Time step used for the frames of [`Node::double_click`] and [`Node::triple_click`].
        ///
        /// Small enough that consecutive clicks land well within egui's double-click window,
        /// even though each queued event is processed in a frame of its own.
        const CLICK_DT: f32 = 0.05;

        let pos = self.rect().center();
        self.queue.lock().push(QueuedEvent {
            event: egui::Event::PointerMoved(pos),
            dt: Some(CLICK_DT),
        });
        for _ in 0..clicks {
            for pressed in [true, false] {
                self.queue.lock().push(QueuedEvent {
                    event: egui::Event::PointerButton {
                        pos,
                        button,
                        pressed,
                        modifiers: Modifiers::default(),
                    },
                    dt: Some(CLICK_DT),
                });
            }
        }
    }

    /// Click the node via accesskit.
    ///
    /// This will trigger a [`accesskit::Action::Click`] action.
    /// In contrast to `click()`, this can also click widgets that are not currently visible.
    pub fn click_accesskit(&self) {
        self.do_accesskit_action(accesskit::Action::Click, None);
    }

    /// Send an [`accesskit::Action::Increment`] action to the node.
    ///
    /// This is how assistive technologies increment widgets
    /// like sliders or panel resize handles.
    pub fn increment_accesskit(&self) {
        self.do_accesskit_action(accesskit::Action::Increment, None);
    }

    /// Send an [`accesskit::Action::Decrement`] action to the node.
    ///
    /// This is how assistive technologies decrement widgets
    /// like sliders or panel resize handles.
    pub fn decrement_accesskit(&self) {
        self.do_accesskit_action(accesskit::Action::Decrement, None);
    }

    /// Send an [`accesskit::Action::SetValue`] action with a numeric value to the node.
    ///
    /// This is how assistive technologies set the value of widgets
    /// like sliders or panel resize handles.
    pub fn set_value_accesskit(&self, value: f64) {
        self.do_accesskit_action(
            accesskit::Action::SetValue,
            Some(accesskit::ActionData::NumericValue(value)),
        );
    }

    /// Send an [`accesskit::Action::SetValue`] action with a text value to the node.
    ///
    /// This is how assistive technologies set the content of text inputs,
    /// e.g. UI Automation's `ValuePattern.SetValue` on Windows.
    pub fn set_text_value_accesskit(&self, value: &str) {
        self.do_accesskit_action(
            accesskit::Action::SetValue,
            Some(accesskit::ActionData::Value(value.into())),
        );
    }

    /// Send an [`accesskit::Action::ReplaceSelectedText`] action to the node.
    ///
    /// This replaces the selection, or inserts at the cursor, in a focused text input.
    pub fn replace_selected_text_accesskit(&self, text: &str) {
        self.do_accesskit_action(
            accesskit::Action::ReplaceSelectedText,
            Some(accesskit::ActionData::Value(text.into())),
        );
    }

    fn do_accesskit_action(&self, action: accesskit::Action, data: Option<accesskit::ActionData>) {
        let (target_node, target_tree) = self.accesskit_node.locate();
        self.event(egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                target_node,
                target_tree,
                action,
                data,
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
