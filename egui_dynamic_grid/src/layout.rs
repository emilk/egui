use crate::Padding;
use egui::{Pos2, Rect, Response, Rgba, Sense, Ui, Vec2};

pub(crate) enum CellSize {
    Absolute(f32),
    Remainder,
}

pub(crate) enum LineDirection {
    /// Cells go from top to bottom
    LeftToRight,
    /// Cells go from left to right
    TopToBottom,
}

pub struct Layout<'a> {
    ui: &'a mut Ui,
    padding: Padding,
    direction: LineDirection,
    rect: Rect,
    pos: Pos2,
    max: Pos2,
}

impl<'a> Layout<'a> {
    pub(crate) fn new(ui: &'a mut Ui, padding: Padding, direction: LineDirection) -> Self {
        let mut rect = ui.available_rect_before_wrap();
        rect.set_left(rect.left() + padding.outer + padding.inner);
        rect.set_top(rect.top() + padding.outer + padding.inner);
        rect.set_width(rect.width() - 2.0 * padding.outer);
        rect.set_height(rect.height() - 2.0 * padding.outer);
        let pos = rect.left_top();

        Self {
            ui,
            padding,
            rect,
            pos,
            max: pos,
            direction,
        }
    }

    pub fn current_y(&self) -> f32 {
        self.rect.top()
    }

    fn cell_rect(&self, width: &CellSize, height: &CellSize) -> Rect {
        Rect {
            min: self.pos,
            max: Pos2 {
                x: match width {
                    CellSize::Absolute(width) => self.pos.x + width,
                    CellSize::Remainder => self.rect.right(),
                },
                y: match height {
                    CellSize::Absolute(height) => self.pos.y + height,
                    CellSize::Remainder => self.rect.bottom(),
                },
            },
        }
    }

    fn set_pos(&mut self, rect: Rect) {
        match self.direction {
            LineDirection::LeftToRight => {
                self.pos.y = rect.bottom() + self.padding.inner;
            }
            LineDirection::TopToBottom => {
                self.pos.x = rect.right() + self.padding.inner;
            }
        }

        self.max.x = self.max.x.max(rect.right() + self.padding.inner);
        self.max.y = self.max.y.max(rect.bottom() + self.padding.inner);
    }

    pub(crate) fn empty(&mut self, width: CellSize, height: CellSize) {
        self.set_pos(self.cell_rect(&width, &height));
    }

    pub(crate) fn add(
        &mut self,
        width: CellSize,
        height: CellSize,
        clip: bool,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Response {
        let rect = self.cell_rect(&width, &height);
        self.cell(rect, clip, add_contents);
        self.set_pos(rect);

        self.ui.allocate_rect(rect, Sense::click())
    }

    pub(crate) fn add_striped(
        &mut self,
        width: CellSize,
        height: CellSize,
        clip: bool,
        add_contents: impl FnOnce(&mut Ui),
    ) -> Response {
        let mut rect = self.cell_rect(&width, &height);
        *rect.top_mut() -= self.padding.inner;
        *rect.left_mut() -= self.padding.inner;

        let text_color: Rgba = self.ui.visuals().text_color().into();
        self.ui
            .painter()
            .rect_filled(rect, 0.0, text_color.multiply(0.2));

        self.add(width, height, clip, add_contents)
    }

    /// only needed for layouts with multiple lines, like Table
    pub fn end_line(&mut self) {
        match self.direction {
            LineDirection::LeftToRight => {
                self.pos.x = self.max.x;
                self.pos.y = self.rect.top();
            }
            LineDirection::TopToBottom => {
                self.pos.y = self.max.y;
                self.pos.x = self.rect.left();
            }
        }
    }

    fn set_rect(&mut self) {
        let mut rect = self.rect;
        rect.set_right(self.max.x);
        rect.set_bottom(self.max.y);

        self.ui
            .allocate_rect(rect, Sense::focusable_noninteractive());
    }

    pub fn done(&mut self) {
        self.set_rect();
    }

    pub fn done_ui(mut self) -> &'a mut Ui {
        self.set_rect();
        self.ui
    }

    fn cell(&mut self, rect: Rect, clip: bool, add_contents: impl FnOnce(&mut Ui)) {
        let mut child_ui = self.ui.child_ui(rect, *self.ui.layout());

        if clip {
            let mut clip_rect = child_ui.clip_rect();
            clip_rect.min = clip_rect
                .min
                .max(rect.min - Vec2::new(self.padding.inner, self.padding.inner));
            clip_rect.max = clip_rect
                .max
                .min(rect.max + Vec2::new(self.padding.inner, self.padding.inner));
            child_ui.set_clip_rect(clip_rect);
        }

        add_contents(&mut child_ui)
    }
}
