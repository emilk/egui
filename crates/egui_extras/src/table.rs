//! Table view with (optional) fixed header and scrolling body.
//! Cell widths are precalculated with given size hints so we can have tables like this:
//! | fixed size | all available space/minimum | 30% of available width | fixed size |
//! Takes all available height, so if you want something below the table, put it in a strip.

use egui::{NumExt as _, Rect, Response, Ui, Vec2};

use crate::{
    layout::{CellDirection, CellSize},
    sizing::Sizing,
    Size, StripLayout,
};

// -----------------------------------------------------------------=----------

#[derive(Clone, Copy, Debug, PartialEq)]
enum InitialColumnSize {
    /// Absolute size in points
    Absolute(f32),

    /// Base on content
    Automatic(f32),

    /// Take all available space
    Remainder,
}

/// Specifies the properties of a column, like its width range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Column {
    initial_width: InitialColumnSize,
    width_range: (f32, f32),
    /// Clip contents if too narrow?
    clip: bool,
}

impl Column {
    /// Automatically sized.
    pub fn auto() -> Self {
        Self::auto_with_initial_suggestion(100.)
    }

    /// Automatically sized.
    ///
    /// The given fallback is a loose suggestion, that may be used to wrap
    /// cell contents, if they contain a wrapping layout.
    /// In most cases though, the given value is ignored.
    pub fn auto_with_initial_suggestion(suggested_width: f32) -> Self {
        Self::new(InitialColumnSize::Automatic(suggested_width))
    }

    /// With this initial width.
    pub fn initial(width: f32) -> Self {
        Self::new(InitialColumnSize::Absolute(width))
    }

    /// Always this exact width, never shrink or grow.
    pub fn exact(width: f32) -> Self {
        Self::new(InitialColumnSize::Absolute(width))
            .range(width..=width)
            .clip(true)
    }

    pub fn remainder() -> Self {
        Self::new(InitialColumnSize::Remainder)
    }

    fn new(initial_width: InitialColumnSize) -> Self {
        Self {
            initial_width,
            width_range: (0.0, f32::INFINITY),
            clip: false,
        }
    }

    /// If `true`: Allow the column to shrink enough to clip the contents.
    /// If `false`: The column will always be wide enough to contain all its content.
    ///
    /// Clipping can make sense if you expect a column to contain a lot of things,
    /// and you don't want it too take up too much space.
    /// If you turn on clipping you should also consider calling [`Self::at_least`].
    ///
    /// Default: `false`.
    pub fn clip(mut self, clip: bool) -> Self {
        self.clip = clip;
        self
    }

    /// Won't shrink below this width (in points).
    ///
    /// Default: 0.0
    pub fn at_least(mut self, minimum: f32) -> Self {
        self.width_range.0 = minimum;
        self
    }

    /// Won't grow above this width (in points).
    ///
    /// Default: [`f32::INFINITY`]
    pub fn at_most(mut self, maximum: f32) -> Self {
        self.width_range.1 = maximum;
        self
    }

    /// Allowed range of movement (in points), if in a resizable [`Table`](crate::table::Table).
    pub fn range(mut self, range: std::ops::RangeInclusive<f32>) -> Self {
        self.width_range = (*range.start(), *range.end());
        self
    }
}

fn to_sizing(columns: &[Column]) -> Sizing {
    let mut sizing = Sizing::default();
    for column in columns {
        let size = match column.initial_width {
            InitialColumnSize::Absolute(width) => Size::exact(width),
            InitialColumnSize::Automatic(suggested_width) => Size::initial(suggested_width),
            InitialColumnSize::Remainder => Size::remainder(),
        }
        .at_least(column.width_range.0)
        .at_most(column.width_range.1);
        sizing.add(size);
    }
    sizing
}

// -----------------------------------------------------------------=----------

/// Builder for a [`Table`] with (optional) fixed header and scrolling body.
///
/// You must pre-allocate all columns with [`Self::column`]/[`Self::columns`].
///
/// If you have multiple [`Table`]:s in the same [`Ui`]
/// you will need to give them unique id:s by surrounding them with [`Ui::push_id`].
///
/// ### Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// use egui_extras::{TableBuilder, Size};
/// TableBuilder::new(ui)
///     .column(Column::auto())
///     .column(Column::remainder())
///     .resizable(true)
///     .auto_size_columns(true)
///     .header(20.0, |mut header| {
///         header.col(|ui| {
///             ui.heading("First column");
///         });
///         header.col(|ui| {
///             ui.heading("Second column");
///         });
///     })
///     .body(|mut body| {
///         body.row(30.0, |mut row| {
///             row.col(|ui| {
///                 ui.label("Hello");
///             });
///             row.col(|ui| {
///                 ui.button("world!");
///             });
///         });
///     });
/// # });
/// ```
pub struct TableBuilder<'a> {
    ui: &'a mut Ui,
    columns: Vec<Column>,
    auto_size_columns: bool,
    scroll: bool,
    striped: bool,
    resizable: bool,
    stick_to_bottom: bool,
    scroll_offset_y: Option<f32>,
    cell_layout: egui::Layout,
}

