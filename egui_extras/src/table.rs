//! Table view with (optional) fixed header and scrolling body.
//! Cell widths are precalculated with given size hints so we can have tables like this:
//! | fixed size | all available space/minimum | 30% of available width | fixed size |
//! Takes all available height, so if you want something below the table, put it in a strip.

use crate::{
    layout::{CellDirection, CellSize},
    sizing::Sizing,
    Size, StripLayout,
};

use egui::{Rect, Response, Ui, Vec2};

/// Builder for a [`Table`] with (optional) fixed header and scrolling body.
///
/// Cell widths are precalculated so we can have tables like this:
///
/// | fixed size | all available space/minimum | 30% of available width | fixed size |
///
/// In contrast to normal egui behavior, columns/rows do *not* grow with its children!
/// Takes all available height, so if you want something below the table, put it in a strip.
///
/// You must pre-allocate all columns with [`Self::column`]/[`Self::columns`].
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
    cell_layout: egui::Layout,
}

impl<'a> TableBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        let cell_layout = *ui.layout();
        Self {
            ui,
            sizing: Default::default(),
            scroll: true,
            striped: false,
            resizable: false,
            clip: true,
            cell_layout,
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

    /// What layout should we use for the individual cells?
    pub fn cell_layout(mut self, cell_layout: egui::Layout) -> Self {
        self.cell_layout = cell_layout;
        self
    }

    /// Allocate space for one column.
    pub fn column(mut self, width: Size) -> Self {
        self.sizing.add(width);
        self
    }

    /// Allocate space for several columns at once.
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
            cell_layout,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));

        let default_widths = sizing.to_lengths(available_width, ui.spacing().item_spacing.x);
        let widths = read_persisted_widths(ui, default_widths, resize_id);

        let table_top = ui.cursor().top();

        {
            let mut layout = StripLayout::new(ui, CellDirection::Horizontal, clip, cell_layout);
            header(TableRow {
                layout: &mut layout,
                widths: &widths,
                width_index: 0,
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
            cell_layout,
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
            cell_layout,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));

        let default_widths = sizing.to_lengths(available_width, ui.spacing().item_spacing.x);
        let widths = read_persisted_widths(ui, default_widths, resize_id);

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
            cell_layout,
        }
        .body(body);
    }
}

