use std::{any::Any, sync::Arc};

use crate::{Context, CursorIcon, Id};

/// Tracking of drag-and-drop payload.
///
/// This is a low-level API.
///
/// For a higher-level API, see:
/// - [`crate::Ui::dnd_drag_source`]
/// - [`crate::Ui::dnd_drop_zone`]
/// - [`crate::Response::dnd_set_drag_payload`]
/// - [`crate::Response::dnd_hover_payload`]
/// - [`crate::Response::dnd_release_payload`]
///
/// See [this example](https://github.com/emilk/egui/blob/master/crates/egui_demo_lib/src/demo/drag_and_drop.rs).
#[doc(alias = "drag and drop")]
#[derive(Clone, Default)]
pub struct DragAndDrop {
    /// If set, something is currently being dragged
    payload: Option<Arc<dyn Any + Send + Sync>>,
}

impl DragAndDrop {
    pub(crate) fn register(ctx: &Context) {
        ctx.on_begin_pass("drag_and_drop_begin_pass", Arc::new(Self::begin_pass));
        ctx.on_end_pass("drag_and_drop_end_pass", Arc::new(Self::end_pass));
    }

    /// Interrupt drag-and-drop if the user presses the escape key.
    ///
    /// This needs to happen at frame start so we can properly capture the escape key.
    fn begin_pass(ctx: &Context) {
        let has_any_payload = Self::has_any_payload(ctx);

        if has_any_payload {
            let abort_dnd_due_to_escape_key =
                ctx.input_mut(|i| i.consume_key(crate::Modifiers::NONE, crate::Key::Escape));

            if abort_dnd_due_to_escape_key {
                Self::clear_payload(ctx);
            }
        }
    }

    /// Interrupt drag-and-drop if the user releases the mouse button.
    ///
    /// This is a catch-all safety net in case user code doesn't capture the drag payload itself.
    /// This must happen at end-of-frame such that we don't shadow the mouse release event from user
    /// code.
    fn end_pass(ctx: &Context) {
        let has_any_payload = Self::has_any_payload(ctx);

        if has_any_payload {
            let abort_dnd_due_to_mouse_release = ctx.input_mut(|i| i.pointer.any_released());

            if abort_dnd_due_to_mouse_release {
                Self::clear_payload(ctx);
            } else {
                // We set the cursor icon only if its default, as the user code might have
                // explicitly set it already.
                ctx.output_mut(|o| {
                    if o.cursor_icon == CursorIcon::Default {
                        o.cursor_icon = CursorIcon::Grabbing;
                    }
                });
            }
        }
    }

    /// Set a drag-and-drop payload.
    ///
    /// This can be read by [`Self::payload`] until the pointer is released.
    pub fn set_payload<Payload>(ctx: &Context, payload: Payload)
    where
        Payload: Any + Send + Sync,
    {
        ctx.data_mut(|data| {
            let state = data.get_temp_mut_or_default::<Self>(Id::NULL);
            state.payload = Some(Arc::new(payload));
        });
    }

    /// Clears the payload, setting it to `None`.
    pub fn clear_payload(ctx: &Context) {
        ctx.data_mut(|data| {
            let state = data.get_temp_mut_or_default::<Self>(Id::NULL);
            state.payload = None;
        });
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
        ctx.data(|data| {
            let state = data.get_temp::<Self>(Id::NULL)?;
            let payload = state.payload?;
            payload.downcast().ok()
        })
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
        ctx.data_mut(|data| {
            let state = data.get_temp_mut_or_default::<Self>(Id::NULL);
            let payload = state.payload.take()?;
            payload.downcast().ok()
        })
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
        ctx.data(|data| {
            let state = data.get_temp::<Self>(Id::NULL);
            state.is_some_and(|state| state.payload.is_some())
        })
    }
}
