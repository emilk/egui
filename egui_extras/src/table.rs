/// Table view with (optional) fixed header and scrolling body.
/// Cell widths are precalculated with given size hints so we can have tables like this:
/// | fixed size | all available space/minimum | 30% of available width | fixed size |
/// Takes all available height, so if you want something below the table, put it in a grid.
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
        let sizing = Sizing::new();

        Self {
            ui,
            padding,
            sizing,
            scroll: true,
            striped: false,
        }
    }

    /// Enable scrollview in body (default: true)
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    /// Enable striped row background (default: false)
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Add size hint for column
    pub fn column(mut self, width: Size) -> Self {
        self.sizing.add_size(width);
        self
    }

    /// Add size hint for column [count] times
    pub fn columns(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add_size(size);
        }
        self
    }

    /// Create a header row which always stays visible and at the top
    pub fn header(self, height: f32, header: impl FnOnce(TableRow<'_, '_>)) -> Table<'a> {
        let widths = self.sizing.into_lengths(
            self.ui.available_rect_before_wrap().width() - 2.0 * self.padding.outer,
            self.padding.inner,
        );
        let ui = self.ui;
        {
            let mut layout = Layout::new(ui, self.padding.clone(), LineDirection::TopToBottom);
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
        }

        Table {
            ui,
            padding: self.padding,
            widths,
            scroll: self.scroll,
            striped: self.striped,
        }
    }

    /// Create table body without a header row
    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let widths = self.sizing.into_lengths(
            self.ui.available_rect_before_wrap().width() - 2.0 * self.padding.outer,
            self.padding.inner,
        );

        Table {
            ui: self.ui,
            padding: self.padding,
            widths,
            scroll: self.scroll,
            striped: self.striped,
        }
        .body(body);
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
    /// Create table body after adding a header row
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

pub struct TableBody<'a> {
    layout: Layout<'a>,
    widths: Vec<f32>,
    striped: bool,
    odd: bool,
    start_y: f32,
    end_y: f32,
}

impl<'a> TableBody<'a> {
    /// Add rows with same height
    /// Is a lot more performant than adding each individual row as non visible rows must not be rendered
    pub fn rows(mut self, height: f32, rows: usize, mut row: impl FnMut(usize, TableRow<'_, '_>)) {
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

    /// Add row with individual height
    pub fn row(&mut self, height: f32, row: impl FnOnce(TableRow<'a, '_>)) {
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

pub struct TableRow<'a, 'b> {
    layout: &'b mut Layout<'a>,
    widths: Vec<f32>,
    striped: bool,
    height: f32,
    clicked: bool,
}

impl<'a, 'b> TableRow<'a, 'b> {
    /// Check if row was clicked
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

    /// Add column, content is wrapped
    pub fn col(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        self._col(false, add_contents)
    }

    /// Add column, content is clipped
    pub fn col_clip(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        self._col(true, add_contents)
    }
}

impl<'a, 'b> Drop for TableRow<'a, 'b> {
    fn drop(&mut self) {
        self.layout.end_line();
    }
}
