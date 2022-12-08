//! Table view with (optional) fixed header and scrolling body.
//! Cell widths are precalculated with given size hints so we can have tables like this:
//! | fixed size | all available space/minimum | 30% of available width | fixed size |
//! Takes all available height, so if you want something below the table, put it in a strip.

use egui::{Align, NumExt as _, Rect, Response, ScrollArea, Ui, Vec2};

use crate::{
    layout::{CellDirection, CellSize},
    StripLayout,
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

    resizable: Option<bool>,
}

impl Column {
    /// Automatically sized based on content.
    ///
    /// If you have many thousands of rows and are therefore using [`TableBody::rows`]
    /// or [`TableBody::heterogeneous_rows`], then the automatic size will only be based
    /// on the currently visible rows.
    pub fn auto() -> Self {
        Self::auto_with_initial_suggestion(100.0)
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

    /// Take all the space remaining after the other columns have
    /// been sized.
    ///
    /// If you have multiple [`Column::remainder`] they all
    /// share the remaining space equally.
    pub fn remainder() -> Self {
        Self::new(InitialColumnSize::Remainder)
    }

    fn new(initial_width: InitialColumnSize) -> Self {
        Self {
            initial_width,
            width_range: (0.0, f32::INFINITY),
            resizable: None,
            clip: false,
        }
    }

    /// Can this column be resized by dragging the column separator?
    ///
    /// If you don't call this, the fallback value of
    /// [`TableBuilder::resizable`] is used (which by default is `false`).
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = Some(resizable);
        self
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

    fn is_auto(&self) -> bool {
        match self.initial_width {
            InitialColumnSize::Automatic(_) => true,
            InitialColumnSize::Absolute(_) | InitialColumnSize::Remainder => false,
        }
    }
}

fn to_sizing(columns: &[Column]) -> crate::sizing::Sizing {
    use crate::Size;

    let mut sizing = crate::sizing::Sizing::default();
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

struct TableScrollOptions {
    vscroll: bool,
    stick_to_bottom: bool,
    scroll_to_row: Option<(usize, Option<Align>)>,
    scroll_offset_y: Option<f32>,
    min_scrolled_height: f32,
    max_scroll_height: f32,
    auto_shrink: [bool; 2],
}

impl Default for TableScrollOptions {
    fn default() -> Self {
        Self {
            vscroll: true,
            stick_to_bottom: false,
            scroll_to_row: None,
            scroll_offset_y: None,
            min_scrolled_height: 200.0,
            max_scroll_height: 800.0,
            auto_shrink: [true; 2],
        }
    }
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
/// use egui_extras::{TableBuilder, Column};
/// TableBuilder::new(ui)
///     .column(Column::auto().resizable(true))
///     .column(Column::remainder())
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
    striped: bool,
    resizable: bool,
    cell_layout: egui::Layout,
    scroll_options: TableScrollOptions,
}

impl<'a> TableBuilder<'a> {
    pub fn new(ui: &'a mut Ui) -> Self {
        let cell_layout = *ui.layout();
        Self {
            ui,
            columns: Default::default(),
            striped: false,
            resizable: false,
            cell_layout,
            scroll_options: Default::default(),
        }
    }

    /// Enable striped row background for improved readability (default: `false`)
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }

    /// Make the columns resizable by dragging.
    ///
    /// You can set this for individual columns with [`Column::resizable`].
    /// [`Self::resizable`] is used as a fallback for any column for which you don't call
    /// [`Column::resizable`].
    ///
    /// If the _last_ column is [`Column::remainder`], then it won't be resizable
    /// (and instead use up the remainder).
    ///
    /// Default is `false`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Enable vertical scrolling in body (default: `true`)
    pub fn vscroll(mut self, vscroll: bool) -> Self {
        self.scroll_options.vscroll = vscroll;
        self
    }

    #[deprecated = "Renamed to vscroll"]
    pub fn scroll(self, vscroll: bool) -> Self {
        self.vscroll(vscroll)
    }

    /// Should the scroll handle stick to the bottom position even as the content size changes
    /// dynamically? The scroll handle remains stuck until manually changed, and will become stuck
    /// once again when repositioned to the bottom. Default: `false`.
    pub fn stick_to_bottom(mut self, stick: bool) -> Self {
        self.scroll_options.stick_to_bottom = stick;
        self
    }

    /// Set a row to scroll to.
    ///
    /// `align` specifies if the row should be positioned in the top, center, or bottom of the view
    /// (using [`Align::TOP`], [`Align::Center`] or [`Align::BOTTOM`]).
    /// If `align` is `None`, the table will scroll just enough to bring the cursor into view.
    ///
    /// See also: [`Self::vertical_scroll_offset`].
    pub fn scroll_to_row(mut self, row: usize, align: Option<Align>) -> Self {
        self.scroll_options.scroll_to_row = Some((row, align));
        self
    }

    /// Set the vertical scroll offset position, in points.
    ///
    /// See also: [`Self::scroll_to_row`].
    pub fn vertical_scroll_offset(mut self, offset: f32) -> Self {
        self.scroll_options.scroll_offset_y = Some(offset);
        self
    }

    /// The minimum height of a vertical scroll area which requires scroll bars.
    ///
    /// The scroll area will only become smaller than this if the content is smaller than this
    /// (and so we don't require scroll bars).
    ///
    /// Default: `200.0`.
    pub fn min_scrolled_height(mut self, min_scrolled_height: f32) -> Self {
        self.scroll_options.min_scrolled_height = min_scrolled_height;
        self
    }

    /// Don't make the scroll area higher than this (add scroll-bars instead!).
    ///
    /// In other words: add scroll-bars when this height is reached.
    /// Default: `800.0`.
    pub fn max_scroll_height(mut self, max_scroll_height: f32) -> Self {
        self.scroll_options.max_scroll_height = max_scroll_height;
        self
    }

    /// For each axis (x,y):
    /// * If true, add blank space outside the table, keeping the table small.
    /// * If false, add blank space inside the table, expanding the table to fit the containing ui.
    ///
    /// Default: `[true; 2]`.
    ///
    /// See [`ScrollArea::auto_shrink`] for more.
    pub fn auto_shrink(mut self, auto_shrink: [bool; 2]) -> Self {
        self.scroll_options.auto_shrink = auto_shrink;
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
            - if self.scroll_options.vscroll {
                self.ui.spacing().scroll_bar_inner_margin
                    + self.ui.spacing().scroll_bar_width
                    + self.ui.spacing().scroll_bar_outer_margin
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
            striped,
            resizable,
            cell_layout,
            scroll_options,
        } = self;

        let state_id = ui.id().with("__table_state");

        let initial_widths =
            to_sizing(&columns).to_lengths(available_width, ui.spacing().item_spacing.x);
        let mut max_used_widths = vec![0.0; initial_widths.len()];
        let (had_state, state) = TableState::load(ui, initial_widths, state_id);
        let is_first_frame = !had_state;
        let first_frame_auto_size_columns = is_first_frame && columns.iter().any(|c| c.is_auto());

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
            state_id,
            columns,
            available_width,
            state,
            max_used_widths,
            first_frame_auto_size_columns,
            resizable,
            striped,
            cell_layout,
            scroll_options,
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
            striped,
            resizable,
            cell_layout,
            scroll_options,
        } = self;

        let state_id = ui.id().with("__table_state");

        let initial_widths =
            to_sizing(&columns).to_lengths(available_width, ui.spacing().item_spacing.x);
        let max_used_widths = vec![0.0; initial_widths.len()];
        let (had_state, state) = TableState::load(ui, initial_widths, state_id);
        let is_first_frame = !had_state;
        let first_frame_auto_size_columns = is_first_frame && columns.iter().any(|c| c.is_auto());

        let table_top = ui.cursor().top();

        Table {
            ui,
            table_top,
            state_id,
            columns,
            available_width,
            state,
            max_used_widths,
            first_frame_auto_size_columns,
            resizable,
            striped,
            cell_layout,
            scroll_options,
        }
        .body(add_body_contents);
    }
}