impl<'a> TableBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        let cell_layout = *ui.layout();
        Self {
            ui,
            columns: Default::default(),
            auto_size_columns: true,
            scroll: true,
            striped: false,
            resizable: false,
            stick_to_bottom: false,
            scroll_offset_y: None,
            cell_layout,
        }
    }

    /// Enable scrollview in body (default: true)
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    /// Enable striped row background for improved readability (default: false)
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Make the columns resizable by dragging.
    ///
    /// If the _last_ column is [`Column::remainder`], then it won't be resizable
    /// (and instead use up the remainder).
    ///
    /// Default is `false`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Automatically chose a size of the columns on the first frame
    /// based on their actual sized.
    ///
    /// [`Sizing::range`] is respected.
    ///
    /// Only works with [`Self::resizable`] set to `true`.
    pub fn auto_size_columns(mut self, auto_size_columns: bool) -> Self {
        self.auto_size_columns = auto_size_columns;
        self
    }

    /// Should the scroll handle stick to the bottom position even as the content size changes
    /// dynamically? The scroll handle remains stuck until manually changed, and will become stuck
    /// once again when repositioned to the bottom. Default: `false`.
    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.stick_to_bottom = stick;
        self
    }

    /// Set the vertical scroll offset position.
    pub fn vertical_scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_offset_y = Some(offset);
        self
    }

    /// What layout should we use for the individual cells?
    pub fn cell_layout(mut self, cell_layout: egui::Layout) -> Self {
        self.cell_layout = cell_layout;
        self
    }

    /// Allocate space for one column.
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }

    /// Allocate space for several columns at once.
    pub fn columns(mut self, column: Column, count: usize) -> Self {
        for _ in 0..count {
            self.columns.push(column);
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
    pub fn header(self, height: f32, add_header_row: impl FnOnce(TableRow<'_, '_>)) -> Table<'a> {
        let available_width = self.available_width();

        let Self {
            ui,
            columns,
            auto_size_columns,
            scroll,
            striped,
            resizable,
            stick_to_bottom,
            scroll_offset_y,
            cell_layout,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));

        let initial_widths =
            to_sizing(&columns).to_lengths(available_width, ui.spacing().item_spacing.x);
        let mut max_used_widths = vec![0.0; initial_widths.len()];
        let (had_state, state) = TableReizeState::load(ui, initial_widths, resize_id);
        let first_frame_auto_size_columns = auto_size_columns && resize_id.is_some() && !had_state;

        let table_top = ui.cursor().top();

        // Hide first-frame-jitters when auto-sizing.
        ui.add_visible_ui(!first_frame_auto_size_columns, |ui| {
            let mut layout = StripLayout::new(ui, CellDirection::Horizontal, cell_layout);
            add_header_row(TableRow {
                layout: &mut layout,
                columns: &columns,
                widths: &state.column_widths,
                max_used_widths: &mut max_used_widths,
                col_index: 0,
                striped: false,
                height,
            });
            layout.allocate_rect();
        });

        Table {
            ui,
            table_top,
            resize_id,
            columns,
            available_width,
            widths: state.column_widths,
            max_used_widths,
            first_frame_auto_size_columns,
            scroll,
            striped,
            stick_to_bottom,
            scroll_offset_y,
            cell_layout,
        }
    }

    /// Create table body without a header row
    pub fn body<F>(self, add_body_contents: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let available_width = self.available_width();

        let Self {
            ui,
            columns,
            auto_size_columns,
            scroll,
            striped,
            resizable,
            stick_to_bottom,
            scroll_offset_y,
            cell_layout,
        } = self;

        let resize_id = resizable.then(|| ui.id().with("__table_resize"));

        let initial_widths =
            to_sizing(&columns).to_lengths(available_width, ui.spacing().item_spacing.x);
        let max_used_widths = vec![0.0; initial_widths.len()];
        let (had_state, state) = TableReizeState::load(ui, initial_widths, resize_id);
        let first_frame_auto_size_columns = auto_size_columns && resize_id.is_some() && !had_state;

        let table_top = ui.cursor().top();

        Table {
            ui,
            table_top,
            resize_id,
            columns,
            available_width,
            widths: state.column_widths,
            max_used_widths,
            first_frame_auto_size_columns,
            scroll,
            striped,
            stick_to_bottom,
            scroll_offset_y,
            cell_layout,
        }
        .body(add_body_contents);
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct TableReizeState {
    column_widths: Vec<f32>,
}

impl TableReizeState {
    /// Returns `true` if it did load.
    fn load(ui: &egui::Ui, default_widths: Vec<f32>, resize_id: Option<egui::Id>) -> (bool, Self) {
        if let Some(resize_id) = resize_id {
            let rect = Rect::from_min_size(ui.available_rect_before_wrap().min, Vec2::ZERO);
            ui.ctx().check_for_id_clash(resize_id, rect, "Table");

            if let Some(state) = ui.data().get_persisted::<Self>(resize_id) {
                // make sure that the stored widths aren't out-dated
                if state.column_widths.len() == default_widths.len() {
                    return (true, state);
                }
            }
        }

        (
            false,
            Self {
                column_widths: default_widths,
            },
        )
    }

    fn store(self, ui: &egui::Ui, resize_id: egui::Id) {
        ui.data().insert_persisted(resize_id, self);
    }
}

// ----------------------------------------------------------------------------

/// Table struct which can construct a [`TableBody`].
///
/// Is created by [`TableBuilder`] by either calling [`TableBuilder::body`] or after creating a header row with [`TableBuilder::header`].
pub struct Table<'a> {
    ui: &'a mut Ui,
    table_top: f32,
    resize_id: Option<egui::Id>,
    columns: Vec<Column>,
    available_width: f32,
    /// Current column widths.
    widths: Vec<f32>,
    /// Accumulated maximum used widths for each column.
    max_used_widths: Vec<f32>,
    first_frame_auto_size_columns: bool,
    scroll: bool,
    striped: bool,
    stick_to_bottom: bool,
    scroll_offset_y: Option<f32>,
    cell_layout: egui::Layout,
}

