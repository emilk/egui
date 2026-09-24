use emath::NumExt;
use crate::{Align2, Id, Rect, Ui, UiBuilder, Vec2};

pub struct Group {
    id: Id,
    rect: Option<Rect>,
    size: Option<Vec2>,
    align2: Align2,
}

impl Group {
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            rect: None,
            size: None,
            align2: Align2::CENTER_CENTER,
        }
    }

    pub fn align2(mut self, align2: Align2) -> Self {
        self.align2 = align2;
        self
    }

    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn ui<T>(self, ui: &mut Ui, content: impl FnOnce(&mut Ui) -> T) -> T {
        let id = ui.id().with(self.id);
        let data_id = id.with("group");

        let rect = if let Some(rect) = self.rect {
            rect
        } else if let Some(size) = self.size {
            let pos = ui.next_widget_position();
            Rect::from_min_size(pos, size)
        } else {
            ui.available_rect_before_wrap()
        };

        let last_size = ui.ctx().data(|mem| mem.get_temp(data_id));

        let mut content_rect = if let Some(size) = last_size {
            let left_top = self.align2.align_size_within_rect(size, rect).left_top();
            Rect::from_min_size(left_top, rect.size())
        } else {
            rect
        };

        // Clamp the content_rect so it doesn't exceed the top left corner
        let offset = (rect.min - content_rect.min).at_least(Vec2::ZERO);
        content_rect = content_rect.translate(offset);

        let mut builder = UiBuilder::new().id_salt(id);

        if last_size.is_none() {
            builder = builder.invisible();
        }

        let response = ui.scope_builder(builder.max_rect(content_rect), content);

        let size = response.response.rect.size();

        if last_size != Some(size) {
            ui.ctx().request_discard("Group size changed");
            ui.ctx().request_repaint();
        }

        ui.ctx().data_mut(|mem| {
            mem.insert_temp(data_id, size);
        });

        response.inner
    }
}
