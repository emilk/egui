//! Table view with (optional) fixed header and scrolling body.
//! Cell widths are precalculated with given size hints so we can have tables like this:
//! | fixed size | all available space/minimum | 30% of available width | fixed size |
//! Takes all available height, so if you want something below the table, put it in a strip.

use crate::{
    layout::{CellDirection, CellSize},
    sizing::Sizing,
    Size, StripLayout,
};

use egui::{Response, Ui};

/// Builder for a [`Table`] with (optional) fixed header and scrolling body.
///
/// Cell widths are precalculated with given size hints so we can have tables like this:
///
/// | fixed size | all available space/minimum | 30% of available width | fixed size |
///
/// In contrast to normal egui behavior, columns/rows do *not* grow with its children!
/// Takes all available height, so if you want something below the table, put it in a strip.
///
/// ### Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui_extras::{TableBuilder, Size};
/// TableBuilder::new(ui)
///     .column(Size::remainder().at_least(100.0))
///     .column(Size::exact(40.0))
///     .header(20.0, |mut header| {
///         header.col(|ui| {
///             ui.heading("Growing");
///         });
///         header.col(|ui| {
///             ui.heading("Fixed");
///         });
///     })
///     .body(|mut body| {
///         body.row(30.0, |mut row| {
///             row.col(|ui| {
///                 ui.label("first row growing cell");
///             });
///             row.col(|ui| {
///                 ui.button("action");
///             });
///         });
///     });
/// # });
/// ```
pub struct TableBuilder<'a> {
    ui: &'a mut Ui,
    sizing: Sizing,
    scroll: bool,
    striped: bool,
    resizable: bool,
    clip: bool,
}

impl<'a> TableBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        let sizing = Sizing::new();

        Self {
            ui,
            sizing,
            scroll: true,
            striped: false,
            resizable: false,
            clip: true,
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

    /// Make the columns resizable by dragging.
    ///
    /// If the _last_ column is [`Size::Remainder`], then it won't be resizable
    /// (and instead use up the remainder).
    ///
    /// Default is `false`.
    ///
    /// If you have multiple [`Table`]:s in the same [`Ui`]
    /// you will need to give them unique id:s with [`Ui::push_id`].
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Should we clip the contents of each cell? Default: `true`.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Add size hint for column
    pub fn column(mut self, width: Size) -> Self {
        self.sizing.add(width);
        self
    }

    /// Add size hint for several columns at once.
    pub fn columns(mut self, size: Size, count: usize) -> Self {
        for _ in 0..count {
            self.sizing.add(size);
        }
        self
    }

    fn available_width(&self) -> f32 {
        self.ui.available_rect_before_wrap().width()
            - if self.scroll {
                self.ui.spacing().item_spacing.x + self.ui.spacing().scroll_bar_width
            } else {
                0.0
            }
    }

    /// Create a header row which always stays visible and at the top
    pub fn header(self, height: f32, header: impl FnOnce(TableRow<'_, '_>)) -> Table<'a> {
        let available_width = self.available_width();

        let Self {
            ui,
            sizing,
            scroll,
            striped,
            resizable,
            clip,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));
        let widths = if let Some(resize_id) = resize_id {
            ui.data().get_persisted(resize_id)
        } else {
            None
        };
        let widths = widths
            .unwrap_or_else(|| sizing.to_lengths(available_width, ui.spacing().item_spacing.x));

        let table_top = ui.cursor().top();

        {
            let mut layout = StripLayout::new(ui, CellDirection::Horizontal, clip);
            header(TableRow {
                layout: &mut layout,
                widths: &widths,
                striped: false,
                height,
            });
            layout.allocate_rect();
        }

        Table {
            ui,
            table_top,
            resize_id,
            sizing,
            available_width,
            widths,
            scroll,
            striped,
            clip,
        }
    }

    /// Create table body without a header row
    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let available_width = self.available_width();

        let Self {
            ui,
            sizing,
            scroll,
            striped,
            resizable,
            clip,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));
        let widths = if let Some(resize_id) = resize_id {
            ui.data().get_persisted(resize_id)
        } else {
            None
        };
        let widths = widths
            .unwrap_or_else(|| sizing.to_lengths(available_width, ui.spacing().item_spacing.x));

        let table_top = ui.cursor().top();

        Table {
            ui,
            table_top,
            resize_id,
            sizing,
            available_width,
            widths,
            scroll,
            striped,
            clip,
        }
        .body(body);
    }
}