impl<'a> Table<'a> {
    /// Create table body after adding a header row
    pub fn body<F>(self, add_body_contents: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let Table {
            ui,
            table_top,
            resize_id,
            columns,
            mut available_width,
            widths,
            mut max_used_widths,
            first_frame_auto_size_columns,
            scroll,
            striped,
            stick_to_bottom,
            scroll_offset_y,
            cell_layout,
        } = self;

        let avail_rect = ui.available_rect_before_wrap();

        let mut new_widths = widths.clone();

        let mut scroll_area = egui::ScrollArea::new([false, scroll])
            .auto_shrink([true; 2])
            .stick_to_bottom(stick_to_bottom);

        if let Some(scroll_offset_y) = scroll_offset_y {
            scroll_area = scroll_area.vertical_scroll_offset(scroll_offset_y);
        }

        let columns_ref = &&columns;
        let widths_ref = &widths;
        let max_used_widths_ref = &mut max_used_widths;

        scroll_area.show(ui, move |ui| {
            // Hide first-frame-jitters when auto-sizing.
            ui.add_visible_ui(!first_frame_auto_size_columns, |ui| {
                let layout = StripLayout::new(ui, CellDirection::Horizontal, cell_layout);

                add_body_contents(TableBody {
                    layout,
                    columns: columns_ref,
                    widths: widths_ref,
                    max_used_widths: max_used_widths_ref,
                    striped,
                    row_nr: 0,
                    start_y: avail_rect.top(),
                    end_y: avail_rect.bottom(),
                });
            });
        });

        let bottom = ui.min_rect().bottom();

        if let Some(resize_id) = resize_id {
            let spacing_x = ui.spacing().item_spacing.x;
            let mut x = avail_rect.left() - spacing_x * 0.5;
            for (i, column_width) in new_widths.iter_mut().enumerate() {
                let column = &columns[i];
                let (min_width, max_width) = column.width_range;
                *column_width = column_width.clamp(min_width, max_width);

                x += *column_width + spacing_x;

                // If the last column is Size::Remainder, then let it fill the remainder!
                let is_last_column = i + 1 == columns.len();
                if is_last_column && column.initial_width == InitialColumnSize::Remainder {
                    let eps = 0.1; // just to avoid some rounding errors.
                    *column_width = available_width - eps;
                    *column_width = column_width.at_least(max_used_widths[i]);
                    *column_width = column_width.clamp(min_width, max_width);
                    break;
                }

                if first_frame_auto_size_columns {
                    *column_width = max_used_widths[i];
                    *column_width = column_width.clamp(min_width, max_width);
                } else {
                    let column_resize_id = ui.id().with("resize_column").with(i);

                    let mut p0 = egui::pos2(x, table_top);
                    let mut p1 = egui::pos2(x, bottom);
                    let line_rect = egui::Rect::from_min_max(p0, p1)
                        .expand(ui.style().interaction.resize_grab_radius_side);

                    let resize_response =
                        ui.interact(line_rect, column_resize_id, egui::Sense::click_and_drag());

                    if resize_response.double_clicked() {
                        // Resize to the minimum of what is needed.

                        *column_width = max_used_widths[i].clamp(min_width, max_width);
                    } else if resize_response.dragged() {
                        if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                            let mut new_width = *column_width + pointer.x - x;
                            if !column.clip || is_last_column {
                                // If we don't clip, we don't want to shrink below the
                                // size that was actually used.
                                new_width = new_width.at_least(max_used_widths[i]);
                            }
                            new_width = new_width.clamp(min_width, max_width);

                            let x = x - *column_width + new_width;
                            p0.x = x;
                            p1.x = x;

                            *column_width = new_width;
                        }
                    }

                    let dragging_something_else = {
                        let pointer = &ui.input().pointer;
                        pointer.any_down() || pointer.any_pressed()
                    };
                    let resize_hover = resize_response.hovered() && !dragging_something_else;

                    if resize_hover || resize_response.dragged() {
                        ui.output().cursor_icon = egui::CursorIcon::ResizeColumn;
                    }

                    let stroke = if resize_response.dragged() {
                        ui.style().visuals.widgets.active.bg_stroke
                    } else if resize_hover {
                        ui.style().visuals.widgets.hovered.bg_stroke
                    } else {
                        // ui.visuals().widgets.inactive.bg_stroke
                        ui.visuals().widgets.noninteractive.bg_stroke
                    };

                    ui.painter().line_segment([p0, p1], stroke);
                };

                available_width -= *column_width + spacing_x;
            }

            let state = TableReizeState {
                column_widths: new_widths,
            };
            state.store(ui, resize_id);
        }
    }
}

