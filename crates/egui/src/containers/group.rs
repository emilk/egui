//! Frame container

use crate::{layers::ShapeIdx, *};
use epaint::*;

/// A group of widgets with a unique id
/// that can be used in centered layouts.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[must_use = "You should call .show()"]
pub struct Group {
    id_source: Id,
    frame: Frame,
}

impl Group {
    pub fn new(id_source: impl Into<Id>) -> Self {
        Self {
            id_source: id_source.into(),
            frame: Frame::default(),
        }
    }

    pub fn frame(mut self, frame: Frame) -> Self {
        self.frame = frame;
        self
    }
}

// ----------------------------------------------------------------------------

pub struct Prepared {
    id: Id,

    /// The frame that was prepared.
    ///
    /// The margin has already been read and used,
    /// but the rest of the fields may be modified.
    pub frame: Frame,

    /// This is where we will insert the frame shape so it ends up behind the content.
    where_to_put_background: ShapeIdx,

    /// Add your widgets to this UI so it ends up within the frame.
    pub content_ui: Ui,
}

impl Group {
    /// Begin a dynamically colored frame.
    ///
    /// This is a more advanced API.
    /// Usually you want to use [`Self::show`] instead.
    ///
    /// See docs for [`Group`] for an example.
    pub fn begin(self, ui: &mut Ui) -> Prepared {
        let Self { id_source, frame } = self;
        let id = ui.make_persistent_id(id_source);

        let where_to_put_background = ui.painter().add(Shape::Noop);

        let prev_inner_size: Option<Vec2> = ui.data(|data| data.get_temp(id));

        let mut inner_rect = if let Some(prev_inner_size) = prev_inner_size {
            let (_, outer_rect) = ui.allocate_space(prev_inner_size + frame.total_margin().sum());
            outer_rect - frame.total_margin()
        } else {
            // Invisible sizing pass
            let outer_rect_bounds = ui.available_rect_before_wrap();
            outer_rect_bounds - frame.total_margin()
        };

        // Make sure we don't shrink to the negative:
        inner_rect.max.x = inner_rect.max.x.max(inner_rect.min.x);
        inner_rect.max.y = inner_rect.max.y.max(inner_rect.min.y);

        let sizing_pass = prev_inner_size.is_none();

        let mut layout = *ui.layout();

        if sizing_pass {
            // TODO(emilk): this code is duplicated from `ui.rs`
            // During the sizing pass we want widgets to use up as little space as possible,
            // so that we measure the only the space we _need_.
            layout.cross_justify = false;
            if layout.cross_align == Align::Center {
                layout.cross_align = Align::Min;
            }
        }

        let mut content_ui = ui.child_ui(
            inner_rect,
            layout,
            Some(UiStackInfo::new(UiKind::Frame).with_frame(frame)),
        );

        if sizing_pass {
            content_ui.set_sizing_pass();
        }

        Prepared {
            id,
            frame,
            where_to_put_background,
            content_ui,
        }
    }

    /// Show the given ui surrounded by this frame.
    pub fn show<R>(self, ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
        self.show_dyn(ui, Box::new(add_contents))
    }

    /// Show using dynamic dispatch.
    pub fn show_dyn<'c, R>(
        self,
        ui: &mut Ui,
        add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
    ) -> InnerResponse<R> {
        let mut prepared = self.begin(ui);
        let ret = add_contents(&mut prepared.content_ui);
        let response = prepared.end(ui);
        InnerResponse::new(ret, response)
    }
}

impl Prepared {
    /// Allocate the space that was used by [`Self::content_ui`].
    ///
    /// This MUST be called, or the parent ui will not know how much space this widget used.
    ///
    /// This can be called before or after [`Self::paint`].
    pub fn allocate_space(&self, ui: &mut Ui) -> Response {
        let inner_rect = self.content_ui.min_rect();
        let outer_rect = inner_rect + self.frame.total_margin();

        // Remember size to next frame
        ui.data_mut(|data| data.insert_temp(self.id, inner_rect.size()));

        ui.allocate_rect(outer_rect, Sense::hover())
    }

    /// Paint the frame.
    ///
    /// This can be called before or after [`Self::allocate_space`].
    pub fn paint(&self, ui: &Ui) {
        let paint_rect = self.content_ui.min_rect() + self.frame.inner_margin;

        if ui.is_rect_visible(paint_rect) {
            let shape = self.frame.paint(paint_rect);
            ui.painter().set(self.where_to_put_background, shape);
        }
    }

    /// Convenience for calling [`Self::allocate_space`] and [`Self::paint`].
    pub fn end(self, ui: &mut Ui) -> Response {
        self.paint(ui);
        self.allocate_space(ui)
    }
}
