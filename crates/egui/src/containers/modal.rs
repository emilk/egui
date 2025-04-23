use crate::{
    Area, Color32, Context, Frame, Id, InnerResponse, Order, Response, Sense, Ui, UiKind,
    WidgetRect,
};
use emath::{Align2, Vec2};

/// A modal dialog.
///
/// Similar to a [`crate::Window`] but centered and with a backdrop that
/// blocks input to the rest of the UI.
///
/// You can show multiple modals on top of each other. The topmost modal will always be
/// the most recently shown one.
/// If multiple modals are newly shown in the same frame, the order of the modals not undefined
/// (either first or second could be top).
pub struct Modal {
    pub area: Area,
    pub backdrop_color: Color32,
    pub frame: Option<Frame>,
}

impl Modal {
    /// Create a new Modal.
    ///
    /// The id is passed to the area.
    pub fn new(id: Id) -> Self {
        Self {
            area: Self::default_area(id),
            backdrop_color: Color32::from_black_alpha(100),
            frame: None,
        }
    }

    /// Returns an area customized for a modal.
    ///
    /// Makes these changes to the default area:
    /// - sense: click + drag
    /// - anchor: center
    /// - order: foreground
    ///
    /// Consider the notes at [`Modal::area`] for more information.
    pub fn default_area(id: Id) -> Area {
        Area::new(id)
            .kind(UiKind::Modal)
            .sense(Sense::click_and_drag())
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .order(Order::Foreground)
            .interactable(true)
    }

    /// Returns an area customized to make a modal draggable.
    ///
    /// Makes these changes to the default area:
    /// - sense: click + drag
    /// - pivot: center
    /// - order: foreground
    ///
    /// Consider the notes at [`Modal::area`] for more information.
    pub fn draggable_area(id: Id) -> Area {
        Area::new(id)
            .kind(UiKind::Modal)
            .sense(Sense::click_and_drag())
            .pivot(Align2::CENTER_CENTER)
            .order(Order::Foreground)
            .interactable(true)
    }

    /// Set the frame of the modal.
    ///
    /// Default is [`Frame::popup`].
    #[inline]
    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Set the backdrop color of the modal.
    ///
    /// Default is `Color32::from_black_alpha(100)`.
    #[inline]
    pub fn backdrop_color(mut self, color: Color32) -> Self {
        self.backdrop_color = color;
        self
    }

    /// Set the area of the modal.
    ///
    /// Default is [`Modal::default_area`].
    ///
    /// If the modal should be draggable, consider using [`Modal::draggable_area`] instead.
    ///
    /// If you want to provide a custom area, make sure it senses [`Sense::CLICK`] and [`Sense::DRAG`].
    /// Otherwise the background backdrop might catch events meant for the content of the modal.
    ///
    /// Also, if the Area satisfies [`Area::is_movable`], the Modal will ignore the stored position and re-center it on modal re-open.
    /// This is affected by [`Area::pivot`], which [`Modal::draggable_area`] sets to [`Align2::CENTER_CENTER`].
    #[inline]
    pub fn area(mut self, area: Area) -> Self {
        self.area = area;
        self
    }

    /// Show the modal.
    pub fn show<T>(self, ctx: &Context, content: impl FnOnce(&mut Ui) -> T) -> ModalResponse<T> {
        let Self {
            mut area,
            backdrop_color,
            frame,
        } = self;

        let (is_top_modal, any_popup_open) = ctx.memory_mut(|mem| {
            mem.set_modal_layer(area.layer());
            (
                mem.top_modal_layer() == Some(area.layer()),
                mem.any_popup_open(),
            )
        });

        // The backdrop response is responsible for checking for click through etc.
        // It needs to be drawn before everything else, so we can use it to block clicks.
        // Thus, we manually add the widget.
        let bg_rect = ctx.screen_rect();
        let bg_sense = Sense::CLICK | Sense::DRAG;
        let backdrop_response = ctx.create_widget(
            WidgetRect {
                id: area.id.with("background rect"),
                layer_id: area.layer(),
                rect: bg_rect,
                interact_rect: bg_rect,
                sense: bg_sense,
                enabled: true,
            },
            true,
        );

        // Should the area be movable, and we are (re-)opening it, try to center it.
        if area.is_movable() && !ctx.memory(|mem| mem.areas().visible_last_frame(&area.layer())) {
            area = area.current_pos(ctx.screen_rect().center());
        }

        let InnerResponse { inner, response } = area.show(ctx, |ui| {
            // The backdrop still needs painting.
            ui.painter().rect_filled(bg_rect, 0.0, backdrop_color);

            let frame = frame.unwrap_or_else(|| Frame::popup(ui.style()));
            frame.show(ui, content).inner
        });

        ModalResponse {
            response,
            backdrop_response,
            inner,
            is_top_modal,
            any_popup_open,
        }
    }
}

/// The response of a modal dialog.
pub struct ModalResponse<T> {
    /// The response of the modal contents
    pub response: Response,

    /// The response of the modal backdrop.
    ///
    /// A click on this means the user clicked outside the modal,
    /// in which case you might want to close the modal.
    pub backdrop_response: Response,

    /// The inner response from the content closure
    pub inner: T,

    /// Is this the topmost modal?
    pub is_top_modal: bool,

    /// Is there any popup open?
    /// We need to check this before the modal contents are shown, so we can know if any popup
    /// was open when checking if the escape key was clicked.
    pub any_popup_open: bool,
}

impl<T> ModalResponse<T> {
    /// Should the modal be closed?
    /// Returns true if:
    ///  - the backdrop was clicked
    ///  - this is the topmost modal, no popup is open and the escape key was pressed
    pub fn should_close(&self) -> bool {
        let ctx = &self.response.ctx;

        // this is a closure so that `Esc` is consumed only if the modal is topmost
        let escape_clicked =
            || ctx.input_mut(|i| i.consume_key(crate::Modifiers::NONE, crate::Key::Escape));

        let ui_close_called = self.response.should_close();

        self.backdrop_response.clicked()
            || ui_close_called
            || (self.is_top_modal && !self.any_popup_open && escape_clicked())
    }
}