/// The body of a table.
/// Is created by calling `body` on a [`Table`] (after adding a header row) or [`TableBuilder`] (without a header row).
pub struct TableBody<'a> {
    layout: StripLayout<'a>,

    columns: &'a [Column],

    /// Current column widths.
    widths: &'a [f32],

    /// Accumulated maximum used widths for each column.
    max_used_widths: &'a mut [f32],

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
        self.widths
    }

    /// Add a single row with the given height.
    ///
    /// If you have many thousands of row it can be more performant to instead use [`Self::rows`] or [`Self::heterogeneous_rows`].
    pub fn row(&mut self, height: f32, add_row_content: impl FnOnce(TableRow<'a, '_>)) {
        add_row_content(TableRow {
            layout: &mut self.layout,
            columns: self.columns,
            widths: self.widths,
            max_used_widths: self.max_used_widths,
            col_index: 0,
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
        mut add_row_content: impl FnMut(usize, TableRow<'_, '_>),
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
            add_row_content(
                idx,
                TableRow {
                    layout: &mut self.layout,
                    columns: self.columns,
                    widths: self.widths,
                    max_used_widths: self.max_used_widths,
                    col_index: 0,
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
    /// heterogeneously-sized rows using [`TableBody::row`] at the cost of the additional complexity
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
        mut add_row_content: impl FnMut(usize, TableRow<'_, '_>),
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
                    columns: self.columns,
                    widths: self.widths,
                    max_used_widths: self.max_used_widths,
                    col_index: 0,
                    striped: self.striped && row_index % 2 == 0,
                    height: row_height,
                };
                add_row_content(row_index, tr);
                break;
            }
        }

        // populate visible rows:
        for (row_index, row_height) in &mut enumerated_heights {
            let tr = TableRow {
                layout: &mut self.layout,
                columns: self.columns,
                widths: self.widths,
                max_used_widths: self.max_used_widths,
                col_index: 0,
                striped: self.striped && row_index % 2 == 0,
                height: row_height,
            };
            add_row_content(row_index, tr);
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
    columns: &'b [Column],
    widths: &'b [f32],
    /// grows during building with the maximum widths
    max_used_widths: &'b mut [f32],
    col_index: usize,
    striped: bool,
    height: f32,
}

impl<'a, 'b> TableRow<'a, 'b> {
    /// Add the contents of a column.
    ///
    /// Return the used space (`min_rect`) plus the [`Response`] of the whole cell.
    pub fn col(&mut self, add_cell_contents: impl FnOnce(&mut Ui)) -> (Rect, Response) {
        let col_index = self.col_index;

        let clip = self.columns.get(col_index).map_or(false, |c| c.clip);

        let width = if let Some(width) = self.widths.get(col_index) {
            self.col_index += 1;
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

        let (used_rect, response) =
            self.layout
                .add(clip, self.striped, width, height, add_cell_contents);

        if let Some(max_w) = self.max_used_widths.get_mut(col_index) {
            *max_w = max_w.max(used_rect.width());
        }

        (used_rect, response)
    }
}

impl<'a, 'b> Drop for TableRow<'a, 'b> {
    fn drop(&mut self) {
        self.layout.end_line();
    }
}
