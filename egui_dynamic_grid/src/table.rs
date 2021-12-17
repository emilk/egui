use crate::{
    layout::{CellSize, LineDirection},
    sizing::Sizing,
    Layout, Padding, Size,
};

use egui::{Response, Ui};
use std::cmp;

pub struct TableBuilder<'a> {
    ui: &'a mut Ui,
    padding: Padding,
    sizing: Sizing,
    scroll: bool,
    striped: bool,
}

impl<'a> TableBuilder<'a> {
    pub fn new(ui: &'a mut Ui, padding: Padding) -> Self {
        let sizing = Sizing::new(
            ui.available_rect_before_wrap().width() - 2.0 * padding.outer,
            padding.inner,
        );

        Self {
            ui,
            padding,
            sizing,
            scroll: true,
            striped: false,
        }
    }

    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    pub fn column(mut self, width: Size) -> Self {
        self.sizing.add_size(width);
        self
    }

    pub fn columns(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add_size(size.clone());
        }
        self
    }

    pub fn header<F>(self, height: f32, header: F) -> Table<'a>
    where
        F: for<'b> FnOnce(TableRow<'a, 'b>),
    {
        let widths = self.sizing.into_lengths();
        let mut layout = Layout::new(self.ui, self.padding.clone(), LineDirection::TopToBottom);
        {
            let row = TableRow {
                layout: &mut layout,
                widths: widths.clone(),
                striped: false,
                height,
                clicked: false,
            };
            header(row);
        }
        let ui = layout.done_ui();

        Table {
            ui,
            padding: self.padding,
            widths,
            scroll: self.scroll,
            striped: self.striped,
        }
    }

    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let widths = self.sizing.into_lengths();

        Table {
            ui: self.ui,
            padding: self.padding,
            widths,
            scroll: self.scroll,
            striped: self.striped,
        }
        .body(body)
    }
}

pub struct Table<'a> {
    ui: &'a mut Ui,
    padding: Padding,
    widths: Vec<f32>,
    scroll: bool,
    striped: bool,
}

impl<'a> Table<'a> {
    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let padding = self.padding;
        let ui = self.ui;
        let widths = self.widths;
        let striped = self.striped;
        let start_y = ui.available_rect_before_wrap().top();
        let end_y = ui.available_rect_before_wrap().bottom();

        egui::ScrollArea::new([false, self.scroll]).show(ui, move |ui| {
            let layout = Layout::new(ui, padding, LineDirection::TopToBottom);

            body(TableBody {
                layout,
                widths,
                striped,
                odd: true,
                start_y,
                end_y,
            });
        });
    }
}

pub struct TableBody<'b> {
    layout: Layout<'b>,
    widths: Vec<f32>,
    striped: bool,
    odd: bool,
    start_y: f32,
    end_y: f32,
}

impl<'a> TableBody<'a> {
    pub fn rows(mut self, height: f32, rows: usize, mut row: impl FnMut(usize, TableRow)) {
        let delta = self.layout.current_y() - self.start_y;
        let mut start = 0;

        if delta < 0.0 {
            start = (-delta / height).floor() as usize;

            let skip_height = start as f32 * height;
            TableRow {
                layout: &mut self.layout,
                widths: self.widths.clone(),
                striped: self.striped && self.odd,
                height: skip_height,
                clicked: false,
            }
            .col(|_| ()); // advances the cursor
        }

        let max_height = self.end_y - self.start_y;
        let count = (max_height / height).ceil() as usize;
        let end = cmp::min(start + count, rows);

        if start % 2 != 0 {
            self.odd = false;
        }

        for idx in start..end {
            row(
                idx,
                TableRow {
                    layout: &mut self.layout,
                    widths: self.widths.clone(),
                    striped: self.striped && self.odd,
                    height,
                    clicked: false,
                },
            );
            self.odd = !self.odd;
        }

        if rows - end > 0 {
            let skip_height = (rows - end) as f32 * height;

            TableRow {
                layout: &mut self.layout,
                widths: self.widths.clone(),
                striped: self.striped && self.odd,
                height: skip_height,
                clicked: false,
            }
            .col(|_| ()); // advances the cursor
        }
    }

    pub fn row<'b>(&'b mut self, height: f32, row: impl FnOnce(TableRow<'a, 'b>)) {
        row(TableRow {
            layout: &mut self.layout,
            widths: self.widths.clone(),
            striped: self.striped && self.odd,
            height,
            clicked: false,
        });

        self.odd = !self.odd;
    }
}

impl<'a> Drop for TableBody<'a> {
    fn drop(&mut self) {
        self.layout.done();
    }
}

pub struct TableRow<'a, 'b> {
    layout: &'b mut Layout<'a>,
    widths: Vec<f32>,
    striped: bool,
    height: f32,
    clicked: bool,
}

impl<'a, 'b> TableRow<'a, 'b> {
    pub fn clicked(&self) -> bool {
        self.clicked
    }

    fn _col(&mut self, clip: bool, add_contents: impl FnOnce(&mut Ui)) -> Response {
        assert!(
            !self.widths.is_empty(),
            "Tried using more table columns then available."
        );

        let width = CellSize::Absolute(self.widths.remove(0));
        let height = CellSize::Absolute(self.height);

        let response;

        if self.striped {
            response = self.layout.add_striped(width, height, clip, add_contents);
        } else {
            response = self.layout.add(width, height, clip, add_contents);
        }

        if response.clicked() {
            self.clicked = true;
        }

        response
    }

    pub fn col(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        self._col(true, add_contents)
    }

    pub fn col_noclip(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        self._col(false, add_contents)
    }
}

impl<'a, 'b> Drop for TableRow<'a, 'b> {
    fn drop(&mut self) {
        self.layout.end_line();
    }
}