/// Table struct which can construct a [`TableBody`].
///
/// Is created by [`TableBuilder`] by either calling [`TableBuilder::body`] or after creating a header row with [`TableBuilder::header`].
pub struct Table<'a> {
    ui: &'a mut Ui,
    table_top: f32,
    resize_id: Option<egui::Id>,
    sizing: Sizing,
    available_width: f32,
    widths: Vec<f32>,
    scroll: bool,
    striped: bool,
    clip: bool,
}

impl<'a> Table<'a> {
    /// Create table body after adding a header row
    pub fn body<F>(self, body: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let Table {
            ui,
            table_top,
            resize_id,
            sizing,
            mut available_width,
            widths,
            scroll,
            striped,
            clip,
        } = self;

        let avail_rect = ui.available_rect_before_wrap();

        let mut new_widths = widths.clone();

        egui::ScrollArea::new([false, scroll])
            .auto_shrink([true; 2])
            .show(ui, move |ui| {
                let layout = StripLayout::new(ui, CellDirection::Horizontal, clip);

                body(TableBody {
                    layout,
                    widths,
                    striped,
                    row_nr: 0,
                    start_y: avail_rect.top(),
                    end_y: avail_rect.bottom(),
                });
            });

        let bottom = ui.min_rect().bottom();

        // TODO: fix frame-delay by interacting before laying out (but painting later).
        if let Some(resize_id) = resize_id {
            let spacing_x = ui.spacing().item_spacing.x;
            let mut x = avail_rect.left() - spacing_x * 0.5;
            for (i, width) in new_widths.iter_mut().enumerate() {
                x += *width + spacing_x;

                // If the last column is Size::Remainder, then let it fill the remainder!
                let last_column = i + 1 == sizing.sizes.len();
                if last_column {
                    if let Size::Remainder { range: (min, max) } = sizing.sizes[i] {
                        let eps = 0.1; // just to avoid some rounding errors.
                        *width = (available_width - eps).clamp(min, max);
                        break;
                    }
                }

                let resize_id = ui.id().with("__panel_resize").with(i);

                let mut p0 = egui::pos2(x, table_top);
                let mut p1 = egui::pos2(x, bottom);
                let line_rect = egui::Rect::from_min_max(p0, p1)
                    .expand(ui.style().interaction.resize_grab_radius_side);
                let mouse_over_resize_line = ui.rect_contains_pointer(line_rect);

                if ui.input().pointer.any_pressed()
                    && ui.input().pointer.any_down()
                    && mouse_over_resize_line
                {
                    ui.memory().set_dragged_id(resize_id);
                }
                let is_resizing = ui.memory().is_being_dragged(resize_id);
                if is_resizing {
                    if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                        let new_width = *width + pointer.x - x;
                        let (min, max) = sizing.sizes[i].range();
                        let new_width = new_width.clamp(min, max);
                        let x = x - *width + new_width;
                        p0.x = x;
                        p1.x = x;

                        *width = new_width;
                    }
                }

                let dragging_something_else =
                    ui.input().pointer.any_down() || ui.input().pointer.any_pressed();
                let resize_hover = mouse_over_resize_line && !dragging_something_else;

                if resize_hover || is_resizing {
                    ui.output().cursor_icon = egui::CursorIcon::ResizeHorizontal;
                }

                let stroke = if is_resizing {
                    ui.style().visuals.widgets.active.bg_stroke
                } else if resize_hover {
                    ui.style().visuals.widgets.hovered.bg_stroke
                } else {
                    // ui.visuals().widgets.inactive.bg_stroke
                    ui.visuals().widgets.noninteractive.bg_stroke
                };
                ui.painter().line_segment([p0, p1], stroke);

                available_width -= *width + spacing_x;
            }

            ui.data().insert_persisted(resize_id, new_widths);
        }
    }
}

