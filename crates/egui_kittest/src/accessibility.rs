//! Checks that every widget a user can act on is reachable by name.

use egui::accessibility::is_input;
use kittest::{AccessKitNode, NodeT as _};

use crate::Harness;

impl<State> Harness<'_, State> {
    /// Every visible input widget that has no accessible name.
    ///
    /// A read-only text field (e.g. selectable text) is not an input, so it needs none.
    /// Each entry names the role, the rect, and the closest named ancestor,
    /// so you can tell which widget is missing its name.
    pub fn unnamed_widgets(&self) -> Vec<String> {
        self.root()
            .children_recursive()
            .map(|node| node.accesskit_node())
            .filter(|node| is_input(node.role()) && !node.is_hidden())
            // The raw flag: the consumer reports every role that cannot be edited as read-only.
            .filter(|node| !node.data().is_read_only())
            .filter(|node| !has_name(node))
            .map(|node| describe(&node))
            .collect()
    }

    /// Panic if any visible input widget has no accessible name.
    ///
    /// A widget without a name cannot be reached: not by a screen reader, and not by
    /// [`kittest::Queryable::get_by_label`]. Give icon-only buttons an alt text, tie text inputs to
    /// their label with [`egui::Response::labelled_by`], or add an [`egui::Response::on_hover_text`].
    ///
    /// This runs whenever the ui has settled (after [`Harness::run`] and friends) and before
    /// every snapshot; turn it off with [`crate::HarnessBuilder::with_accessibility_check`].
    #[track_caller]
    pub fn check_accessibility(&self) {
        let unnamed = self.unnamed_widgets();
        assert!(
            unnamed.is_empty(),
            "{} widget(s) lack accessible name. You can disable this check with HarnessBuilder::with_accessibility_check(false). The widgets:\n{}",
            unnamed.len(),
            unnamed.join("\n")
        );
    }
}

fn has_name(node: &AccessKitNode<'_>) -> bool {
    let has_label = node.label().is_some_and(|label| !label.trim().is_empty());
    // A hint is what a screen reader falls back to for an unlabelled text field.
    // Read the raw data: the consumer hides the placeholder once the field has text.
    let has_placeholder = node
        .data()
        .placeholder()
        .is_some_and(|placeholder| !placeholder.trim().is_empty());
    has_label || has_placeholder
}

fn describe(node: &AccessKitNode<'_>) -> String {
    let ancestor = core::iter::successors(node.parent(), AccessKitNode::parent)
        .find_map(|ancestor| ancestor.label())
        .map_or_else(String::new, |label| format!(" inside {label:?}"));
    format!("{:?} at {:?}{ancestor}", node.role(), node.raw_bounds())
}