// ----------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
struct TableState {
    column_widths: Vec<f32>,
}

impl TableState {
    /// Returns `true` if it did load.
    fn load(ui: &egui::Ui, default_widths: Vec<f32>, state_id: egui::Id) -> (bool, Self) {
        let rect = Rect::from_min_size(ui.available_rect_before_wrap().min, Vec2::ZERO);
        ui.ctx().check_for_id_clash(state_id, rect, "Table");

        if let Some(state) = ui.data().get_persisted::<Self>(state_id) {
            // make sure that the stored widths aren't out-dated
            if state.column_widths.len() == default_widths.len() {
                return (true, state);
            }
        }

        (
            false,
            Self {
                column_widths: default_widths,
            },
        )
    }

    fn store(self, ui: &egui::Ui, state_id: egui::Id) {
        ui.data().insert_persisted(state_id, self);
    }
}

// ----------------------------------------------------------------------------

/// Table struct which can construct a [`TableBody`].
///
/// Is created by [`TableBuilder`] by either calling [`TableBuilder::body`] or after creating a header row with [`TableBuilder::header`].
pub struct Table<'a> {
    ui: &'a mut Ui,
    table_top: f32,
    state_id: egui::Id,
    columns: Vec<Column>,
    available_width: f32,
    state: TableState,
    /// Accumulated maximum used widths for each column.
    max_used_widths: Vec<f32>,
    first_frame_auto_size_columns: bool,
    resizable: bool,
    striped: bool,
    cell_layout: egui::Layout,

    scroll_options: TableScrollOptions,
}