/// The body of a table.
/// Is created by calling `body` on a [`Table`] (after adding a header row) or [`TableBuilder`] (without a header row).
pub struct TableBody<'a> {
    layout: StripLayout<'a>,
    widths: Vec<f32>,
    striped: bool,
    row_nr: usize,
    start_y: f32,
    end_y: f32,
}

impl<'a> TableBody<'a> {
    fn y_progress(&self) -> f32 {
        self.start_y - self.layout.current_y()
    }

    /// Return a vector containing all column widths for this table body.
    ///
    /// This is primarily meant for use with [`TableBody::heterogeneous_rows`] in cases where row
    /// heights are expected to according to the width of one or more cells -- for example, if text
    /// is wrapped rather than clippped within the cell.
    pub fn widths(&self) -> &[f32] {
        &self.widths
    }

    /// Add rows with varying heights.
    ///
    /// This takes a very slight performance hit compared to [`TableBody::rows`] due to the need to
    /// iterate over all row heights in to calculate the virtual table height above and below the
    /// visible region, but it is many orders of magnitude more performant than adding individual
    /// heterogenously-sized rows using [`TableBody::row`] at the cost of the additional complexity
    /// that comes with pre-calculating row heights and representing them as an iterator.
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui_extras::{TableBuilderSize};
    /// TableBuilder::new(ui)
    ///     .column(Size::remainder().at_least(100.0))
    ///     .body(|mut body| {
    ///         let row_heights: Vec<f32> = vec![60.0, 18.0, 31.0, 240.0];
    ///         body.heterogeneous_rows(row_heights.iter(), |row_index, mut row| {
    ///             let thick = row_index % 6 == 0;
    ///             row.col(|ui| {
    ///                 ui.centered_and_justified(|ui| {
    ///                     ui.label(row_index.to_string());
    ///                 });
    ///             });
    ///             row.col(|ui| {
    ///                 ui.centered_and_justified(|ui| {
    ///                     ui.label(clock_emoji(row_index));
    ///                 });
    ///             });
    ///             row.col(|ui| {
    ///                 ui.centered_and_justified(|ui| {
    ///                     ui.style_mut().wrap = Some(false);
    ///                     if thick {
    ///                         ui.heading("Extra thick row");
    ///                     } else {
    ///                         ui.label("Normal row");
    ///                     }
    ///                 });
    ///             });
    ///         });
    ///     });
    /// # });
    /// ```
    pub fn heterogeneous_rows(
        &mut self,
        heights: impl Iterator<Item = f32>,
        mut populate_row: impl FnMut(usize, TableRow<'_, '_>),
    ) {
        // VIRTUAL_EXTENSION represents how far above and below the limits of the visible rectangle
        // that we should consider when determining which rows to actually populate. this provides
        // the illusion of table rows sliding out of view (rather than disappearing abruptly)
        const VIRTUAL_EXTENSION: f32 = 30.0;

        // in order for each row to retain its striped color as the table is scrolled, we need an
        // iterator with the boolean built in based on the enumerated index of the iterator element
        let mut striped_heights = heights
            .enumerate()
            .map(|(index, height)| (index, index % 2 == 0, height));

        let max_height = self.end_y - self.start_y + VIRTUAL_EXTENSION;
        let y_progress = self.y_progress() - VIRTUAL_EXTENSION;

        // cumulative height of all rows above those being displayed
        let mut height_above_visible: f64 = 0.0;
        // cumulative height of all rows below those being displayed
        let mut height_below_visible: f64 = 0.0;

        // calculate height above visible table range
        while let Some((row_index, striped, height)) = striped_heights.next() {
            // when y_progress is greater than height above 0, we need to increment the row index
            // and update the height above visble with the current height then continue
            if height_above_visible >= y_progress as f64 {
                self.add_buffer(height_above_visible as f32);
                let tr = TableRow {
                    layout: &mut self.layout,
                    widths: &self.widths,
                    striped: self.striped && striped,
                    height,
                };
                self.row_nr += 1;
                populate_row(row_index, tr);
                break;
            }
            height_above_visible += height as f64;
        }

        // populate visible rows
        let mut current_height: f64 = 0.0; // used to track height of visible rows
        while let Some((row_index, striped, height)) = striped_heights.next() {
            if current_height > max_height as f64 {
                break;
            }
            let tr = TableRow {
                layout: &mut self.layout,
                widths: &self.widths,
                striped: self.striped && striped,
                height,
            };
            self.row_nr += 1;
            populate_row(row_index, tr);
            current_height += height as f64;
        }

        // calculate height below the visible table range
        while let Some((_, _, height)) = striped_heights.next() {
            height_below_visible += height as f64
        }

        // if height below visible is > 0 here then we need to add a buffer to allow the table to
        // accurately calculate the "virtual" scrollbar position
        if height_below_visible > 0.0 {
            self.add_buffer(height_below_visible as f32);
        }
    }