fn read_persisted_widths(
    ui: &egui::Ui,
    default_widths: Vec<f32>,
    resize_id: Option<egui::Id>,
) -> Vec<f32> {
    if let Some(resize_id) = resize_id {
        let rect = Rect::from_min_size(ui.available_rect_before_wrap().min, Vec2::ZERO);
        ui.ctx().check_for_id_clash(resize_id, rect, "Table");
        if let Some(persisted) = ui.data().get_persisted::<Vec<f32>>(resize_id) {
            // make sure that the stored widths aren't out-dated
            if persisted.len() == default_widths.len() {
                return persisted;
            }
        }
    }

    default_widths
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
    cell_layout: egui::Layout,
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
            cell_layout,
        } = self;

        let avail_rect = ui.available_rect_before_wrap();

        let mut new_widths = widths.clone();

        egui::ScrollArea::new([false, scroll])
            .auto_shrink([true; 2])
            .show(ui, move |ui| {
                let layout = StripLayout::new(ui, CellDirection::Horizontal, clip, cell_layout);

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

        // TODO(emilk): fix frame-delay by interacting before laying out (but painting later).
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
                    ui.output().cursor_icon = egui::CursorIcon::ResizeColumn;
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
    fn scroll_offset_y(&self) -> f32 {
        self.start_y - self.layout.rect.top()
    }

    /// Return a vector containing all column widths for this table body.
    ///
    /// This is primarily meant for use with [`TableBody::heterogeneous_rows`] in cases where row
    /// heights are expected to according to the width of one or more cells -- for example, if text
    /// is wrapped rather than clipped within the cell.
    pub fn widths(&self) -> &[f32] {
        &self.widths
    }

    /// Add a single row with the given height.
    ///
    /// If you have many thousands of row it can be more performant to instead use [`Self::rows`] or [`Self::heterogeneous_rows`].
    pub fn row(&mut self, height: f32, row: impl FnOnce(TableRow<'a, '_>)) {
        row(TableRow {
            layout: &mut self.layout,
            widths: &self.widths,
            width_index: 0,
            striped: self.striped && self.row_nr % 2 == 0,
            height,
        });

        self.row_nr += 1;
    }

    /// Add many rows with same height.
    ///
    /// Is a lot more performant than adding each individual row as non visible rows must not be rendered.
    ///
    /// If you need many rows with different heights, use [`Self::heterogeneous_rows`] instead.
    ///
    /// ### Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// use egui_extras::{TableBuilder, Size};
    /// TableBuilder::new(ui)
    ///     .column(Size::remainder().at_least(100.0))
    ///     .body(|mut body| {
    ///         let row_height = 18.0;
    ///         let num_rows = 10_000;
    ///         body.rows(row_height, num_rows, |row_index, mut row| {
    ///             row.col(|ui| {
    ///                 ui.label("First column");
    ///             });
    ///         });
    ///     });
    /// # });
    /// ```
    pub fn rows(
        mut self,
        row_height_sans_spacing: f32,
        total_rows: usize,
        mut row: impl FnMut(usize, TableRow<'_, '_>),
    ) {
        let spacing = self.layout.ui.spacing().item_spacing;
        let row_height_with_spacing = row_height_sans_spacing + spacing.y;

        let scroll_offset_y = self
            .scroll_offset_y()
            .min(total_rows as f32 * row_height_with_spacing);
        let max_height = self.end_y - self.start_y;
        let mut min_row = 0;

        if scroll_offset_y > 0.0 {
            min_row = (scroll_offset_y / row_height_with_spacing).floor() as usize;
            self.add_buffer(min_row as f32 * row_height_with_spacing);
        }

        let max_row =
            ((scroll_offset_y + max_height) / row_height_with_spacing).ceil() as usize + 1;
        let max_row = max_row.min(total_rows);

        for idx in min_row..max_row {
            row(
                idx,
                TableRow {
                    layout: &mut self.layout,
                    widths: &self.widths,
                    width_index: 0,
                    striped: self.striped && idx % 2 == 0,
                    height: row_height_sans_spacing,
                },
            );
        }

        if total_rows - max_row > 0 {
            let skip_height = (total_rows - max_row) as f32 * row_height_with_spacing;
            self.add_buffer(skip_height - spacing.y);
        }
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
    /// use egui_extras::{TableBuilder, Size};
    /// TableBuilder::new(ui)
    ///     .column(Size::remainder().at_least(100.0))
    ///     .body(|mut body| {
    ///         let row_heights: Vec<f32> = vec![60.0, 18.0, 31.0, 240.0];
    ///         body.heterogeneous_rows(row_heights.into_iter(), |row_index, mut row| {
    ///             let thick = row_index % 6 == 0;
    ///             row.col(|ui| {
    ///                 ui.centered_and_justified(|ui| {
    ///                     ui.label(row_index.to_string());
    ///                 });
    ///             });
    ///         });
    ///     });
    /// # });
    /// ```
    pub fn heterogeneous_rows(
        mut self,
        heights: impl Iterator<Item = f32>,
        mut populate_row: impl FnMut(usize, TableRow<'_, '_>),
    ) {
        let spacing = self.layout.ui.spacing().item_spacing;
        let mut enumerated_heights = heights.enumerate();

        let max_height = self.end_y - self.start_y;
        let scroll_offset_y = self.scroll_offset_y() as f64;

        let mut cursor_y: f64 = 0.0;

        // Skip the invisible rows, and populate the first non-virtual row.
        for (row_index, row_height) in &mut enumerated_heights {
            let old_cursor_y = cursor_y;
            cursor_y += (row_height + spacing.y) as f64;
            if cursor_y >= scroll_offset_y {
                // This row is visible:
                self.add_buffer(old_cursor_y as f32);
                let tr = TableRow {
                    layout: &mut self.layout,
                    widths: &self.widths,
                    width_index: 0,
                    striped: self.striped && row_index % 2 == 0,
                    height: row_height,
                };
                populate_row(row_index, tr);
                break;
            }
        }

        // populate visible rows:
        for (row_index, row_height) in &mut enumerated_heights {
            let tr = TableRow {
                layout: &mut self.layout,
                widths: &self.widths,
                width_index: 0,
                striped: self.striped && row_index % 2 == 0,
                height: row_height,
            };
            populate_row(row_index, tr);
            cursor_y += (row_height + spacing.y) as f64;

            if cursor_y > scroll_offset_y + max_height as f64 {
                break;
            }
        }

        // calculate height below the visible table range:
        let mut height_below_visible: f64 = 0.0;
        for (_, height) in enumerated_heights {
            height_below_visible += height as f64;
        }
        if height_below_visible > 0.0 {
            // we need to add a buffer to allow the table to
            // accurately calculate the scrollbar position
            self.add_buffer(height_below_visible as f32);
        }
    }

    // Create a table row buffer of the given height to represent the non-visible portion of the
    // table.
    fn add_buffer(&mut self, height: f32) {
        self.layout.skip_space(egui::vec2(0.0, height));
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
    width_index: usize,
    striped: bool,
    height: f32,
}

impl<'a, 'b> TableRow<'a, 'b> {
    /// Add the contents of a column.
    pub fn col(&mut self, add_contents: impl FnOnce(&mut Ui)) -> Response {
        let width = if let Some(width) = self.widths.get(self.width_index) {
            self.width_index += 1;
            *width
        } else {
            crate::log_or_panic!(
                "Added more `Table` columns than were pre-allocated ({} pre-allocated)",
                self.widths.len()
            );
            8.0 // anything will look wrong, so pick something that is obviously wrong
        };

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