impl<'a> Table<'a> {
    /// Access the contained [`egui::Ui`].
    ///
    /// You can use this to e.g. modify the [`egui::Style`] with [`egui::Ui::style_mut`].
    pub fn ui_mut(&mut self) -> &mut egui::Ui {
        self.ui
    }

    /// Create table body after adding a header row
    pub fn body<F>(self, add_body_contents: F)
    where
        F: for<'b> FnOnce(TableBody<'b>),
    {
        let Table {
            ui,
            table_top,
            state_id,
            columns,
            resizable,
            mut available_width,
            mut state,
            mut max_used_widths,
            first_frame_auto_size_columns,
            striped,
            cell_layout,
            scroll_options,
        } = self;

        let TableScrollOptions {
            vscroll,
            stick_to_bottom,
            scroll_to_row,
            scroll_offset_y,
            min_scrolled_height,
            max_scroll_height,
            auto_shrink,
        } = scroll_options;

        let avail_rect = ui.available_rect_before_wrap();

        let mut scroll_area = ScrollArea::new([false, vscroll])
            .auto_shrink([true; 2])
            .stick_to_bottom(stick_to_bottom)
            .min_scrolled_height(min_scrolled_height)
            .max_height(max_scroll_height)
            .auto_shrink(auto_shrink);

        if let Some(scroll_offset_y) = scroll_offset_y {
            scroll_area = scroll_area.vertical_scroll_offset(scroll_offset_y);
        }

        let columns_ref = &columns;
        let widths_ref = &state.column_widths;
        let max_used_widths_ref = &mut max_used_widths;

        scroll_area.show(ui, move |ui| {
            let mut scroll_to_y_range = None;

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
                    scroll_to_row: scroll_to_row.map(|(r, _)| r),
                    scroll_to_y_range: &mut scroll_to_y_range,
                });

                if scroll_to_row.is_some() && scroll_to_y_range.is_none() {
                    // TableBody::row didn't find the right row, so scroll to the bottom:
                    scroll_to_y_range = Some((f32::INFINITY, f32::INFINITY));
                }
            });

            if let Some((min_y, max_y)) = scroll_to_y_range {
                let x = 0.0; // ignored, we only have vertical scrolling
                let rect = egui::Rect::from_min_max(egui::pos2(x, min_y), egui::pos2(x, max_y));
                let align = scroll_to_row.and_then(|(_, a)| a);
                ui.scroll_to_rect(rect, align);
            }
        });

        let bottom = ui.min_rect().bottom();

        let spacing_x = ui.spacing().item_spacing.x;
        let mut x = avail_rect.left() - spacing_x * 0.5;
        for (i, column_width) in state.column_widths.iter_mut().enumerate() {
            let column = &columns[i];
            let column_is_resizable = column.resizable.unwrap_or(resizable);
            let (min_width, max_width) = column.width_range;

            if !column.clip {
                // Unless we clip we don't want to shrink below the
                // size that was actually used:
                *column_width = column_width.at_least(max_used_widths[i]);
            }
            *column_width = column_width.clamp(min_width, max_width);

            let is_last_column = i + 1 == columns.len();

            if is_last_column && column.initial_width == InitialColumnSize::Remainder {
                // If the last column is 'remainder', then let it fill the remainder!
                let eps = 0.1; // just to avoid some rounding errors.
                *column_width = available_width - eps;
                *column_width = column_width.at_least(max_used_widths[i]);
                *column_width = column_width.clamp(min_width, max_width);
                break;
            }

            x += *column_width + spacing_x;

            if column.is_auto() && (first_frame_auto_size_columns || !column_is_resizable) {
                *column_width = max_used_widths[i];
                *column_width = column_width.clamp(min_width, max_width);
            } else if column_is_resizable {
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
                        if !column.clip {
                            // Unless we clip we don't want to shrink below the
                            // size that was actually used.
                            // However, we still want to allow content that shrinks when you try
                            // to make the column less wide, so we allow some small shrinkage each frame:
                            // big enough to allow shrinking over time, small enough not to look ugly when
                            // shrinking fails. This is a bit of a HACK around immediate mode.
                            let max_shrinkage_per_frame = 8.0;
                            new_width =
                                new_width.at_least(max_used_widths[i] - max_shrinkage_per_frame);
                        }
                        new_width = new_width.clamp(min_width, max_width);

                        let x = x - *column_width + new_width;
                        (p0.x, p1.x) = (x, x);

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

        state.store(ui, state_id);
    }
}