    /// Add rows with same height.
    ///
    /// Is a lot more performant than adding each individual row as non visible rows must not be rendered
    pub fn rows(mut self, height: f32, rows: usize, mut row: impl FnMut(usize, TableRow<'_, '_>)) {
        let y_progress = self.y_progress();
        let mut start = 0;

        if y_progress > 0.0 {
            start = (y_progress / height).floor() as usize;

            self.add_buffer(y_progress);
        }

        let max_height = self.end_y - self.start_y;
        let count = (max_height / height).ceil() as usize;
        let end = rows.min(start + count);

        for idx in start..end {
            row(
                idx,
                TableRow {
                    layout: &mut self.layout,
                    widths: &self.widths,
                    striped: self.striped && idx % 2 == 0,
                    height,
                },
            );
        }

        if rows - end > 0 {
            let skip_height = (rows - end) as f32 * height;

            self.add_buffer(skip_height);
        }
    }

    /// Add row with individual height
    pub fn row(&mut self, height: f32, row: impl FnOnce(TableRow<'a, '_>)) {
        row(TableRow {
            layout: &mut self.layout,
            widths: &self.widths,
            striped: self.striped && self.row_nr % 2 == 0,
            height,
        });

        self.row_nr += 1;
    }

    // Create a table row buffer of the given height to represent the non-visible portion of the
    // table.
    fn add_buffer(&mut self, height: f32) {
        TableRow {
            layout: &mut self.layout,
            widths: &self.widths,
            striped: false,
            height: height,
        }
        .col(|_| ()); // advances the cursor
    }
}

impl<'a> Drop for TableBody<'a> {
    fn drop(&mut self) {
        self.layout.allocate_rect();
    }
}

/// The row of a table.
/// Is created by [`TableRow`] for each created [`TableBody::row`] or each visible row in rows created by calling [`TableBody::rows`].
pub struct TableRow<'a, 'b> {
    layout: &'b mut StripLayout<'a>,
    widths: &'b [f32],
    striped: bool,
    height: f32,
}

impl<'a, 'b> TableRow<'a, 'b> {
    /// Add the contents of a column.
    pub fn col(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        assert!(
            !self.widths.is_empty(),
            "Tried using more table columns than available."
        );

        let width = self.widths[0];
        self.widths = &self.widths[1..];
        let width = CellSize::Absolute(width);
        let height = CellSize::Absolute(self.height);

        if self.striped {
            self.layout.add_striped(width, height, add_contents)
        } else {
            self.layout.add(width, height, add_contents)
        }
    }
}

impl<'a, 'b> Drop for TableRow<'a, 'b> {
    fn drop(&mut self) {
        self.layout.end_line();
    }
}
