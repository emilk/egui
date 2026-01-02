use std::{any::Any, sync::Arc};

use crate::{Context, CursorIcon, Plugin, Ui};

/// Plugin for tracking drag-and-drop payload.
///
/// This plugin stores the current drag-and-drop payload internally and handles
/// automatic cleanup when the drag operation ends (via Escape key or mouse release).
///
/// This is a low-level API. For a higher-level API, see:
/// - [`crate::Ui::dnd_drag_source`]
/// - [`crate::Ui::dnd_drop_zone`]
/// - [`crate::Response::dnd_set_drag_payload`]
/// - [`crate::Response::dnd_hover_payload`]
/// - [`crate::Response::dnd_release_payload`]
///
/// This is a built-in plugin in egui, automatically registered during [`Context`] creation.
///
/// See [this example](https://github.com/emilk/egui/blob/main/crates/egui_demo_lib/src/demo/drag_and_drop.rs).
#[doc(alias = "drag and drop")]
#[derive(Clone, Default)]
pub struct DragAndDrop {
    /// The current drag-and-drop payload, if any. Automatically cleared when drag ends.
    payload: Option<Arc<dyn Any + Send + Sync>>,
}

impl Plugin for DragAndDrop {
    fn debug_name(&self) -> &'static str {
        "DragAndDrop"
    }

    /// Interrupt drag-and-drop if the user presses the escape key.
    ///
    /// This needs to happen at frame start so we can properly capture the escape key.
    fn on_begin_pass(&mut self, ui: &mut Ui) {
        let has_any_payload = self.payload.is_some();

        if has_any_payload {
            let abort_dnd_due_to_escape_key =
                ui.input_mut(|i| i.consume_key(crate::Modifiers::NONE, crate::Key::Escape));

            if abort_dnd_due_to_escape_key {
                self.payload = None;
            }
        }
    }

    /// Interrupt drag-and-drop if the user releases the mouse button.
    ///
    /// This is a catch-all safety net in case user code doesn't capture the drag payload itself.
    /// This must happen at end-of-frame such that we don't shadow the mouse release event from user
    /// code.
    fn on_end_pass(&mut self, ui: &mut Ui) {
        let has_any_payload = self.payload.is_some();

        if has_any_payload {
            let abort_dnd_due_to_mouse_release = ui.input_mut(|i| i.pointer.any_released());

            if abort_dnd_due_to_mouse_release {
                self.payload = None;
            } else {
                // We set the cursor icon only if its default, as the user code might have
                // explicitly set it already.
                ui.output_mut(|o| {
                    if o.cursor_icon == CursorIcon::Default {
                        o.cursor_icon = CursorIcon::Grabbing;
                    }
                });
            }
        }
    }
}

impl DragAndDrop {
    /// Set a drag-and-drop payload.
    ///
    /// This can be read by [`Self::payload`] until the pointer is released.
    pub fn set_payload<Payload>(ctx: &Context, payload: Payload)
    where
        Payload: Any + Send + Sync,
    {
        ctx.plugin::<Self>().lock().payload = Some(Arc::new(payload));
    }

    /// Clears the payload, setting it to `None`.
    pub fn clear_payload(ctx: &Context) {
        ctx.plugin::<Self>().lock().payload = None;
    }

    /// Retrieve the payload, if any.
    ///
    /// Returns `None` if there is no payload, or if it is not of the requested type.
    ///
    /// Returns `Some` both during a drag and on the frame the pointer is released
    /// (if there is a payload).
    pub fn payload<Payload>(ctx: &Context) -> Option<Arc<Payload>>
    where
        Payload: Any + Send + Sync,
    {
        Arc::clone(ctx.plugin::<Self>().lock().payload.as_ref()?)
            .downcast()
            .ok()
    }

    /// Retrieve and clear the payload, if any.
    ///
    /// Returns `None` if there is no payload, or if it is not of the requested type.
    ///
    /// Returns `Some` both during a drag and on the frame the pointer is released
    /// (if there is a payload).
    pub fn take_payload<Payload>(ctx: &Context) -> Option<Arc<Payload>>
    where
        Payload: Any + Send + Sync,
    {
        ctx.plugin::<Self>().lock().payload.take()?.downcast().ok()
    }

    /// Are we carrying a payload of the given type?
    ///
    /// Returns `true` both during a drag and on the frame the pointer is released
    /// (if there is a payload).
    pub fn has_payload_of_type<Payload>(ctx: &Context) -> bool
    where
        Payload: Any + Send + Sync,
    {
        Self::payload::<Payload>(ctx).is_some()
    }

    /// Are we carrying a payload?
    ///
    /// Returns `true` both during a drag and on the frame the pointer is released
    /// (if there is a payload).
    pub fn has_any_payload(ctx: &Context) -> bool {
        ctx.plugin::<Self>().lock().payload.is_some()
    }
}