/// The body of a table.
///
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

    /// Look for this row to scroll to.
    scroll_to_row: Option<usize>,

    /// If we find the correct row to scroll to,
    /// this is set to the y-range of the row.
    scroll_to_y_range: &'a mut Option<(f32, f32)>,
}

impl<'a> TableBody<'a> {
    /// Access the contained [`egui::Ui`].
    ///
    /// You can use this to e.g. modify the [`egui::Style`] with [`egui::Ui::style_mut`].
    pub fn ui_mut(&mut self) -> &mut egui::Ui {
        self.layout.ui
    }

    /// Where in screen-space is the table body?
    pub fn max_rect(&self) -> Rect {
        self.layout
            .rect
            .translate(egui::vec2(0.0, self.scroll_offset_y()))
    }

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
        let top_y = self.layout.cursor.y;
        add_row_content(TableRow {
            layout: &mut self.layout,
            columns: self.columns,
            widths: self.widths,
            max_used_widths: self.max_used_widths,
            col_index: 0,
            striped: self.striped && self.row_nr % 2 == 0,
            height,
        });
        let bottom_y = self.layout.cursor.y;

        if Some(self.row_nr) == self.scroll_to_row {
            *self.scroll_to_y_range = Some((top_y, bottom_y));
        }

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
    /// use egui_extras::{TableBuilder, Column};
    /// TableBuilder::new(ui)
    ///     .column(Column::remainder().at_least(100.0))
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

        if let Some(scroll_to_row) = self.scroll_to_row {
            let scroll_to_row = scroll_to_row.at_most(total_rows.saturating_sub(1)) as f32;
            *self.scroll_to_y_range = Some((
                self.layout.cursor.y + scroll_to_row * row_height_with_spacing,
                self.layout.cursor.y + (scroll_to_row + 1.0) * row_height_with_spacing,
            ));
        }

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
    /// use egui_extras::{TableBuilder, Column};
    /// TableBuilder::new(ui)
    ///     .column(Column::remainder().at_least(100.0))
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

        let scroll_to_y_range_offset = self.layout.cursor.y as f64;

        let mut cursor_y: f64 = 0.0;

        // Skip the invisible rows, and populate the first non-virtual row.
        for (row_index, row_height) in &mut enumerated_heights {
            let old_cursor_y = cursor_y;
            cursor_y += (row_height + spacing.y) as f64;

            if Some(row_index) == self.scroll_to_row {
                *self.scroll_to_y_range = Some((
                    (scroll_to_y_range_offset + old_cursor_y) as f32,
                    (scroll_to_y_range_offset + cursor_y) as f32,
                ));
            }

            if cursor_y >= scroll_offset_y {
                // This row is visible:
                self.add_buffer(old_cursor_y as f32); // skip all the invisible rows

                add_row_content(
                    row_index,
                    TableRow {
                        layout: &mut self.layout,
                        columns: self.columns,
                        widths: self.widths,
                        max_used_widths: self.max_used_widths,
                        col_index: 0,
                        striped: self.striped && row_index % 2 == 0,
                        height: row_height,
                    },
                );
                break;
            }
        }

        // populate visible rows:
        for (row_index, row_height) in &mut enumerated_heights {
            let top_y = cursor_y;
            add_row_content(
                row_index,
                TableRow {
                    layout: &mut self.layout,
                    columns: self.columns,
                    widths: self.widths,
                    max_used_widths: self.max_used_widths,
                    col_index: 0,
                    striped: self.striped && row_index % 2 == 0,
                    height: row_height,
                },
            );
            cursor_y += (row_height + spacing.y) as f64;

            if Some(row_index) == self.scroll_to_row {
                *self.scroll_to_y_range = Some((
                    (scroll_to_y_range_offset + top_y) as f32,
                    (scroll_to_y_range_offset + cursor_y) as f32,
                ));
            }

            if cursor_y > scroll_offset_y + max_height as f64 {
                break;
            }
        }

        // calculate height below the visible table range:
        let mut height_below_visible: f64 = 0.0;
        for (row_index, row_height) in enumerated_heights {
            height_below_visible += (row_height + spacing.y) as f64;

            let top_y = cursor_y;
            cursor_y += (row_height + spacing.y) as f64;
            if Some(row_index) == self.scroll_to_row {
                *self.scroll_to_y_range = Some((
                    (scroll_to_y_range_offset + top_y) as f32,
                    (scroll_to_y_range_offset + cursor_y) as f32,
                ));
            }
        }

        if self.scroll_to_row.is_some() && self.scroll_to_y_range.is_none() {
            // Catch desire to scroll past the end:
            *self.scroll_to_y_range = Some((
                (scroll_to_y_range_offset + cursor_y) as f32,
                (scroll_to_y_range_offset + cursor_y) as f32,
            ));
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
