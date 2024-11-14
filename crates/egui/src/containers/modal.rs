use crate::{
    Area, Color32, Context, Frame, Id, InnerResponse, Order, Response, Sense, Ui, UiBuilder,
};
use emath::{Align2, Vec2};

pub struct Modal {
    pub area: Area,
    pub backdrop_color: Color32,
    pub frame: Option<Frame>,
}

impl Modal {
    pub fn new(id: Id) -> Self {
        Self {
            area: Area::new(id)
                .sense(Sense::hover())
                .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                .order(Order::Foreground),
            backdrop_color: Color32::from_black_alpha(100),
            frame: None,
        }
    }

    pub fn show<T>(self, ctx: &Context, content: impl FnOnce(&mut Ui) -> T) -> ModalResponse<T> {
        let (is_top_modal, any_popup_open) = ctx.memory_mut(|mem| {
            mem.set_modal_layer(self.area.layer());
            (
                mem.top_modal_layer() == Some(self.area.layer()),
                mem.any_popup_open(),
            )
        });
        let InnerResponse {
            inner: (inner, backdrop_response),
            response,
        } = self.area.show(ctx, |ui| {
            // TODO: Is screen_rect the right thing to use here?
            let mut backdrop = ui.new_child(UiBuilder::new().max_rect(ui.ctx().screen_rect()));
            let backdrop_response = backdrop_ui(&mut backdrop, self.backdrop_color);

            let frame = self.frame.unwrap_or_else(|| Frame::popup(ui.style()));

            // We need the extra scope with the sense since frame can't have a sense and since we
            // need to prevent the clicks from passing through to the backdrop.
            let inner = ui
                .scope_builder(
                    UiBuilder::new().sense(Sense {
                        click: true,
                        drag: true,
                        focusable: false,
                    }),
                    |ui| frame.show(ui, content).inner,
                )
                .inner;

            (inner, backdrop_response)
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

fn backdrop_ui(ui: &mut Ui, color: Color32) -> Response {
    // Ensure we capture any click and drag events
    let response = ui.allocate_response(
        ui.available_size(),
        Sense {
            click: true,
            drag: true,
            focusable: false,
        },
    );

    ui.painter().rect_filled(response.rect, 0.0, color);

    response
}

pub struct ModalResponse<T> {
    pub response: Response,
    pub backdrop_response: Response,
    pub inner: T,
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
    ///  - this is the top most modal, no popup is open and the escape key was pressed
    pub fn should_close(&self) -> bool {
        let ctx = &self.response.ctx;
        let escape_clicked = ctx.input(|i| i.key_pressed(crate::Key::Escape));
        self.backdrop_response.clicked()
            || (self.is_top_modal && !self.any_popup_open && escape_clicked)
    }
}
